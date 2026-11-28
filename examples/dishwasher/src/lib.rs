//! Multi-domain Dishwasher model (power, electronics, fluidics, mechanics, logic, HMI, heat, noise)
use sysml_dsl::ir::Model;
use sysml_dsl::prelude::*;

#[sysml::package(name = "DishwasherModel")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
#[sysml::import("SysmlStdComms::*")]
#[sysml::import("SysmlStdPower::*")]
#[sysml::import("SysmlStdMechanics::*")]
#[sysml::import("SysmlStdElectronics::*")]
#[sysml::import("SysmlStdHmi::*")]
pub mod dishwasher_model {
    use super::*;
    use sysml_std::prelude::*;

    // =========================================================================
    // 1. State Machine Triggers (Items)
    // =========================================================================

    /// Cycle start/selection command issued by HMI to controller.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct CycleStartTrigger {
        #[sysml::attr(default = 333.15, unit = "K")]
        pub target_temp_k: f64,
        #[sysml::attr(dimensionless, default = 1.0)]
        pub start_pressed_flag: f64,
    }

    /// Water fill level event reported by the turbidity/level sensor.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct WaterLevelTrigger {
        #[sysml::attr(dimensionless, default = 1.0)]
        pub fill_complete_flag: f64,
        #[sysml::attr(dimensionless, default = 0.0)]
        pub flood_detected_flag: f64,
    }

    /// Thermal status event reported during wash heating.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct WashThermalTrigger {
        #[sysml::attr(default = 333.15, unit = "K")]
        pub water_temp_k: f64,
        #[sysml::attr(dimensionless, default = 0.0)]
        pub wash_timer_done_flag: f64,
    }

    /// Rinse/dry/drain stage completion event.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct StageProgressTrigger {
        #[sysml::attr(dimensionless, default = 0.0)]
        pub stage_complete_flag: f64,
        #[sysml::attr(dimensionless, default = 0.0)]
        pub flood_detected_flag: f64,
    }

    /// Door latch state change event from safety interlock switch.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct DoorLatchTrigger {
        #[sysml::attr(dimensionless, default = 1.0)]
        pub door_closed_flag: f64,
    }

    // =========================================================================
    // 2. Modeled Physics Calculations (#[sysml::calc])
    // =========================================================================

    /// Total electrical energy consumed over a complete wash cycle in Joules.
    #[sysml::calc]
    pub fn total_cycle_energy_j(
        heating_j: f64,
        pump_power_w: f64,
        pump_duration_s: f64,
        electronics_power_w: f64,
        cycle_duration_s: f64,
    ) -> f64 {
        let pump_j = pump_power_w * pump_duration_s;
        let electronics_j = electronics_power_w * cycle_duration_s;
        heating_j + pump_j + electronics_j
    }

    // =========================================================================
    // 3. Requirements & Constraints (0 Orphans)
    // =========================================================================

    /// DW-001: The wash heating subsystem shall reach at least 333.15 K (60 C) for hygienic sanitation.
    #[sysml::requirement(id = "DW-001")]
    #[sysml::require_constraint(achieved_wash_temp_k >= min_wash_temp_k)]
    #[derive(Debug, Clone, Copy)]
    pub struct HygienicWashTemperatureReq {
        #[sysml::attr(default = 333.15, unit = "K")]
        pub min_wash_temp_k: Quantity<f64>,
        #[sysml::attr(default = 338.15, unit = "K")]
        pub achieved_wash_temp_k: Quantity<f64>,
    }

    /// DW-002: The enclosed dishwasher shall emit <= 46.0 dB acoustic noise into the ambient room.
    #[sysml::requirement(id = "DW-002")]
    #[sysml::require_constraint(emitted_noise_db <= max_noise_db)]
    #[derive(Debug, Clone, Copy)]
    pub struct QuietOperationNoiseReq {
        #[sysml::attr(dimensionless, default = 46.0)]
        pub max_noise_db: f64,
        #[sysml::attr(dimensionless, default = 44.0)]
        pub emitted_noise_db: f64,
    }

    /// DW-003: Total simultaneous dishwasher electrical draw shall not exceed 2200 W mains limit.
    #[sysml::requirement(id = "DW-003")]
    #[sysml::require_constraint(peak_power_w <= max_mains_power_w)]
    #[derive(Debug, Clone, Copy)]
    pub struct PeakMainsPowerBudgetReq {
        #[sysml::attr(default = 2200.0, unit = "W")]
        pub max_mains_power_w: Quantity<f64>,
        #[sysml::attr(default = 1920.0, unit = "W")]
        pub peak_power_w: Quantity<f64>,
    }

    /// DW-004: Opening the door shall interlock heater and pump power within 0.050 s.
    #[sysml::requirement(id = "DW-004")]
    #[sysml::require_constraint(cutoff_latency_s <= max_interlock_latency_s)]
    #[derive(Debug, Clone, Copy)]
    pub struct DoorOpenSafetyInterlockReq {
        #[sysml::attr(default = 0.050, unit = "s")]
        pub max_interlock_latency_s: Quantity<f64>,
        #[sysml::attr(default = 0.015, unit = "s")]
        pub cutoff_latency_s: Quantity<f64>,
    }

    // =========================================================================
    // 4. Functional Actions
    // =========================================================================

    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct RegulatePowerAndHeating;

    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct CirculateAndDrainWashWater;

    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ExecuteWashCycleLogic;

    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct PresentCycleStatusAndChime;

    // =========================================================================
    // 5. Subsystem 1: PowerSubsystem
    // =========================================================================

    /// 230 V / 50 Hz AC mains input feed specializing `AcMainsSupply`.
    #[sysml::part]
    #[sysml::specializes(AcMainsSupply)]
    #[derive(Debug, Clone, Copy)]
    pub struct DishwasherAcMains {
        #[sysml::attr(redefines = "nominalVoltage", default = 230.0, unit = "V")]
        pub nominal_voltage: Quantity<f64>,
        #[sysml::attr(redefines = "maxCurrent", default = 16.0, unit = "A")]
        pub max_current: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
    }

    /// 12 V / 5 V DC logic rail power supply specializing `AcDcPowerSupply`.
    #[sysml::part]
    #[sysml::specializes(AcDcPowerSupply)]
    #[derive(Debug, Clone, Copy)]
    pub struct LogicDcPsu {
        #[sysml::attr(redefines = "ratedPower", default = 20.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(redefines = "outputVoltage", default = 12.0, unit = "V")]
        pub output_voltage: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "efficiency", default = 0.88)]
        pub efficiency: f64,
        #[sysml::attr(default = 5.0, unit = "V")]
        pub mcu_rail_voltage: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
    }

    /// Triac / relay power switching stage for the wash heater and pumps.
    #[sysml::part]
    #[sysml::specializes(PowerSwitchStage)]
    #[derive(Debug, Clone, Copy)]
    pub struct TriacRelayStage {
        #[sysml::attr(redefines = "ratedPower", default = 2100.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(redefines = "outputVoltage", default = 230.0, unit = "V")]
        pub output_voltage: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
        #[sysml::port(redefines)]
        pub cmd_in: Conjugated<CommandPort>,
    }

    /// 1800 W tubular resistive sump heater converting electrical energy to wash water heat.
    #[sysml::part]
    #[sysml::specializes(ResistiveHeater)]
    #[sysml::satisfy(req = HygienicWashTemperatureReq, id = "DW-001")]
    #[derive(Debug, Clone, Copy)]
    pub struct WashHeater {
        #[sysml::attr(redefines = "ratedPower", default = 1800.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "efficiency", default = 0.96)]
        pub efficiency: f64,
        #[sysml::attr(default = 338.15, unit = "K")]
        pub max_operating_temp: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub heat_delivery: ThermalPort,
    }

    /// Power conversion & switching domain: mains feed, 12V/5V DC PSU, triac/relay driver, 1800 W heater.
    #[sysml::part]
    #[sysml::perform(regulate: RegulatePowerAndHeating)]
    #[sysml::satisfy(req = PeakMainsPowerBudgetReq, id = "DW-003")]
    #[sysml::connect(
        interface = ElectricalLink,
        name = "mainsToDcPsu",
        from = mains_supply.power_out,
        to = dc_psu.power_in
    )]
    #[sysml::connect(
        interface = ElectricalLink,
        name = "mainsToSwitchStage",
        from = mains_supply.power_out,
        to = power_switch.power_in
    )]
    #[sysml::connect(
        interface = ElectricalLink,
        name = "switchToHeater",
        from = power_switch.power_out,
        to = wash_heater.power_in
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct PowerSubsystem {
        /// Aggregate of the subsystem's loads: counted through its parts, not again here.
        #[sysml::attr(budget = "none", default = 1920.0, unit = "W")]
        pub peak_power_draw: Quantity<f64>,
        #[sysml::attr(default = 12.0, unit = "V")]
        pub logic_rail_voltage: Quantity<f64>,
        #[sysml::attr(default = 5.0, unit = "V")]
        pub mcu_rail_voltage: Quantity<f64>,
        #[sysml::port]
        pub dc_rail_out: ElectricalPowerPort,
        #[sysml::port]
        pub switched_pump_power_out: ElectricalPowerPort,
        #[sysml::port]
        pub switch_cmd_in: Conjugated<CommandPort>,
        #[sysml::port]
        pub heater_thermal_out: ThermalPort,
        #[sysml::port]
        pub psu_waste_heat_out: ThermalPort,
        #[sysml::part]
        pub mains_supply: DishwasherAcMains,
        #[sysml::part]
        pub dc_psu: LogicDcPsu,
        #[sysml::part]
        pub power_switch: TriacRelayStage,
        #[sysml::part]
        pub wash_heater: WashHeater,
    }

    // =========================================================================
    // 6. Subsystem 2: FluidicMechSubsystem
    // =========================================================================

    /// Cold-water inlet solenoid valve.
    #[sysml::part]
    #[sysml::specializes(FluidValve)]
    #[derive(Debug, Clone, Copy)]
    pub struct InletSolenoidValve {
        #[sysml::attr(redefines = "maxFlowRate", default = 0.12, unit = "kg/s")]
        pub max_flow_rate: Quantity<f64>,
        #[sysml::port(redefines)]
        pub cmd_in: Conjugated<CommandPort>,
        #[sysml::port(redefines)]
        pub fluid_in: Conjugated<FluidPort>,
        #[sysml::port(redefines)]
        pub fluid_out: FluidPort,
    }

    /// High-flow wash circulation pump feeding spray arms.
    #[sysml::part]
    #[sysml::specializes(FluidPump)]
    #[derive(Debug, Clone, Copy)]
    pub struct CirculationPump {
        #[sysml::attr(redefines = "flowRate", default = 0.75, unit = "kg/s")]
        pub flow_rate: Quantity<f64>,
        #[sysml::attr(redefines = "ratedPower", default = 95.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "noiseLevelDb", default = 43.0)]
        pub noise_level_db: f64,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub cmd_in: Conjugated<CommandPort>,
        #[sysml::port(redefines)]
        pub fluid_in: Conjugated<FluidPort>,
        #[sysml::port(redefines)]
        pub fluid_out: FluidPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
    }

    /// Waste-water drain pump discharging sump water.
    #[sysml::part]
    #[sysml::specializes(FluidPump)]
    #[derive(Debug, Clone, Copy)]
    pub struct DrainPump {
        #[sysml::attr(redefines = "flowRate", default = 0.35, unit = "kg/s")]
        pub flow_rate: Quantity<f64>,
        #[sysml::attr(redefines = "ratedPower", default = 35.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "noiseLevelDb", default = 45.0)]
        pub noise_level_db: f64,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub cmd_in: Conjugated<CommandPort>,
        #[sysml::port(redefines)]
        pub fluid_in: Conjugated<FluidPort>,
        #[sysml::port(redefines)]
        pub fluid_out: FluidPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
    }

    /// Rotary hydraulic spray arm assembly dispersing pressurized wash water.
    #[sysml::part]
    #[sysml::specializes(FluidDistributor)]
    #[derive(Debug, Clone, Copy)]
    pub struct SprayArmAssembly {
        #[sysml::attr(dimensionless, redefines = "noiseLevelDb", default = 42.0)]
        pub noise_level_db: f64,
        #[sysml::port(redefines)]
        pub fluid_in: Conjugated<FluidPort>,
        #[sysml::port(redefines)]
        pub fluid_out: FluidPort,
    }

    /// Insulated stainless-steel wash tub with water volume, thermal retention, and acoustic insulation.
    #[sysml::part]
    #[sysml::specializes(ThermalEnclosure)]
    #[sysml::satisfy(req = QuietOperationNoiseReq, id = "DW-002")]
    #[derive(Debug, Clone, Copy)]
    pub struct WashTub {
        #[sysml::attr(dimensionless, redefines = "noiseLevelDb", default = 44.0)]
        pub noise_level_db: f64,
        #[sysml::attr(redefines = "thermalConductanceWPerK", default = 1.8, unit = "W/K")]
        pub thermal_conductance_w_per_k: f64,
        #[sysml::attr(dimensionless, redefines = "acousticAttenuationDb", default = 14.0)]
        pub acoustic_attenuation_db: f64,
        #[sysml::attr(default = 4.5, unit = "L")]
        pub water_volume: Quantity<f64>,
        /// Water charge (a consumable load), not equipment mass.
        #[sysml::attr(budget = "none", default = 4.5, unit = "kg")]
        pub water_mass: Quantity<f64>,
        #[sysml::port(redefines)]
        pub heat_in: Conjugated<ThermalPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
        #[sysml::port]
        pub fluid_in: Conjugated<FluidPort>,
        #[sysml::port]
        pub fluid_out: FluidPort,
    }

    /// Fluidic & mechanical domain: inlet valve, circulation/drain pumps, spray arms, insulated tub.
    #[sysml::part]
    #[sysml::perform(circulate_and_drain: CirculateAndDrainWashWater)]
    #[sysml::connect(
        interface = FluidLink,
        name = "inletToTub",
        from = inlet_valve.fluid_out,
        to = wash_tub.fluid_in
    )]
    #[sysml::connect(
        interface = FluidLink,
        name = "tubToCircPump",
        from = wash_tub.fluid_out,
        to = circ_pump.fluid_in
    )]
    #[sysml::connect(
        interface = FluidLink,
        name = "circPumpToSprayArm",
        from = circ_pump.fluid_out,
        to = spray_arm.fluid_in
    )]
    #[sysml::connect(
        interface = FluidLink,
        name = "sprayArmToTub",
        from = spray_arm.fluid_out,
        to = wash_tub.fluid_in
    )]
    #[sysml::connect(
        interface = FluidLink,
        name = "tubToDrainPump",
        from = wash_tub.fluid_out,
        to = drain_pump.fluid_in
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct FluidicMechSubsystem {
        #[sysml::port]
        pub pump_power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port]
        pub fluidic_cmd_in: Conjugated<CommandPort>,
        #[sysml::port]
        pub tub_heat_in: Conjugated<ThermalPort>,
        #[sysml::port]
        pub tub_heat_to_sensor: ThermalPort,
        #[sysml::port]
        pub tub_heat_to_ambient: ThermalPort,
        #[sysml::port]
        pub tub_fluid_to_sensor: FluidPort,
        #[sysml::port]
        pub ambient_noise_out: AcousticPort,
        #[sysml::part]
        pub inlet_valve: InletSolenoidValve,
        #[sysml::part]
        pub circ_pump: CirculationPump,
        #[sysml::part]
        pub drain_pump: DrainPump,
        #[sysml::part]
        pub spray_arm: SprayArmAssembly,
        #[sysml::part]
        pub wash_tub: WashTub,
    }

    // =========================================================================
    // 7. Subsystem 3: ElectronicsLogicSubsystem
    // =========================================================================

    /// Embedded wash cycle microcontroller orchestrating heating, pumps, sensors, and safety.
    #[sysml::part]
    #[sysml::specializes(Microcontroller)]
    #[derive(Debug, Clone, Copy)]
    pub struct WashControllerMcu {
        #[sysml::attr(redefines = "powerDraw", default = 2.5, unit = "W")]
        pub power_draw: Quantity<f64>,
        #[sysml::attr(default = 333.15, unit = "K")]
        pub target_wash_temp_k: Quantity<f64>,
        #[sysml::attr(dimensionless, default = 0.5)]
        pub door_closed_factor: f64,
        #[sysml::attr(dimensionless, default = 0.5)]
        pub flood_trip_factor: f64,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub cmd_out: CommandPort,
        #[sysml::port(redefines)]
        pub sensor_in: Conjugated<TelemetryPort>,
        #[sysml::port]
        pub mcu_hmi_cmd_in: Conjugated<CommandPort>,
        #[sysml::port]
        pub mcu_hmi_status_out: TelemetryPort,
        #[sysml::port]
        pub level_telemetry_in: Conjugated<TelemetryPort>,
        #[sysml::port]
        pub interlock_telemetry_in: Conjugated<TelemetryPort>,
    }

    /// Sump NTC temperature sensor.
    #[sysml::part]
    #[sysml::specializes(TemperatureSensor)]
    #[derive(Debug, Clone, Copy)]
    pub struct WaterTempSensor {
        #[sysml::attr(redefines = "maxMeasurableTemp", default = 368.15, unit = "K")]
        pub max_measurable_temp: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub thermal_sense: Conjugated<ThermalPort>,
        #[sysml::port(redefines)]
        pub reading_out: TelemetryPort,
    }

    /// Optical turbidity and sump flood/water-level sensor.
    #[sysml::part]
    #[sysml::specializes(FluidSensor)]
    #[derive(Debug, Clone, Copy)]
    pub struct TurbidityLevelSensor {
        #[sysml::attr(dimensionless, default = 1.0)]
        pub optical_gain_factor: f64,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub fluid_sense: Conjugated<FluidPort>,
        #[sysml::port(redefines)]
        pub reading_out: TelemetryPort,
    }

    /// Mechanical-electrical door latch safety interlock switch.
    #[sysml::part]
    #[sysml::specializes(SafetyInterlockSwitch)]
    #[sysml::satisfy(req = DoorOpenSafetyInterlockReq, id = "DW-004")]
    #[derive(Debug, Clone, Copy)]
    pub struct DoorLatchInterlock {
        #[sysml::attr(redefines = "cutoffLatency", default = 0.015, unit = "s")]
        pub cutoff_latency: Quantity<f64>,
        #[sysml::port(redefines)]
        pub mechanical_actuation: Conjugated<HmiInputPort>,
        #[sysml::port(redefines)]
        pub interlock_status: TelemetryPort,
    }

    /// Wash cycle state machine supervised by `WashControllerMcu`.
    #[sysml::state(subject = WashControllerMcu)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum WashCycleState {
        /// Waiting for user cycle start with door latched.
        #[sysml::initial]
        #[sysml::transition(
            on = CycleStartTrigger,
            guard = event.start_pressed_flag >= 1.0,
            to = Fill
        )]
        Idle,
        /// Filling sump through inlet solenoid valve.
        #[sysml::transition(
            on = WaterLevelTrigger,
            guard = event.flood_detected_flag >= subject.flood_trip_factor,
            to = FloodFault
        )]
        #[sysml::transition(
            on = WaterLevelTrigger,
            guard = event.fill_complete_flag >= 1.0,
            to = WashHeat
        )]
        #[sysml::transition(
            on = DoorLatchTrigger,
            guard = event.door_closed_flag < subject.door_closed_factor,
            to = DoorOpenPause
        )]
        Fill,
        /// Circulating wash water while energizing 1800 W heater to reach >= 333.15 K.
        #[sysml::transition(
            on = DoorLatchTrigger,
            guard = event.door_closed_flag < subject.door_closed_factor,
            to = DoorOpenPause
        )]
        #[sysml::transition(
            on = WaterLevelTrigger,
            guard = event.flood_detected_flag >= subject.flood_trip_factor,
            to = FloodFault
        )]
        #[sysml::transition(
            on = WashThermalTrigger,
            guard = (event.water_temp_k >= subject.target_wash_temp_k) && (event.wash_timer_done_flag >= 1.0),
            to = Rinse
        )]
        WashHeat,
        /// Fresh-water rinse circulation stage.
        #[sysml::transition(
            on = DoorLatchTrigger,
            guard = event.door_closed_flag < subject.door_closed_factor,
            to = DoorOpenPause
        )]
        #[sysml::transition(
            on = StageProgressTrigger,
            guard = event.flood_detected_flag >= subject.flood_trip_factor,
            to = FloodFault
        )]
        #[sysml::transition(
            on = StageProgressTrigger,
            guard = event.stage_complete_flag >= 1.0,
            to = Dry
        )]
        Rinse,
        /// Residual-heat / condensation drying stage.
        #[sysml::transition(
            on = DoorLatchTrigger,
            guard = event.door_closed_flag < subject.door_closed_factor,
            to = DoorOpenPause
        )]
        #[sysml::transition(
            on = StageProgressTrigger,
            guard = event.stage_complete_flag >= 1.0,
            to = Drain
        )]
        Dry,
        /// Final sump drain back to Idle.
        #[sysml::transition(
            on = StageProgressTrigger,
            guard = event.flood_detected_flag >= subject.flood_trip_factor,
            to = FloodFault
        )]
        #[sysml::transition(
            on = StageProgressTrigger,
            guard = event.stage_complete_flag >= 1.0,
            to = Idle
        )]
        Drain,
        /// Safety-interlocked pause when door is opened mid-cycle; heater and pumps disabled.
        #[sysml::transition(
            on = DoorLatchTrigger,
            guard = event.door_closed_flag >= subject.door_closed_factor,
            to = WashHeat
        )]
        #[sysml::transition(
            on = WaterLevelTrigger,
            guard = event.flood_detected_flag >= subject.flood_trip_factor,
            to = FloodFault
        )]
        DoorOpenPause,
        /// Sump pan flood float triggered: close inlet valve, isolate heater, engage emergency drain.
        #[sysml::transition(
            on = StageProgressTrigger,
            guard = (event.flood_detected_flag < subject.flood_trip_factor) && (event.stage_complete_flag >= 1.0),
            to = Drain
        )]
        FloodFault,
    }

    /// Electronics & logic domain: MCU, temperature sensor, turbidity/level sensor, door interlock.
    #[sysml::part]
    #[sysml::perform(execute_logic: ExecuteWashCycleLogic)]
    #[sysml::exhibit(name = "washCycleStateMachine", state = WashCycleState)]
    #[sysml::connect(
        interface = TelemetryChannel,
        name = "tempToMcu",
        from = temp_sensor.reading_out,
        to = mcu.sensor_in
    )]
    #[sysml::connect(
        interface = TelemetryChannel,
        name = "turbidityToMcu",
        from = turbidity_sensor.reading_out,
        to = mcu.level_telemetry_in
    )]
    #[sysml::connect(
        interface = TelemetryChannel,
        name = "doorInterlockToMcu",
        from = door_interlock.interlock_status,
        to = mcu.interlock_telemetry_in
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct ElectronicsLogicSubsystem {
        #[sysml::port]
        pub dc_power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port]
        pub actuator_cmd_out: CommandPort,
        #[sysml::port]
        pub hmi_cmd_in: Conjugated<CommandPort>,
        #[sysml::port]
        pub hmi_status_out: TelemetryPort,
        #[sysml::port]
        pub thermal_sense_in: Conjugated<ThermalPort>,
        #[sysml::port]
        pub fluid_sense_in: Conjugated<FluidPort>,
        #[sysml::port]
        pub door_actuation_in: Conjugated<HmiInputPort>,
        #[sysml::part]
        pub mcu: WashControllerMcu,
        #[sysml::part]
        pub temp_sensor: WaterTempSensor,
        #[sysml::part]
        pub turbidity_sensor: TurbidityLevelSensor,
        #[sysml::part]
        pub door_interlock: DoorLatchInterlock,
    }

    // =========================================================================
    // 8. Subsystem 4: HmiSubsystem
    // =========================================================================

    /// Front-panel buttons, remaining-time LED/LCD display, and end-of-cycle chime.
    #[sysml::part]
    #[sysml::specializes(HmiPanel)]
    #[derive(Debug, Clone, Copy)]
    pub struct DishwasherControlPanel {
        #[sysml::attr(dimensionless, redefines = "chimeLevelDb", default = 58.0)]
        pub chime_level_db: f64,
        /// Wash-program duration, not a processing-path latency.
        #[sysml::attr(budget = "none", default = 5400.0, unit = "s")]
        pub nominal_cycle_duration: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub user_in: Conjugated<HmiInputPort>,
        #[sysml::port(redefines)]
        pub user_out: HmiFeedbackPort,
        #[sysml::port(redefines)]
        pub cmd_to_logic: CommandPort,
        #[sysml::port(redefines)]
        pub status_from_logic: Conjugated<TelemetryPort>,
        #[sysml::port(redefines)]
        pub chime_out: AcousticPort,
    }

    /// Human-Machine Interface subsystem wrapping the dishwasher control panel.
    #[sysml::part]
    #[sysml::perform(present_hmi: PresentCycleStatusAndChime)]
    #[derive(Debug, Clone, Copy)]
    pub struct HmiSubsystem {
        #[sysml::port]
        pub hmi_power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port]
        pub operator_in: Conjugated<HmiInputPort>,
        #[sysml::port]
        pub operator_out: HmiFeedbackPort,
        #[sysml::port]
        pub logic_cmd_out: CommandPort,
        #[sysml::port]
        pub logic_status_in: Conjugated<TelemetryPort>,
        #[sysml::port]
        pub chime_noise_out: AcousticPort,
        #[sysml::part]
        pub panel: DishwasherControlPanel,
    }

    // =========================================================================
    // 9. Dishwasher Product Assembly & Top-Level Context
    // =========================================================================

    /// Complete multi-domain Dishwasher appliance composed of 4 clean domain subsystems.
    #[sysml::part]
    #[sysml::connect(
        interface = ElectricalLink,
        name = "dcToElectronics",
        from = power.dc_rail_out,
        to = electronics.dc_power_in
    )]
    #[sysml::connect(
        interface = ElectricalLink,
        name = "dcToHmi",
        from = power.dc_rail_out,
        to = hmi.hmi_power_in
    )]
    #[sysml::connect(
        interface = ElectricalLink,
        name = "switchedPowerToPumps",
        from = power.switched_pump_power_out,
        to = fluidic_mech.pump_power_in
    )]
    #[sysml::connect(
        interface = CommandChannel,
        name = "mcuToPowerSwitch",
        from = electronics.actuator_cmd_out,
        to = power.switch_cmd_in
    )]
    #[sysml::connect(
        interface = CommandChannel,
        name = "mcuToFluidicActuators",
        from = electronics.actuator_cmd_out,
        to = fluidic_mech.fluidic_cmd_in
    )]
    #[sysml::connect(
        interface = CommandChannel,
        name = "hmiToMcuCmd",
        from = hmi.logic_cmd_out,
        to = electronics.hmi_cmd_in
    )]
    #[sysml::connect(
        interface = TelemetryChannel,
        name = "mcuToHmiStatus",
        from = electronics.hmi_status_out,
        to = hmi.logic_status_in
    )]
    #[sysml::connect(
        interface = ThermalLink,
        name = "heaterToTub",
        from = power.heater_thermal_out,
        to = fluidic_mech.tub_heat_in
    )]
    #[sysml::connect(
        interface = ThermalLink,
        name = "tubHeatToTempSensor",
        from = fluidic_mech.tub_heat_to_sensor,
        to = electronics.thermal_sense_in
    )]
    #[sysml::connect(
        interface = FluidLink,
        name = "tubFluidToTurbiditySensor",
        from = fluidic_mech.tub_fluid_to_sensor,
        to = electronics.fluid_sense_in
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct Dishwasher {
        #[sysml::part]
        pub power: PowerSubsystem,
        #[sysml::part]
        pub fluidic_mech: FluidicMechSubsystem,
        #[sysml::part]
        pub electronics: ElectronicsLogicSubsystem,
        #[sysml::part]
        pub hmi: HmiSubsystem,
    }

    /// Top-level operational context wiring HumanOperator, Dishwasher, and AmbientEnvironment.
    #[sysml::part]
    #[sysml::connect(
        interface = HmiInputChannel,
        name = "operatorToPanel",
        from = operator.user_input,
        to = dishwasher.hmi.operator_in
    )]
    #[sysml::connect(
        interface = HmiInputChannel,
        name = "operatorToDoorLatch",
        from = operator.user_input,
        to = dishwasher.electronics.door_actuation_in
    )]
    #[sysml::connect(
        interface = HmiFeedbackChannel,
        name = "panelToOperator",
        from = dishwasher.hmi.operator_out,
        to = operator.user_feedback
    )]
    #[sysml::connect(
        interface = ThermalLink,
        name = "tubToAmbient",
        from = dishwasher.fluidic_mech.tub_heat_to_ambient,
        to = ambient.heat_sink
    )]
    #[sysml::connect(
        interface = ThermalLink,
        name = "psuToAmbient",
        from = dishwasher.power.psu_waste_heat_out,
        to = ambient.heat_sink
    )]
    #[sysml::connect(
        interface = AcousticLink,
        name = "fluidicMechToAmbientNoise",
        from = dishwasher.fluidic_mech.ambient_noise_out,
        to = ambient.noise_sink
    )]
    #[sysml::connect(
        interface = AcousticLink,
        name = "chimeToAmbient",
        from = dishwasher.hmi.chime_noise_out,
        to = ambient.noise_sink
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct DishwasherContext {
        #[sysml::part]
        pub operator: HumanOperator,
        #[sysml::part]
        pub dishwasher: Dishwasher,
        #[sysml::part]
        pub ambient: AmbientEnvironment,
    }
}

