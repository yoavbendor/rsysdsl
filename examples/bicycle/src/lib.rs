//! Human-powered Bicycle multi-domain model: mechanics, HMI, energy flow,
//! friction heat, and acoustic noise built on `sysml-std`.

use sysml_dsl::prelude::*;
use sysml_std::prelude::*;

#[sysml::package(name = "BicycleModel")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
#[sysml::import("SysmlStdComms::*")]
#[sysml::import("SysmlStdMechanics::*")]
#[sysml::import("SysmlStdHmi::*")]
pub mod bicycle_model {
    use super::*;

    /// Abstract bicycle frame specializing `StructuralFrame` with tire clearance geometry.
    #[sysml::part(abstract)]
    #[sysml::specializes(StructuralFrame)]
    #[derive(Debug, Clone, Copy)]
    pub struct BicycleFrame {
        #[sysml::attr(redefines = "mass", default = 1.6, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "maxLoad", default = 1200.0, unit = "N")]
        pub max_load: Quantity<f64>,
        #[sysml::attr(default = 0.042, unit = "m")]
        pub tire_clearance: Quantity<f64>,
    }

    /// Lightweight aerodynamic road bicycle frame variant.
    #[sysml::part]
    #[sysml::specializes(BicycleFrame)]
    #[derive(Debug, Clone, Copy)]
    pub struct RoadFrame {
        #[sysml::attr(redefines = "mass", default = 0.95, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "maxLoad", default = 1100.0, unit = "N")]
        pub max_load: Quantity<f64>,
        #[sysml::attr(redefines = "tireClearance", default = 0.032, unit = "m")]
        pub tire_clearance: Quantity<f64>,
    }

    /// Reinforced off-road mountain bicycle frame variant.
    #[sysml::part]
    #[sysml::specializes(BicycleFrame)]
    #[derive(Debug, Clone, Copy)]
    pub struct MountainFrame {
        #[sysml::attr(redefines = "mass", default = 2.1, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "maxLoad", default = 1500.0, unit = "N")]
        pub max_load: Quantity<f64>,
        #[sysml::attr(redefines = "tireClearance", default = 0.065, unit = "m")]
        pub tire_clearance: Quantity<f64>,
    }

    /// Product-line frame variation selecting between road and mountain geometries.
    #[sysml::variation(of = BicycleFrame)]
    #[derive(Debug, Clone, Copy)]
    pub enum FrameVariant {
        Road(RoadFrame),
        Mountain(MountainFrame),
    }

    /// Chain-and-sprocket drivetrain specializing `MechanicalTransmission`.
    #[sysml::part]
    #[sysml::specializes(MechanicalTransmission)]
    #[derive(Debug, Clone, Copy)]
    pub struct Drivetrain {
        #[sysml::attr(redefines = "mass", default = 1.4, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "gearRatio", default = 2.5)]
        pub gear_ratio: f64,
        #[sysml::attr(redefines = "efficiency", default = 0.96)]
        pub efficiency: f64,
        #[sysml::attr(redefines = "heatDissipation", default = 8.0, unit = "W")]
        pub heat_dissipation: Quantity<f64>,
        #[sysml::attr(redefines = "noiseLevelDb", default = 52.0)]
        pub noise_level_db: f64,
        #[sysml::attr(default = 50)]
        pub chainring_teeth: u32,
        #[sysml::attr(default = 20)]
        pub cog_teeth: u32,
        #[sysml::port(redefines)]
        pub mech_in: Conjugated<MechanicalPowerPort>,
        #[sysml::port(redefines)]
        pub mech_out: MechanicalPowerPort,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
    }

    /// Rear wheel and tire assembly specializing `RotatingWheel`.
    #[sysml::part]
    #[sysml::specializes(RotatingWheel)]
    #[derive(Debug, Clone, Copy)]
    pub struct WheelAssembly {
        #[sysml::attr(redefines = "mass", default = 1.8, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "heatDissipation", default = 5.0, unit = "W")]
        pub heat_dissipation: Quantity<f64>,
        #[sysml::attr(redefines = "noiseLevelDb", default = 58.0)]
        pub noise_level_db: f64,
        #[sysml::attr(redefines = "rollingResistanceCoeff", default = 0.004)]
        pub rolling_resistance_coeff: f64,
        #[sysml::attr(redefines = "wheelRadius", default = 0.34, unit = "m")]
        pub wheel_radius: Quantity<f64>,
        #[sysml::port(redefines)]
        pub drive_in: Conjugated<MechanicalPowerPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
    }

