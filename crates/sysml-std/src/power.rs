//! Electrical power sources, converters, battery packs, and resistive/electronic loads.

use sysml_dsl::prelude::*;

/// sysml-std library package. Abstract power bases declare units with no placeholder defaults;
/// concrete standard supplies/stages carry generic library defaults that specializing models
/// should redefine when their source specifies values (`rsysdsl lint` reports inherited ones
/// as `inherited_std_default`).
#[sysml::package(library, name = "SysmlStdPower")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
pub mod sysml_std_power {
    use super::*;
    use crate::flows::sysml_std_flows::{
        AcousticPort, CommandPort, DataBusPort, ElectricalPowerPort, RadiativePort, TelemetryPort,
        ThermalPort,
    };

    /// Abstract base for any electrical power-handling subsystem with thermal dissipation.
    #[sysml::part(abstract)]
    #[derive(Debug, Clone, Copy)]
    pub struct PowerElement {
        #[sysml::attr(unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(dimensionless)]
        pub efficiency: f64,
        #[sysml::attr(unit = "W")]
        pub waste_heat: Quantity<f64>,
        #[sysml::port]
        pub heat_out: ThermalPort,
    }

    /// Abstract electrical power source delivering regulated or mains voltage.
    #[sysml::part(abstract)]
    #[sysml::specializes(PowerElement)]
    #[derive(Debug, Clone, Copy)]
    pub struct PowerSource {
        #[sysml::attr(unit = "V")]
        pub nominal_voltage: Quantity<f64>,
        #[sysml::attr(unit = "A")]
        pub max_current: Quantity<f64>,
        #[sysml::port]
        pub power_out: ElectricalPowerPort,
    }

    /// Wall AC mains power supply input (120V/230V, 50/60 Hz).
    #[sysml::part]
    #[sysml::specializes(PowerSource)]
    #[derive(Debug, Clone, Copy)]
    pub struct AcMainsSupply {
        #[sysml::attr(redefines = "nominalVoltage", default = 230.0, unit = "V")]
        pub nominal_voltage: Quantity<f64>,
        #[sysml::attr(redefines = "maxCurrent", default = 16.0, unit = "A")]
        pub max_current: Quantity<f64>,
        #[sysml::attr(default = 50.0, unit = "Hz")]
        pub mains_frequency: Quantity<f64>,
    }

    /// Abstract rechargeable battery pack with charge input, BMS bus/telemetry, and thermal limits.
    /// Inherits `nominal_voltage` and `max_current` from [`PowerSource`].
    #[sysml::part(abstract)]
    #[sysml::specializes(PowerSource)]
    #[derive(Debug, Clone, Copy)]
    pub struct BatteryPack {
        #[sysml::attr(unit = "J")]
        pub energy_capacity: Quantity<f64>,
        #[sysml::attr(dimensionless)]
        pub state_of_charge: f64,
        #[sysml::attr(unit = "K")]
        pub max_cell_temp: Quantity<f64>,
        #[sysml::port]
        pub charge_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port]
        pub bms_telemetry: TelemetryPort,
        #[sysml::port]
        pub bms_bus: DataBusPort,
    }

    /// Abstract electrical power converter (AC-DC PSU, DC-DC buck, HV transformer, inverter).
    #[sysml::part(abstract)]
    #[sysml::specializes(PowerElement)]
    #[derive(Debug, Clone, Copy)]
    pub struct PowerConverter {
        #[sysml::attr(unit = "V")]
        pub output_voltage: Quantity<f64>,
        #[sysml::port]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port]
        pub power_out: ElectricalPowerPort,
    }

    /// Switched-mode AC-DC power supply generating low-voltage DC logic rails.
    #[sysml::part]
    #[sysml::specializes(PowerConverter)]
    #[derive(Debug, Clone, Copy)]
    pub struct AcDcPowerSupply {
        #[sysml::attr(redefines = "outputVoltage", default = 12.0, unit = "V")]
        pub output_voltage: Quantity<f64>,
    }

    /// Smart CC/CV battery charger with digital BMS handshake bus.
    #[sysml::part]
    #[sysml::specializes(AcDcPowerSupply)]
    #[derive(Debug, Clone, Copy)]
    pub struct BatteryCharger {
        #[sysml::attr(default = 4.0, unit = "A")]
        pub max_charge_current: Quantity<f64>,
        #[sysml::port]
        pub bms_bus: DataBusPort,
    }

    /// High-voltage step-up power transformer (e.g. microwave HV magnetron supply).
    #[sysml::part]
    #[sysml::specializes(PowerConverter)]
    #[derive(Debug, Clone, Copy)]
    pub struct HighVoltageTransformer {
        #[sysml::attr(redefines = "outputVoltage", default = 2100.0, unit = "V")]
        pub output_voltage: Quantity<f64>,
        #[sysml::attr(dimensionless, default = 48.0)]
        pub hum_noise_db: f64,
        #[sysml::port]
        pub noise_out: AcousticPort,
    }

    /// Electronically controlled power stage / MOSFET inverter / relay switch.
    #[sysml::part]
    #[sysml::specializes(PowerConverter)]
    #[derive(Debug, Clone, Copy)]
    pub struct PowerSwitchStage {
        #[sysml::port]
        pub cmd_in: Conjugated<CommandPort>,
    }

    /// Three-phase PWM MOSFET inverter driving brushless motors with fault telemetry.
    #[sysml::part]
    #[sysml::specializes(PowerSwitchStage)]
    #[derive(Debug, Clone, Copy)]
    pub struct ThreePhaseInverter {
        #[sysml::attr(default = 20000.0, unit = "Hz")]
        pub pwm_carrier_freq: Quantity<f64>,
        #[sysml::attr(default = 60.0, unit = "A")]
        pub overcurrent_limit: Quantity<f64>,
        #[sysml::port]
        pub fault_telemetry: TelemetryPort,
    }

    /// Abstract electrical load consuming electrical power and rejecting waste heat.
    #[sysml::part(abstract)]
    #[sysml::specializes(PowerElement)]
    #[derive(Debug, Clone, Copy)]
    pub struct ElectricalLoad {
        #[sysml::port]
        pub power_in: Conjugated<ElectricalPowerPort>,
    }

    /// Resistive electric heating element converting electrical power into intentional heat.
    #[sysml::part]
    #[sysml::specializes(ElectricalLoad)]
    #[derive(Debug, Clone, Copy)]
    pub struct ResistiveHeater {
        #[sysml::port]
        pub heat_delivery: ThermalPort,
    }

    /// Electrical-to-electromagnetic emitter load (magnetron RF tube, IR/UV lamp, laser diode).
    #[sysml::part(abstract)]
    #[sysml::specializes(ElectricalLoad)]
    #[derive(Debug, Clone, Copy)]
    pub struct RadiativeEmitterLoad {
        #[sysml::attr(unit = "Hz")]
        pub emission_frequency: Quantity<f64>,
        #[sysml::attr(unit = "W")]
        pub radiative_output_power: Quantity<f64>,
        #[sysml::port]
        pub radiation_out: RadiativePort,
    }
}
