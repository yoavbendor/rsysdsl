//! Closed-loop executable verification & fault-injection runner.
//!
//! Strictly separated from `apollo11-model` (Tier 3 runner -> Tier 1/2 model):
//! - Uses auto-generated `ApolloSpacecraft::default()` (thresholds/masses live ONLY in SysML attrs).
//! - Feeds `TouchdownTelemetry` directly into `Apollo11MissionPhases::step_with(&event, &ds)`
//!   (no duplicated abort `if` logic!).
//! - Calls `apollo11_model::calculations_package::ascent_burn_duration(...)` and `total_stage_mass(...)`.
//! - Calls `EvaluableRequirement::evaluate(&req)` directly on `HLR-R002`, `CLR-R050`, `CLR-R057`,
//!   `CLR-R058` (zero duplicated requirement comparisons!).
//! - Runs on the generic `sysml-sim` fixed-step runner: [`PoweredDescentScenario`] implements
//!   `sysml_sim::Scenario`; [`scenarios()`] registers the scenarios with `rsysdsl verify`.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sysml_ir::{Quantity, VerificationReport};
use sysml_sim::{Fault, Monitor, RegisteredScenario, Scenario, ScenarioOutcome};

use apollo11_model::calculations_package::{
    ascent_burn_duration, exhaust_velocity, total_stage_mass,
};
use apollo11_model::mission_phases_package::*;
use apollo11_model::requirements_package::*;
use apollo11_model::technical_components_package::*;

const LUNAR_GRAVITY_M_S2: f64 = 1.625;

/// Optional fault-injection profile applied directly to the SysML hardware model.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FaultProfile {
    /// Override minimum DPS throttle percentage (nominal: 10%). Setting e.g. 65%
    /// prevents deep throttling during terminal hover and causes excessive touchdown velocity.
    pub stuck_min_throttle_percent: Option<f64>,
    /// Lateral guidance drift velocity added during terminal approach (m/s).
    pub guidance_lateral_drift_m_s: f64,
    /// Landing site radial offset due to boulder avoidance / navigation error (m).
    pub landing_zone_offset_m: f64,
    /// Scale factor on Ascent Propulsion System thrust (nominal: 1.0).
    pub ascent_thrust_scale: f64,
}

impl FaultProfile {
    pub fn nominal() -> Self {
        Self {
            stuck_min_throttle_percent: None,
            guidance_lateral_drift_m_s: 0.0,
            landing_zone_offset_m: 185.0,
            ascent_thrust_scale: 1.0,
        }
    }

    pub fn dps_throttle_valve_fault() -> Self {
        Self {
            stuck_min_throttle_percent: Some(65.0),
            guidance_lateral_drift_m_s: 1.45,
            landing_zone_offset_m: 1420.0,
            ascent_thrust_scale: 1.0,
        }
    }
}

/// Telemetry snapshot produced by executing the Powered Descent & Ascent simulation
/// directly on `ApolloSpacecraft`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescentTelemetry {
    pub phase_history: Vec<String>,
    pub final_phase: String,
    pub touchdown_vertical_velocity_m_s: f64,
    pub touchdown_horizontal_velocity_m_s: f64,
    pub landing_zone_deviation_m: f64,
    pub remaining_descent_propellant_kg: f64,
    pub dps_min_throttle_percent: f64,
    pub dps_max_throttle_percent: f64,
    pub ascent_burn_duration_s: f64,
}

/// Terminal powered-descent + lunar-ascent check as a `sysml_sim::Scenario`.
///
/// The struct is the simulation state: the SysML-annotated `ApolloSpacecraft`, the
/// executable `Apollo11MissionPhases` state machine, and the final-approach plant
/// (altitude, vertical velocity, remaining DPS propellant). The plant uses the same
/// semi-implicit Euler update at `dt = 0.05 s` as before the `sysml-sim` port, so the
/// verdicts are unchanged.
pub struct PoweredDescentScenario {
    name: String,
    fault: FaultProfile,
    pub spacecraft: ApolloSpacecraft,
    pub phases: Apollo11MissionPhases,
    pub phase_history: Vec<String>,
    altitude_m: f64,
    vy_m_s: f64,
    prop_remaining_kg: f64,
    throttle_frac: f64,
    climbed_out: bool,
    telemetry: Option<DescentTelemetry>,
}

