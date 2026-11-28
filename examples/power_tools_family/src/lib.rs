//! Shared-platform Cordless Power Tool Family (18V Battery, Drill, Impact Driver, Circular Saw)
//!
//! Showcases shared modular parts, inheritance (`#[sysml::specializes]`),
//! attribute/port redefinitions (`redefines`), and product-line variants (`#[sysml::variation]`)
//! built cleanly on top of `sysml-std v1.0`.

use sysml_dsl::prelude::*;
use sysml_std::prelude::*;

#[sysml::package(name = "PowerToolsFamilyModel")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
#[sysml::import("SysmlStdComms::*")]
#[sysml::import("SysmlStdPower::*")]
#[sysml::import("SysmlStdMechanics::*")]
#[sysml::import("SysmlStdElectronics::*")]
#[sysml::import("SysmlStdHmi::*")]
pub mod power_tools_family_model {
    use super::*;

    // =========================================================================
    // 1. Shared 18V Battery Platform (specializes sysml-std `BatteryPack`)
    // =========================================================================

    /// Abstract 18V lithium-ion battery platform sharing voltage rail, BMS bus, and thermal/power ports.
    #[sysml::part(abstract)]
    #[sysml::specializes(BatteryPack)]
    #[derive(Debug, Clone, Copy)]
    pub struct Battery18VBase {
        #[sysml::attr(redefines = "nominalVoltage", default = 18.0, unit = "V")]
        pub nominal_voltage: Quantity<f64>,
        #[sysml::attr(redefines = "maxCurrent", default = 45.0, unit = "A")]
        pub max_current: Quantity<f64>,
        #[sysml::attr(redefines = "maxCellTemp", default = 323.15, unit = "K")]
        pub max_cell_temp: Quantity<f64>,
        #[sysml::port(redefines)]
        pub charge_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub bms_bus: DataBusPort,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub bms_telemetry: TelemetryPort,
    }

    /// Compact 18V 2.0 Ah pack (36 Wh = 129,600 J) optimized for low mass.
    #[sysml::part]
    #[sysml::specializes(Battery18VBase)]
    #[derive(Debug, Clone, Copy)]
    pub struct Battery18V2Ah {
        #[sysml::attr(redefines = "energyCapacity", default = 129600.0, unit = "J")]
        pub energy_capacity: Quantity<f64>,
        #[sysml::attr(redefines = "mass", default = 0.40, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::port(redefines)]
        pub charge_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub bms_bus: DataBusPort,
    }

    /// High-capacity 18V 5.0 Ah pack (90 Wh = 324,000 J) for sustained high-power tools.
    #[sysml::part]
    #[sysml::specializes(Battery18VBase)]
    #[derive(Debug, Clone, Copy)]
    pub struct Battery18V5Ah {
        #[sysml::attr(redefines = "energyCapacity", default = 324000.0, unit = "J")]
        pub energy_capacity: Quantity<f64>,
        #[sysml::attr(redefines = "mass", default = 0.68, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "maxCurrent", default = 70.0, unit = "A")]
        pub max_current: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
        #[sysml::port(redefines)]
        pub bms_telemetry: TelemetryPort,
        #[sysml::port(redefines)]
        pub bms_bus: DataBusPort,
    }

    /// Interchangeable 18V slide-on battery pack product-line variation.
    #[sysml::variation(of = Battery18VBase)]
    #[derive(Debug, Clone, Copy)]
    pub enum BatteryPackVariant {
        Compact2Ah(Battery18V2Ah),
        Pro5Ah(Battery18V5Ah),
    }

    // =========================================================================
    // 2. Shared Tool Subsystems (with `#[sysml::port(redefines)]`)
    // =========================================================================

    /// Embedded tool MCU running variable-speed PWM control and BMS thermal/overcurrent cutoff.
    #[sysml::part]
    #[sysml::specializes(Microcontroller)]
    #[derive(Debug, Clone, Copy)]
    pub struct ToolControllerMcu {
        #[sysml::attr(redefines = "supplyVoltage", default = 3.3, unit = "V")]
        pub supply_voltage: Quantity<f64>,
        #[sysml::attr(default = 20000.0, unit = "Hz")]
        pub pwm_carrier_freq: Quantity<f64>,
        #[sysml::attr(default = 343.15, unit = "K")]
        pub cutoff_temp: Quantity<f64>,
        #[sysml::attr(default = 65.0, unit = "A")]
        pub overcurrent_limit: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub sensor_in: Conjugated<TelemetryPort>,
        #[sysml::port(redefines)]
        pub cmd_out: CommandPort,
        #[sysml::port(redefines)]
        pub bus: DataBusPort,
        #[sysml::port]
        pub trigger_cmd_in: Conjugated<CommandPort>,
        #[sysml::port]
        pub bms_status_in: Conjugated<TelemetryPort>,
        #[sysml::port]
        pub hmi_status_out: TelemetryPort,
        #[sysml::port]
        pub guard_sensor_in: Conjugated<TelemetryPort>,
    }

