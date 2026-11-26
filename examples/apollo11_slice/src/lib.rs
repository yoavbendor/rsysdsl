//! Complete 5-layer vertical slice of Airbus Central R&T's Apollo 11 SysML v2
//! mission model authored directly in idiomatic, strongly typed Rust.
//!
//! Layers covered end-to-end:
//! 1. **CoSMA Base** (`cosma_package`): `MassedComponent`, `Function`, `LogicalComponent`,
//!    `HardwareComponent`, `PowerProvider`, `PowerConsumer`, `Phase`, `System`
//! 2. **Purpose / Mission Phases** (`mission_phases_package`): `PoweredDescentPhase` +
//!    executable `Apollo11MissionPhases` state machine
//! 3. **Requirements** (`requirements_package`): `HLR-R002`, `CLR-R018`, `CLR-R050`,
//!    `CLR-R057`, `CLR-R058`
//! 4. **Functions & Logical** (`functions_package`, `logical_components_package`):
//!    `ExecuteDescentBurn`, `ExecuteAscentBurn`, `Spacecraft` (`perform action`)
//! 5. **Technical Ports, Hardware & Specification** (`technical_ports_package`,
//!    `technical_components_package`, `system_specification_package`):
//!    `LMStagingPort`, `DockingPort`, `LMStagingInterface`, `LunarModuleDescentStage`,
//!    `LunarModuleAscentStage`, `ApolloLunarModule`, `ApolloSpacecraft`, `satisfy` links.

pub mod simulation;

use sysml_dsl::prelude::*;

#[sysml::package(library, name = "CoSMAPackage")]
#[sysml::import("ISQ::*")]
#[sysml::import("SI::*")]
pub mod cosma_package {
    use sysml_dsl::prelude::*;

    /// Base physical component carrying mass and recursive subcomponents.
    #[sysml::part]
    #[derive(Debug, Clone, Default)]
    pub struct MassedComponent {
        #[sysml::part(mult = "*")]
        pub subcomponents: Vec<MassedComponent>,
        #[sysml::attr(subsets = "ISQ::mass")]
        pub mass: Quantity<f64>,
    }

    #[sysml::action(abstract)]
    #[derive(Debug, Clone, Default)]
    pub struct Operation;

    #[sysml::action(abstract)]
    #[derive(Debug, Clone, Default)]
    pub struct Function;

    #[sysml::part(abstract)]
    #[derive(Debug, Clone, Default)]
    pub struct LogicalComponent;

    #[sysml::part(abstract)]
    #[derive(Debug, Clone, Default)]
    pub struct TechnicalComponent {
        #[sysml::attr(subsets = "ISQ::frequency")]
        pub failure_rate: Quantity<f64>,
    }

    #[sysml::part(abstract)]
    #[sysml::specializes(TechnicalComponent, MassedComponent)]
    #[derive(Debug, Clone, Default)]
    pub struct HardwareComponent;

    #[sysml::part(abstract)]
    #[sysml::specializes(HardwareComponent)]
    #[derive(Debug, Clone, Default)]
    pub struct PowerProvider {
        #[sysml::attr(subsets = "ISQ::power", default = "0", unit = "W")]
        pub power_generated: Quantity<f64>,
    }

    #[sysml::part(abstract)]
    #[sysml::specializes(HardwareComponent)]
    #[derive(Debug, Clone, Default)]
    pub struct PowerConsumer {
        #[sysml::attr(subsets = "ISQ::power")]
        pub power_load: Quantity<f64>,
    }

    #[sysml::state(abstract)]
    #[derive(Debug, Clone, Default)]
    pub struct Phase;

    #[sysml::part]
    #[sysml::specializes(MassedComponent)]
    #[derive(Debug, Clone, Default)]
    pub struct System;
}

#[sysml::package(name = "OperationsPackage")]
#[sysml::import("CoSMAPackage::*")]
pub mod operations_package {
    use super::cosma_package::*;
    use sysml_dsl::prelude::*;

    #[sysml::action]
    #[sysml::specializes(Operation)]
    #[derive(Debug, Clone, Default)]
    pub struct ExecutePoweredDescentBurn;

    #[sysml::action]
    #[sysml::specializes(Operation)]
    #[derive(Debug, Clone, Default)]
    pub struct MonitorDescentTelemetry;