    /// Disc/rim brake system specializing `FrictionBrake`.
    #[sysml::part]
    #[sysml::specializes(FrictionBrake)]
    #[derive(Debug, Clone, Copy)]
    pub struct BrakeSystem {
        #[sysml::attr(redefines = "mass", default = 0.9, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "heatDissipation", default = 1500.0, unit = "W")]
        pub heat_dissipation: Quantity<f64>,
        #[sysml::attr(redefines = "noiseLevelDb", default = 72.0)]
        pub noise_level_db: f64,
        #[sysml::attr(redefines = "maxBrakingForce", default = 1800.0, unit = "N")]
        pub max_braking_force: Quantity<f64>,
        #[sysml::attr(redefines = "thermalCapacityPower", default = 2500.0, unit = "W")]
        pub thermal_capacity_power: Quantity<f64>,
        #[sysml::port(redefines)]
        pub actuation_in: Conjugated<HmiInputPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
    }

    /// Handlebar steering and brake lever cockpit specializing `MechanicalHmiControl`.
    #[sysml::part]
    #[sysml::specializes(MechanicalHmiControl)]
    #[derive(Debug, Clone, Copy)]
    pub struct SteeringCockpit {
        #[sysml::attr(redefines = "mass", default = 0.85, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(default = 0.42, unit = "m")]
        pub handlebar_width: Quantity<f64>,
        #[sysml::port(redefines)]
        pub user_in: Conjugated<HmiInputPort>,
        #[sysml::port]
        pub brake_lever_in: Conjugated<HmiInputPort>,
    }

    /// Complete bicycle assembly integrating frame, drivetrain, rear wheel, brakes, and cockpit.
    #[sysml::part]
    #[sysml::satisfy(req = DrivetrainEfficiencyReq, id = "BIKE-001")]
    #[sysml::satisfy(req = TotalMassLimitReq, id = "BIKE-002")]
    #[sysml::satisfy(req = BrakeThermalCapacityReq, id = "BIKE-003")]
    #[sysml::connect(
        interface = MechanicalLink,
        name = "drivetrainToWheel",
        from = drivetrain.mech_out,
        to = rear_wheel.drive_in
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct Bicycle {
        #[sysml::attr(default = 9.2, unit = "kg")]
        pub total_mass: Quantity<f64>,
        #[sysml::part]
        pub frame: RoadFrame,
        #[sysml::part]
        pub drivetrain: Drivetrain,
        #[sysml::part]
        pub rear_wheel: WheelAssembly,
        #[sysml::part]
        pub brakes: BrakeSystem,
        #[sysml::part]
        pub cockpit: SteeringCockpit,
    }

    /// Top-level human-powered bicycle context connecting rider, bicycle, and ambient environment.
    #[sysml::part]
    #[sysml::connect(
        interface = MechanicalLink,
        name = "riderToDrivetrain",
        from = rider.mech_effort,
        to = bicycle.drivetrain.mech_in
    )]
    #[sysml::connect(
        interface = HmiInputChannel,
        name = "riderToBrakes",
        from = rider.user_input,
        to = bicycle.brakes.actuation_in
    )]
    #[sysml::connect(
        interface = HmiInputChannel,
        name = "riderToCockpitSteering",
        from = rider.user_input,
        to = bicycle.cockpit.user_in
    )]
    #[sysml::connect(
        interface = HmiInputChannel,
        name = "riderToCockpitBrakeLever",
        from = rider.user_input,
        to = bicycle.cockpit.brake_lever_in
    )]
    #[sysml::connect(
        interface = ThermalLink,
        name = "drivetrainHeatToAmbient",
        from = bicycle.drivetrain.heat_out,
        to = ambient.heat_sink
    )]
    #[sysml::connect(
        interface = ThermalLink,
        name = "wheelHeatToAmbient",
        from = bicycle.rear_wheel.heat_out,
        to = ambient.heat_sink
    )]
    #[sysml::connect(
        interface = ThermalLink,
        name = "brakeHeatToAmbient",
        from = bicycle.brakes.heat_out,
        to = ambient.heat_sink
    )]
    #[sysml::connect(
        interface = AcousticLink,
        name = "drivetrainNoiseToAmbient",
        from = bicycle.drivetrain.noise_out,
        to = ambient.noise_sink
    )]
    #[sysml::connect(
        interface = AcousticLink,
        name = "wheelNoiseToAmbient",
        from = bicycle.rear_wheel.noise_out,
        to = ambient.noise_sink
    )]
    #[sysml::connect(
        interface = AcousticLink,
        name = "brakeNoiseToAmbient",
        from = bicycle.brakes.noise_out,
        to = ambient.noise_sink
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct BicycleContext {
        #[sysml::part]
        pub rider: HumanOperator,
        #[sysml::part]
        pub bicycle: Bicycle,
        #[sysml::part]
        pub ambient: AmbientEnvironment,
    }

    // --- Requirements ---

    /// The bicycle drivetrain shall achieve at least 94% mechanical transmission efficiency.
    #[sysml::requirement(id = "BIKE-001")]
    #[sysml::require_constraint(drivetrain_efficiency >= min_efficiency)]
    #[derive(Debug, Clone, Copy)]
    pub struct DrivetrainEfficiencyReq {
        #[sysml::attr(default = 0.94)]
        pub min_efficiency: f64,
        #[sysml::attr(default = 0.96)]
        pub drivetrain_efficiency: f64,
    }

    /// The complete bicycle shall have a total mass not exceeding 12.0 kg.
    #[sysml::requirement(id = "BIKE-002")]
    #[sysml::require_constraint(total_mass_kg <= max_mass_kg)]
    #[derive(Debug, Clone, Copy)]
    pub struct TotalMassLimitReq {
        #[sysml::attr(default = 12.0, unit = "kg")]
        pub max_mass_kg: Quantity<f64>,
        #[sysml::attr(default = 9.2, unit = "kg")]
        pub total_mass_kg: Quantity<f64>,
    }

    /// The brake system thermal dissipation capacity shall meet or exceed peak braking power.
    #[sysml::requirement(id = "BIKE-003")]
    #[sysml::require_constraint(brake_thermal_capacity_w >= peak_braking_power_w)]
    #[derive(Debug, Clone, Copy)]
    pub struct BrakeThermalCapacityReq {
        #[sysml::attr(default = 2500.0, unit = "W")]
        pub brake_thermal_capacity_w: Quantity<f64>,
        #[sysml::attr(default = 1500.0, unit = "W")]
        pub peak_braking_power_w: Quantity<f64>,
    }
}