    /// Ergonomic tool trigger HMI: variable-speed trigger switch, direction selector, and LED worklight.
    #[sysml::part]
    #[sysml::specializes(HmiPanel)]
    #[derive(Debug, Clone, Copy)]
    pub struct TriggerHmi {
        #[sysml::attr(redefines = "mass", default = 0.08, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(budget = "power", default = 1.5, unit = "W")]
        pub worklight_power: Quantity<f64>,
        #[sysml::attr(default = true)]
        pub forward_direction: bool,
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
    }

    /// 3-phase MOSFET inverter stage specializing `sysml-std::ThreePhaseInverter`.
    #[sysml::part]
    #[sysml::specializes(ThreePhaseInverter)]
    #[derive(Debug, Clone, Copy)]
    pub struct MotorInverterStage {
        #[sysml::attr(redefines = "outputVoltage", default = 18.0, unit = "V")]
        pub output_voltage: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "efficiency", default = 0.96)]
        pub efficiency: f64,
        #[sysml::attr(redefines = "ratedPower", default = 1200.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(redefines = "overcurrentLimit", default = 70.0, unit = "A")]
        pub overcurrent_limit: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
        #[sysml::port(redefines)]
        pub cmd_in: Conjugated<CommandPort>,
    }

    /// Compact planetary reduction gearbox coupling the brushless motor to the tool spindle.
    #[sysml::part]
    #[sysml::specializes(MechanicalTransmission)]
    #[derive(Debug, Clone, Copy)]
    pub struct PlanetaryGearbox {
        #[sysml::attr(redefines = "mass", default = 0.35, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "gearRatio", default = 12.0)]
        pub gear_ratio: f64,
        #[sysml::attr(dimensionless, redefines = "efficiency", default = 0.92)]
        pub efficiency: f64,
        #[sysml::attr(dimensionless, redefines = "noiseLevelDb", default = 74.0)]
        pub noise_level_db: f64,
        #[sysml::port(redefines)]
        pub mech_in: Conjugated<MechanicalPowerPort>,
    }

    /// High-power brushless DC spindle motor shared across the 18V tool family.
    #[sysml::part]
    #[sysml::specializes(BrushlessDcMotor)]
    #[derive(Debug, Clone, Copy)]
    pub struct SpindleMotor {
        #[sysml::attr(redefines = "mass", default = 0.42, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "ratedPower", default = 600.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "efficiency", default = 0.89)]
        pub efficiency: f64,
        #[sysml::attr(dimensionless, redefines = "noiseLevelDb", default = 72.0)]
        pub noise_level_db: f64,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub cmd_in: Conjugated<CommandPort>,
        #[sysml::port(redefines)]
        pub mech_out: MechanicalPowerPort,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
    }

    /// Tool-mounted NTC thermistor monitoring spindle motor winding temperature.
    #[sysml::part]
    #[sysml::specializes(TemperatureSensor)]
    #[derive(Debug, Clone, Copy)]
    pub struct MotorTempSensor {
        #[sysml::attr(redefines = "maxMeasurableTemp", default = 423.15, unit = "K")]
        pub max_measurable_temp: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub reading_out: TelemetryPort,
        #[sysml::port(redefines)]
        pub thermal_sense: Conjugated<ThermalPort>,
    }

    // =========================================================================
    // 3. Tool-Specific Mechanical End-Effectors, Safety Interlock & Charger
    // =========================================================================

    /// Keyless 13 mm chuck gearbox variant for rotary drilling and fastening.
    #[sysml::part]
    #[sysml::specializes(PlanetaryGearbox)]
    #[derive(Debug, Clone, Copy)]
    pub struct DrillChuckGearbox {
        #[sysml::attr(dimensionless, redefines = "gearRatio", default = 18.0)]
        pub gear_ratio: f64,
        #[sysml::attr(dimensionless, redefines = "efficiency", default = 0.92)]
        pub efficiency: f64,
        #[sysml::attr(dimensionless, redefines = "noiseLevelDb", default = 78.0)]
        pub noise_level_db: f64,
        #[sysml::attr(default = 0.013, unit = "m")]
        pub chuck_capacity: Quantity<f64>,
        #[sysml::port(redefines)]
        pub mech_in: Conjugated<MechanicalPowerPort>,
    }

    /// Rotary anvil hammer mechanism generating high-torque tangential impacts.
    #[sysml::part]
    #[sysml::specializes(MechanicalTransmission)]
    #[derive(Debug, Clone, Copy)]
    pub struct ImpactAnvilMechanism {
        #[sysml::attr(redefines = "mass", default = 0.42, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "gearRatio", default = 9.0)]
        pub gear_ratio: f64,
        #[sysml::attr(dimensionless, redefines = "efficiency", default = 0.86)]
        pub efficiency: f64,
        #[sysml::attr(dimensionless, redefines = "noiseLevelDb", default = 92.0)]
        pub noise_level_db: f64,
        #[sysml::attr(default = 55.0, unit = "Hz")]
        pub impact_rate: Quantity<f64>,
        #[sysml::port(redefines)]
        pub mech_in: Conjugated<MechanicalPowerPort>,
    }

    /// Circular saw blade arbor and blade-guard mechanical safety interlock.
    #[sysml::part]
    #[sysml::specializes(MechanicalTransmission)]
    #[derive(Debug, Clone, Copy)]
    pub struct CircularBladeArbor {
        #[sysml::attr(redefines = "mass", default = 0.48, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "gearRatio", default = 3.2)]
        pub gear_ratio: f64,
        #[sysml::attr(dimensionless, redefines = "efficiency", default = 0.94)]
        pub efficiency: f64,
        #[sysml::attr(dimensionless, redefines = "noiseLevelDb", default = 90.0)]
        pub noise_level_db: f64,
        #[sysml::attr(default = 0.165, unit = "m")]
        pub blade_diameter: Quantity<f64>,
        #[sysml::port(redefines)]
        pub mech_in: Conjugated<MechanicalPowerPort>,
    }

    /// Blade-guard mechanical interlock switch gating circular saw spin-up.
    #[sysml::part]
    #[sysml::specializes(SafetyInterlockSwitch)]
    #[derive(Debug, Clone, Copy)]
    pub struct BladeGuardInterlock {
        #[sysml::attr(redefines = "mass", default = 0.04, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "cutoffLatency", default = 0.012, unit = "s")]
        pub cutoff_latency: Quantity<f64>,
        #[sysml::port(redefines)]
        pub interlock_status: TelemetryPort,
    }

    /// Mains-powered 18V rapid battery charger specializing `sysml-std::BatteryCharger`.
    #[sysml::part]
    #[sysml::specializes(BatteryCharger)]
    #[derive(Debug, Clone, Copy)]
    pub struct RapidBatteryCharger18V {
        #[sysml::attr(redefines = "outputVoltage", default = 21.0, unit = "V")]
        pub output_voltage: Quantity<f64>,
        #[sysml::attr(redefines = "ratedPower", default = 160.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "efficiency", default = 0.91)]
        pub efficiency: f64,
        #[sysml::attr(redefines = "maxChargeCurrent", default = 8.0, unit = "A")]
        pub max_charge_current: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub bms_bus: DataBusPort,
    }

    /// Workshop AC wall mains outlet feeding the battery charger.
    #[sysml::part]
    #[sysml::specializes(AcMainsSupply)]
    #[derive(Debug, Clone, Copy)]
    pub struct WorkshopAcMains {
        #[sysml::attr(redefines = "nominalVoltage", default = 230.0, unit = "V")]
        pub nominal_voltage: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
    }

    /// Workshop ambient thermal and acoustic sink for physical boundary closure.
    #[sysml::part]
    #[sysml::specializes(AmbientEnvironment)]
    #[derive(Debug, Clone, Copy)]
    pub struct WorkshopAmbient {
        #[sysml::attr(redefines = "ambientTemperature", default = 295.15, unit = "K")]
        pub ambient_temperature: Quantity<f64>,
        #[sysml::port(redefines)]
        pub heat_sink: Conjugated<ThermalPort>,
        #[sysml::port(redefines)]
        pub noise_sink: Conjugated<AcousticPort>,
    }

    /// Workshop operator interacting with tool triggers and status LEDs.
    #[sysml::part]
    #[sysml::specializes(HumanOperator)]
    #[derive(Debug, Clone, Copy)]
    pub struct WorkshopOperator {
        #[sysml::attr(redefines = "mass", default = 75.0, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::port(redefines)]
        pub user_input: HmiInputPort,
        #[sysml::port(redefines)]
        pub user_feedback: Conjugated<HmiFeedbackPort>,
    }

    // =========================================================================
    // 4. Functional Actions & Executable Tool Safety State Machine
    // =========================================================================

    #[sysml::action]
    pub struct RegulateMotorPwm;

    #[sysml::action]
    pub struct MonitorThermalAndCurrent;

    #[sysml::action]
    pub struct DeliverSpindleTorque;

    /// Trigger/sensor telemetry snapshot evaluated by the tool safety state machine.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ToolSafetyTelemetry {
        #[sysml::attr(dimensionless, default = 0.0)]
        pub trigger_demand: f64,
        #[sysml::attr(default = 298.15, unit = "K")]
        pub motor_temp: f64,
        #[sysml::attr(default = 0.0, unit = "A")]
        pub phase_current: f64,
    }

    /// Guarded operating & safety-cutoff state machine for the 18V cordless tool controller.
    #[sysml::state(subject = ToolControllerMcu)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ToolOperatingState {
        #[sysml::initial]
        #[sysml::transition(
            on = ToolSafetyTelemetry,
            guard = event.trigger_demand > 0.05 && event.motor_temp <= subject.cutoff_temp && event.phase_current <= subject.overcurrent_limit,
            to = Running
        )]
        Idle,
        #[sysml::transition(
            on = ToolSafetyTelemetry,
            guard = event.motor_temp > subject.cutoff_temp,
            to = ThermalCutoff
        )]
        #[sysml::transition(
            on = ToolSafetyTelemetry,
            guard = event.phase_current > subject.overcurrent_limit,
            to = OvercurrentFault
        )]
        #[sysml::transition(
            on = ToolSafetyTelemetry,
            guard = event.trigger_demand <= 0.05 && event.motor_temp <= subject.cutoff_temp && event.phase_current <= subject.overcurrent_limit,
            to = Idle
        )]
        Running,
        #[sysml::transition(
            on = ToolSafetyTelemetry,
            guard = event.trigger_demand <= 0.05 && event.motor_temp <= subject.cutoff_temp - 10.0,
            to = Idle
        )]
        ThermalCutoff,
        #[sysml::transition(
            on = ToolSafetyTelemetry,
            guard = event.trigger_demand <= 0.05 && event.phase_current <= 1.0,
            to = Idle
        )]
        OvercurrentFault,
    }

    // =========================================================================
    // 5. Requirements with Compiled Constraints & Quantitative Verification
    // =========================================================================

    /// Shared 18V tool platform must sustain at least 300 s of continuous runtime under nominal load.
    #[sysml::requirement(id = "TOOL-001")]
    #[sysml::require_constraint(actual_runtime_s >= min_runtime_s)]
    #[derive(Debug, Clone, Copy)]
    pub struct BatteryRuntimeRequirement {
        #[sysml::attr(default = 300.0, unit = "s")]
        pub min_runtime_s: Quantity<f64>,
        #[sysml::attr(default = 589.09, unit = "s")]
        pub actual_runtime_s: Quantity<f64>,
    }

    /// Motor temperature rise under sustained rated load must stay within thermal safety limit (<= 45 K rise).
    #[sysml::requirement(id = "TOOL-002")]
    #[sysml::require_constraint(motor_temp_rise_k <= max_temp_rise_k)]
    #[derive(Debug, Clone, Copy)]
    pub struct MotorThermalSafetyRequirement {
        #[sysml::attr(default = 45.0, unit = "K")]
        pub max_temp_rise_k: Quantity<f64>,
        #[sysml::attr(default = 34.8, unit = "K")]
        pub motor_temp_rise_k: Quantity<f64>,
    }

    /// Tool acoustic emission at the operator ear must remain within occupational limit (<= 95 dBA).
    #[sysml::requirement(id = "TOOL-003")]
    #[sysml::require_constraint(measured_noise_db <= max_noise_db)]
    #[derive(Debug, Clone, Copy)]
    pub struct AcousticEmissionLimitRequirement {
        #[sysml::attr(dimensionless, default = 95.0)]
        pub max_noise_db: f64,
        #[sysml::attr(dimensionless, default = 92.0)]
        pub measured_noise_db: f64,
    }

    // =========================================================================
    // 6. Abstract Parent Tool & 3 Product-Line Tool Variants
    // =========================================================================

    /// Abstract cordless 18V power tool platform composing shared battery, inverter, motor, MCU, trigger, and thermal sensor.
    #[sysml::part(abstract)]
    #[sysml::perform(pwm: RegulateMotorPwm)]
    #[sysml::perform(monitor: MonitorThermalAndCurrent)]
    #[sysml::perform(torque: DeliverSpindleTorque)]
    #[sysml::satisfy(req = BatteryRuntimeRequirement, id = "TOOL-001")]
    #[sysml::satisfy(req = MotorThermalSafetyRequirement, id = "TOOL-002")]
    #[sysml::satisfy(req = AcousticEmissionLimitRequirement, id = "TOOL-003")]
    #[sysml::connect(interface = ElectricalLink, name = "batteryToInverter", from = battery.power_out, to = inverter.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "batteryToMcu", from = battery.power_out, to = mcu.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "batteryToTrigger", from = battery.power_out, to = trigger.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "batteryToTempSensor", from = battery.power_out, to = temp_sensor.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "inverterToMotor", from = inverter.power_out, to = motor.power_in)]
    #[sysml::connect(interface = CommandChannel, name = "triggerToMcuCmd", from = trigger.cmd_to_logic, to = mcu.trigger_cmd_in)]
    #[sysml::connect(interface = TelemetryChannel, name = "mcuToTriggerStatus", from = mcu.hmi_status_out, to = trigger.status_from_logic)]
    #[sysml::connect(interface = CommandChannel, name = "mcuToInverterCmd", from = mcu.cmd_out, to = inverter.cmd_in)]
    #[sysml::connect(interface = CommandChannel, name = "mcuToMotorCmd", from = mcu.cmd_out, to = motor.cmd_in)]
    #[sysml::connect(interface = TelemetryChannel, name = "bmsToMcuTelemetry", from = battery.bms_telemetry, to = mcu.bms_status_in)]
    #[sysml::connect(interface = TelemetryChannel, name = "tempSensorToMcu", from = temp_sensor.reading_out, to = mcu.sensor_in)]
    #[sysml::connect(interface = I2cChannel, name = "batteryToMcuBmsBus", from = battery.bms_bus, to = mcu.bus)]
    #[sysml::connect(interface = ThermalLink, name = "motorToTempSensor", from = motor.heat_out, to = temp_sensor.thermal_sense)]
    #[derive(Debug, Clone, Copy)]
    pub struct CordlessToolBase {
        /// Tool rated power: the power budget role, inherited by every `:>> ratedPower`.
        #[sysml::attr(budget = "power", default = 500.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(dimensionless, default = 85.0)]
        pub max_noise_db: f64,
        /// Complete tool mass: the mass budget role, inherited by every `:>> mass`.
        #[sysml::attr(budget = "mass", default = 1.80, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::port]
        pub user_in: Conjugated<HmiInputPort>,
        #[sysml::port]
        pub user_out: HmiFeedbackPort,
        #[sysml::port]
        pub heat_out: ThermalPort,
        #[sysml::port]
        pub noise_out: AcousticPort,
        #[sysml::part]
        pub battery: Battery18V5Ah,
        #[sysml::part]
        pub inverter: MotorInverterStage,
        #[sysml::part]
        pub motor: SpindleMotor,
        #[sysml::part]
        pub mcu: ToolControllerMcu,
        #[sysml::part]
        pub trigger: TriggerHmi,
        #[sysml::part]
        pub temp_sensor: MotorTempSensor,
    }

    /// 18V brushless cordless drill-driver specializing `CordlessToolBase` with a 2-speed planetary chuck gearbox.
    #[sysml::part]
    #[sysml::specializes(CordlessToolBase)]
    #[sysml::connect(interface = MechanicalLink, name = "motorToChuck", from = base.motor.mech_out, to = chuck_gearbox.mech_in, from_sysml = "motor.mechOut")]
    #[derive(Debug, Clone, Copy)]
    pub struct CordlessDrill {
        #[sysml::attr(redefines = "ratedPower", default = 550.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "maxNoiseDb", default = 79.0)]
        pub max_noise_db: f64,
        #[sysml::attr(redefines = "mass", default = 1.95, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(default = 65.0, unit = "N*m")]
        pub max_torque_nm: Quantity<f64>,
        #[sysml::port(redefines)]
        pub user_in: Conjugated<HmiInputPort>,
        #[sysml::port(redefines)]
        pub user_out: HmiFeedbackPort,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
        #[sysml::part]
        pub chuck_gearbox: DrillChuckGearbox,
        /// The inherited platform, reached as `base.motor` by `connect`: a reference, not a
        /// second owned platform (the tool *is* a `CordlessToolBase`, so budgets count it once).
        #[sysml::part(ref)]
        pub base: CordlessToolBase,
    }

    /// 18V cordless impact driver specializing `CordlessToolBase` with a tangential anvil hammer mechanism.
    #[sysml::part]
    #[sysml::specializes(CordlessToolBase)]
    #[sysml::connect(interface = MechanicalLink, name = "motorToAnvil", from = base.motor.mech_out, to = anvil_mechanism.mech_in, from_sysml = "motor.mechOut")]
    #[derive(Debug, Clone, Copy)]
    pub struct ImpactDriver {
        #[sysml::attr(redefines = "ratedPower", default = 650.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "maxNoiseDb", default = 92.0)]
        pub max_noise_db: f64,
        #[sysml::attr(redefines = "mass", default = 1.75, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(default = 180.0, unit = "N*m")]
        pub max_impact_torque_nm: Quantity<f64>,
        #[sysml::port(redefines)]
        pub user_in: Conjugated<HmiInputPort>,
        #[sysml::port(redefines)]
        pub user_out: HmiFeedbackPort,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
        #[sysml::part]
        pub anvil_mechanism: ImpactAnvilMechanism,
        /// The inherited platform, reached as `base.motor` by `connect`: a reference, not a
        /// second owned platform (the tool *is* a `CordlessToolBase`, so budgets count it once).
        #[sysml::part(ref)]
        pub base: CordlessToolBase,
    }

    /// 18V cordless circular saw specializing `CordlessToolBase` with a 165 mm blade arbor and mechanical guard interlock.
    #[sysml::part]
    #[sysml::specializes(CordlessToolBase)]
    #[sysml::connect(interface = MechanicalLink, name = "motorToArbor", from = base.motor.mech_out, to = blade_arbor.mech_in, from_sysml = "motor.mechOut")]
    #[sysml::connect(interface = TelemetryChannel, name = "guardInterlockToMcu", from = guard_interlock.interlock_status, to = base.mcu.guard_sensor_in, to_sysml = "mcu.guardSensorIn")]
    #[derive(Debug, Clone, Copy)]
    pub struct CircularSaw {
        #[sysml::attr(redefines = "ratedPower", default = 1100.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(dimensionless, redefines = "maxNoiseDb", default = 90.0)]
        pub max_noise_db: f64,
        #[sysml::attr(redefines = "mass", default = 3.10, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(default = 0.057, unit = "m")]
        pub max_cut_depth: Quantity<f64>,
        #[sysml::port(redefines)]
        pub user_in: Conjugated<HmiInputPort>,
        #[sysml::port(redefines)]
        pub user_out: HmiFeedbackPort,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
        #[sysml::part]
        pub blade_arbor: CircularBladeArbor,
        #[sysml::part]
        pub guard_interlock: BladeGuardInterlock,
        /// The inherited platform, reached as `base.motor` by `connect`: a reference, not a
        /// second owned platform (the tool *is* a `CordlessToolBase`, so budgets count it once).
        #[sysml::part(ref)]
        pub base: CordlessToolBase,
    }

    /// Product-line variation over the shared-platform 18V cordless tool family.
    #[sysml::variation(of = CordlessToolBase)]
    #[derive(Debug, Clone, Copy)]
    pub enum PowerToolVariant {
        Drill(CordlessDrill),
        Impact(ImpactDriver),
        Saw(CircularSaw),
    }

    // =========================================================================
    // 7. Top-Level Workshop Context
    // =========================================================================

    /// Top-level workshop context wiring the operator, all three 18V tools, rapid charger, AC mains, and ambient sink.
    #[sysml::part]
    // Operator <-> Tools HMI links
    #[sysml::connect(interface = HmiInputChannel, name = "operatorToDrill", from = operator.user_input, to = drill.user_in)]
    #[sysml::connect(interface = HmiFeedbackChannel, name = "drillToOperator", from = drill.user_out, to = operator.user_feedback)]
    #[sysml::connect(interface = HmiInputChannel, name = "operatorToImpact", from = operator.user_input, to = impact_driver.user_in)]
    #[sysml::connect(interface = HmiFeedbackChannel, name = "impactToOperator", from = impact_driver.user_out, to = operator.user_feedback)]
    #[sysml::connect(interface = HmiInputChannel, name = "operatorToSaw", from = operator.user_input, to = circular_saw.user_in)]
    #[sysml::connect(interface = HmiFeedbackChannel, name = "sawToOperator", from = circular_saw.user_out, to = operator.user_feedback)]
    // AC Mains -> Battery Charger -> Spare 18V Pack
    #[sysml::connect(interface = ElectricalLink, name = "mainsToCharger", from = ac_mains.power_out, to = charger.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "chargerToSpareBattery", from = charger.power_out, to = spare_battery.charge_in)]
    #[sysml::connect(interface = I2cChannel, name = "chargerToSpareBatteryBms", from = charger.bms_bus, to = spare_battery.bms_bus)]
    // Thermal rejection to WorkshopAmbient
    #[sysml::connect(interface = ThermalLink, name = "drillHeatToAmbient", from = drill.heat_out, to = ambient.heat_sink)]
    #[sysml::connect(interface = ThermalLink, name = "impactHeatToAmbient", from = impact_driver.heat_out, to = ambient.heat_sink)]
    #[sysml::connect(interface = ThermalLink, name = "sawHeatToAmbient", from = circular_saw.heat_out, to = ambient.heat_sink)]
    #[sysml::connect(interface = ThermalLink, name = "chargerHeatToAmbient", from = charger.heat_out, to = ambient.heat_sink)]
    #[sysml::connect(interface = ThermalLink, name = "spareBatteryHeatToAmbient", from = spare_battery.heat_out, to = ambient.heat_sink)]
    // Acoustic emission to WorkshopAmbient
    #[sysml::connect(interface = AcousticLink, name = "drillNoiseToAmbient", from = drill.noise_out, to = ambient.noise_sink)]
    #[sysml::connect(interface = AcousticLink, name = "impactNoiseToAmbient", from = impact_driver.noise_out, to = ambient.noise_sink)]
    #[sysml::connect(interface = AcousticLink, name = "sawNoiseToAmbient", from = circular_saw.noise_out, to = ambient.noise_sink)]
    #[derive(Debug, Clone, Copy)]
    pub struct PowerToolWorkshop {
        #[sysml::part]
        pub operator: WorkshopOperator,
        #[sysml::part]
        pub drill: CordlessDrill,
        #[sysml::part]
        pub impact_driver: ImpactDriver,
        #[sysml::part]
        pub circular_saw: CircularSaw,
        #[sysml::part]
        pub charger: RapidBatteryCharger18V,
        #[sysml::part]
        pub spare_battery: Battery18V2Ah,
        #[sysml::part]
        pub ac_mains: WorkshopAcMains,
        #[sysml::part]
        pub ambient: WorkshopAmbient,
    }
}