    #[sysml::action]
    #[sysml::specializes(Operation)]
    #[derive(Debug, Clone, Default)]
    pub struct PerformManualPiloting;

    #[sysml::action]
    #[sysml::specializes(Operation)]
    #[derive(Debug, Clone, Default)]
    pub struct ExecuteLunarAscentBurn;
}

#[sysml::package(name = "MissionPhasesPackage")]
#[sysml::import("CoSMAPackage::*")]
#[sysml::import("OperationsPackage::*")]
pub mod mission_phases_package {
    use super::cosma_package::*;
    use super::operations_package::*;
    use sysml_dsl::prelude::*;

    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct PreparationPhaseCompletedNotification;

    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct LaunchPhaseCompletedNotification;

    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct LunarOrbitOpsDescentPrepCompletedNotification;

    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct PoweredDescentPhaseCompletedNotification;

    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct PoweredDescentAbortTrigger;

    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct LunarSurfaceOpsCompletedNotification;

    /// Powered Descent mission phase culminating in Eagle touchdown at Tranquility Base.
    #[sysml::state]
    #[sysml::specializes(Phase)]
    #[sysml::do_action(
        name = "poweredDescentOperations",
        steps = [
            executePoweredDescentBurn: ExecutePoweredDescentBurn,
            monitorDescentTelemetry: MonitorDescentTelemetry,
            performManualPiloting: PerformManualPiloting,
        ]
    )]
    #[derive(Debug, Clone, Default)]
    pub struct PoweredDescentPhase;

    /// Executable Apollo 11 mission phase state machine (both emits SysML v2
    /// transitions and executes directly in `cargo test` simulations).
    #[sysml::state(name = "Apollo11MissionPhases")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Apollo11MissionPhases {
        #[sysml::initial]
        #[sysml::transition(to = Preparation)]
        Initial,

        #[sysml::transition(on = PreparationPhaseCompletedNotification, to = Launch)]
        Preparation,

        #[sysml::transition(on = LaunchPhaseCompletedNotification, to = LunarOrbitOps)]
        Launch,

        #[sysml::transition(on = LunarOrbitOpsDescentPrepCompletedNotification, to = PoweredDescent)]
        LunarOrbitOps,

        #[sysml::transition(on = PoweredDescentPhaseCompletedNotification, to = LunarSurfaceOps)]
        #[sysml::transition(on = PoweredDescentAbortTrigger, to = AbortToLunarOrbit)]
        PoweredDescent,

        #[sysml::transition(on = LunarSurfaceOpsCompletedNotification, to = MissionComplete)]
        LunarSurfaceOps,

        AbortToLunarOrbit,

        MissionComplete,
    }
}

#[sysml::package(name = "RequirementsPackage")]
#[sysml::import("SI::*")]
#[sysml::import("ISQ::*")]
#[sysml::import("ScalarValues::*")]
pub mod requirements_package {
    use sysml_dsl::prelude::*;

    /// The Apollo 11 Mission shall achieve a soft landing of the crewed lunar lander on the lunar surface within the designated landing zone (Tranquility Base), by July 20, 1969.
    #[sysml::requirement(id = "HLR-R002")]
    #[sysml::rationale("A soft and precise lunar landing is fundamental to the 'Go to Moon' goal and ensures the integrity of the lander and safety of the crew for surface operations and subsequent ascent.")]
    #[sysml::require_constraint("(actualVerticalVelocity <= maxVerticalVelocity) and (actualHorizontalVelocity <= maxHorizontalVelocity) and (actualLandingZoneDeviation <= maxLandingZoneDeviation)")]
    #[derive(Debug, Clone)]
    pub struct LunarLanderSoftLandingRequirement {
        #[sysml::attr(subsets = "ISQ::speed")]
        pub actual_vertical_velocity: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::speed", value = "2", unit = "m/s")]
        pub max_vertical_velocity: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::speed")]
        pub actual_horizontal_velocity: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::speed", value = "0.5", unit = "m/s")]
        pub max_horizontal_velocity: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::length")]
        pub actual_landing_zone_deviation: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::length", value = "1000", unit = "m")]
        pub max_landing_zone_deviation: Quantity<f64>,
    }

