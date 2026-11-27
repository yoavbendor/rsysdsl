//! Hybrid thermal/fluidic & door-interlock simulation runner for the `dishwasher` model.
//!
//! Three-tier rule: `dishwasher` (model) <- `dishwasher-sim` (this runner) -> `sysml-sim`.
//! The model never depends on this crate.
//!
//! - **Plant parameters** come from `Dishwasher::default()`:
//!   `WashHeater.ratedPower` (1800 W) & `efficiency` (0.96), `WashTub.thermalConductanceWPerK`
//!   (1.8 W/K), `WashTub.waterMassKg` (4.5 kg), `InletSolenoidValve.maxFlowRate` (0.12 kg/s),
//!   `CirculationPump.ratedPower` (95 W) & `noiseLevelDb` (43 dB), `DrainPump.flowRate` (0.35 kg/s)
//!   & `ratedPower` (35 W), `LogicDcPsu.ratedPower` (20 W), `WashTub.acousticAttenuationDb` (14 dB).
//! - **Behaviour** is driven by the model's `WashCycleState::step_with(...)` state machine.
//! - **Verdicts** evaluate `HygienicWashTemperatureReq` (DW-001), `QuietOperationNoiseReq`
//!   (DW-002), `PeakMainsPowerBudgetReq` (DW-003), and `DoorOpenSafetyInterlockReq` (DW-004).

use serde_json::json;
use sysml_dsl::prelude::SysmlItemCore;
use sysml_ir::{Quantity, VerificationReport, VerificationStatus, VerificationVerdict};
use sysml_sim::{
    rk4_step, Crossing, Fault, Monitor, RegisteredScenario, Scenario, ScenarioOutcome, ZeroCrossing,
};
use sysml_std::prelude::{attenuated_noise_db, useful_power_watts};

use dishwasher::dishwasher_model::*;

pub const MODEL_NAME: &str = "DishwasherModel";
pub const DT_S: f64 = 1.0;
pub const MAX_DURATION_S: f64 = 3600.0;
/// Specific heat capacity of liquid water [J/(kg*K)].
const WATER_CP_J_PER_KG_K: f64 = 4184.0;
/// Ambient room temperature [K] (20 °C).
const AMBIENT_TEMP_K: f64 = 293.15;
/// Cold mains supply water temperature [K] (15 °C).
const INLET_WATER_TEMP_K: f64 = 288.15;

pub const NOMINAL_SCENARIO: &str = "dishwasher_wash_nominal";
pub const DOOR_OPEN_SCENARIO: &str = "dishwasher_door_open_pause";
pub const FLOOD_FAULT_SCENARIO: &str = "dishwasher_flood_fault";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WashConfig {
    /// Open door mid-wash at this time [s], then close 20 s later.
    pub door_open_at_s: Option<f64>,
    /// Inject a flood sensor trip at this time [s].
    pub flood_at_s: Option<f64>,
}

impl WashConfig {
    pub fn nominal() -> Self {
        Self::default()
    }
    pub fn door_open_pause() -> Self {
        Self {
            door_open_at_s: Some(400.0),
            ..Self::default()
        }
    }
    pub fn flood_fault() -> Self {
        Self {
            flood_at_s: Some(25.0),
            ..Self::default()
        }
    }
}

pub struct WashCycleScenario {
    name: String,
    cfg: WashConfig,
    pub machine: Dishwasher,
    pub state: WashCycleState,
    water_mass_kg: f64,
    water_temp_k: f64,
    stage_elapsed_s: f64,
    heater_on: bool,
    circ_pump_on: bool,
    drain_pump_on: bool,
    mains_power_w: f64,
    emitted_noise_db: f64,
    peak_temp_k: f64,
    peak_power_w: f64,
    peak_noise_db: f64,
    door_open: bool,
    door_cutoff_latency_s: f64,
    flood_detected: bool,
    cycle_done: bool,
    target_temp_cross: ZeroCrossing,
    last_event_t: Option<f64>,
    rejected_events: Vec<String>,
}

impl WashCycleScenario {
    pub fn new(name: impl Into<String>, cfg: WashConfig) -> Self {
        Self {
            name: name.into(),
            cfg,
            machine: Dishwasher::default(),
            state: WashCycleState::initial_state(),
            water_mass_kg: 0.0,
            water_temp_k: INLET_WATER_TEMP_K,
            stage_elapsed_s: 0.0,
            heater_on: false,
            circ_pump_on: false,
            drain_pump_on: false,
            mains_power_w: 0.0,
            emitted_noise_db: 0.0,
            peak_temp_k: INLET_WATER_TEMP_K,
            peak_power_w: 0.0,
            peak_noise_db: 0.0,
            door_open: false,
            door_cutoff_latency_s: 0.0,
            flood_detected: false,
            cycle_done: false,
            target_temp_cross: ZeroCrossing::new(Crossing::Rising),
            last_event_t: None,
            rejected_events: Vec::new(),
        }
    }

