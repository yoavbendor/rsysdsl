//! Ferrocene-ready `#![no_std]` ISO 26262 ASIL-D Flight Controller example.
//!
//! Compile-checks with zero `std`, zero `alloc`, and zero `serde`:
//! ```bash
//! cargo check -p embedded-flight-controller --no-default-features
//! ```
//! While with the host `reflect` feature enabled (default), the EXACT SAME source
//! emits OMG-valid SysML v2 containing `@SafetyGoal`, `@FMEA`, and `@AsilDecomposition`
//! metadata annotations and validates with 0 errors and 0 warnings against Open-MBEE + OMG stdlib!

#![no_std]

#[cfg(any(test, feature = "reflect"))]
extern crate std;

use sysml_dsl::prelude::*;

#[sysml::package(no_std, name = "EmbeddedFlightController")]
#[sysml::import("ISO26262Metadata::*")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("ISQ::*")]
#[sysml::import("SI::*")]
pub mod flight_controller {
    use super::*;

    /// Top-level ISO 26262 ASIL-D safety goal: prevent uncommanded thrust by closing
    /// propellant cutoff valves within the 50 ms (0.050 s) Fault Tolerant Time Interval (FTTI).
    #[sysml::requirement(id = "SG-001")]
    #[sysml::safety_goal(asil = AsilD, safe_state = "CutOffValvesClosed", ftti_ms = 50)]
    #[sysml::require_constraint(actual_cutoff_latency_s <= max_ftti_s)]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct PreventUncommandedThrust {
        #[sysml::attr(default = 0.050, unit = "s")]
        pub max_ftti_s: Quantity<f64>,
        #[sysml::attr(default = 0.018, unit = "s")]
        pub actual_cutoff_latency_s: Quantity<f64>,
    }

    /// Valve cutoff command transmitted over the dual-channel actuator bus.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct ValveCutoffCommand {
        #[sysml::attr(default = 1.0)]
        pub close_valve: f64,
    }

    /// Telemetry event emitted by the independent monitor when thrust exceeds safe limits.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FaultSensorTelemetry {
        #[sysml::attr(default = 0.0, unit = "s")]
        pub detected_overthrust_s: f64,
    }

    /// Directed actuator command port.
    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ActuatorCommandPort {
        #[sysml::out_item]
        pub cmd: ValveCutoffCommand,
    }

    /// Typed interface binding an actuator driver to its valve cutoff receiver.
    #[sysml::interface]
    pub struct ActuatorBusInterface {
        #[sysml::end]
        pub driver: ActuatorCommandPort,
        #[sysml::end]
        pub valve_receiver: Conjugated<ActuatorCommandPort>,
    }

    /// Primary flight control channel (ASIL B(D)) — independent from `MonitorChannel`.
    #[sysml::part]
    #[sysml::independent(MonitorChannel)]
    #[sysml::fmea(
        failure_mode = "SpuriousThrottleHigh",
        effect = "UncommandedThrust",
        severity = 10,
        occurrence = 2,
        detection = 3,
        asil = AsilB
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct PrimaryControlChannel {
        #[sysml::attr(default = 0.012, unit = "s")]
        pub reaction_time_s: Quantity<f64>,
        #[sysml::port]
        pub cmd_out: ActuatorCommandPort,
    }

    /// Independent hardware/software monitor channel (ASIL B(D)) — independent from `PrimaryControlChannel`.
    #[sysml::part]
    #[sysml::independent(PrimaryControlChannel)]
    #[sysml::fmea(
        failure_mode = "WatchdogLatchStuck",
        effect = "DelayedSafeCutoff",
        severity = 9,
        occurrence = 1,
        detection = 2,
        asil = AsilB
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct MonitorChannel {
        #[sysml::attr(default = 0.006, unit = "s")]
        pub trip_latency_s: Quantity<f64>,
        #[sysml::port]
        pub cutoff_in: Conjugated<ActuatorCommandPort>,
    }

    /// Dual-channel ASIL-D flight controller assembly proving ISO 26262-9 ASIL D -> B(D) + B(D)
    /// decomposition with compile-time independence witnesses.
    #[sysml::part]
    #[sysml::satisfy(req = PreventUncommandedThrust, id = "SG-001")]
    #[sysml::asil_decompose(
        goal = PreventUncommandedThrust,
        left = PrimaryControlChannel,
        left_asil = AsilB,
        right = MonitorChannel,
        right_asil = AsilB
    )]
    #[sysml::connect(
        interface = ActuatorBusInterface,
        name = "primaryToMonitorCutoffLink",
        from = primary.cmd_out,
        to = monitor.cutoff_in
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct FlightControllerAssembly {
        #[sysml::attr(default = 0.050, unit = "s")]
        pub max_allowed_fault_duration_s: Quantity<f64>,
        #[sysml::part]
        pub primary: PrimaryControlChannel,
        #[sysml::part]
        pub monitor: MonitorChannel,
    }

    /// Guarded `#![no_std]` controller safety state machine.
    #[sysml::state(subject = FlightControllerAssembly)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ControllerMode {
        #[sysml::initial]
        #[sysml::transition(
            on = FaultSensorTelemetry,
            guard = event.detected_overthrust_s > subject.max_allowed_fault_duration_s,
            to = SafeCutoff
        )]
        Nominal,
        SafeCutoff,
    }

    /// Pure modeled calculation: remaining FTTI safety margin (s).
    #[sysml::calc]
    pub fn cutoff_margin_s(max_ftti_s: f64, actual_latency_s: f64) -> f64 {
        max_ftti_s - actual_latency_s
    }
}