    impl Default for LunarLanderSoftLandingRequirement {
        fn default() -> Self {
            Self {
                actual_vertical_velocity: Quantity::m_per_s(0.0),
                max_vertical_velocity: Quantity::m_per_s(2.0),
                actual_horizontal_velocity: Quantity::m_per_s(0.0),
                max_horizontal_velocity: Quantity::m_per_s(0.5),
                actual_landing_zone_deviation: Quantity::m(0.0),
                max_landing_zone_deviation: Quantity::m(1000.0),
            }
        }
    }

    /// The LunarModuleDescentStage shall provide a main engine with a throttleable thrust range.
    #[sysml::requirement(id = "CLR-R018")]
    #[sysml::rationale("A variable thrust is essential for the powered descent, allowing the crew to precisely control the rate of descent for a soft landing.")]
    #[derive(Debug, Clone, Default)]
    pub struct LMDSThrottleableEngine;

    /// The LunarModuleDescentStage shall command thrust to ensure that at the moment of footpad touchdown, vertical velocity <= 3.05 m/s and horizontal velocity <= 1.22 m/s.
    #[sysml::requirement(id = "CLR-R050")]
    #[sysml::rationale("These limits prevent structural damage to the ascent stage interface or a post-impact tip-over of the LM.")]
    #[sysml::require_constraint("(actualVerticalVelocity <= maxTouchdownVerticalVelocity) and (actualHorizontalVelocity <= maxTouchdownHorizontalVelocity)")]
    #[derive(Debug, Clone)]
    pub struct LMDSVelocityLimits {
        #[sysml::attr(subsets = "ISQ::speed")]
        pub actual_vertical_velocity: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::speed", value = "3.05", unit = "m/s")]
        pub max_touchdown_vertical_velocity: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::speed")]
        pub actual_horizontal_velocity: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::speed", value = "1.22", unit = "m/s")]
        pub max_touchdown_horizontal_velocity: Quantity<f64>,
    }

    impl Default for LMDSVelocityLimits {
        fn default() -> Self {
            Self {
                actual_vertical_velocity: Quantity::m_per_s(0.0),
                max_touchdown_vertical_velocity: Quantity::m_per_s(3.05),
                actual_horizontal_velocity: Quantity::m_per_s(0.0),
                max_touchdown_horizontal_velocity: Quantity::m_per_s(1.22),
            }
        }
    }

    /// The lunar lander descent propulsion system shall provide throttleable thrust from 10% to 100% of maximum thrust.
    #[sysml::requirement(id = "CLR-R057")]
    #[sysml::rationale("Throttling capability provides the fine control necessary to manage descent rate and fuel consumption during the lunar landing.")]
    #[sysml::require_constraint("(actualMinThrottlePercentage <= minRequiredThrottlePercentage) and (actualMaxThrottlePercentage >= maxRequiredThrottlePercentage)")]
    #[derive(Debug, Clone)]
    pub struct DPSThrottlingRange {
        #[sysml::attr(ty = "Real")]
        pub actual_min_throttle_percentage: Quantity<f64>,
        #[sysml::attr(ty = "Real", value = "10", unit = "%")]
        pub min_required_throttle_percentage: Quantity<f64>,
        #[sysml::attr(ty = "Real")]
        pub actual_max_throttle_percentage: Quantity<f64>,
        #[sysml::attr(ty = "Real", value = "100", unit = "%")]
        pub max_required_throttle_percentage: Quantity<f64>,
    }

    impl Default for DPSThrottlingRange {
        fn default() -> Self {
            Self {
                actual_min_throttle_percentage: Quantity::percent(10.0),
                min_required_throttle_percentage: Quantity::percent(10.0),
                actual_max_throttle_percentage: Quantity::percent(100.0),
                max_required_throttle_percentage: Quantity::percent(100.0),
            }
        }
    }

    /// The lunar lander ascent propulsion system shall achieve a burn duration of <= 400 seconds to insert the ascent stage into lunar orbit.
    #[sysml::requirement(id = "CLR-R058")]
    #[sysml::rationale("A specific burn duration is required to ensure the ascent stage achieves the correct lunar orbit for rendezvous.")]
    #[sysml::require_constraint("actualAscentBurnDuration <= maxAscentBurnDuration")]
    #[derive(Debug, Clone)]
    pub struct LMASBurnDuration {
        #[sysml::attr(subsets = "ISQ::time")]
        pub actual_ascent_burn_duration: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::time", value = "400", unit = "s")]
        pub max_ascent_burn_duration: Quantity<f64>,
    }