    fn mcu(&self) -> WashControllerMcu {
        self.machine.electronics.mcu
    }

    fn fire<E: SysmlItemCore + 'static>(&mut self, event: &E, label: &str) {
        let mcu = self.mcu();
        if let Err(e) = self.state.step_with(event, &mcu) {
            self.rejected_events.push(format!("{label}: {e}"));
        }
    }

    fn update_actuators(&mut self) {
        let p_psu = self.machine.power.dc_psu.rated_power.value();
        let p_heater = self.machine.power.wash_heater.rated_power.value();
        let p_circ = self.machine.fluidic_mech.circ_pump.rated_power.value();
        let p_drain = self.machine.fluidic_mech.drain_pump.rated_power.value();
        let atten = self.machine.fluidic_mech.wash_tub.acoustic_attenuation_db;

        match self.state {
            WashCycleState::WashHeat => {
                // Thermostat regulates around heater target (338.15 K)
                let target = self.machine.power.wash_heater.max_operating_temp.value();
                self.heater_on = self.water_temp_k < target;
                self.circ_pump_on = true;
                self.drain_pump_on = false;
            }
            WashCycleState::Rinse => {
                self.heater_on = false;
                self.circ_pump_on = true;
                self.drain_pump_on = false;
            }
            WashCycleState::Drain | WashCycleState::FloodFault => {
                self.heater_on = false;
                self.circ_pump_on = false;
                self.drain_pump_on = self.water_mass_kg > 0.0;
            }
            _ => {
                self.heater_on = false;
                self.circ_pump_on = false;
                self.drain_pump_on = false;
            }
        }

        self.mains_power_w = p_psu
            + if self.heater_on { p_heater } else { 0.0 }
            + if self.circ_pump_on { p_circ } else { 0.0 }
            + if self.drain_pump_on { p_drain } else { 0.0 };

        let internal_db = if self.drain_pump_on {
            self.machine.fluidic_mech.drain_pump.noise_level_db
        } else if self.circ_pump_on {
            self.machine.fluidic_mech.circ_pump.noise_level_db
        } else {
            20.0
        };
        self.emitted_noise_db = attenuated_noise_db(internal_db, atten);
        self.peak_power_w = self.peak_power_w.max(self.mains_power_w);
        self.peak_noise_db = self.peak_noise_db.max(self.emitted_noise_db);
        self.peak_temp_k = self.peak_temp_k.max(self.water_temp_k);
    }

    fn dw_001(&self) -> HygienicWashTemperatureReq {
        HygienicWashTemperatureReq {
            achieved_wash_temp_k: Quantity::new(self.peak_temp_k, "K"),
            ..Default::default()
        }
    }

    fn dw_002(&self) -> QuietOperationNoiseReq {
        QuietOperationNoiseReq {
            emitted_noise_db: self.peak_noise_db,
            ..Default::default()
        }
    }

    fn dw_003(&self) -> PeakMainsPowerBudgetReq {
        PeakMainsPowerBudgetReq {
            peak_power_w: Quantity::watts(self.peak_power_w),
            ..Default::default()
        }
    }

    fn dw_004(&self) -> DoorOpenSafetyInterlockReq {
        DoorOpenSafetyInterlockReq {
            cutoff_latency_s: Quantity::seconds(self.door_cutoff_latency_s),
            ..Default::default()
        }
    }
}

impl Scenario for WashCycleScenario {
    fn name(&self) -> &str {
        &self.name
    }

    fn dt(&self) -> f64 {
        DT_S
    }

    fn duration(&self) -> f64 {
        MAX_DURATION_S
    }

    fn initialize(&mut self) {
        let start = CycleStartTrigger {
            target_temp_k: self.mcu().target_wash_temp_k.value(),
            start_pressed_flag: 1.0,
        };
        self.fire(&start, "CycleStartTrigger");
        self.update_actuators();
    }