impl PoweredDescentScenario {
    pub fn new(name: impl Into<String>, spacecraft: ApolloSpacecraft, fault: FaultProfile) -> Self {
        Self {
            name: name.into(),
            fault,
            spacecraft,
            phases: Apollo11MissionPhases::initial_state(),
            phase_history: Vec::new(),
            altitude_m: 0.0,
            vy_m_s: 0.0,
            prop_remaining_kg: 0.0,
            throttle_frac: 0.0,
            climbed_out: false,
            telemetry: None,
        }
    }

    /// Telemetry produced by `finalize` (touchdown / abort decision).
    pub fn telemetry(&self) -> &DescentTelemetry {
        self.telemetry
            .as_ref()
            .expect("telemetry is available after the scenario has run")
    }

    fn ds(&self) -> &LunarModuleDescentStage {
        &self.spacecraft.lunar_module.lunar_module_descent_stage
    }

    fn hlr_r002(&self) -> LunarLanderSoftLandingRequirement {
        let tm = self.telemetry();
        LunarLanderSoftLandingRequirement {
            actual_vertical_velocity: Quantity::m_per_s(tm.touchdown_vertical_velocity_m_s),
            actual_horizontal_velocity: Quantity::m_per_s(tm.touchdown_horizontal_velocity_m_s),
            actual_landing_zone_deviation: Quantity::m(tm.landing_zone_deviation_m),
            ..Default::default()
        }
    }

    fn clr_r050(&self) -> LMDSVelocityLimits {
        let tm = self.telemetry();
        LMDSVelocityLimits {
            actual_vertical_velocity: Quantity::m_per_s(tm.touchdown_vertical_velocity_m_s),
            actual_horizontal_velocity: Quantity::m_per_s(tm.touchdown_horizontal_velocity_m_s),
            ..Default::default()
        }
    }

    /// Sampled every step straight from the (possibly faulted) descent-stage hardware.
    fn clr_r057(&self) -> DPSThrottlingRange {
        let ds = self.ds();
        DPSThrottlingRange {
            actual_min_throttle_percentage: Quantity::percent(ds.min_throttle_percent.value()),
            actual_max_throttle_percentage: Quantity::percent(ds.max_throttle_percent.value()),
            ..Default::default()
        }
    }

    fn clr_r058(&self) -> LMASBurnDuration {
        LMASBurnDuration {
            actual_ascent_burn_duration: Quantity::seconds(self.telemetry().ascent_burn_duration_s),
            ..Default::default()
        }
    }
}

impl Scenario for PoweredDescentScenario {
    fn name(&self) -> &str {
        &self.name
    }

    fn dt(&self) -> f64 {
        0.05
    }

    /// 4000 steps of 0.05 s.
    fn duration(&self) -> f64 {
        200.0
    }

    fn initialize(&mut self) {
        // Step through nominal pre-descent mission phases using strongly typed SysML triggers
        let phases = &mut self.phases;
        self.phase_history = vec![phases.sysml_state_name().to_string()];
        phases.step("auto").expect("Initial -> Preparation");
        self.phase_history.push(phases.sysml_state_name().to_string());
        phases
            .step_on::<PreparationPhaseCompletedNotification>()
            .expect("Preparation -> Launch");
        self.phase_history.push(phases.sysml_state_name().to_string());
        phases
            .step_on::<LaunchPhaseCompletedNotification>()
            .expect("Launch -> LunarOrbitOps");
        self.phase_history.push(phases.sysml_state_name().to_string());
        phases
            .step_on::<LunarOrbitOpsDescentPrepCompletedNotification>()
            .expect("LunarOrbitOps -> PoweredDescent");
        self.phase_history.push(phases.sysml_state_name().to_string());

        // Terminal final-approach initial conditions (environment model lives in this runner crate)
        self.altitude_m = 150.0;
        self.vy_m_s = -12.0;
        self.prop_remaining_kg = self.ds().propellant_mass.value() * 0.14;
    }