/// Build the complete SysML v2 model with `sysml-std` packages + `PowerToolsFamilyModel`.
pub fn build_model() -> Model {
    let mut pkgs = sysml_std::std_packages();
    pkgs.push(power_tools_family_model::sysml_package());
    Model::new(pkgs)
}

#[cfg(test)]
mod tests {
    use super::power_tools_family_model::*;
    use super::*;
    use sysml_emit::{audit_model_with_stdlib, emit_model, lint_model};

    #[test]
    fn model_audits_zero_errors_zero_warnings_zero_orphans() {
        let model = build_model();
        let sysml = emit_model(&model);
        let audit = audit_model_with_stdlib("power_tools_family", &model, None, None, true);
        assert!(audit.stdlib_loaded, "OMG sysml.library must be loaded");
        assert!(
            audit.ok
                && audit.validation_errors == 0
                && audit.validation_warnings == 0
                && audit.orphan_requirements.is_empty(),
            "audit failed: {audit:#?}\n--- SYSML ---\n{sysml}"
        );
        assert_eq!(audit.total_requirements, 3);
        assert_eq!(audit.satisfied_requirements, 3);
    }

    #[test]
    fn model_lints_clean_and_shows_platform_reuse() {
        let model = build_model();
        let report = lint_model("power_tools_family", &model);
        assert!(
            report.clean && report.summary.warning_count == 0 && report.summary.info_count == 0,
            "lint report not clean: {:#?}",
            report.findings
        );
        assert_eq!(report.summary.variations, 2);
        assert!(report.summary.redefinitions > 0);
        assert!(report.summary.specializations >= 12);
    }

