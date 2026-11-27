//! FMEA-driven fault-injection runner for the `microwave` model (stage S3).
//!
//! Three-tier rule: `microwave` (model) <- `microwave-sim` (this runner) -> `sysml-sim`.
//! The model never depends on this crate.
//!
//! What is executed, and where each number comes from:
//! - **Plant parameters** come from the model's generated `Default`s (`MicrowaveOven::default()`):
//!   magnetron `radiativeOutputPower` (900 W), waveguide `transmissionEfficiency` (0.98),
//!   cavity `rfCouplingEfficiency` (0.88), `PrimaryDoorInterlock.cutoffLatency` (5 ms),
//!   `MonitorDoorInterlock.cutoffLatency` (8 ms), `HvPowerRelay.switchingCutoffLatency` (8 ms),
//!   `CookStartCommand.requestedRfPowerW` (900 W).
//! - **Behaviour** is the model's own `MicrowaveState::step_with(...)` state machine (subject
//!   `MicrowaveMcu`); the door-open guard `door_open > door_open_trip_factor` is the model's.
//! - **Verdicts** call the model's `EvaluableRequirement::evaluate()` for MW-001 and MW-004; the
//!   safe state is `<DoorInterlockCutoffRequirement as SafetyGoal>::SAFE_STATE` and the FTTI is
//!   `SafetyGoal::FTTI_MS` (20 ms, taken from MW-004's `max_cutoff_latency_s`).
//! - **Faults** are the model's `#[sysml::fmea]` entries (`ContactsWeldedClosed` on the primary
//!   and monitor door interlocks), looked up from the generated `PartDef::fmea` metadata.
//!
//! Scenario stimulus (not model values): the 120 s reheat duration and the door opening at
//! t = 45 s come from `docs/DEVELOPMENT_PLAN.md` S3; the model has no cook-time attribute.
//! The fixed step `dt = 1 ms` is a solver choice fine enough to resolve the 5/8 ms latencies.
//!
//! Runner assumptions (not in the model; listed in every report via the module docs):
//! 1. Either interlock reporting "door open" is forwarded to the MCU as one
//!    `DoorInterlockEvent { door_open: 1, door_closed: 0 }` (1-out-of-2 trip, matching the
//!    "dual redundant" wording of MW-004). MCU processing time is not modelled (0 s).
//! 2. Cutoff chain = interlock `cutoffLatency` -> MCU state change -> HV relay
//!    `switchingCutoffLatency` -> RF output 0 W. Relay closing is instantaneous.
//! 3. RF output is the magnetron's rated `radiativeOutputPower` while the relay is closed and
//!    0 W otherwise (no filament warm-up, no thermal derating). Absorbed food energy is
//!    integrated with `rk4_step`; food/cavity *temperature* is not simulated because the model
//!    has no specific-heat or thermal-mass attributes for the load, so MW-002 (cooling) and
//!    MW-003 (noise) are not verified by these scenarios.
//!
//! Limitation: simulation verifies the model as written, not the hardware.

use serde_json::json;
use sysml_dsl::prelude::{SafetyGoal, SysmlItemCore};
use sysml_ir::{
    FmeaEntry, Quantity, SysmlItem, SysmlPart, VerificationReport, VerificationStatus,
    VerificationVerdict,
};
use sysml_sim::{rk4_step, Fault, Monitor, RegisteredScenario, Scenario, ScenarioOutcome};
use sysml_std::prelude::useful_power_watts;

use microwave::microwave_model::*;

/// Model name used in verification reports (the `#[sysml::package]` name).
pub const MODEL_NAME: &str = "MicrowaveModel";
/// Fixed solver step [s].
pub const DT_S: f64 = 0.001;
/// Reheat duration [s] — scenario stimulus from `DEVELOPMENT_PLAN.md` S3 (no model attribute).
pub const REHEAT_DURATION_S: f64 = 120.0;
/// Door-opening time [s] — scenario stimulus from `DEVELOPMENT_PLAN.md` S3.
pub const DOOR_OPEN_AT_S: f64 = 45.0;
/// FMEA failure mode injected on the door interlocks (from the model's `#[sysml::fmea]`).
pub const WELDED_CLOSED: &str = "ContactsWeldedClosed";

pub const NOMINAL_SCENARIO: &str = "microwave_reheat_nominal";
pub const DOOR_OPEN_SCENARIO: &str = "microwave_door_open_mid_cook";
pub const PRIMARY_WELDED_SCENARIO: &str = "microwave_door_open_primary_interlock_welded";
pub const BROKEN_INTERLOCK_SCENARIO: &str = "microwave_door_open_both_interlocks_welded";

