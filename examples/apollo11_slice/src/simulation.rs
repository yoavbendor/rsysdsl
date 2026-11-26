//! Closed-loop executable verification & fault-injection simulation running
//! directly on the `#[sysml::*]` Rust DSL structs and state-machine enum.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sysml_ir::{
    Quantity, SysmlRequirement, VerificationReport, VerificationStatus, VerificationVerdict,
};

use crate::mission_phases_package::*;
use crate::requirements_package::*;
use crate::technical_components_package::*;

const LUNAR_GRAVITY_M_S2: f64 = 1.625;
const G0_M_S2: f64 = 9.80665;

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
            landing_zone_offset_m: 185.0, // Eagle landed well within the 1000 m Tranquility Base target
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
/// directly on `ApolloLunarModule`.
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

/// Execute a closed-loop terminal powered-descent + lunar-ascent physics check
/// directly on the SysML-annotated `ApolloSpacecraft` and `Apollo11MissionPhases`.
pub fn simulate_powered_descent(
    spacecraft: &mut ApolloSpacecraft,
    fault: &FaultProfile,
) -> DescentTelemetry {
    let mut phases = Apollo11MissionPhases::initial_state();
    let mut phase_history = vec![phases.sysml_state_name().to_string()];

    // Step through nominal pre-descent mission phases using strongly typed SysML triggers
    phases.step("auto").expect("Initial -> Preparation");
    phase_history.push(phases.sysml_state_name().to_string());
    phases
        .step_on::<PreparationPhaseCompletedNotification>()
        .expect("Preparation -> Launch");
    phase_history.push(phases.sysml_state_name().to_string());
    phases
        .step_on::<LaunchPhaseCompletedNotification>()
        .expect("Launch -> LunarOrbitOps");
    phase_history.push(phases.sysml_state_name().to_string());
    phases
        .step_on::<LunarOrbitOpsDescentPrepCompletedNotification>()
        .expect("LunarOrbitOps -> PoweredDescent");
    phase_history.push(phases.sysml_state_name().to_string());

    let lm = &mut spacecraft.lunar_module;
    if let Some(stuck_min) = fault.stuck_min_throttle_percent {
        lm.lunar_module_descent_stage.min_throttle_percent = Quantity::percent(stuck_min);
    }

    let ds = &lm.lunar_module_descent_stage;
    let as_ = &lm.lunar_module_ascent_stage;

    // Terminal final-approach physics (from 150 m altitude, -12 m/s descent rate)
    let mut altitude_m = 150.0_f64;
    let mut vy_m_s = -12.0_f64; // negative = downward
    let mut prop_remaining_kg = ds.propellant_mass.value() * 0.14; // ~1155 kg at terminal approach
    let dry_total_kg =
        ds.dry_mass.value() + as_.dry_mass.value() + as_.propellant_mass.value();
    let max_thrust_n = ds.max_thrust.value() * 1000.0;
    let min_frac = ds.min_throttle_percent.value() / 100.0;
    let max_frac = ds.max_throttle_percent.value() / 100.0;
    let ve_m_s = ds.specific_impulse.value() * G0_M_S2;

    let dt = 0.05_f64;
    let mut steps = 0usize;
    while altitude_m > 0.0 && steps < 4000 {
        steps += 1;
        let mass_kg = dry_total_kg + prop_remaining_kg;
        // Desired soft descent profile: target -0.8 m/s near surface
        let target_vy = -(0.8 + 0.075 * altitude_m).min(12.0);
        let desired_accel = LUNAR_GRAVITY_M_S2 + 1.8 * (target_vy - vy_m_s);
        let desired_thrust_n = (mass_kg * desired_accel).max(0.0);
        let commanded_frac = (desired_thrust_n / max_thrust_n).clamp(min_frac, max_frac);
        let actual_thrust_n = if prop_remaining_kg > 0.0 {
            commanded_frac * max_thrust_n
        } else {
            0.0
        };

        let mdot = actual_thrust_n / ve_m_s;
        prop_remaining_kg = (prop_remaining_kg - mdot * dt).max(0.0);

        let accel_y = (actual_thrust_n / mass_kg) - LUNAR_GRAVITY_M_S2;
        vy_m_s += accel_y * dt;
        altitude_m += vy_m_s * dt;

        // If stuck throttle causes upward climb > 250 m without settling, simulate emergency cutoff
        if altitude_m > 250.0 {
            vy_m_s = -4.2;
            break;
        }
    }

    let touchdown_vy = vy_m_s.abs();
    let touchdown_vx = 0.22 + fault.guidance_lateral_drift_m_s;

    // Ascent burn duration check from LMAS propellant mass, thrust, and Isp (Apollo 11: ~360-380s)
    let ascent_thrust_n = as_.max_thrust.value() * 1000.0 * fault.ascent_thrust_scale.max(0.1);
    let ascent_ve = as_.specific_impulse.value() * G0_M_S2;
    let ascent_mdot = ascent_thrust_n / ascent_ve;
    // Apollo 11 burned ~2030 kg of the 2370 kg LMAS propellant to reach lunar orbit
    let ascent_prop_burned_kg = (as_.propellant_mass.value() * 0.86).min(as_.propellant_mass.value());
    let ascent_burn_duration_s = ascent_prop_burned_kg / ascent_mdot;

    // Transition to LunarSurfaceOps if within structural velocity limits, else AbortToLunarOrbit
    let vel_req = LMDSVelocityLimits::default();
    if touchdown_vy <= vel_req.max_touchdown_vertical_velocity.value()
        && touchdown_vx <= vel_req.max_touchdown_horizontal_velocity.value()
    {
        phases
            .step_on::<PoweredDescentPhaseCompletedNotification>()
            .expect("PoweredDescent -> LunarSurfaceOps");
    } else {
        phases
            .step_on::<PoweredDescentAbortTrigger>()
            .expect("PoweredDescent -> AbortToLunarOrbit");
    }
    phase_history.push(phases.sysml_state_name().to_string());

    DescentTelemetry {
        phase_history,
        final_phase: phases.sysml_state_name().to_string(),
        touchdown_vertical_velocity_m_s: round3(touchdown_vy),
        touchdown_horizontal_velocity_m_s: round3(touchdown_vx),
        landing_zone_deviation_m: round3(fault.landing_zone_offset_m),
        remaining_descent_propellant_kg: round3(prop_remaining_kg),
        dps_min_throttle_percent: ds.min_throttle_percent.value(),
        dps_max_throttle_percent: ds.max_throttle_percent.value(),
        ascent_burn_duration_s: round3(ascent_burn_duration_s),
    }
}

