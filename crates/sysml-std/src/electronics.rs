//! Embedded electronics, microcontrollers, and physical sensors.

use sysml_dsl::prelude::*;

/// sysml-std library package. Abstract electronics bases declare units with no placeholder
/// defaults; concrete sensor types carry generic library defaults that specializing models
/// should redefine when their source specifies values (`rsysdsl lint` reports inherited ones
/// as `inherited_std_default`).
#[sysml::package(library, name = "SysmlStdElectronics")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
pub mod sysml_std_electronics {
    use super::*;
    use crate::flows::sysml_std_flows::{
        CommandPort, DataBusPort, ElectricalPowerPort, FluidPort, TelemetryPort, ThermalPort,
    };

    /// Abstract electronic PCB/IC unit with DC power input and waste heat dissipation.
    #[sysml::part(abstract)]
    #[derive(Debug, Clone, Copy)]
    pub struct ElectronicUnit {
        #[sysml::attr(budget = "mass", unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(unit = "V")]
        pub supply_voltage: Quantity<f64>,
        #[sysml::attr(budget = "power", unit = "W")]
        pub power_draw: Quantity<f64>,
        #[sysml::port]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port]
        pub heat_out: ThermalPort,
    }

    /// Abstract embedded microcontroller executing control logic and state machines.
    #[sysml::part(abstract)]
    #[sysml::specializes(ElectronicUnit)]
    #[derive(Debug, Clone, Copy)]
    pub struct Microcontroller {
        #[sysml::attr(unit = "Hz")]
        pub clock_freq: Quantity<f64>,
        #[sysml::port]
        pub sensor_in: Conjugated<TelemetryPort>,
        #[sysml::port]
        pub cmd_out: CommandPort,
        #[sysml::port]
        pub bus: DataBusPort,
    }

    /// Abstract physical-to-digital transducer / sensor.
    #[sysml::part(abstract)]
    #[sysml::specializes(ElectronicUnit)]
    #[derive(Debug, Clone, Copy)]
    pub struct SensorBase {
        #[sysml::attr(unit = "Hz")]
        pub sample_rate: Quantity<f64>,
        #[sysml::port]
        pub reading_out: TelemetryPort,
    }

    /// Temperature sensor (NTC thermistor / RTD / thermocouple).
    #[sysml::part]
    #[sysml::specializes(SensorBase)]
    #[derive(Debug, Clone, Copy)]
    pub struct TemperatureSensor {
        #[sysml::attr(default = 373.15, unit = "K")]
        pub max_measurable_temp: Quantity<f64>,
        #[sysml::port]
        pub thermal_sense: Conjugated<ThermalPort>,
    }

    /// Fluid property sensor (water level, turbidity, flow rate, or pressure).
    #[sysml::part]
    #[sysml::specializes(SensorBase)]
    #[derive(Debug, Clone, Copy)]
    pub struct FluidSensor {
        #[sysml::port]
        pub fluid_sense: Conjugated<FluidPort>,
    }
}