    impl Default for LMASBurnDuration {
        fn default() -> Self {
            Self {
                actual_ascent_burn_duration: Quantity::seconds(0.0),
                max_ascent_burn_duration: Quantity::seconds(400.0),
            }
        }
    }
}

#[sysml::package(name = "FunctionsPackage")]
#[sysml::import("CoSMAPackage::*")]
#[sysml::import("OperationsPackage::*")]
pub mod functions_package {
    use super::cosma_package::*;
    use sysml_dsl::prelude::*;

    /// Represents the complete state of a vehicle system, including configuration, resources, and trajectory.
    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct VehicleStatus;

    /// Separates the Lunar Module from the Command/Service Module in lunar orbit.
    #[sysml::action]
    #[sysml::specializes(Function)]
    #[derive(Debug, Clone, Default)]
    pub struct SeparateLanderFromOrbiter {
        #[sysml::in_param]
        pub docked_status: VehicleStatus,
        #[sysml::out_param]
        pub separated_status: VehicleStatus,
    }

    /// Fires the Descent Propulsion System to bring the Lunar Module safely to the lunar surface.
    #[sysml::action]
    #[sysml::specializes(Function)]
    #[sysml::refines(supplier = "OperationsPackage::ExecutePoweredDescentBurn")]
    #[derive(Debug, Clone, Default)]
    pub struct ExecuteDescentBurn {
        #[sysml::in_param]
        pub orbit_status: VehicleStatus,
        #[sysml::out_param]
        pub landed_status: VehicleStatus,
    }

    /// Fires the Ascent Propulsion System to lift the LM Ascent Stage off the lunar surface into orbit.
    #[sysml::action]
    #[sysml::specializes(Function)]
    #[sysml::refines(supplier = "OperationsPackage::ExecuteLunarAscentBurn")]
    #[derive(Debug, Clone, Default)]
    pub struct ExecuteAscentBurn {
        #[sysml::in_param]
        pub surface_status: VehicleStatus,
        #[sysml::out_param]
        pub orbit_status: VehicleStatus,
    }
}

#[sysml::package(name = "LogicalComponentsPackage")]
#[sysml::import("CoSMAPackage::*")]
#[sysml::import("FunctionsPackage::*")]
pub mod logical_components_package {
    use super::cosma_package::*;
    use super::functions_package::*;
    use sysml_dsl::prelude::*;

    /// The vehicles that travel to, operate at, and return from the Moon.
    #[sysml::part]
    #[sysml::specializes(LogicalComponent)]
    #[sysml::perform(separate_lander_from_orbiter: SeparateLanderFromOrbiter)]
    #[sysml::perform(execute_descent_burn: ExecuteDescentBurn)]
    #[sysml::perform(execute_ascent_burn: ExecuteAscentBurn)]
    #[derive(Debug, Clone, Default)]
    pub struct Spacecraft;
}

#[sysml::package(name = "TechnicalPortsPackage")]
#[sysml::import("SI::*")]
pub mod technical_ports_package {
    use sysml_dsl::prelude::*;

    /// Represents the transfer of physical forces (compression, tension, torsion).
    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct StructuralLoad;

    /// Represents a discrete command or electrical signal.
    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct CommandSignal;

    /// Represents a flow of vehicle health and status data.
    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct TelemetryData;

    /// Represents a flow of electrical energy.
    #[sysml::item]
    #[derive(Debug, Clone, Default)]
    pub struct ElectricalPower;

    /// A single definition for the Lunar Module staging port, viewed from the platform (Descent Stage).
    #[sysml::port]
    #[derive(Debug, Clone, Default)]
    pub struct LMStagingPort {
        #[sysml::inout]
        pub loads: StructuralLoad,
        #[sysml::out]
        pub commands: CommandSignal,
        #[sysml::out]
        pub power_and_data: ElectricalPower,
    }

    /// A symmetric port for docking, where flows can be bidirectional.
    #[sysml::port(symmetric)]
    #[derive(Debug, Clone, Default)]
    pub struct DockingPort {
        #[sysml::inout]
        pub loads: StructuralLoad,
        #[sysml::inout]
        pub power: ElectricalPower,
        #[sysml::inout]
        pub data: TelemetryData,
    }