    fn step(&mut self, _t: f64, dt: f64) {
        let lm = &self.spacecraft.lunar_module;
        let ds = &lm.lunar_module_descent_stage;
        let as_ = &lm.lunar_module_ascent_stage;
        let dry_total_kg = ds.dry_mass.value()
            + total_stage_mass(as_.dry_mass.value(), as_.propellant_mass.value());
        let max_thrust_n = ds.max_thrust.value();
        let min_frac = ds.min_throttle_percent.value() / 100.0;
        let max_frac = ds.max_throttle_percent.value() / 100.0;
        let ve_m_s = exhaust_velocity(ds.specific_impulse.value());

        let mass_kg = dry_total_kg + self.prop_remaining_kg;
        let target_vy = -(0.8 + 0.075 * self.altitude_m).min(12.0);
        let desired_accel = LUNAR_GRAVITY_M_S2 + 1.8 * (target_vy - self.vy_m_s);
        let desired_thrust_n = (mass_kg * desired_accel).max(0.0);
        let commanded_frac = (desired_thrust_n / max_thrust_n).clamp(min_frac, max_frac);
        let actual_thrust_n = if self.prop_remaining_kg > 0.0 {
            commanded_frac * max_thrust_n
        } else {
            0.0
        };
        self.throttle_frac = actual_thrust_n / max_thrust_n;

        let mdot = actual_thrust_n / ve_m_s;
        self.prop_remaining_kg = (self.prop_remaining_kg - mdot * dt).max(0.0);

        let accel_y = (actual_thrust_n / mass_kg) - LUNAR_GRAVITY_M_S2;
        self.vy_m_s += accel_y * dt;
        self.altitude_m += self.vy_m_s * dt;

        if self.altitude_m > 250.0 {
            // Stuck-throttle climb-out: approach abandoned.
            self.vy_m_s = -4.2;
            self.climbed_out = true;
        }
    }

    fn is_done(&self, _t: f64) -> bool {
        self.altitude_m <= 0.0 || self.climbed_out
    }

    fn finalize(&mut self, _t: f64) {
        let touchdown_vy = self.vy_m_s.abs();
        let touchdown_vx = 0.22 + self.fault.guidance_lateral_drift_m_s;
        let lm = &self.spacecraft.lunar_module;
        let ds = &lm.lunar_module_descent_stage;
        let as_ = &lm.lunar_module_ascent_stage;

        // Modeled #[sysml::calc] calculation invoked directly from `apollo11-model`!
        let burn_s = ascent_burn_duration(
            as_.propellant_mass.value(),
            as_.max_thrust.value() * self.fault.ascent_thrust_scale.max(0.1),
            as_.specific_impulse.value(),
            0.86,
        );

        // Feed typed `TouchdownTelemetry` directly into the guarded SysML state machine —
        // the landing vs. abort decision executes the EXACT guard expression emitted to `.sysml`!
        let touchdown_event = TouchdownTelemetry {
            vertical_velocity: Quantity::m_per_s(touchdown_vy),
            horizontal_velocity: Quantity::m_per_s(touchdown_vx),
        };
        self.phases
            .step_with(&touchdown_event, ds)
            .expect("PoweredDescent -> LunarSurfaceOps or AbortToLunarOrbit via compiled SysML guard");
        self.phase_history
            .push(self.phases.sysml_state_name().to_string());

        self.telemetry = Some(DescentTelemetry {
            phase_history: self.phase_history.clone(),
            final_phase: self.phases.sysml_state_name().to_string(),
            touchdown_vertical_velocity_m_s: round3(touchdown_vy),
            touchdown_horizontal_velocity_m_s: round3(touchdown_vx),
            landing_zone_deviation_m: round3(self.fault.landing_zone_offset_m),
            remaining_descent_propellant_kg: round3(self.prop_remaining_kg),
            dps_min_throttle_percent: ds.min_throttle_percent.value(),
            dps_max_throttle_percent: ds.max_throttle_percent.value(),
            ascent_burn_duration_s: round3(burn_s),
        });
    }

