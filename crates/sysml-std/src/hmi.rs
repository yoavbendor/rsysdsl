//! Human-Machine Interface (HMI) abstractions: human operator, input controls, displays, and chimes.

use sysml_dsl::prelude::*;

/// sysml-std library package. Abstract HMI bases declare units with no placeholder defaults;
/// concrete `HumanOperator` carries reference human defaults (`rsysdsl lint` reports inherited
/// ones as `inherited_std_default`).
#[sysml::package(library, name = "SysmlStdHmi")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
#[sysml::import("SysmlStdMechanics::*")]
pub mod sysml_std_hmi {
    use super::*;
    use crate::flows::sysml_std_flows::{
        AcousticPort, CommandPort, ElectricalPowerPort, HmiFeedbackPort, HmiInputPort,
        MechanicalPowerPort, TelemetryPort, ThermalPort,
    };
    use crate::mechanics::sysml_std_mechanics::PhysicalComponent;

    /// Human operator / rider / appliance user interacting via controls, displays, and mechanical effort.
    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct HumanOperator {
        #[sysml::attr(default = 75.0, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(default = 200.0, unit = "W")]
        pub sustained_mech_power: Quantity<f64>,
        #[sysml::port]
        pub user_input: HmiInputPort,
        #[sysml::port]
        pub user_feedback: Conjugated<HmiFeedbackPort>,
        #[sysml::port]
        pub mech_effort: MechanicalPowerPort,
    }

    /// Abstract zero-electricity mechanical HMI control (handlebars, brake levers, pedals, knobs).
    #[sysml::part(abstract)]
    #[sysml::specializes(PhysicalComponent)]
    #[derive(Debug, Clone, Copy)]
    pub struct MechanicalHmiControl {
        #[sysml::port]
        pub user_in: Conjugated<HmiInputPort>,
    }

    /// Abstract powered HMI control panel bridging human interaction with embedded electronics.
    #[sysml::part(abstract)]
    #[derive(Debug, Clone, Copy)]
    pub struct HmiPanel {
        #[sysml::attr(unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(unit = "W")]
        pub power_draw: Quantity<f64>,
        #[sysml::attr(dimensionless)]
        pub chime_level_db: f64,
        #[sysml::port]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port]
        pub user_in: Conjugated<HmiInputPort>,
        #[sysml::port]
        pub user_out: HmiFeedbackPort,
        #[sysml::port]
        pub cmd_to_logic: CommandPort,
        #[sysml::port]
        pub status_from_logic: Conjugated<TelemetryPort>,
        #[sysml::port]
        pub chime_out: AcousticPort,
        #[sysml::port]
        pub heat_out: ThermalPort,
    }
}
