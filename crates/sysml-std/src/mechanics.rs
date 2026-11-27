//! Mechanical, fluidic, thermal-management, and environmental sink components.

use sysml_dsl::prelude::*;

#[sysml::package(library, name = "SysmlStdMechanics")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
pub mod sysml_std_mechanics {
    use super::*;
    use crate::flows::sysml_std_flows::{
        AcousticPort, CommandPort, ElectricalPowerPort, FluidPort, HmiInputPort,
        MechanicalPowerPort, RadiativePort, TelemetryPort, ThermalPort,
    };

    /// Abstract physical component with mass, thermal rejection, and acoustic emission.
    #[sysml::part(abstract)]
    #[derive(Debug, Clone, Copy)]
    pub struct PhysicalComponent {
        #[sysml::attr(default = 1.0, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(default = 0.0, unit = "W")]
        pub heat_dissipation: Quantity<f64>,
        #[sysml::attr(default = 30.0)]
        pub noise_level_db: f64,
        #[sysml::port]
        pub heat_out: ThermalPort,
        #[sysml::port]
        pub noise_out: AcousticPort,
    }

    /// Abstract load-bearing structural frame / chassis / enclosure.
    #[sysml::part(abstract)]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct StructuralFrame {
        #[sysml::attr(default = 500.0, unit = "N")]
        pub max_load: Quantity<f64>,
    }

    /// Abstract thermally & acoustically insulated enclosure/cavity/tub (dishwasher tub, oven cavity).
    #[sysml::part(abstract)]
    #[sysml::specializes(StructuralFrame)]
    #[derive(Debug, Clone, Copy)]
    pub struct ThermalEnclosure {
        #[sysml::attr(default = 2.0)]
        pub thermal_conductance_w_per_k: f64,
        #[sysml::attr(default = 12.0)]
        pub acoustic_attenuation_db: f64,
        #[sysml::port]
        pub heat_in: Conjugated<ThermalPort>,
    }

    /// Abstract electromagnetic / radiative waveguide or optical conduit.
    #[sysml::part(abstract)]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct RadiativeConduit {
        #[sysml::attr(default = 0.96)]
        pub transmission_efficiency: f64,
        #[sysml::port]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port]
        pub radiation_out: RadiativePort,
    }

    /// Abstract friction brake converting kinetic/mechanical energy into braking heat.
    #[sysml::part(abstract)]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct FrictionBrake {
        #[sysml::attr(default = 800.0, unit = "N")]
        pub max_braking_force: Quantity<f64>,
        #[sysml::attr(default = 1500.0, unit = "W")]
        pub thermal_capacity_power: Quantity<f64>,
        #[sysml::port]
        pub actuation_in: Conjugated<HmiInputPort>,
    }

    /// Abstract rotating wheel/tire assembly with rolling resistance and road/hub acoustic emission.
    #[sysml::part(abstract)]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct RotatingWheel {
        #[sysml::attr(default = 0.34, unit = "m")]
        pub wheel_radius: Quantity<f64>,
        #[sysml::attr(default = 0.004)]
        pub rolling_resistance_coeff: f64,
        #[sysml::port]
        pub drive_in: Conjugated<MechanicalPowerPort>,
    }

    /// Abstract hydraulic spray arm / nozzle / fluid manifold distributor.
    #[sysml::part(abstract)]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct FluidDistributor {
        #[sysml::port]
        pub fluid_in: Conjugated<FluidPort>,
        #[sysml::port]
        pub fluid_out: FluidPort,
    }

    /// Abstract electric rotary motor converting electrical power to mechanical shaft power.
    #[sysml::part(abstract)]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct ElectricMotor {
        #[sysml::attr(default = 300.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(default = 0.85)]
        pub efficiency: f64,
        #[sysml::attr(default = 50.0, unit = "Hz")]
        pub rated_rotation_freq: Quantity<f64>,
        #[sysml::port]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port]
        pub cmd_in: Conjugated<CommandPort>,
        #[sysml::port]
        pub mech_out: MechanicalPowerPort,
    }

    /// High-efficiency brushless DC motor (cordless power tools, variable-speed pumps).
    #[sysml::part]
    #[sysml::specializes(ElectricMotor)]
    #[derive(Debug, Clone, Copy)]
    pub struct BrushlessDcMotor {
        #[sysml::attr(redefines = "efficiency", default = 0.88)]
        pub efficiency: f64,
    }

    /// Abstract mechanical transmission (gearbox, chain drive, belt, reducer).
    #[sysml::part(abstract)]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct MechanicalTransmission {
        #[sysml::attr(default = 1.0)]
        pub gear_ratio: f64,
        #[sysml::attr(default = 0.94)]
        pub efficiency: f64,
        #[sysml::port]
        pub mech_in: Conjugated<MechanicalPowerPort>,
        #[sysml::port]
        pub mech_out: MechanicalPowerPort,
    }

    /// Abstract electrically driven fluid pump (circulation, drain, coolant).
    #[sysml::part(abstract)]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct FluidPump {
        #[sysml::attr(default = 0.25, unit = "kg/s")]
        pub flow_rate: Quantity<f64>,
        #[sysml::attr(default = 60.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::port]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port]
        pub cmd_in: Conjugated<CommandPort>,
        #[sysml::port]
        pub fluid_in: Conjugated<FluidPort>,
        #[sysml::port]
        pub fluid_out: FluidPort,
    }

    /// Electrically actuated fluid solenoid valve.
    #[sysml::part]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct FluidValve {
        #[sysml::attr(default = 0.10, unit = "kg/s")]
        pub max_flow_rate: Quantity<f64>,
        #[sysml::port]
        pub cmd_in: Conjugated<CommandPort>,
        #[sysml::port]
        pub fluid_in: Conjugated<FluidPort>,
        #[sysml::port]
        pub fluid_out: FluidPort,
    }

    /// Forced-air cooling fan removing waste heat and emitting fan noise.
    #[sysml::part]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct CoolingFan {
        #[sysml::attr(default = 0.03, unit = "kg/s")]
        pub airflow_rate: Quantity<f64>,
        #[sysml::attr(default = 15.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::port]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port]
        pub heat_in: Conjugated<ThermalPort>,
    }

    /// Mechanical safety interlock switch (e.g. appliance door latch, saw guard, trigger lock).
    #[sysml::part]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct SafetyInterlockSwitch {
        #[sysml::attr(default = 0.015, unit = "s")]
        pub cutoff_latency: Quantity<f64>,
        #[sysml::port]
        pub mechanical_actuation: Conjugated<HmiInputPort>,
        #[sysml::port]
        pub interlock_status: TelemetryPort,
    }

    /// Ambient room / outdoor thermal and acoustic sink for physical boundary closure.
    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct AmbientEnvironment {
        #[sysml::attr(default = 295.15, unit = "K")]
        pub ambient_temperature: Quantity<f64>,
        #[sysml::attr(default = 101325.0, unit = "Pa")]
        pub ambient_pressure: Quantity<f64>,
        #[sysml::port]
        pub heat_sink: Conjugated<ThermalPort>,
        #[sysml::port]
        pub noise_sink: Conjugated<AcousticPort>,
    }
}