    /// Defines the contract for the connection between the two stages of the Lunar Module.
    #[sysml::interface]
    #[derive(Debug, Clone, Default)]
    pub struct LMStagingInterface {
        #[sysml::end]
        pub platform: LMStagingPort,
        #[sysml::end]
        pub ascender: Conjugated<LMStagingPort>,
    }

    /// Defines the contract for a symmetric docking connection between two vehicles.
    #[sysml::interface]
    #[derive(Debug, Clone, Default)]
    pub struct DockingInterface {
        #[sysml::end]
        pub vehicle1: DockingPort,
        #[sysml::end]
        pub vehicle2: DockingPort,
    }
}

#[sysml::package(name = "TechnicalComponentsPackage")]
#[sysml::import("CoSMAPackage::*")]
#[sysml::import("SI::*")]
#[sysml::import("ISQ::*")]
#[sysml::import("LogicalComponentsPackage::*")]
#[sysml::import("TechnicalPortsPackage::*")]
#[sysml::import("RequirementsPackage::*")]
pub mod technical_components_package {
    use super::cosma_package::*;
    use super::logical_components_package::*;
    use super::requirements_package::*;
    use super::technical_ports_package::*;
    use sysml_dsl::prelude::*;

    #[sysml::part(abstract)]
    #[sysml::specializes(HardwareComponent)]
    #[derive(Debug, Clone, Default)]
    pub struct FuelledComponent {
        #[sysml::attr(subsets = "ISQ::mass")]
        pub propellant_mass: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::mass")]
        pub dry_mass: Quantity<f64>,
        #[sysml::attr(redefines = "mass", expr = "dryMass + propellantMass")]
        pub total_stage_mass: Quantity<f64>,
    }

    #[sysml::part(abstract)]
    #[sysml::specializes(FuelledComponent)]
    #[derive(Debug, Clone, Default)]
    pub struct PropelledSpacecraft {
        #[sysml::attr(subsets = "ISQ::force")]
        pub max_thrust: Quantity<f64>,
        #[sysml::attr(subsets = "ISQ::time")]
        pub specific_impulse: Quantity<f64>,
    }

    /// The lower portion of the Lunar Module, used for powered landing and as a launch platform for the ascent stage.
    #[sysml::part]
    #[sysml::specializes(PropelledSpacecraft, PowerConsumer, PowerProvider)]
    #[sysml::satisfy(req = LMDSThrottleableEngine, id = "CLR-R018")]
    #[sysml::satisfy(req = LMDSVelocityLimits, id = "CLR-R050")]
    #[sysml::satisfy(req = DPSThrottlingRange, id = "CLR-R057")]
    #[derive(Debug, Clone)]
    pub struct LunarModuleDescentStage {
        #[sysml::attr(redefines = "dryMass", value = "2100", unit = "kg")]
        pub dry_mass: Quantity<f64>,
        #[sysml::attr(redefines = "propellantMass", value = "8250", unit = "kg")]
        pub propellant_mass: Quantity<f64>,
        #[sysml::attr(redefines = "maxThrust", value = "45", unit = "kN")]
        pub max_thrust: Quantity<f64>,
        #[sysml::attr(redefines = "specificImpulse", value = "305", unit = "s")]
        pub specific_impulse: Quantity<f64>,
        #[sysml::attr(redefines = "powerGenerated", value = "1500", unit = "W")]
        pub power_generated: Quantity<f64>,
        #[sysml::attr(redefines = "powerLoad", value = "1000", unit = "W")]
        pub power_load: Quantity<f64>,
        #[sysml::attr(redefines = "failureRate", value = "3.0E-6", unit = "1/h")]
        pub failure_rate: Quantity<f64>,
        #[sysml::attr(ty = "Real", value = "10", unit = "%")]
        pub min_throttle_percent: Quantity<f64>,
        #[sysml::attr(ty = "Real", value = "100", unit = "%")]
        pub max_throttle_percent: Quantity<f64>,
        #[sysml::port]
        pub upper_stage_port: LMStagingPort,
    }