    #[test]
    fn state_machine_guards_and_fault_transitions_execute() {
        let mcu = ToolControllerMcu::default();
        let mut state = ToolOperatingState::initial_state();
        assert_eq!(state, ToolOperatingState::Idle);

        // Normal trigger pull -> Running
        let nominal_pull = ToolSafetyTelemetry {
            trigger_demand: 0.75,
            motor_temp: 305.0,
            phase_current: 28.0,
        };
        assert_eq!(
            state.step_with(&nominal_pull, &mcu).unwrap(),
            ToolOperatingState::Running
        );

        // Overtemperature event -> ThermalCutoff
        let overheat = ToolSafetyTelemetry {
            trigger_demand: 0.80,
            motor_temp: 350.0,
            phase_current: 30.0,
        };
        assert_eq!(
            state.step_with(&overheat, &mcu).unwrap(),
            ToolOperatingState::ThermalCutoff
        );

        // Cooldown with trigger released -> Idle
        let cooled = ToolSafetyTelemetry {
            trigger_demand: 0.0,
            motor_temp: 325.0,
            phase_current: 0.0,
        };
        assert_eq!(
            state.step_with(&cooled, &mcu).unwrap(),
            ToolOperatingState::Idle
        );

        // Start running again, then trigger OvercurrentFault
        assert_eq!(
            state.step_with(&nominal_pull, &mcu).unwrap(),
            ToolOperatingState::Running
        );
        let stall = ToolSafetyTelemetry {
            trigger_demand: 1.0,
            motor_temp: 315.0,
            phase_current: 78.0,
        };
        assert_eq!(
            state.step_with(&stall, &mcu).unwrap(),
            ToolOperatingState::OvercurrentFault
        );

        // Release trigger with current cleared -> Idle
        let cleared = ToolSafetyTelemetry {
            trigger_demand: 0.0,
            motor_temp: 315.0,
            phase_current: 0.0,
        };
        assert_eq!(
            state.step_with(&cleared, &mcu).unwrap(),
            ToolOperatingState::Idle
        );
    }