#[cfg(test)]
mod tests {
    use super::flight_controller::*;
    use sysml_dsl::prelude::*;

    #[test]
    fn no_std_safety_contracts_and_state_machine_execute() {
        // Auto-generated defaults from SysML attributes
        let assembly = FlightControllerAssembly::default();
        assert_eq!(assembly.max_allowed_fault_duration_s.value, 0.050);
        assert_eq!(assembly.primary.reaction_time_s.value, 0.012);
        assert_eq!(assembly.monitor.trip_latency_s.value, 0.006);

        // Safety goal trait constants & compiled requirement constraint
        assert_eq!(PreventUncommandedThrust::REQ_ID, "SG-001");
        assert_eq!(<PreventUncommandedThrust as SafetyGoal>::Asil::LABEL, "ASIL_D");
        assert_eq!(PreventUncommandedThrust::SAFE_STATE, "CutOffValvesClosed");
        assert_eq!(PreventUncommandedThrust::FTTI_MS, 50);

        let total_latency = assembly.primary.reaction_time_s.value
            + assembly.monitor.trip_latency_s.value;
        let margin = cutoff_margin_s(0.050, total_latency);
        assert!((margin - 0.032).abs() < 1e-12);

        let sg_pass = PreventUncommandedThrust {
            actual_cutoff_latency_s: Quantity::new(total_latency, "s"),
            ..PreventUncommandedThrust::default()
        };
        assert!(sg_pass.evaluate());

        let sg_fail = PreventUncommandedThrust {
            actual_cutoff_latency_s: Quantity::new(0.065, "s"),
            ..PreventUncommandedThrust::default()
        };
        assert!(!sg_fail.evaluate());

        // Guarded #![no_std] state machine execution (zero heap allocation!)
        let mut mode = ControllerMode::initial_state();
        assert_eq!(mode, ControllerMode::Nominal);

        // Transient below 50 ms (0.015 s) does NOT trip safe cutoff
        let transient = FaultSensorTelemetry {
            detected_overthrust_s: 0.015,
        };
        assert!(mode.step_with(&transient, &assembly).is_err());
        assert_eq!(mode, ControllerMode::Nominal);

        // Sustained fault > 50 ms (0.055 s) deterministically transitions to SafeCutoff
        let fault = FaultSensorTelemetry {
            detected_overthrust_s: 0.055,
        };
        assert_eq!(
            mode.step_with(&fault, &assembly).unwrap(),
            ControllerMode::SafeCutoff
        );
    }

    #[cfg(feature = "reflect")]
    #[test]
    fn emitted_iso26262_sysml_validates_zero_errors_zero_warnings_with_stdlib() {
        use std::vec;
        use sysml_emit::{
            discover_omg_stdlib, emit_model, iso26262_metadata_package,
            validate_sysml_source_with_stdlib,
        };

        let model = Model::new(vec![
            iso26262_metadata_package(),
            crate::flight_controller::sysml_package(),
        ]);
        let sysml = emit_model(&model);

        assert!(sysml.contains("@SafetyGoal {"));
        assert!(sysml.contains("asil = \"ASIL_D\";"));
        assert!(sysml.contains("safeState = \"CutOffValvesClosed\";"));
        assert!(sysml.contains("fttiMs = 50;"));
        assert!(sysml.contains("@FMEA {"));
        assert!(sysml.contains("rpn = 60;"));
        assert!(sysml.contains("@AsilDecomposition {"));
        assert!(sysml.contains("goal = \"SG-001\";"));

        let stdlib = discover_omg_stdlib().expect("OMG sysml.library fixture must exist");
        let report = validate_sysml_source_with_stdlib(
            "embedded_flight_controller.sysml",
            &sysml,
            Some(stdlib.as_path()),
        );
        assert!(report.stdlib_loaded, "stdlib must load");
        assert!(
            report.valid && report.error_count == 0,
            "ISO 26262 SysML had errors: {:#?}\n--- SYSML ---\n{sysml}",
            (&report.syntax_errors, &report.context_errors, &report.model_errors)
        );
        assert_eq!(
            report.warning_count, 0,
            "ISO 26262 SysML had warnings: {:#?}\n--- SYSML ---\n{sysml}",
            report.warnings
        );
    }
}