/// Builds the complete `sysml_ir::Model` prepending `sysml-std` packages + `DishwasherModel`.
pub fn build_model() -> Model {
    let mut packages = sysml_std::std_packages();
    packages.push(dishwasher_model::sysml_package());
    Model::new(packages)
}

#[cfg(test)]
mod tests {
    use super::dishwasher_model::*;
    use super::*;
    use sysml_emit::{audit_model_with_stdlib, emit_model, lint_model};
    use sysml_std::prelude::{
        attenuated_noise_db, convective_heat_loss_w, heating_time_s, sensible_heat_energy_j,
    };

    #[test]
    fn dishwasher_model_audits_zero_errors_zero_warnings_zero_orphans() {
        let model = build_model();
        let sysml = emit_model(&model);
        let audit = audit_model_with_stdlib("dishwasher", &model, None, None, true);
        assert!(audit.stdlib_loaded, "OMG sysml.library must be loaded");
        assert_eq!(audit.total_requirements, 4);
        assert_eq!(audit.satisfied_requirements, 4);
        assert!(
            audit.orphan_requirements.is_empty(),
            "unexpected orphan requirements: {:?}",
            audit.orphan_requirements
        );
        assert!(
            audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
            "dishwasher audit failed: {audit:#?}\n--- SYSML ---\n{sysml}"
        );
    }