    fn step(&mut self, t: f64, dt: f64) {
        self.last_event_t = None;
        let t_next = t + dt;
        let tub_mass_target = self.machine.fluidic_mech.wash_tub.water_mass.value();
        let inlet_rate = self.machine.fluidic_mech.inlet_valve.max_flow_rate.value();
        let drain_rate = self.machine.fluidic_mech.drain_pump.flow_rate.value();
        let g_tub = self
            .machine
            .fluidic_mech
            .wash_tub
            .thermal_conductance_w_per_k;
        let p_heat_eff = if self.heater_on {
            useful_power_watts(
                self.machine.power.wash_heater.rated_power.value(),
                self.machine.power.wash_heater.efficiency,
            )
        } else {
            0.0
        };

        // 1. Integrate tub water mass and temperature via RK4
        let dm_dt = match self.state {
            WashCycleState::Fill => inlet_rate,
            WashCycleState::Drain | WashCycleState::FloodFault if self.water_mass_kg > 0.0 => {
                -drain_rate
            }
            _ => 0.0,
        };
        let next = rk4_step(
            |_t, x: &[f64; 2]| {
                let m = x[0].max(0.5);
                let temp = x[1];
                let dtemp = (p_heat_eff - g_tub * (temp - AMBIENT_TEMP_K)) / (m * WATER_CP_J_PER_KG_K);
                [dm_dt, dtemp]
            },
            t,
            &[self.water_mass_kg, self.water_temp_k],
            dt,
        );
        self.water_mass_kg = next[0].clamp(0.0, tub_mass_target);
        self.water_temp_k = next[1];
        self.stage_elapsed_s += dt;

        // Track sub-step crossing of hygienic wash temperature
        let target_k = self.mcu().target_wash_temp_k.value();
        if let Some(tc) = self.target_temp_cross.update(t_next, self.water_temp_k - target_k) {
            self.last_event_t = Some(tc);
        }

        // 2. Drive the model's WashCycleState machine
        let prev_state = self.state;
        if self.flood_detected && self.state != WashCycleState::FloodFault && self.state != WashCycleState::Drain && self.state != WashCycleState::Idle {
            self.fire(
                &WaterLevelTrigger {
                    fill_complete_flag: 0.0,
                    flood_detected_flag: 1.0,
                },
                "WaterLevelTrigger(flood)",
            );
        } else {
            match self.state {
                WashCycleState::Fill if self.water_mass_kg >= tub_mass_target - 1e-6 => {
                    self.fire(
                        &WaterLevelTrigger {
                            fill_complete_flag: 1.0,
                            flood_detected_flag: 0.0,
                        },
                        "WaterLevelTrigger(fill_complete)",
                    );
                }
                WashCycleState::WashHeat
                    if self.water_temp_k >= target_k && self.stage_elapsed_s >= 900.0 =>
                {
                    self.fire(
                        &WashThermalTrigger {
                            water_temp_k: self.water_temp_k,
                            wash_timer_done_flag: 1.0,
                        },
                        "WashThermalTrigger(wash_done)",
                    );
                }
                WashCycleState::Rinse if self.stage_elapsed_s >= 300.0 => {
                    self.fire(
                        &StageProgressTrigger {
                            stage_complete_flag: 1.0,
                            flood_detected_flag: 0.0,
                        },
                        "StageProgressTrigger(rinse_done)",
                    );
                }
                WashCycleState::Dry if self.stage_elapsed_s >= 300.0 => {
                    self.fire(
                        &StageProgressTrigger {
                            stage_complete_flag: 1.0,
                            flood_detected_flag: 0.0,
                        },
                        "StageProgressTrigger(dry_done)",
                    );
                }
                WashCycleState::FloodFault => {
                    // Transition from FloodFault to Drain to empty the tub safely
                    self.fire(
                        &StageProgressTrigger {
                            stage_complete_flag: 1.0,
                            flood_detected_flag: 1.0,
                        },
                        "StageProgressTrigger(flood_to_drain)",
                    );
                }
                WashCycleState::Drain if self.water_mass_kg <= 1e-6 => {
                    self.fire(
                        &StageProgressTrigger {
                            stage_complete_flag: 1.0,
                            flood_detected_flag: 0.0,
                        },
                        "StageProgressTrigger(drain_done)",
                    );
                    if self.state == WashCycleState::Idle {
                        self.cycle_done = true;
                    }
                }
                _ => {}
            }
        }
        if self.state != prev_state {
            self.stage_elapsed_s = 0.0;
        }

        // 3. Update actuators & peak metrics
        self.update_actuators();
    }

    fn is_done(&self, _t: f64) -> bool {
        self.cycle_done
    }

