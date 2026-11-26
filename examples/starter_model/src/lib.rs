//! Minimal 1-file starter template for authoring a new system in `rsysdsl`.
//!
//! Copy this crate to start a new Visio diagram or system architecture model:
//! 1. Define payloads (`#[sysml::item]`), ports (`#[sysml::port]`), and interfaces (`#[sysml::interface]`).
//! 2. Define subsystem blocks (`#[sysml::part]`) and one top-level assembly composing them with `#[sysml::connect]`.
//! 3. Add requirements (`#[sysml::requirement]`) with `#[sysml::rationale("Source: ...")]` provenance and `#[sysml::satisfy]`.
//! 4. Run `cargo test -p starter-model` and `cargo run -q -p starter-model --bin emit -- -o target/starter_model.sysml`.
//! 5. Open `target/starter_model.sysml` in VS Code (SysML v2 Support / `sysml-reactflow`).

use sysml_dsl::prelude::*;

#[sysml::package(name = "StarterArchitecture")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("ModelingMetadata::*")]
pub mod starter_architecture {
    use super::*;

    /// Temperature telemetry sample sent from the sensor to the controller.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct TempReading {
        #[sysml::attr(default = 295.0, unit = "K")]
        pub temperature: Quantity<f64>,
    }

    /// Cooling command sent from the controller to the thermal actuator.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct CoolingCommand {
        #[sysml::attr(default = 0.0, unit = "W")]
        pub target_cooling_power: Quantity<f64>,
    }

    /// Directed telemetry output port (connect to `Conjugated<TempSensorPort>`).
    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct TempSensorPort {
        #[sysml::out_item]
        pub reading: TempReading,
    }

    /// Directed actuator command port (connect to `Conjugated<ActuatorCmdPort>`).
    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ActuatorCmdPort {
        #[sysml::out_item]
        pub command: CoolingCommand,
    }

    /// Typed interface between temperature sensor and controller.
    #[sysml::interface]
    pub struct SensorBus {
        #[sysml::end]
        pub source: TempSensorPort,
        #[sysml::end]
        pub sink: Conjugated<TempSensorPort>,
    }

    /// Typed interface between controller and cooling actuator.
    #[sysml::interface]
    pub struct ActuatorBus {
        #[sysml::end]
        pub source: ActuatorCmdPort,
        #[sysml::end]
        pub sink: Conjugated<ActuatorCmdPort>,
    }

    /// Requirement with explicit document provenance and a compiled constraint.
    #[sysml::requirement(id = "SYS-REQ-001")]
    #[sysml::rationale("Source: Thermal Control Concept v1.0, Section 3.2 (max operating temperature 350 K)")]
    #[sysml::require_constraint(measured_temp_k <= max_temp_k)]
    #[derive(Debug, Clone, Copy)]
    pub struct MaxOperatingTemperature {
        #[sysml::attr(default = 350.0, unit = "K")]
        pub max_temp_k: Quantity<f64>,
        #[sysml::attr(default = 310.0, unit = "K")]
        pub measured_temp_k: Quantity<f64>,
    }

    /// Temperature sensing subsystem (Visio box 1).
    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct TemperatureSensor {
        #[sysml::attr(default = 10.0, unit = "Hz")]
        pub sample_rate: Quantity<f64>,
        #[sysml::port]
        pub telemetry_out: TempSensorPort,
    }

    /// Thermal control unit subsystem (Visio box 2).
    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct ThermalController {
        #[sysml::attr(default = 0.05, unit = "s")]
        pub control_period: Quantity<f64>,
        #[sysml::port]
        pub telemetry_in: Conjugated<TempSensorPort>,
        #[sysml::port]
        pub cmd_out: ActuatorCmdPort,
    }

    /// Active cooling actuator subsystem (Visio box 3).
    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct CoolingActuator {
        #[sysml::attr(default = 250.0, unit = "W")]
        pub max_cooling_power: Quantity<f64>,
        #[sysml::port]
        pub cmd_in: Conjugated<ActuatorCmdPort>,
    }

    /// Top-level system context composing all subsystems and Visio arrows for IBD rendering.
    #[sysml::part]
    #[sysml::satisfy(req = MaxOperatingTemperature, id = "SYS-REQ-001")]
    #[sysml::connect(
        interface = SensorBus,
        name = "sensorToController",
        from = sensor.telemetry_out,
        to = controller.telemetry_in
    )]
    #[sysml::connect(
        interface = ActuatorBus,
        name = "controllerToActuator",
        from = controller.cmd_out,
        to = actuator.cmd_in
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct StarterThermalSystem {
        #[sysml::part]
        pub sensor: TemperatureSensor,
        #[sysml::part]
        pub controller: ThermalController,
        #[sysml::part]
        pub actuator: CoolingActuator,
    }
}

/// Assemble all `#[sysml::package]` modules in this crate into a single `sysml_ir::Model`.
pub fn build_model() -> Model {
    Model::new(vec![starter_architecture::sysml_package()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysml_emit::{audit_model_with_stdlib, emit_model};

    #[test]
    fn starter_model_emits_and_audits_zero_errors_zero_warnings() {
        let model = build_model();
        let sysml = emit_model(&model);
        assert!(sysml.contains("package StarterArchitecture"));
        assert!(sysml.contains("interface sensorToController : SensorBus connect sensor.telemetryOut to controller.telemetryIn;"));

        let audit = audit_model_with_stdlib("starter_model", &model, None, None, true);
        assert!(audit.stdlib_loaded, "OMG sysml.library must be loaded");
        assert!(
            audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
            "starter_model audit failed: {audit:#?}\n--- SYSML ---\n{sysml}"
        );
    }

    #[test]
    fn starter_model_requirement_constraint_executes() {
        let nominal = starter_architecture::MaxOperatingTemperature::default();
        assert!(nominal.evaluate());

        let overheat = starter_architecture::MaxOperatingTemperature {
            measured_temp_k: Quantity::new(365.0, "K"),
            ..starter_architecture::MaxOperatingTemperature::default()
        };
        assert!(!overheat.evaluate());
    }
}