    #[test]
    fn dishwasher_model_lints_clean() {
        let model = build_model();
        let report = lint_model("dishwasher", &model);
        assert!(
            report.clean && report.summary.warning_count == 0,
            "lint report had warnings: {report:#?}"
        );
        assert_eq!(
            report.summary.info_count, 0,
            "lint report had infos: {report:#?}"
        );
    }

    #[test]
    fn dishwasher_physics_calcs_execute_accurately() {
        // 4.5 kg wash water heated from 288.15 K (15 C) to 338.15 K (65 C): dT = 50 K, c_p = 4184 J/(kg*K)
        let heat_j = sensible_heat_energy_j(4.5, 4184.0, 50.0);
        assert!((heat_j - 941_400.0).abs() < 1e-6);

        // 1800 W heater at 96% thermal efficiency -> 1728 W effective -> ~544.79 s
        let t_heat_s = heating_time_s(heat_j, 1800.0, 0.96);
        assert!((t_heat_s - (941_400.0 / 1728.0)).abs() < 1e-6);

        // Tub thermal loss at 65 C (338.15 K) in 22 C (295.15 K) ambient with G = 1.8 W/K -> 77.4 W
        let tub_loss_w = convective_heat_loss_w(1.8, 338.15, 295.15);
        assert!((tub_loss_w - 77.4).abs() < 1e-6);

        // Attenuated acoustic emission with 14 dB tub insulation from 58 dB source -> 44 dB <= 46 dB
        let emitted_db = attenuated_noise_db(58.0, 14.0);
        assert!((emitted_db - 44.0).abs() < 1e-6);

        // Total cycle energy: heating + 95 W pump for 2400 s + 4.5 W electronics for 5400 s
        let total_j = total_cycle_energy_j(heat_j, 95.0, 2400.0, 4.5, 5400.0);
        assert!((total_j - (941_400.0 + 228_000.0 + 24_300.0)).abs() < 1e-6);
    }