    fn signals(&self) -> Vec<(&'static str, f64)> {
        vec![
            ("altitude_m", self.altitude_m),
            ("vertical_velocity_m_s", self.vy_m_s),
            ("descent_propellant_kg", self.prop_remaining_kg),
            ("dps_throttle_fraction", self.throttle_frac),
        ]
    }

    fn mode(&self) -> Option<&str> {
        Some(self.phases.sysml_state_name())
    }

    /// Every predicate is the model's macro-compiled `EvaluableRequirement::evaluate()`.
    fn monitors(&self) -> Vec<Monitor<Self>> {
        vec![
            Monitor::requirement(Self::hlr_r002)
                .at_end()
                .satisfied_by("ApolloSpacecraft.lunarModule")
                .messages(
                    "Soft landing within Tranquility Base limits achieved",
                    "Soft landing envelope violated",
                )
                .with_evidence(|s: &Self| {
                    let (tm, r) = (s.telemetry(), s.hlr_r002());
                    json!({
                        "actual_vertical_velocity_m_s": tm.touchdown_vertical_velocity_m_s,
                        "max_vertical_velocity_m_s": r.max_vertical_velocity.value(),
                        "actual_horizontal_velocity_m_s": tm.touchdown_horizontal_velocity_m_s,
                        "max_horizontal_velocity_m_s": r.max_horizontal_velocity.value(),
                        "actual_landing_zone_deviation_m": tm.landing_zone_deviation_m,
                        "max_landing_zone_deviation_m": r.max_landing_zone_deviation.value(),
                        "final_phase": tm.final_phase,
                    })
                }),
            Monitor::requirement(Self::clr_r050)
                .at_end()
                .satisfied_by("ApolloSpacecraft.lunarModule.lunarModuleDescentStage")
                .messages(
                    "Touchdown velocities within footpad structural limits",
                    "Footpad touchdown velocity structural limit exceeded",
                )
                .with_margin(|s: &Self| {
                    let r = s.clr_r050();
                    (r.max_touchdown_vertical_velocity.value() - r.actual_vertical_velocity.value())
                        .min(
                            r.max_touchdown_horizontal_velocity.value()
                                - r.actual_horizontal_velocity.value(),
                        )
                })
                .with_evidence(|s: &Self| {
                    let (tm, r) = (s.telemetry(), s.clr_r050());
                    json!({
                        "actual_vertical_velocity_m_s": tm.touchdown_vertical_velocity_m_s,
                        "limit_vertical_velocity_m_s": r.max_touchdown_vertical_velocity.value(),
                        "actual_horizontal_velocity_m_s": tm.touchdown_horizontal_velocity_m_s,
                        "limit_horizontal_velocity_m_s": r.max_touchdown_horizontal_velocity.value(),
                    })
                }),
            Monitor::requirement(Self::clr_r057)
                .satisfied_by("ApolloSpacecraft.lunarModule.lunarModuleDescentStage")
                .messages(
                    "DPS throttling range covers 10%..100%",
                    "DPS throttling range degraded below 10%..100% envelope",
                )
                .with_margin(|s: &Self| {
                    let r = s.clr_r057();
                    (r.min_required_throttle_percentage.value()
                        - r.actual_min_throttle_percentage.value())
                    .min(
                        r.actual_max_throttle_percentage.value()
                            - r.max_required_throttle_percentage.value(),
                    )
                })
                .with_evidence(|s: &Self| {
                    let (tm, r) = (s.telemetry(), s.clr_r057());
                    json!({
                        "actual_min_throttle_percent": tm.dps_min_throttle_percent,
                        "required_min_throttle_percent": r.min_required_throttle_percentage.value(),
                        "actual_max_throttle_percent": tm.dps_max_throttle_percent,
                        "required_max_throttle_percent": r.max_required_throttle_percentage.value(),
                    })
                }),
            Monitor::requirement(Self::clr_r058)
                .at_end()
                .satisfied_by("ApolloSpacecraft.lunarModule.lunarModuleAscentStage")
                .messages(
                    "LMAS ascent burn achieves lunar orbit within 400 s limit",
                    "LMAS ascent burn exceeds 400 s duration limit",
                )
                .with_margin(|s: &Self| {
                    let r = s.clr_r058();
                    r.max_ascent_burn_duration.value() - r.actual_ascent_burn_duration.value()
                })
                .with_evidence(|s: &Self| {
                    let (tm, r) = (s.telemetry(), s.clr_r058());
                    json!({
                        "actual_ascent_burn_duration_s": tm.ascent_burn_duration_s,
                        "max_ascent_burn_duration_s": r.max_ascent_burn_duration.value(),
                        "remaining_descent_propellant_kg": tm.remaining_descent_propellant_kg,
                    })
                }),
        ]
    }