/// A failure mode taken from a part's `#[sysml::fmea(...)]` metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelFmea {
    /// SysML part definition name (e.g. `PrimaryDoorInterlock`).
    pub part: String,
    pub entry: FmeaEntry,
}

impl ModelFmea {
    /// Fault name used in traces: `<Part>::<failureMode>`.
    pub fn fault_name(&self) -> String {
        format!("{}::{}", self.part, self.entry.failure_mode)
    }
}

/// Look up `failure_mode` in the model's FMEA entries for part `P`.
/// Panics if the model does not declare it (the scenario would be unfounded).
pub fn model_fmea<P: SysmlPart>(failure_mode: &str) -> ModelFmea {
    let def = P::part_def();
    let entry = def
        .fmea
        .iter()
        .find(|e| e.failure_mode == failure_mode)
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "model part `{}` declares no #[sysml::fmea(failure_mode = \"{failure_mode}\")]",
                def.name
            )
        });
    ModelFmea {
        part: def.name,
        entry,
    }
}

/// Numeric `#[sysml::attr(default = ...)]` of an item attribute, read from the model's IR
/// (`#[sysml::item]` structs derive a plain zero `Default`, so the modeled default lives only
/// in the IR). Panics if the attribute or its numeric default is missing.
pub fn item_attr_default<I: SysmlItem>(sysml_attr: &str) -> f64 {
    let def = I::item_def();
    def.attributes
        .iter()
        .find(|a| a.name.as_deref() == Some(sysml_attr))
        .and_then(|a| a.value.as_deref())
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or_else(|| {
            panic!(
                "item `{}` has no numeric default for `{sysml_attr}`",
                def.name
            )
        })
}

/// Which faults a reheat run injects.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ReheatConfig {
    /// Door opens at this time (operator action / hazardous situation), if any.
    pub door_open_at_s: Option<f64>,
    /// FMEA `PrimaryDoorInterlock::ContactsWeldedClosed`, injected at t = 0.
    pub primary_welded: bool,
    /// FMEA `MonitorDoorInterlock::ContactsWeldedClosed`, injected at t = 0.
    pub monitor_welded: bool,
}

impl ReheatConfig {
    pub fn nominal() -> Self {
        Self::default()
    }
    pub fn door_open() -> Self {
        Self {
            door_open_at_s: Some(DOOR_OPEN_AT_S),
            ..Self::default()
        }
    }
    pub fn primary_welded() -> Self {
        Self {
            primary_welded: true,
            ..Self::door_open()
        }
    }
    /// Deliberately broken interlock: both switches welded closed (expected to violate MW-004).
    pub fn broken_interlock() -> Self {
        Self {
            primary_welded: true,
            monitor_welded: true,
            ..Self::door_open()
        }
    }
}

/// Reheat cycle with optional door-open and interlock faults, as a `sysml_sim::Scenario`.
///
/// The struct is the simulation state: the model's `MicrowaveOven` (parameters), its
/// `MicrowaveState` machine, and the runner-side interlock / relay / RF plant.
pub struct ReheatScenario {
    name: String,
    cfg: ReheatConfig,
    pub oven: MicrowaveOven,
    pub state: MicrowaveState,
    k: u64,
    door_open: bool,
    door_open_k: Option<u64>,
    primary_welded: bool,
    monitor_welded: bool,
    primary_reports_open: bool,
    monitor_reports_open: bool,
    mcu_notified_open: bool,
    relay_closed: bool,
    relay_open_due_k: Option<u64>,
    cutoff_k: Option<u64>,
    cycle_completed: bool,
    rf_power_w: f64,
    absorbed_energy_j: f64,
    min_rf_while_cooking_w: Option<f64>,
    rejected_events: Vec<String>,
}

fn steps_for(latency_s: f64) -> u64 {
    // Round up so a latency between grid points is never shortened.
    ((latency_s / DT_S) - 1e-9).ceil().max(0.0) as u64
}

fn steps_to_s(steps: u64) -> f64 {
    steps as f64 / (1.0 / DT_S).round()
}

impl ReheatScenario {
    pub fn new(name: impl Into<String>, cfg: ReheatConfig) -> Self {
        Self {
            name: name.into(),
            cfg,
            oven: MicrowaveOven::default(),
            state: MicrowaveState::initial_state(),
            k: 0,
            door_open: false,
            door_open_k: None,
            primary_welded: false,
            monitor_welded: false,
            primary_reports_open: false,
            monitor_reports_open: false,
            mcu_notified_open: false,
            relay_closed: false,
            relay_open_due_k: None,
            cutoff_k: None,
            cycle_completed: false,
            rf_power_w: 0.0,
            absorbed_energy_j: 0.0,
            min_rf_while_cooking_w: None,
            rejected_events: Vec::new(),
        }
    }