    #[test]
    fn physics_calcs_and_requirements_evaluate_accurately() {
        let bat5 = Battery18V5Ah::default();
        let drill = CordlessDrill::default();
        let saw = CircularSaw::default();

        // Reuse `battery_runtime_s` & `cascaded_waste_heat_w` from `sysml_std::prelude::*`
        // 324,000 J / 550 W = 589.0909... s (>= 300 s)
        let drill_runtime = battery_runtime_s(bat5.energy_capacity.value, drill.rated_power.value);
        assert!((drill_runtime - 589.090909).abs() < 1e-4);

        // 324,000 J / 1100 W = 294.5454... s (< 300 s continuous full load)
        let saw_runtime = battery_runtime_s(bat5.energy_capacity.value, saw.rated_power.value);
        assert!((saw_runtime - 294.545454).abs() < 1e-4);

        // Waste heat across inverter (0.96) * motor (0.89) * drill gearbox (0.92)
        let inv = MotorInverterStage::default();
        let mot = SpindleMotor::default();
        let gbx = DrillChuckGearbox::default();
        let waste_w = cascaded_waste_heat_w(
            drill.rated_power.value,
            inv.efficiency,
            mot.efficiency,
            gbx.efficiency,
        );
        let expected_eff = 0.96 * 0.89 * 0.92;
        assert!((waste_w - 550.0 * (1.0 - expected_eff)).abs() < 1e-9);

        // Requirement evaluation checks (TOOL-001, TOOL-002, TOOL-003)
        let req_rt_pass = BatteryRuntimeRequirement {
            actual_runtime_s: Quantity::new(drill_runtime, "s"),
            ..BatteryRuntimeRequirement::default()
        };
        assert!(req_rt_pass.evaluate());

        let req_rt_fail = BatteryRuntimeRequirement {
            actual_runtime_s: Quantity::new(saw_runtime, "s"),
            ..BatteryRuntimeRequirement::default()
        };
        assert!(!req_rt_fail.evaluate());

        let req_thermal_pass = MotorThermalSafetyRequirement::default();
        assert!(req_thermal_pass.evaluate());
        let req_thermal_fail = MotorThermalSafetyRequirement {
            motor_temp_rise_k: Quantity::new(52.0, "K"),
            ..MotorThermalSafetyRequirement::default()
        };
        assert!(!req_thermal_fail.evaluate());

        let req_acoustic_pass = AcousticEmissionLimitRequirement {
            measured_noise_db: ImpactDriver::default().max_noise_db,
            ..AcousticEmissionLimitRequirement::default()
        };
        assert!(req_acoustic_pass.evaluate());
        let req_acoustic_fail = AcousticEmissionLimitRequirement {
            measured_noise_db: 98.5,
            ..AcousticEmissionLimitRequirement::default()
        };
        assert!(!req_acoustic_fail.evaluate());
    }
}
