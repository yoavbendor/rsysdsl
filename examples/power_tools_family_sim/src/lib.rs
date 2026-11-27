//! Battery discharge & thermal-cutoff simulation runner for the `power_tools_family` model.
//!
//! Three-tier rule: `power-tools-family` (model) <- `power-tools-family-sim` (this runner) -> `sysml-sim`.
//! The model never depends on this crate.
//!
//! - **Plant parameters** come from `CordlessDrill::default()`, `Battery18V5Ah::default()`,
//!   `Battery18V2Ah::default()`, and `WorkshopAmbient::default()`:
//!   battery energy capacity (324 kJ for 5 Ah, 129.6 kJ for 2 Ah), drill `ratedPower` (550 W),
//!   inverter `efficiency` (0.96), motor `efficiency` (0.89), MCU `cutoffTemp` (343.15 K),
//!   ambient temperature (295.15 K), and `maxNoiseDb` (79 dB).
//! - **Behaviour** is driven by `ToolOperatingState::step_with(...)` with `ZeroCrossing` thermal
//!   cutoff detection.
//! - **Verdicts** evaluate `BatteryRuntimeRequirement` (TOOL-001), `MotorThermalSafetyRequirement`
//!   (TOOL-002), and `AcousticEmissionLimitRequirement` (TOOL-003).

use serde_json::json;
use sysml_ir::{Quantity, VerificationReport, VerificationStatus, VerificationVerdict};
use sysml_sim::{
    rk4_step, Crossing, Fault, Monitor, RegisteredScenario, Scenario, ScenarioOutcome, ZeroCrossing,
};

use power_tools_family::power_tools_family_model::*;

pub const MODEL_NAME: &str = "PowerToolsFamily";
pub const DT_S: f64 = 1.0;
pub const MAX_DURATION_S: f64 = 700.0;
/// Lumped motor thermal capacitance [J/K] (0.42 kg copper/steel motor).
const MOTOR_THERMAL_CAP_J_PER_K: f64 = 200.0;
/// Nominal motor-to-ambient thermal conductance [W/K] under fan cooling.
const NOMINAL_THERMAL_COND_W_PER_K: f64 = 2.6;
/// Degraded thermal conductance [W/K] when cooling vents are blocked.
const BLOCKED_VENT_COND_W_PER_K: f64 = 0.8;