    /// A stuck DPS throttle valve is injected on the descent-stage hardware at t = 0.
    fn faults(&self) -> Vec<Fault<Self>> {
        match self.fault.stuck_min_throttle_percent {
            Some(stuck_min) => vec![Fault::at(
                0.0,
                format!("dps_throttle_valve_stuck_min_{stuck_min}pct"),
                move |s: &mut Self| {
                    s.spacecraft
                        .lunar_module
                        .lunar_module_descent_stage
                        .min_throttle_percent = Quantity::percent(stuck_min);
                },
            )],
            None => Vec::new(),
        }
    }
}

/// Execute a closed-loop terminal powered-descent + lunar-ascent physics check
/// directly on the SysML-annotated `ApolloSpacecraft` and `Apollo11MissionPhases`.
pub fn simulate_powered_descent(
    spacecraft: &mut ApolloSpacecraft,
    fault: &FaultProfile,
) -> DescentTelemetry {
    let mut scenario = PoweredDescentScenario::new(
        "powered_descent",
        std::mem::take(spacecraft),
        fault.clone(),
    );
    sysml_sim::run(&mut scenario);
    let telemetry = scenario.telemetry().clone();
    *spacecraft = scenario.spacecraft;
    telemetry
}

/// Run a named powered-descent scenario on `sysml-sim` and return the requirement
/// report plus the full deterministic run (trace + per-requirement verdicts).
pub fn run_scenario(scenario_name: &str, fault: &FaultProfile) -> ScenarioOutcome {
    // Auto-generated Default from SysML `#[sysml::attr(value = ..., unit = ...)]`!
    let mut scenario =
        PoweredDescentScenario::new(scenario_name, ApolloSpacecraft::default(), fault.clone());
    let run = sysml_sim::run(&mut scenario);
    ScenarioOutcome {
        report: run.to_report(MODEL_NAME),
        run,
    }
}

/// Evaluate all Apollo 11 Powered Descent requirements using the macro-compiled
/// `EvaluableRequirement::evaluate(&req)` methods (zero duplicated predicates!).
pub fn verify_scenario(scenario_name: &str, fault: &FaultProfile) -> VerificationReport {
    run_scenario(scenario_name, fault).report
}

pub const MODEL_NAME: &str = "Apollo11PoweredDescentModel";
pub const NOMINAL_SCENARIO: &str = "nominal_tranquility_base_landing";
pub const FAULT_SCENARIO: &str = "fault_dps_throttle_stuck_65pct";

pub fn run_nominal_verification() -> VerificationReport {
    verify_scenario(NOMINAL_SCENARIO, &FaultProfile::nominal())
}

pub fn run_fault_injection_verification() -> VerificationReport {
    verify_scenario(FAULT_SCENARIO, &FaultProfile::dps_throttle_valve_fault())
}