/// Evaluate all Apollo 11 Powered Descent requirements against simulated telemetry
/// and return a JSON-serializable `VerificationReport`.
pub fn verify_scenario(scenario_name: &str, fault: &FaultProfile) -> VerificationReport {
    let mut spacecraft = ApolloSpacecraft::apollo11();
    let telemetry = simulate_powered_descent(&mut spacecraft, fault);

    let mut verdicts = Vec::new();

    // 1. HLR-R002: LunarLanderSoftLandingRequirement
    let mut hlr_r002 = LunarLanderSoftLandingRequirement::default();
    hlr_r002.actual_vertical_velocity = Quantity::m_per_s(telemetry.touchdown_vertical_velocity_m_s);
    hlr_r002.actual_horizontal_velocity =
        Quantity::m_per_s(telemetry.touchdown_horizontal_velocity_m_s);
    hlr_r002.actual_landing_zone_deviation = Quantity::m(telemetry.landing_zone_deviation_m);
    let hlr_ok = hlr_r002.actual_vertical_velocity.value()
        <= hlr_r002.max_vertical_velocity.value()
        && hlr_r002.actual_horizontal_velocity.value()
            <= hlr_r002.max_horizontal_velocity.value()
        && hlr_r002.actual_landing_zone_deviation.value()
            <= hlr_r002.max_landing_zone_deviation.value();
    verdicts.push(VerificationVerdict {
        req_id: LunarLanderSoftLandingRequirement::REQ_ID.to_string(),
        req_name: LunarLanderSoftLandingRequirement::SYSML_NAME.to_string(),
        status: if hlr_ok {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        scenario: scenario_name.to_string(),
        satisfied_by: "ApolloSpacecraft.lunarModule".to_string(),
        evidence: json!({
            "actual_vertical_velocity_m_s": telemetry.touchdown_vertical_velocity_m_s,
            "max_vertical_velocity_m_s": hlr_r002.max_vertical_velocity.value(),
            "actual_horizontal_velocity_m_s": telemetry.touchdown_horizontal_velocity_m_s,
            "max_horizontal_velocity_m_s": hlr_r002.max_horizontal_velocity.value(),
            "actual_landing_zone_deviation_m": telemetry.landing_zone_deviation_m,
            "max_landing_zone_deviation_m": hlr_r002.max_landing_zone_deviation.value(),
            "final_phase": telemetry.final_phase,
        }),
        message: if hlr_ok {
            "Soft landing within Tranquility Base limits achieved".to_string()
        } else {
            "Soft landing envelope violated".to_string()
        },
    });

    // 2. CLR-R050: LMDSVelocityLimits
    let mut clr_r050 = LMDSVelocityLimits::default();
    clr_r050.actual_vertical_velocity = Quantity::m_per_s(telemetry.touchdown_vertical_velocity_m_s);
    clr_r050.actual_horizontal_velocity =
        Quantity::m_per_s(telemetry.touchdown_horizontal_velocity_m_s);
    let vel_ok = clr_r050.actual_vertical_velocity.value()
        <= clr_r050.max_touchdown_vertical_velocity.value()
        && clr_r050.actual_horizontal_velocity.value()
            <= clr_r050.max_touchdown_horizontal_velocity.value();
    verdicts.push(VerificationVerdict {
        req_id: LMDSVelocityLimits::REQ_ID.to_string(),
        req_name: LMDSVelocityLimits::SYSML_NAME.to_string(),
        status: if vel_ok {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        scenario: scenario_name.to_string(),
        satisfied_by: "ApolloSpacecraft.lunarModule.lunarModuleDescentStage".to_string(),
        evidence: json!({
            "actual_vertical_velocity_m_s": telemetry.touchdown_vertical_velocity_m_s,
            "limit_vertical_velocity_m_s": clr_r050.max_touchdown_vertical_velocity.value(),
            "actual_horizontal_velocity_m_s": telemetry.touchdown_horizontal_velocity_m_s,
            "limit_horizontal_velocity_m_s": clr_r050.max_touchdown_horizontal_velocity.value(),
        }),
        message: if vel_ok {
            "Touchdown velocities within footpad structural limits".to_string()
        } else {
            "Footpad touchdown velocity structural limit exceeded".to_string()
        },
    });

    // 3. CLR-R057: DPSThrottlingRange
    let mut clr_r057 = DPSThrottlingRange::default();
    clr_r057.actual_min_throttle_percentage = Quantity::percent(telemetry.dps_min_throttle_percent);
    clr_r057.actual_max_throttle_percentage = Quantity::percent(telemetry.dps_max_throttle_percent);
    let dps_ok = clr_r057.actual_min_throttle_percentage.value()
        <= clr_r057.min_required_throttle_percentage.value()
        && clr_r057.actual_max_throttle_percentage.value()
            >= clr_r057.max_required_throttle_percentage.value();
    verdicts.push(VerificationVerdict {
        req_id: DPSThrottlingRange::REQ_ID.to_string(),
        req_name: DPSThrottlingRange::SYSML_NAME.to_string(),
        status: if dps_ok {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        scenario: scenario_name.to_string(),
        satisfied_by: "ApolloSpacecraft.lunarModule.lunarModuleDescentStage".to_string(),
        evidence: json!({
            "actual_min_throttle_percent": telemetry.dps_min_throttle_percent,
            "required_min_throttle_percent": clr_r057.min_required_throttle_percentage.value(),
            "actual_max_throttle_percent": telemetry.dps_max_throttle_percent,
            "required_max_throttle_percent": clr_r057.max_required_throttle_percentage.value(),
        }),
        message: if dps_ok {
            "DPS throttling range covers 10%..100%".to_string()
        } else {
            "DPS throttling range degraded below 10%..100% envelope".to_string()
        },
    });

    // 4. CLR-R058: LMASBurnDuration
    let mut clr_r058 = LMASBurnDuration::default();
    clr_r058.actual_ascent_burn_duration = Quantity::seconds(telemetry.ascent_burn_duration_s);
    let lmas_ok = clr_r058.actual_ascent_burn_duration.value()
        <= clr_r058.max_ascent_burn_duration.value();
    verdicts.push(VerificationVerdict {
        req_id: LMASBurnDuration::REQ_ID.to_string(),
        req_name: LMASBurnDuration::SYSML_NAME.to_string(),
        status: if lmas_ok {
            VerificationStatus::Pass
        } else {
            VerificationStatus::Fail
        },
        scenario: scenario_name.to_string(),
        satisfied_by: "ApolloSpacecraft.lunarModule.lunarModuleAscentStage".to_string(),
        evidence: json!({
            "actual_ascent_burn_duration_s": telemetry.ascent_burn_duration_s,
            "max_ascent_burn_duration_s": clr_r058.max_ascent_burn_duration.value(),
            "remaining_descent_propellant_kg": telemetry.remaining_descent_propellant_kg,
        }),
        message: if lmas_ok {
            "LMAS ascent burn achieves lunar orbit within 400 s limit".to_string()
        } else {
            "LMAS ascent burn exceeds 400 s duration limit".to_string()
        },
    });

    VerificationReport::from_verdicts("Apollo11PoweredDescentSlice", verdicts)
}

pub fn run_nominal_verification() -> VerificationReport {
    verify_scenario("nominal_tranquility_base_landing", &FaultProfile::nominal())
}

pub fn run_fault_injection_verification() -> VerificationReport {
    verify_scenario(
        "fault_dps_throttle_stuck_65pct",
        &FaultProfile::dps_throttle_valve_fault(),
    )
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate4_nominal_powered_descent_verifies_all_requirements() {
        let report = run_nominal_verification();
        assert!(
            report.all_passed,
            "Expected nominal scenario to pass all requirements, got: {}",
            serde_json::to_string_pretty(&report).unwrap()
        );
        assert_eq!(report.passed, 4);
        assert_eq!(report.failed, 0);
        // Verify JSON serialization contains exact Apollo 11 requirement IDs
        let json_str = serde_json::to_string(&report).unwrap();
        assert!(json_str.contains("HLR-R002"));
        assert!(json_str.contains("CLR-R050"));
        assert!(json_str.contains("CLR-R057"));
        assert!(json_str.contains("CLR-R058"));
    }

    #[test]
    fn gate4_fault_injection_detects_requirement_violations() {
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
    }

    #[test]
    fn gate4_state_machine_rejects_illegal_mission_phase_jump() {
        let mut phases = Apollo11MissionPhases::initial_state();
        assert_eq!(phases, Apollo11MissionPhases::Initial);
        phases.step("auto").unwrap();
        assert_eq!(phases, Apollo11MissionPhases::Preparation);
        // Trying to fire PoweredDescentPhaseCompletedNotification directly from Preparation must fail!
        let err = phases
            .step_on::<PoweredDescentPhaseCompletedNotification>()
            .unwrap_err();
        assert_eq!(err.from, "preparation");
        assert_eq!(err.trigger, "PoweredDescentPhaseCompletedNotification");
        assert_eq!(phases, Apollo11MissionPhases::Preparation);
    }
}