/// Build the complete `BicycleModel` alongside `sysml-std` foundation packages.
pub fn build_model() -> Model {
    let mut pkgs = sysml_std::std_packages();
    pkgs.push(bicycle_model::sysml_package());
    Model::new(pkgs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bicycle_model::*;
    use sysml_emit::{audit_model_with_stdlib, emit_model, lint_model};

    #[test]
    fn bicycle_model_audits_zero_errors_zero_warnings_zero_orphans() {
        let model = build_model();
        let sysml = emit_model(&model);
        let audit = audit_model_with_stdlib("bicycle", &model, None, None, true);
        assert!(audit.stdlib_loaded, "OMG sysml.library must be loaded");
        assert!(
            audit.ok,
            "bicycle audit failed: {audit:#?}\n--- SYSML ---\n{sysml}"
        );
        assert_eq!(
            audit.validation_errors, 0,
            "validation errors: {audit:#?}\n--- SYSML ---\n{sysml}"
        );
        assert_eq!(
            audit.validation_warnings, 0,
            "validation warnings: {audit:#?}\n--- SYSML ---\n{sysml}"
        );
        assert!(
            audit.orphan_requirements.is_empty(),
            "orphan requirements: {:?}",
            audit.orphan_requirements
        );
    }

    #[test]
    fn bicycle_model_lints_clean() {
        let model = build_model();
        let lint = lint_model("bicycle", &model);
        assert!(lint.clean, "bicycle lint failed: {lint:#?}");
    }

    #[test]
    fn bicycle_physics_calcs_execute_accurately() {
        assert!((useful_power_watts(250.0, 0.96) - 240.0).abs() < 1e-9);
        assert!((waste_heat_watts(250.0, 0.96) - 10.0).abs() < 1e-9);
        // 0.5 * 1.225 * 0.32 * 10^3 = 196.0 W
        assert!((aero_drag_power_w(1.225, 0.32, 10.0) - 196.0).abs() < 1e-9);
        // 0.004 * 85 * 9.81 * 10 = 33.354 W
        assert!((rolling_resistance_power_w(0.004, 85.0, 9.81, 10.0) - 33.354).abs() < 1e-9);
        // 85 * 4.0 * 10.0 = 3400.0 W
        assert!((braking_heat_power_w(85.0, 4.0, 10.0) - 3400.0).abs() < 1e-9);
    }

    #[test]
    fn bicycle_requirements_evaluate_nominal_and_violated() {
        let req1_ok = DrivetrainEfficiencyReq::default();
        assert!(req1_ok.evaluate());
        let req1_bad = DrivetrainEfficiencyReq {
            drivetrain_efficiency: 0.90,
            ..DrivetrainEfficiencyReq::default()
        };
        assert!(!req1_bad.evaluate());

        let req2_ok = TotalMassLimitReq::default();
        assert!(req2_ok.evaluate());
        let req2_bad = TotalMassLimitReq {
            total_mass_kg: Quantity::new(13.5, "kg"),
            ..TotalMassLimitReq::default()
        };
        assert!(!req2_bad.evaluate());

        let req3_ok = BrakeThermalCapacityReq::default();
        assert!(req3_ok.evaluate());
        let req3_bad = BrakeThermalCapacityReq {
            peak_braking_power_w: Quantity::new(3000.0, "W"),
            ..BrakeThermalCapacityReq::default()
        };
        assert!(!req3_bad.evaluate());
    }
}