/// Scenarios this runner registers with `rsysdsl verify`.
pub fn scenarios() -> Vec<RegisteredScenario> {
    vec![
        RegisteredScenario {
            name: NOMINAL_SCENARIO,
            aliases: &["nominal"],
            key: "nominal",
            model: MODEL_NAME,
            description: "Apollo 11 terminal powered descent to Tranquility Base + LM ascent burn; all requirements must pass",
            expect_pass: true,
            run: || run_scenario(NOMINAL_SCENARIO, &FaultProfile::nominal()),
        },
        RegisteredScenario {
            name: FAULT_SCENARIO,
            aliases: &["fault"],
            key: "fault_injection",
            model: MODEL_NAME,
            description: "DPS throttle valve stuck at 65% minimum + guidance drift; abort guard must fire and requirements must fail",
            expect_pass: false,
            run: || run_scenario(FAULT_SCENARIO, &FaultProfile::dps_throttle_valve_fault()),
        },
    ]
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysml_ir::VerificationStatus;

    #[test]
    fn sysml_sim_run_records_trace_fault_and_phase_events() {
        let outcome = run_scenario(FAULT_SCENARIO, &FaultProfile::dps_throttle_valve_fault());
        let run = &outcome.run;
        assert_eq!(
            run.requirement_ids,
            vec!["HLR-R002", "CLR-R050", "CLR-R057", "CLR-R058"]
        );
        // Stuck throttle injected at t = 0 -> CLR-R057 is violated from the first sample.
        let dps = run.verdict("CLR-R057").unwrap();
        assert_eq!(dps.first_violation_s, Some(0.0));
        assert_eq!(dps.min_margin, Some(-55.0));
        assert_eq!(dps.samples, run.steps + 1);
        let names: Vec<_> = run.trace.events.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "dps_throttle_valve_stuck_min_65pct",
                "poweredDescent",
                "abortToLunarOrbit"
            ]
        );
        assert!(run.stopped_early);
        assert_eq!(run.trace.time_s.len(), run.steps + 1);
        // Deterministic JSON trace.
        let again = run_scenario(FAULT_SCENARIO, &FaultProfile::dps_throttle_valve_fault());
        assert_eq!(run.to_json(), again.run.to_json());
    }

    #[test]
    fn gate4_nominal_powered_descent_verifies_all_requirements_via_compiled_constraints() {
        let report = run_nominal_verification();
        assert!(
            report.all_passed,
            "Expected nominal scenario to pass all requirements, got: {}",
            serde_json::to_string_pretty(&report).unwrap()
        );
        assert_eq!(report.passed, 4);
        assert_eq!(report.failed, 0);
        // Verify auto-generated Default populated real Airbus Apollo 11 masses from SysML attributes
        let sc = ApolloSpacecraft::default();
        assert_eq!(sc.lunar_module.lunar_module_descent_stage.dry_mass.value(), 2100.0);
        assert_eq!(sc.lunar_module.lunar_module_descent_stage.dry_mass.unit, "kg");
        assert_eq!(sc.lunar_module.lunar_module_ascent_stage.max_thrust.value(), 16000.0);
        assert_eq!(sc.lunar_module.lunar_module_ascent_stage.max_thrust.unit, "N");
        // Verify nominal touchdown reached LunarSurfaceOps via compiled SysML guard
        assert_eq!(report.verdicts[0].evidence["final_phase"], "lunarSurfaceOps");
    }

    #[test]
    fn gate4_fault_injection_triggers_abort_guard_and_requirement_failures() {
        let report = run_fault_injection_verification();
        assert!(!report.all_passed);
        let hlr = report
            .verdicts
            .iter()
            .find(|v| v.req_id == "HLR-R002")
            .unwrap();
        let dps = report
            .verdicts
            .iter()
            .find(|v| v.req_id == "CLR-R057")
            .unwrap();
        assert_eq!(hlr.status, VerificationStatus::Fail);
        assert_eq!(dps.status, VerificationStatus::Fail);
        // Compiled SysML state-machine guard routed faulted touchdown to AbortToLunarOrbit!
        assert_eq!(hlr.evidence["final_phase"], "abortToLunarOrbit");
    }

    #[test]
    fn gate4_state_machine_rejects_illegal_mission_phase_jump() {
        let mut phases = Apollo11MissionPhases::initial_state();
        let ds = LunarModuleDescentStage::default();
        assert_eq!(phases, Apollo11MissionPhases::Initial);
        phases.step("auto").unwrap();
        assert_eq!(phases, Apollo11MissionPhases::Preparation);
        // Trying to fire TouchdownTelemetry directly from Preparation must fail!
        let err = phases
            .step_with(&TouchdownTelemetry::default(), &ds)
            .unwrap_err();
        assert_eq!(err.from, "preparation");
        assert_eq!(err.trigger, "TouchdownTelemetry");
        assert_eq!(phases, Apollo11MissionPhases::Preparation);
    }
}