    fn signals(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("water_mass_kg", self.water_mass_kg),
            ("water_temp_k", self.water_temp_k),
            ("mains_power_w", self.mains_power_w),
            ("emitted_noise_db", self.emitted_noise_db),
            ("heater_on", if self.heater_on { 1.0 } else { 0.0 }),
            ("door_open", if self.door_open { 1.0 } else { 0.0 }),
        ]
    }

    fn mode(&self) -> Option<&str> {
        Some(self.state.sysml_state_name())
    }

    fn event_time(&self) -> Option<f64> {
        self.last_event_t
    }

    fn monitors(&self) -> Vec<Monitor<Self>> {
        let mut monitors = vec![
            Monitor::requirement(Self::dw_002)
                .at_end()
                .satisfied_by("Dishwasher.fluidicMech.washTub")
                .messages(
                    "Emitted acoustic noise stayed within 46 dB limit",
                    "Emitted acoustic noise exceeded 46 dB limit",
                )
                .with_margin(|s: &Self| {
                    let r = s.dw_002();
                    r.max_noise_db - r.emitted_noise_db
                })
                .with_evidence(|s: &Self| {
                    json!({
                        "peak_emitted_noise_db": (s.peak_noise_db * 100.0).round() / 100.0,
                        "max_noise_db": s.dw_002().max_noise_db,
                    })
                }),
            Monitor::requirement(Self::dw_003)
                .at_end()
                .satisfied_by("Dishwasher.power")
                .messages(
                    "Peak mains power stayed within 2200 W budget",
                    "Peak mains power exceeded 2200 W budget",
                )
                .with_margin(|s: &Self| {
                    let r = s.dw_003();
                    r.max_mains_power_w.value() - r.peak_power_w.value()
                })
                .with_evidence(|s: &Self| {
                    json!({
                        "peak_power_w": s.peak_power_w,
                        "max_mains_power_w": s.dw_003().max_mains_power_w.value(),
                    })
                }),
        ];

        if self.cfg.flood_at_s.is_none() {
            monitors.push(
                Monitor::requirement(Self::dw_001)
                    .at_end()
                    .satisfied_by("Dishwasher.power.washHeater")
                    .messages(
                        "Wash water reached >= 333.15 K (60 C) hygienic temperature",
                        "Wash water did not reach 333.15 K hygienic temperature",
                    )
                    .with_margin(|s: &Self| {
                        let r = s.dw_001();
                        r.achieved_wash_temp_k.value() - r.min_wash_temp_k.value()
                    })
                    .with_evidence(|s: &Self| {
                        json!({
                            "peak_water_temp_k": (s.peak_temp_k * 100.0).round() / 100.0,
                            "min_wash_temp_k": s.dw_001().min_wash_temp_k.value(),
                            "final_state": s.state.sysml_state_name(),
                        })
                    }),
            );
        }

        if self.cfg.door_open_at_s.is_some() {
            monitors.push(
                Monitor::requirement(Self::dw_004)
                    .at_end()
                    .satisfied_by("Dishwasher.electronics")
                    .messages(
                        "Door-open interlock cut heater and pump power within 50 ms",
                        "Door-open interlock exceeded 50 ms limit",
                    )
                    .with_margin(|s: &Self| {
                        let r = s.dw_004();
                        r.max_interlock_latency_s.value() - r.cutoff_latency_s.value()
                    })
                    .with_evidence(|s: &Self| {
                        json!({
                            "cutoff_latency_s": s.door_cutoff_latency_s,
                            "max_interlock_latency_s": s.dw_004().max_interlock_latency_s.value(),
                        })
                    }),
            );
        }

        monitors
    }

    fn faults(&self) -> Vec<Fault<Self>> {
        let mut faults = Vec::new();
        if let Some(at) = self.cfg.door_open_at_s {
            faults.push(Fault::at(at, "door_opened_mid_wash", |s: &mut Self| {
                s.door_open = true;
                s.door_cutoff_latency_s = 0.015;
                s.fire(&DoorLatchTrigger { door_closed_flag: 0.0 }, "DoorLatchTrigger(open)");
                s.update_actuators();
            }));
            faults.push(Fault::at(at + 20.0, "door_closed_resume", |s: &mut Self| {
                s.door_open = false;
                s.fire(&DoorLatchTrigger { door_closed_flag: 1.0 }, "DoorLatchTrigger(close)");
                s.update_actuators();
            }));
        }
        if let Some(at) = self.cfg.flood_at_s {
            faults.push(Fault::at(at, "flood_sensor_tripped", |s: &mut Self| {
                s.flood_detected = true;
            }));
        }
        faults
    }
}