    impl LunarModuleDescentStage {
        /// Construct the Apollo 11 Eagle (LM-5) Descent Stage with Airbus model parameters.
        pub fn apollo11_eagle() -> Self {
            Self {
                dry_mass: Quantity::kg(2100.0),
                propellant_mass: Quantity::kg(8250.0),
                max_thrust: Quantity::kn(45.0),
                specific_impulse: Quantity::seconds(305.0),
                power_generated: Quantity::watts(1500.0),
                power_load: Quantity::watts(1000.0),
                failure_rate: Quantity::per_hour(3.0e-6),
                min_throttle_percent: Quantity::percent(10.0),
                max_throttle_percent: Quantity::percent(100.0),
                upper_stage_port: LMStagingPort::default(),
            }
        }
    }

    /// The upper portion of the Lunar Module, serving as the crew cabin and ascent vehicle.
    #[sysml::part]
    #[sysml::specializes(PropelledSpacecraft, PowerConsumer, PowerProvider)]
    #[sysml::satisfy(req = LMASBurnDuration, id = "CLR-R058")]
    #[derive(Debug, Clone)]
    pub struct LunarModuleAscentStage {
        #[sysml::attr(redefines = "dryMass", value = "2220", unit = "kg")]
        pub dry_mass: Quantity<f64>,
        #[sysml::attr(redefines = "propellantMass", value = "2370", unit = "kg")]
        pub propellant_mass: Quantity<f64>,
        #[sysml::attr(redefines = "maxThrust", value = "16", unit = "kN")]
        pub max_thrust: Quantity<f64>,
        #[sysml::attr(redefines = "specificImpulse", value = "311", unit = "s")]
        pub specific_impulse: Quantity<f64>,
        #[sysml::attr(redefines = "powerGenerated", value = "1500", unit = "W")]
        pub power_generated: Quantity<f64>,
        #[sysml::attr(redefines = "powerLoad", value = "1200", unit = "W")]
        pub power_load: Quantity<f64>,
        #[sysml::attr(redefines = "failureRate", value = "2.5E-6", unit = "1/h")]
        pub failure_rate: Quantity<f64>,
        #[sysml::port]
        pub lower_stage_port: Conjugated<LMStagingPort>,
        #[sysml::port]
        pub docking_port: DockingPort,
    }

    impl LunarModuleAscentStage {
        /// Construct the Apollo 11 Eagle (LM-5) Ascent Stage with Airbus model parameters.
        pub fn apollo11_eagle() -> Self {
            Self {
                dry_mass: Quantity::kg(2220.0),
                propellant_mass: Quantity::kg(2370.0),
                max_thrust: Quantity::kn(16.0),
                specific_impulse: Quantity::seconds(311.0),
                power_generated: Quantity::watts(1500.0),
                power_load: Quantity::watts(1200.0),
                failure_rate: Quantity::per_hour(2.5e-6),
                lower_stage_port: Conjugated::new(),
                docking_port: DockingPort::default(),
            }
        }
    }

    /// The two-stage vehicle that performed the lunar landing and ascent.
    #[sysml::part]
    #[sysml::specializes(HardwareComponent, PowerConsumer, PowerProvider)]
    #[sysml::connect(
        interface = LMStagingInterface,
        name = "lmInterstage",
        from = lunar_module_descent_stage.upper_stage_port,
        to = lunar_module_ascent_stage.lower_stage_port
    )]
    #[sysml::satisfy(req = LunarLanderSoftLandingRequirement, id = "HLR-R002")]
    #[derive(Debug, Clone)]
    pub struct ApolloLunarModule {
        #[sysml::part]
        pub lunar_module_descent_stage: LunarModuleDescentStage,
        #[sysml::part]
        pub lunar_module_ascent_stage: LunarModuleAscentStage,
        #[sysml::attr(
            redefines = "powerGenerated",
            expr = "lunarModuleDescentStage.powerGenerated + lunarModuleAscentStage.powerGenerated"
        )]
        pub power_generated: Quantity<f64>,
        #[sysml::attr(
            redefines = "powerLoad",
            expr = "lunarModuleDescentStage.powerLoad + lunarModuleAscentStage.powerLoad"
        )]
        pub power_load: Quantity<f64>,
    }

    impl ApolloLunarModule {
        pub fn apollo11_eagle() -> Self {
            let ds = LunarModuleDescentStage::apollo11_eagle();
            let as_ = LunarModuleAscentStage::apollo11_eagle();
            let power_generated = ds.power_generated + as_.power_generated;
            let power_load = ds.power_load + as_.power_load;
            Self {
                lunar_module_descent_stage: ds,
                lunar_module_ascent_stage: as_,
                power_generated,
                power_load,
            }
        }

        /// Total wet mass of LM-5 (Descent + Ascent stages).
        pub fn total_wet_mass_kg(&self) -> f64 {
            self.lunar_module_descent_stage.dry_mass.value()
                + self.lunar_module_descent_stage.propellant_mass.value()
                + self.lunar_module_ascent_stage.dry_mass.value()
                + self.lunar_module_ascent_stage.propellant_mass.value()
        }
    }

    /// The Apollo spacecraft stack realizing the logical `Spacecraft` functions.
    #[sysml::part]
    #[sysml::specializes(System, Spacecraft)]
    #[derive(Debug, Clone)]
    pub struct ApolloSpacecraft {
        #[sysml::part]
        pub lunar_module: ApolloLunarModule,
    }

    impl ApolloSpacecraft {
        pub fn apollo11() -> Self {
            Self {
                lunar_module: ApolloLunarModule::apollo11_eagle(),
            }
        }
    }
}