    fn mcu(&self) -> &MicrowaveMcu {
        &self.oven.control_safety.mcu
    }

    fn primary_latency_steps(&self) -> u64 {
        steps_for(
            self.oven
                .control_safety
                .primary_interlock
                .cutoff_latency
                .value(),
        )
    }

    fn monitor_latency_steps(&self) -> u64 {
        steps_for(
            self.oven
                .control_safety
                .monitor_interlock
                .cutoff_latency
                .value(),
        )
    }

    fn relay_latency_steps(&self) -> u64 {
        steps_for(self.oven.power_hv.hv_relay.switching_cutoff_latency.value())
    }

    fn rated_rf_w(&self) -> f64 {
        self.oven
            .rf_cooking
            .magnetron
            .radiative_output_power
            .value()
    }

    /// RF power absorbed by the load: magnetron RF x waveguide x cavity coupling.
    fn absorbed_power_w(&self, rf_w: f64) -> f64 {
        let rf = &self.oven.rf_cooking;
        useful_power_watts(
            useful_power_watts(rf_w, rf.waveguide.transmission_efficiency),
            rf.cavity.rf_coupling_efficiency,
        )
    }

    fn fire<E: SysmlItemCore + 'static>(&mut self, event: &E, label: &str) {
        let mcu = *self.mcu();
        if let Err(e) = self.state.step_with(event, &mcu) {
            self.rejected_events.push(format!("{label}: {e}"));
        }
    }

    /// Door-open -> HV-off latency as defined by MW-004: elapsed time while the cutoff is
    /// pending, the measured value once RF is 0 W in the safe state, 0 s if the door never
    /// opened.
    pub fn cutoff_latency_s(&self) -> f64 {
        match (self.door_open_k, self.cutoff_k) {
            (Some(o), Some(c)) => steps_to_s(c - o),
            (Some(o), None) => steps_to_s(self.k - o),
            _ => 0.0,
        }
    }

    fn mw_001(&self) -> MagnetronRfOutputRequirement {
        MagnetronRfOutputRequirement {
            rf_output_power_w: Quantity::watts(self.min_rf_while_cooking_w.unwrap_or(0.0)),
            ..Default::default()
        }
    }

    fn mw_004(&self) -> DoorInterlockCutoffRequirement {
        DoorInterlockCutoffRequirement {
            actual_cutoff_latency_s: Quantity::seconds(self.cutoff_latency_s()),
            ..Default::default()
        }
    }

    /// FMEA entries injected by this scenario.
    pub fn injected_fmea(&self) -> Vec<ModelFmea> {
        let mut v = Vec::new();
        if self.cfg.primary_welded {
            v.push(model_fmea::<PrimaryDoorInterlock>(WELDED_CLOSED));
        }
        if self.cfg.monitor_welded {
            v.push(model_fmea::<MonitorDoorInterlock>(WELDED_CLOSED));
        }
        v
    }

    fn update_relay_and_rf(&mut self) {
        if self.state == MicrowaveState::Cooking {
            self.relay_closed = true;
            self.relay_open_due_k = None;
        } else if self.relay_closed {
            let due = *self
                .relay_open_due_k
                .get_or_insert(self.k + self.relay_latency_steps());
            if self.k >= due {
                self.relay_closed = false;
                self.relay_open_due_k = None;
            }
        }
        self.rf_power_w = if self.relay_closed {
            self.rated_rf_w()
        } else {
            0.0
        };
        if self.state == MicrowaveState::Cooking && self.relay_closed {
            let rf = self.rf_power_w;
            self.min_rf_while_cooking_w =
                Some(self.min_rf_while_cooking_w.map_or(rf, |m| m.min(rf)));
        }
    }
}

impl Scenario for ReheatScenario {
    fn name(&self) -> &str {
        &self.name
    }

    fn dt(&self) -> f64 {
        DT_S
    }

    fn duration(&self) -> f64 {
        REHEAT_DURATION_S
    }

    /// Operator starts a reheat with the door closed at the model's default requested power.
    fn initialize(&mut self) {
        // `CookStartCommand` derives a zero `Default`; take the modeled attribute default instead.
        let start = CookStartCommand {
            door_closed: 1.0,
            requested_rf_power_w: item_attr_default::<CookStartCommand>("requestedRfPowerW"),
        };
        self.fire(&start, "CookStartCommand");
        self.update_relay_and_rf();
    }

