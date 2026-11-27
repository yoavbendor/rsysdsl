//! `sysml-std` — Universal SI-unit Engineering Standard Library for `rsysdsl`.
//!
//! Provides reusable, OMG-valid SysML v2 abstract hierarchies across:
//! - [`flows`]: Conserved Energy, Material/Fluid, Thermal, Radiative, Acoustic, and Information flows & physics calcs
//! - [`comms`]: `InfoChannel` & `DigitalBusChannel` -> UART, I2C, SPI, CAN-FD, Ethernet, HMI channels
//! - [`power`]: `PowerSource` (`AcMainsSupply`, `BatteryPack`), `PowerConverter`, `ResistiveHeater`
//! - [`mechanics`]: `PhysicalComponent`, `StructuralFrame`, `ElectricMotor`, `MechanicalTransmission`, `FluidPump`, `FluidValve`, `CoolingFan`, `SafetyInterlockSwitch`, `AmbientEnvironment`
//! - [`electronics`]: `ElectronicUnit`, `Microcontroller`, `SensorBase`, `TemperatureSensor`, `FluidSensor`
//! - [`hmi`]: `HumanOperator`, `HmiPanel`
//! - [`natural`]: `NaturalBody`, `StarBody`, `PlanetaryBody` (`RockyPlanet`, `GasGiant`, `IceGiant`, `NaturalSatellite`), `FluidReservoir`
//!
//! ## Library defaults policy
//! Abstract bases (`PowerElement`, `PowerSource`, `BatteryPack`, `PhysicalComponent`,
//! `ElectricMotor`, `Microcontroller`, `SensorBase`, `HmiPanel`, `NaturalBody`, etc.), flow
//! items (`ElectricalEnergy`, `ThermalEnergy`, `FluidFlow`, ...), and generic links
//! (`ElectricalLink`, `MechanicalLink`, `CommandChannel`, `TelemetryChannel`) declare SI units
//! with **no numeric defaults**, so specializing models never inherit fictional placeholders.
//! Concrete standard parts and protocol channels (`AcMainsSupply`, `UartChannel`, `I2cChannel`,
//! `SpiChannel`, `CanFdChannel`, `EthernetChannel`, `AmbientEnvironment`, `DeepSpaceSink`, etc.)
//! carry normative/reference library defaults: a specializing model must redefine every value its
//! source specifies (`#[sysml::attr(redefines = "mainsFrequency", default = 60.0, unit = "Hz")]`),
//! or blank any open one with `redefines = "...", tbd_value = "reason"`. `rsysdsl lint` reports
//! inherited concrete defaults as `inherited_std_default` (info).
//! Emit with `emit --omit-std` to keep these packages out of the model's `.sysml` file.

pub mod comms;
pub mod electronics;
pub mod flows;
pub mod hmi;
pub mod mechanics;
pub mod natural;
pub mod power;

/// Convenience re-export of all standard library SysML packages and Rust types.
#[allow(ambiguous_glob_reexports)]
pub mod prelude {
    pub use crate::comms::sysml_std_comms::*;
    pub use crate::electronics::sysml_std_electronics::*;
    pub use crate::flows::sysml_std_flows::*;
    pub use crate::hmi::sysml_std_hmi::*;
    pub use crate::mechanics::sysml_std_mechanics::*;
    pub use crate::natural::sysml_std_natural::*;
    pub use crate::power::sysml_std_power::*;
}

/// Returns all `sysml-std` library packages in dependency order for `Model::new(...)`.
pub fn std_packages() -> Vec<sysml_dsl::ir::Package> {
    vec![
        flows::sysml_std_flows::sysml_package(),
        comms::sysml_std_comms::sysml_package(),
        power::sysml_std_power::sysml_package(),
        mechanics::sysml_std_mechanics::sysml_package(),
        electronics::sysml_std_electronics::sysml_package(),
        hmi::sysml_std_hmi::sysml_package(),
        natural::sysml_std_natural::sysml_package(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysml_dsl::ir::Model;
    use sysml_emit::{emit_model, validate_sysml_source_with_stdlib};

    #[test]
    fn sysml_std_validates_zero_errors_zero_warnings_against_omg_stdlib() {
        let model = Model::new(std_packages());
        let text = emit_model(&model);
        let report = validate_sysml_source_with_stdlib("sysml_std.sysml", &text, None);
        assert!(
            report.valid,
            "OMG stdlib validation errors:\n{:#?}\n\nEmitted SysML:\n{}",
            report, text
        );
        assert_eq!(
            report.warning_count, 0,
            "OMG stdlib warnings:\n{:#?}\n\nEmitted SysML:\n{}",
            report.warnings, text
        );
    }

    #[test]
    fn sysml_std_physics_calcs_execute_accurately() {
        use crate::flows::sysml_std_flows::*;
        assert!((electrical_power_watts(18.0, 20.0) - 360.0).abs() < 1e-9);
        assert!((useful_power_watts(1000.0, 0.85) - 850.0).abs() < 1e-9);
        assert!((waste_heat_watts(1000.0, 0.85) - 150.0).abs() < 1e-9);
        assert!((mechanical_power_watts(120.0, 2.5) - 300.0).abs() < 1e-9);
        assert!((thermal_rise_k(50.0, 0.4) - 20.0).abs() < 1e-9);
        assert!((mass_balance_rate(15.0, 15.0)).abs() < 1e-9);
    }
}