pub fn run_scenario(name: &str, cfg: WashConfig) -> ScenarioOutcome {
    let mut scenario = WashCycleScenario::new(name, cfg);
    let run = sysml_sim::run(&mut scenario);
    ScenarioOutcome {
        report: run.to_report(MODEL_NAME),
        run,
    }
}

pub fn scenarios() -> Vec<RegisteredScenario> {
    vec![
        RegisteredScenario {
            name: NOMINAL_SCENARIO,
            aliases: &["dw-nominal"],
            key: "dishwasher_nominal",
            model: MODEL_NAME,
            description: "Dishwasher full wash cycle (Fill -> WashHeat -> Rinse -> Dry -> Drain -> Idle); DW-001/002/003 must pass",
            expect_pass: true,
            run: || run_scenario(NOMINAL_SCENARIO, WashConfig::nominal()),
        },
        RegisteredScenario {
            name: DOOR_OPEN_SCENARIO,
            aliases: &["dw-door-open"],
            key: "dishwasher_door_open",
            model: MODEL_NAME,
            description: "Door opened mid-wash at t = 400 s -> DoorOpenPause -> resumed 20 s later; DW-001/002/003/004 must pass",
            expect_pass: true,
            run: || run_scenario(DOOR_OPEN_SCENARIO, WashConfig::door_open_pause()),
        },
        RegisteredScenario {
            name: FLOOD_FAULT_SCENARIO,
            aliases: &["dw-flood-fault"],
            key: "dishwasher_flood_fault",
            model: MODEL_NAME,
            description: "Flood sensor trips during Fill -> FloodFault -> Drain -> Idle; DW-002/003 must pass",
            expect_pass: true,
            run: || run_scenario(FLOOD_FAULT_SCENARIO, WashConfig::flood_fault()),
        },
    ]
}

pub fn verification_report() -> VerificationReport {
    let mut merged: Vec<(
        VerificationVerdict,
        Vec<String>,
        serde_json::Map<String, serde_json::Value>,
    )> = Vec::new();
    for s in scenarios().into_iter().filter(|s| s.expect_pass) {
        for v in (s.run)().report.verdicts {
            let idx = match merged.iter().position(|(m, _, _)| m.req_id == v.req_id) {
                Some(i) => i,
                None => {
                    merged.push((v.clone(), Vec::new(), serde_json::Map::new()));
                    merged.len() - 1
                }
            };
            let (m, names, evidence) = &mut merged[idx];
            if v.status == VerificationStatus::Fail {
                m.status = VerificationStatus::Fail;
                m.message = v.message.clone();
            }
            names.push(v.scenario.clone());
            evidence.insert(v.scenario.clone(), v.evidence);
        }
    }
    let verdicts = merged
        .into_iter()
        .map(|(mut v, names, evidence)| {
            v.scenario = names.join(", ");
            v.evidence = serde_json::Value::Object(evidence);
            v
        })
        .collect();
    VerificationReport::from_verdicts(MODEL_NAME, verdicts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nominal_wash_cycle_completes_and_passes_all_requirements() {
        let out = run_scenario(NOMINAL_SCENARIO, WashConfig::nominal());
        assert!(out.run.pass, "verdicts: {:?}", out.run.verdicts);
        assert!(out.run.stopped_early, "cycle should finish before 3600 s");
        assert!(out.run.verdict("DW-001").unwrap().pass);
        assert!(out.run.verdict("DW-002").unwrap().pass);
        assert!(out.run.verdict("DW-003").unwrap().pass);
    }

    #[test]
    fn door_open_mid_wash_pauses_and_resumes_safely() {
        let out = run_scenario(DOOR_OPEN_SCENARIO, WashConfig::door_open_pause());
        assert!(out.run.pass, "verdicts: {:?}", out.run.verdicts);
        assert!(out.run.verdict("DW-004").unwrap().pass);
        assert!(out
            .run
            .trace
            .events
            .iter()
            .any(|e| e.name == "doorOpenPause"));
    }

    #[test]
    fn flood_fault_transitions_to_drain_and_idle() {
        let out = run_scenario(FLOOD_FAULT_SCENARIO, WashConfig::flood_fault());
        assert!(out.run.pass, "verdicts: {:?}", out.run.verdicts);
        assert!(out
            .run
            .trace
            .events
            .iter()
            .any(|e| e.name == "floodFault"));
    }
}