pub const DRILL_5AH_SCENARIO: &str = "power_tool_drill_5ah_nominal";
pub const DRILL_2AH_SCENARIO: &str = "power_tool_drill_2ah_short_runtime";
pub const OVERHEAT_SCENARIO: &str = "power_tool_overheat_cutoff";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackChoice {
    Pack5Ah,
    Pack2Ah,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ToolSimConfig {
    pub pack: PackChoice,
    /// Block cooling vents at this time [s], causing motor overheat.
    pub block_vents_at_s: Option<f64>,
}

impl ToolSimConfig {
    pub fn drill_5ah() -> Self {
        Self {
            pack: PackChoice::Pack5Ah,
            block_vents_at_s: None,
        }
    }
    pub fn drill_2ah() -> Self {
        Self {
            pack: PackChoice::Pack2Ah,
            block_vents_at_s: None,
        }
    }
    pub fn overheat_fault() -> Self {
        Self {
            pack: PackChoice::Pack5Ah,
            block_vents_at_s: Some(40.0),
        }
    }
}

pub struct ToolDutyScenario {
    name: String,
    cfg: ToolSimConfig,
    pub drill: CordlessDrill,
    pub ambient: WorkshopAmbient,
    pub state: ToolOperatingState,
    battery_energy_j: f64,
    initial_energy_j: f64,
    motor_temp_k: f64,
    thermal_cond_w_per_k: f64,
    runtime_s: f64,
    peak_temp_rise_k: f64,
    phase_current_a: f64,
    cutoff_cross: ZeroCrossing,
    last_event_t: Option<f64>,
    vents_blocked: bool,
}

impl ToolDutyScenario {
    pub fn new(name: impl Into<String>, cfg: ToolSimConfig) -> Self {
        let drill = CordlessDrill::default();
        let ambient = WorkshopAmbient::default();
        let energy_j = match cfg.pack {
            PackChoice::Pack5Ah => Battery18V5Ah::default().energy_capacity.value(),
            PackChoice::Pack2Ah => Battery18V2Ah::default().energy_capacity.value(),
        };
        let t_amb = ambient.ambient_temperature.value();
        Self {
            name: name.into(),
            cfg,
            drill,
            ambient,
            state: ToolOperatingState::initial_state(),
            battery_energy_j: energy_j,
            initial_energy_j: energy_j,
            motor_temp_k: t_amb,
            thermal_cond_w_per_k: NOMINAL_THERMAL_COND_W_PER_K,
            runtime_s: 0.0,
            peak_temp_rise_k: 0.0,
            phase_current_a: 0.0,
            cutoff_cross: ZeroCrossing::new(Crossing::Rising),
            last_event_t: None,
            vents_blocked: false,
        }
    }

    fn mcu(&self) -> ToolControllerMcu {
        self.drill.base.mcu
    }

    fn t_amb(&self) -> f64 {
        self.ambient.ambient_temperature.value()
    }

    fn tool_001(&self) -> BatteryRuntimeRequirement {
        BatteryRuntimeRequirement {
            actual_runtime_s: Quantity::seconds(self.runtime_s),
            ..Default::default()
        }
    }

    fn tool_002(&self) -> MotorThermalSafetyRequirement {
        MotorThermalSafetyRequirement {
            motor_temp_rise_k: Quantity::new(self.motor_temp_k - self.t_amb(), "K"),
            ..Default::default()
        }
    }

    fn tool_003(&self) -> AcousticEmissionLimitRequirement {
        AcousticEmissionLimitRequirement {
            measured_noise_db: self.drill.max_noise_db,
            ..Default::default()
        }
    }
}

impl Scenario for ToolDutyScenario {
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
        let mcu = self.mcu();
        let ev = ToolSafetyTelemetry {
            trigger_demand: 1.0,
            motor_temp: self.motor_temp_k,
            phase_current: 0.0,
        };
        let _ = self.state.step_with(&ev, &mcu);
    }

    fn step(&mut self, t: f64, dt: f64) {
        self.last_event_t = None;
        let t_next = t + dt;
        let p_rated = self.drill.rated_power.value();
        let v_nom = self.drill.base.inverter.output_voltage.value();
        let eta_inv = self.drill.base.inverter.efficiency;
        let eta_mot = self.drill.base.motor.efficiency;
        let t_amb = self.t_amb();
        let g_th = self.thermal_cond_w_per_k;

        let running = self.state == ToolOperatingState::Running && self.battery_energy_j > 0.0;
        let p_batt = if running { p_rated } else { 0.0 };
        let p_loss = if running {
            p_rated * (1.0 - eta_inv * eta_mot)
        } else {
            0.0
        };
        self.phase_current_a = if running { p_batt / v_nom } else { 0.0 };
        if running {
            self.runtime_s += dt;
        }

        let next = rk4_step(
            |_t, x: &[f64; 2]| {
                let d_e = -p_batt;
                let d_temp = (p_loss - g_th * (x[1] - t_amb)) / MOTOR_THERMAL_CAP_J_PER_K;
                [d_e, d_temp]
            },
            t,
            &[self.battery_energy_j, self.motor_temp_k],
            dt,
        );
        self.battery_energy_j = next[0].max(0.0);
        self.motor_temp_k = next[1];
        self.peak_temp_rise_k = self.peak_temp_rise_k.max(self.motor_temp_k - t_amb);

        // Sub-step crossing of MCU cutoff temperature
        let cutoff_k = self.mcu().cutoff_temp.value();
        if let Some(tc) = self.cutoff_cross.update(t_next, self.motor_temp_k - cutoff_k) {
            self.last_event_t = Some(tc);
        }

        // Feed telemetry into ToolOperatingState
        let mcu = self.mcu();
        let trigger = if self.battery_energy_j > 0.0 { 1.0 } else { 0.0 };
        let ev = ToolSafetyTelemetry {
            trigger_demand: trigger,
            motor_temp: self.motor_temp_k,
            phase_current: self.phase_current_a,
        };
        let _ = self.state.step_with(&ev, &mcu);
    }

    fn is_done(&self, _t: f64) -> bool {
        self.battery_energy_j <= 0.0
            || (self.vents_blocked && self.state == ToolOperatingState::ThermalCutoff)
    }

    fn signals(&self) -> Vec<(&'static str, f64)> {
        let soc = if self.initial_energy_j > 0.0 {
            self.battery_energy_j / self.initial_energy_j
        } else {
            0.0
        };
        vec![
            ("battery_soc", soc),
            ("battery_energy_j", self.battery_energy_j),
            ("motor_temp_k", self.motor_temp_k),
            ("motor_temp_rise_k", self.motor_temp_k - self.t_amb()),
            ("phase_current_a", self.phase_current_a),
            ("runtime_s", self.runtime_s),
        ]
    }

    fn mode(&self) -> Option<&str> {
        Some(self.state.sysml_state_name())
    }

    fn event_time(&self) -> Option<f64> {
        self.last_event_t
    }

    fn monitors(&self) -> Vec<Monitor<Self>> {
        vec![
            Monitor::requirement(Self::tool_001)
                .at_end()
                .satisfied_by("CordlessDrill.base.battery")
                .messages(
                    "Battery pack met >= 300 s sustained runtime requirement",
                    "Battery pack depleted before 300 s minimum runtime",
                )
                .with_margin(|s: &Self| {
                    let r = s.tool_001();
                    r.actual_runtime_s.value() - r.min_runtime_s.value()
                })
                .with_evidence(|s: &Self| {
                    json!({
                        "actual_runtime_s": s.runtime_s,
                        "min_runtime_s": s.tool_001().min_runtime_s.value(),
                        "pack": format!("{:?}", s.cfg.pack),
                    })
                }),
            Monitor::requirement(Self::tool_002)
                .evidence_at_violation()
                .satisfied_by("CordlessDrill.base.motor")
                .messages(
                    "Motor temperature rise stayed within 45 K safety limit",
                    "Motor temperature rise exceeded 45 K safety limit",
                )
                .with_margin(|s: &Self| {
                    let r = s.tool_002();
                    r.max_temp_rise_k.value() - r.motor_temp_rise_k.value()
                })
                .with_evidence(|s: &Self| {
                    json!({
                        "motor_temp_rise_k": ((s.motor_temp_k - s.t_amb()) * 100.0).round() / 100.0,
                        "max_temp_rise_k": s.tool_002().max_temp_rise_k.value(),
                        "state": s.state.sysml_state_name(),
                    })
                }),
            Monitor::requirement(Self::tool_003)
                .at_end()
                .satisfied_by("CordlessDrill")
                .messages(
                    "Tool acoustic emission stayed within 95 dBA limit",
                    "Tool acoustic emission exceeded 95 dBA limit",
                )
                .with_margin(|s: &Self| {
                    let r = s.tool_003();
                    r.max_noise_db - r.measured_noise_db
                })
                .with_evidence(|s: &Self| {
                    json!({
                        "measured_noise_db": s.drill.max_noise_db,
                        "max_noise_db": s.tool_003().max_noise_db,
                    })
                }),
        ]
    }

    fn faults(&self) -> Vec<Fault<Self>> {
        let mut faults = Vec::new();
        if let Some(at) = self.cfg.block_vents_at_s {
            faults.push(Fault::at(at, "cooling_vents_blocked", |s: &mut Self| {
                s.vents_blocked = true;
                s.thermal_cond_w_per_k = BLOCKED_VENT_COND_W_PER_K;
            }));
        }
        faults
    }
}