    fn step(&mut self, t: f64, dt: f64) {
        // 1. Plant over [t, t + dt]: absorbed RF energy in the food load (RK4).
        let p_abs = self.absorbed_power_w(self.rf_power_w);
        self.absorbed_energy_j = rk4_step(|_, _| [p_abs], t, &[self.absorbed_energy_j], dt)[0];
        self.k += 1;

        // 2. Door interlocks: each reports "open" after its own cutoffLatency unless welded.
        if let Some(o) = self.door_open_k {
            let since = self.k - o;
            if !self.primary_welded && since >= self.primary_latency_steps() {
                self.primary_reports_open = true;
            }
            if !self.monitor_welded && since >= self.monitor_latency_steps() {
                self.monitor_reports_open = true;
            }
        }

        // 3. MCU: the model's state machine decides (1oo2 door-open event; cook timer).
        if (self.primary_reports_open || self.monitor_reports_open) && !self.mcu_notified_open {
            self.mcu_notified_open = true;
            let ev = DoorInterlockEvent {
                door_open: 1.0,
                door_closed: 0.0,
            };
            self.fire(&ev, "DoorInterlockEvent");
        }
        if !self.cycle_completed
            && self.state == MicrowaveState::Cooking
            && self.k >= steps_for(REHEAT_DURATION_S)
        {
            self.cycle_completed = true;
            self.fire(&CookStopEvent { cycle_done: 1.0 }, "CookStopEvent");
        }

        // 4. HV relay + RF output.
        self.update_relay_and_rf();

        // 5. Cutoff complete: RF 0 W *and* the state machine is in the safety goal's safe state.
        if self.door_open_k.is_some()
            && self.cutoff_k.is_none()
            && self.rf_power_w == 0.0
            && self.state.sysml_state_name()
                == <DoorInterlockCutoffRequirement as SafetyGoal>::SAFE_STATE
        {
            self.cutoff_k = Some(self.k);
        }
    }

    fn signals(&self) -> Vec<(&'static str, f64)> {
        let b = |v: bool| if v { 1.0 } else { 0.0 };
        vec![
            ("door_open", b(self.door_open)),
            (
                "primary_interlock_reports_open",
                b(self.primary_reports_open),
            ),
            (
                "monitor_interlock_reports_open",
                b(self.monitor_reports_open),
            ),
            ("hv_relay_closed", b(self.relay_closed)),
            ("rf_power_w", self.rf_power_w),
            ("absorbed_energy_j", self.absorbed_energy_j),
            ("cutoff_latency_s", self.cutoff_latency_s()),
        ]
    }

    fn mode(&self) -> Option<&str> {
        Some(self.state.sysml_state_name())
    }

    fn monitors(&self) -> Vec<Monitor<Self>> {
        let mut monitors = vec![Monitor::requirement(Self::mw_001)
            .at_end()
            .satisfied_by("MicrowaveOven.rfCooking.magnetron")
            .messages(
                "Magnetron RF output stayed >= 800 W while cooking",
                "Magnetron RF output fell below 800 W while cooking",
            )
            .with_margin(|s: &Self| {
                let r = s.mw_001();
                r.rf_output_power_w.value() - r.min_rf_power_w.value()
            })
            .with_evidence(|s: &Self| {
                let r = s.mw_001();
                json!({
                    "min_rf_output_while_cooking_w": r.rf_output_power_w.value(),
                    "min_rf_power_w": r.min_rf_power_w.value(),
                    "absorbed_energy_j": (s.absorbed_energy_j * 1000.0).round() / 1000.0,
                    "final_state": s.state.sysml_state_name(),
                    "parameter_source": "MagnetronTube.radiativeOutputPower (model default)",
                })
            })];

        if self.cfg.door_open_at_s.is_some() {
            monitors.push(
                Monitor::requirement(Self::mw_004)
                    .satisfied_by("MicrowaveOven.controlSafety")
                    .messages(
                        "HV/RF cut off and safe state reached within the 20 ms limit after door opening",
                        "HV/RF not cut off within the 20 ms limit after door opening",
                    )
                    .with_margin(|s: &Self| {
                        let r = s.mw_004();
                        r.max_cutoff_latency_s.value() - r.actual_cutoff_latency_s.value()
                    })
                    .with_evidence(|s: &Self| {
                        let r = s.mw_004();
                        let fmea: Vec<_> = s
                            .injected_fmea()
                            .iter()
                            .map(|f| {
                                json!({
                                    "part": f.part,
                                    "failure_mode": f.entry.failure_mode,
                                    "effect": f.entry.effect,
                                })
                            })
                            .collect();
                        json!({
                            "door_opened_s": s.door_open_k.map(steps_to_s),
                            "cutoff_complete_s": s.cutoff_k.map(steps_to_s),
                            "actual_cutoff_latency_s": r.actual_cutoff_latency_s.value(),
                            "max_cutoff_latency_s": r.max_cutoff_latency_s.value(),
                            "ftti_ms": <DoorInterlockCutoffRequirement as SafetyGoal>::FTTI_MS,
                            "safe_state": <DoorInterlockCutoffRequirement as SafetyGoal>::SAFE_STATE,
                            "final_state": s.state.sysml_state_name(),
                            "final_rf_power_w": s.rf_power_w,
                            "primary_interlock_reported_open": s.primary_reports_open,
                            "monitor_interlock_reported_open": s.monitor_reports_open,
                            "injected_fmea": fmea,
                            "rejected_events": s.rejected_events,
                        })
                    }),
            );
        }
        monitors
    }