    #[test]
    fn dishwasher_requirements_evaluate_nominal_and_violated() {
        assert!(HygienicWashTemperatureReq::default().evaluate());
        assert!(!HygienicWashTemperatureReq {
            achieved_wash_temp_k: Quantity::new(323.15, "K"),
            ..HygienicWashTemperatureReq::default()
        }
        .evaluate());

        let emitted_db = attenuated_noise_db(58.0, WashTub::default().acoustic_attenuation_db);
        assert!(QuietOperationNoiseReq {
            emitted_noise_db: emitted_db,
            ..QuietOperationNoiseReq::default()
        }
        .evaluate());
        assert!(!QuietOperationNoiseReq {
            emitted_noise_db: 52.0,
            ..QuietOperationNoiseReq::default()
        }
        .evaluate());

        assert!(PeakMainsPowerBudgetReq::default().evaluate());
        assert!(!PeakMainsPowerBudgetReq {
            peak_power_w: Quantity::new(2350.0, "W"),
            ..PeakMainsPowerBudgetReq::default()
        }
        .evaluate());

        assert!(DoorOpenSafetyInterlockReq {
            cutoff_latency_s: DoorLatchInterlock::default().cutoff_latency,
            ..DoorOpenSafetyInterlockReq::default()
        }
        .evaluate());
        assert!(!DoorOpenSafetyInterlockReq {
            cutoff_latency_s: Quantity::new(0.080, "s"),
            ..DoorOpenSafetyInterlockReq::default()
        }
        .evaluate());
    }