pub fn run_scenario(name: &str, cfg: ToolSimConfig) -> ScenarioOutcome {
    let mut scenario = ToolDutyScenario::new(name, cfg);
    let run = sysml_sim::run(&mut scenario);
    ScenarioOutcome {
        report: run.to_report(MODEL_NAME),
        run,
    }
}

pub fn scenarios() -> Vec<RegisteredScenario> {
    vec![
        RegisteredScenario {
            name: DRILL_5AH_SCENARIO,
            aliases: &["tool-5ah"],
            key: "power_tool_5ah",
            model: MODEL_NAME,
            description: "Cordless drill on 18V 5Ah pack under rated 550 W load; TOOL-001/002/003 must pass",
            expect_pass: true,
            run: || run_scenario(DRILL_5AH_SCENARIO, ToolSimConfig::drill_5ah()),
        },
        RegisteredScenario {
            name: DRILL_2AH_SCENARIO,
            aliases: &["tool-2ah"],
            key: "power_tool_2ah",
            model: MODEL_NAME,
            description: "Cordless drill on 18V 2Ah pack under rated 550 W load; TOOL-001 must FAIL (variant trade-off)",
            expect_pass: false,
            run: || run_scenario(DRILL_2AH_SCENARIO, ToolSimConfig::drill_2ah()),
        },
        RegisteredScenario {
            name: OVERHEAT_SCENARIO,
            aliases: &["tool-overheat"],
            key: "power_tool_overheat",
            model: MODEL_NAME,
            description: "Blocked cooling vents at t = 40 s -> motor crosses cutoffTemp -> ThermalCutoff; TOOL-002 must FAIL",
            expect_pass: false,
            run: || run_scenario(OVERHEAT_SCENARIO, ToolSimConfig::overheat_fault()),
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
    fn drill_5ah_passes_all_requirements() {
        let out = run_scenario(DRILL_5AH_SCENARIO, ToolSimConfig::drill_5ah());
        assert!(out.run.pass, "verdicts: {:?}", out.run.verdicts);
        assert!(out.run.verdict("TOOL-001").unwrap().pass);
        assert!(out.run.verdict("TOOL-002").unwrap().pass);
        assert!(out.run.verdict("TOOL-003").unwrap().pass);
    }

    #[test]
    fn drill_2ah_fails_runtime_requirement_only() {
        let out = run_scenario(DRILL_2AH_SCENARIO, ToolSimConfig::drill_2ah());
        assert!(!out.run.pass);
        assert!(!out.run.verdict("TOOL-001").unwrap().pass);
        assert!(out.run.verdict("TOOL-002").unwrap().pass);
        assert!(out.run.verdict("TOOL-003").unwrap().pass);
    }

    #[test]
    fn blocked_vents_triggers_thermal_cutoff_and_fails_tool_002() {
        let out = run_scenario(OVERHEAT_SCENARIO, ToolSimConfig::overheat_fault());
        assert!(!out.run.pass);
        assert!(!out.run.verdict("TOOL-002").unwrap().pass);
        assert!(out
            .run
            .trace
            .events
            .iter()
            .any(|e| e.name == "thermalCutoff"));
    }
}