    fn faults(&self) -> Vec<Fault<Self>> {
        let mut faults = Vec::new();
        if self.cfg.primary_welded {
            let f = model_fmea::<PrimaryDoorInterlock>(WELDED_CLOSED);
            faults.push(Fault::at(0.0, f.fault_name(), |s: &mut Self| {
                s.primary_welded = true;
            }));
        }
        if self.cfg.monitor_welded {
            let f = model_fmea::<MonitorDoorInterlock>(WELDED_CLOSED);
            faults.push(Fault::at(0.0, f.fault_name(), |s: &mut Self| {
                s.monitor_welded = true;
            }));
        }
        if let Some(at) = self.cfg.door_open_at_s {
            faults.push(Fault::at(at, "door_opened_mid_cook", |s: &mut Self| {
                s.door_open = true;
                s.door_open_k = Some(s.k);
            }));
        }
        faults
    }
}

/// Run one reheat scenario and return the report plus the full run.
pub fn run_scenario(name: &str, cfg: ReheatConfig) -> ScenarioOutcome {
    let mut scenario = ReheatScenario::new(name, cfg);
    let run = sysml_sim::run(&mut scenario);
    ScenarioOutcome {
        report: run.to_report(MODEL_NAME),
        run,
    }
}

/// Scenarios this runner registers with `rsysdsl verify`.
pub fn scenarios() -> Vec<RegisteredScenario> {
    vec![
        RegisteredScenario {
            name: NOMINAL_SCENARIO,
            aliases: &["mw-nominal"],
            key: "microwave_nominal",
            model: MODEL_NAME,
            description: "Microwave 120 s reheat at the model's default 900 W; MW-001 must pass",
            expect_pass: true,
            run: || run_scenario(NOMINAL_SCENARIO, ReheatConfig::nominal()),
        },
        RegisteredScenario {
            name: DOOR_OPEN_SCENARIO,
            aliases: &["mw-door-open"],
            key: "microwave_door_open",
            model: MODEL_NAME,
            description: "Door opened at t = 45 s mid-cook; interlocks must reach doorOpenCutoff with RF 0 W within 20 ms (MW-004)",
            expect_pass: true,
            run: || run_scenario(DOOR_OPEN_SCENARIO, ReheatConfig::door_open()),
        },
        RegisteredScenario {
            name: PRIMARY_WELDED_SCENARIO,
            aliases: &["mw-primary-welded"],
            key: "microwave_primary_interlock_welded",
            model: MODEL_NAME,
            description: "FMEA PrimaryDoorInterlock::ContactsWeldedClosed + door opened at 45 s; monitor interlock alone must still meet MW-004",
            expect_pass: true,
            run: || run_scenario(PRIMARY_WELDED_SCENARIO, ReheatConfig::primary_welded()),
        },
        RegisteredScenario {
            name: BROKEN_INTERLOCK_SCENARIO,
            aliases: &["mw-broken-interlock"],
            key: "microwave_broken_interlock",
            model: MODEL_NAME,
            description: "Deliberately broken interlock: both door interlocks ContactsWeldedClosed + door opened at 45 s; MW-004 must FAIL (proves the monitor detects violations)",
            expect_pass: false,
            run: || run_scenario(BROKEN_INTERLOCK_SCENARIO, ReheatConfig::broken_interlock()),
        },
    ]
}

/// Verification evidence for the trace matrix: the verdicts of every scenario that is
/// *expected to pass* (the deliberately broken variant is a monitor self-test, not evidence
/// about the model), merged per requirement. A requirement passes only if it passed in every
/// scenario that checked it; `scenario` lists those scenarios and `evidence` maps each
/// scenario name to its evidence.
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
mod tests;