    #[test]
    fn dishwasher_state_machine_handles_nominal_door_pause_and_flood_fault() {
        let mcu = WashControllerMcu::default();
        let mut state = WashCycleState::initial_state();
        assert_eq!(state, WashCycleState::Idle);

        // 1. Start cycle -> Fill
        let start = CycleStartTrigger {
            target_temp_k: 333.15,
            start_pressed_flag: 1.0,
        };
        assert_eq!(state.step_with(&start, &mcu).unwrap(), WashCycleState::Fill);

        // 2. Fill complete (no flood) -> WashHeat
        let fill_ok = WaterLevelTrigger {
            fill_complete_flag: 1.0,
            flood_detected_flag: 0.0,
        };
        assert_eq!(
            state.step_with(&fill_ok, &mcu).unwrap(),
            WashCycleState::WashHeat
        );

        // 3. Door opened mid-wash -> immediate DoorOpenPause
        let door_open = DoorLatchTrigger {
            door_closed_flag: 0.0,
        };
        assert_eq!(
            state.step_with(&door_open, &mcu).unwrap(),
            WashCycleState::DoorOpenPause
        );

        // 4. Door closed -> resume WashHeat
        let door_closed = DoorLatchTrigger {
            door_closed_flag: 1.0,
        };
        assert_eq!(
            state.step_with(&door_closed, &mcu).unwrap(),
            WashCycleState::WashHeat
        );

        // 5. Wash target temp below threshold does not advance to Rinse
        let too_cool = WashThermalTrigger {
            water_temp_k: 320.0,
            wash_timer_done_flag: 1.0,
        };
        assert!(state.step_with(&too_cool, &mcu).is_err());
        assert_eq!(state, WashCycleState::WashHeat);

        // 6. Wash target temp >= 333.15 K and timer done -> Rinse
        let hot_done = WashThermalTrigger {
            water_temp_k: 335.15,
            wash_timer_done_flag: 1.0,
        };
        assert_eq!(
            state.step_with(&hot_done, &mcu).unwrap(),
            WashCycleState::Rinse
        );

        // 7. Flood float triggered during Rinse -> immediate FloodFault
        let flood = StageProgressTrigger {
            stage_complete_flag: 0.0,
            flood_detected_flag: 1.0,
        };
        assert_eq!(
            state.step_with(&flood, &mcu).unwrap(),
            WashCycleState::FloodFault
        );

        // 8. Flood cleared after emergency drain stage -> Drain -> Idle
        let flood_cleared = StageProgressTrigger {
            stage_complete_flag: 1.0,
            flood_detected_flag: 0.0,
        };
        assert_eq!(
            state.step_with(&flood_cleared, &mcu).unwrap(),
            WashCycleState::Drain
        );
        assert_eq!(
            state.step_with(&flood_cleared, &mcu).unwrap(),
            WashCycleState::Idle
        );

        // 9. Nominal Dry -> Drain -> Idle path check
        assert_eq!(state.step_with(&start, &mcu).unwrap(), WashCycleState::Fill);
        assert_eq!(
            state.step_with(&fill_ok, &mcu).unwrap(),
            WashCycleState::WashHeat
        );
        assert_eq!(
            state.step_with(&hot_done, &mcu).unwrap(),
            WashCycleState::Rinse
        );
        let stage_done = StageProgressTrigger {
            stage_complete_flag: 1.0,
            flood_detected_flag: 0.0,
        };
        assert_eq!(
            state.step_with(&stage_done, &mcu).unwrap(),
            WashCycleState::Dry
        );
        assert_eq!(
            state.step_with(&stage_done, &mcu).unwrap(),
            WashCycleState::Drain
        );
        assert_eq!(
            state.step_with(&stage_done, &mcu).unwrap(),
            WashCycleState::Idle
        );
    }
}