#[sysml::package(name = "SystemSpecificationPackage")]
#[sysml::import("RequirementsPackage::*")]
#[sysml::import("TechnicalComponentsPackage::*")]
pub mod system_specification_package {
    use super::requirements_package::*;
    use super::technical_components_package::*;
    use sysml_dsl::prelude::*;

    /// Apollo 11 Powered Descent & Lunar Module requirements specification.
    #[sysml::specification(name = "apollo11PoweredDescentSpecification")]
    #[sysml::subject(name = "apolloSpacecraft", ty = ApolloSpacecraft)]
    #[sysml::satisfy(req = LunarLanderSoftLandingRequirement, id = "HLR-R002", requirement = "hlr-R002", by = "apolloSpacecraft.lunarModule")]
    #[sysml::satisfy(req = LMDSThrottleableEngine, id = "CLR-R018", requirement = "clr-R018", by = "apolloSpacecraft.lunarModule.lunarModuleDescentStage")]
    #[sysml::satisfy(req = LMDSVelocityLimits, id = "CLR-R050", requirement = "clr-R050", by = "apolloSpacecraft.lunarModule.lunarModuleDescentStage")]
    #[sysml::satisfy(req = DPSThrottlingRange, id = "CLR-R057", requirement = "clr-R057", by = "apolloSpacecraft.lunarModule.lunarModuleDescentStage")]
    #[sysml::satisfy(req = LMASBurnDuration, id = "CLR-R058", requirement = "clr-R058", by = "apolloSpacecraft.lunarModule.lunarModuleAscentStage")]
    #[derive(Debug, Clone, Default)]
    pub struct Apollo11PoweredDescentSpecification {
        #[sysml::requirement_ref(name = "hlr-R002")]
        pub hlr_r002: LunarLanderSoftLandingRequirement,
        #[sysml::requirement_ref(name = "clr-R018")]
        pub clr_r018: LMDSThrottleableEngine,
        #[sysml::requirement_ref(name = "clr-R050")]
        pub clr_r050: LMDSVelocityLimits,
        #[sysml::requirement_ref(name = "clr-R057")]
        pub clr_r057: DPSThrottlingRange,
        #[sysml::requirement_ref(name = "clr-R058")]
        pub clr_r058: LMASBurnDuration,
    }
}

/// Build the complete 5-layer Apollo 11 vertical slice `sysml_ir::Model`.
pub fn build_apollo11_slice_model() -> sysml_ir::Model {
    sysml_ir::Model::new(vec![
        cosma_package::sysml_package(),
        operations_package::sysml_package(),
        mission_phases_package::sysml_package(),
        requirements_package::sysml_package(),
        functions_package::sysml_package(),
        logical_components_package::sysml_package(),
        technical_ports_package::sysml_package(),
        technical_components_package::sysml_package(),
        system_specification_package::sysml_package(),
    ])
}

/// Emit the complete 5-layer Apollo 11 vertical slice to deterministic OMG SysML v2 text.
pub fn emit_apollo11_slice_sysml() -> String {
    sysml_emit::emit_model(&build_apollo11_slice_model())
}
