//! Multi-domain Microwave Oven model (AC/HV power, magnetron RF, safety interlocks,
//! electronics, mechanics/cooling, HMI, heat, and noise).
//!
//! Specializes `sysml-std` v1.0 abstractions (`RadiativeEmitterLoad`, `RadiativeConduit`,
//! `ThermalEnclosure`, `SafetyInterlockSwitch`, `HighVoltageTransformer`, `CoolingFan`,
//! `Microcontroller`, `TemperatureSensor`, `HmiPanel`) using `#[sysml::port(redefines)]`
//! and reuses `sysml-std` physics calculations (`useful_power_watts`, `waste_heat_watts`,
//! `sensible_heat_energy_j`, `heating_time_s`, `forced_convection_temp_rise_k`,
//! `attenuated_noise_db`).

use sysml_dsl::ir::Model;
use sysml_dsl::prelude::*;

#[sysml::package(name = "MicrowaveModel")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
#[sysml::import("SysmlStdComms::*")]
#[sysml::import("SysmlStdPower::*")]
#[sysml::import("SysmlStdMechanics::*")]
#[sysml::import("SysmlStdElectronics::*")]
#[sysml::import("SysmlStdHmi::*")]
#[sysml::import("ModelingMetadata::*")]
#[sysml::import("ISO26262Metadata::*")]
pub mod microwave_model {
    use super::*;
    use sysml_std::prelude::*;

    // =========================================================================
    // 1. Domain Events & State Machine Triggers
    // =========================================================================

    /// Operator cooking start request carrying door safety state and target RF power.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct CookStartCommand {
        #[sysml::attr(default = 0.0)]
        pub door_closed: f64,
        #[sysml::attr(default = 900.0, unit = "W")]
        pub requested_rf_power_w: f64,
    }

    /// Door interlock switch transition telemetry (`door_open > 0.5` forces immediate HV cutoff).
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct DoorInterlockEvent {
        #[sysml::attr(default = 0.0)]
        pub door_open: f64,
        #[sysml::attr(default = 1.0)]
        pub door_closed: f64,
    }

    /// Cavity/magnetron thermal monitor reading (`measured_temp_k > max_safe_cavity_temp_k` triggers fault cutoff).
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct ThermalMonitorEvent {
        #[sysml::attr(default = 315.0, unit = "K")]
        pub measured_temp_k: f64,
    }

    /// Cooking timer completion or operator cancel event returning oven to standby.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct CookStopEvent {
        #[sysml::attr(default = 1.0)]
        pub cycle_done: f64,
    }

    // =========================================================================
    // 2. Functional Actions (Behavior Allocation)
    // =========================================================================

    /// Generate 2.45 GHz microwave RF energy from high-voltage excitation.
    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct GenerateRfEnergy;

    /// Guide 2.45 GHz RF energy into the shielded cooking cavity and rotate load for uniform heating.
    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct RadiateAndHeatLoad;

    /// Force ambient cooling air across magnetron fins and HV transformer windings.
    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct CoolMagnetronAndHv;

    /// Supervise dual door interlocks, cavity temperature, HV relay command, and user interface.
    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct SuperviseSafetyAndCooking;

    /// Render countdown timer, accept keypad entry, and sound completion buzzer.
    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ManageUserInterface;

    // =========================================================================
    // 3. Requirements & Quantitative Constraints (0 Orphans)
    // =========================================================================

    /// MW-001: The magnetron shall deliver at least 800 W of 2.45 GHz RF output power.
    #[sysml::requirement(id = "MW-001")]
    #[sysml::require_constraint(rf_output_power_w >= min_rf_power_w)]
    #[derive(Debug, Clone, Copy)]
    pub struct MagnetronRfOutputRequirement {
        #[sysml::attr(default = 800.0, unit = "W")]
        pub min_rf_power_w: Quantity<f64>,
        /// Evidence: supplied by `microwave_sim` (MW-001 monitor); 900 W is a model estimate.
        #[sysml::attr(default = 900.0, unit = "W", evidence)]
        pub rf_output_power_w: Quantity<f64>,
    }

    /// MW-002: Forced-air cooling shall limit magnetron temperature rise to <= 45 K under full load.
    #[sysml::requirement(id = "MW-002")]
    #[sysml::require_constraint(actual_temp_rise_k <= max_temp_rise_k)]
    #[derive(Debug, Clone, Copy)]
    pub struct MagnetronCoolingTempRequirement {
        #[sysml::attr(default = 45.0, unit = "K")]
        pub max_temp_rise_k: Quantity<f64>,
        #[sysml::attr(default = 33.17, unit = "K", evidence)]
        pub actual_temp_rise_k: Quantity<f64>,
    }

    /// MW-003: Steady-state operating acoustic noise (cooling fan + HV transformer hum) shall not exceed 58 dB.
    #[sysml::requirement(id = "MW-003")]
    #[sysml::require_constraint(operating_noise_db <= max_noise_db)]
    #[derive(Debug, Clone, Copy)]
    pub struct AcousticNoiseLimitRequirement {
        #[sysml::attr(default = 58.0)]
        pub max_noise_db: f64,
        #[sysml::attr(default = 54.0, evidence)]
        pub operating_noise_db: f64,
    }

    /// MW-004: Dual redundant door interlocks shall cut off HV magnetron excitation within 20 ms of door opening.
    ///
    /// Also carries the door-interlock safety goal (hazard: RF exposure with the door open;
    /// safe state: `DoorOpenCutoff`, i.e. HV/RF off).
    /// - `ftti_ms = 20`: taken from this requirement's own `max_cutoff_latency_s = 0.020 s`
    ///   (same quantity: door opening -> HV excitation cut off). Not derived from a separate
    ///   hazard analysis.
    /// - `asil = AsilD`: PLACEHOLDER — no source. The model has no hazard analysis / risk
    ///   assessment; the highest integrity level is used as a conservative stand-in and must be
    ///   replaced by a real classification.
    #[sysml::requirement(id = "MW-004")]
    #[sysml::rationale("Safety goal metadata: FTTI 20 ms from max_cutoff_latency_s (MW-004); ASIL D is a PLACEHOLDER with no source.")]
    #[sysml::safety_goal(asil = AsilD, safe_state = "doorOpenCutoff", ftti_ms = 20)]
    #[sysml::require_constraint(actual_cutoff_latency_s <= max_cutoff_latency_s)]
    #[derive(Debug, Clone, Copy)]
    pub struct DoorInterlockCutoffRequirement {
        #[sysml::attr(default = 0.020, unit = "s")]
        pub max_cutoff_latency_s: Quantity<f64>,
        /// Evidence: supplied by `microwave_sim` (MW-004 monitor); 8 ms is a model estimate.
        #[sysml::attr(default = 0.008, unit = "s", evidence)]
        pub actual_cutoff_latency_s: Quantity<f64>,
    }

    // =========================================================================
    // 4. Shared Abstract Domain Classification
    // =========================================================================

    /// Shared abstract trait for electrically driven thermal-and-acoustic power equipment
    /// (HV magnetron transformer and forced-air cooling fan).
    #[sysml::part(abstract)]
    #[derive(Debug, Clone, Copy)]
    pub struct ElectroThermalAcousticUnit {}

    // =========================================================================
    // 5. Subsystem 1: PowerHvSubsystem (AC Mains, Logic PSU, HV Relay, HV Transformer)
    // =========================================================================

    /// AC mains wall supply connection (`230 V`, `50 Hz`) feeding the microwave oven.
    #[sysml::part]
    #[sysml::specializes(AcMainsSupply)]
    #[derive(Debug, Clone, Copy)]
    pub struct MicrowaveAcMainsInput {
        #[sysml::attr(redefines = "nominalVoltage", default = 230.0, unit = "V")]
        pub nominal_voltage: Quantity<f64>,
        #[sysml::attr(redefines = "maxCurrent", default = 16.0, unit = "A")]
        pub max_current: Quantity<f64>,
        #[sysml::attr(redefines = "mainsFrequency", default = 50.0, unit = "Hz")]
        pub mains_frequency: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
    }

    /// Low-voltage switched-mode AC-DC PSU supplying 5 V logic rail to control electronics and HMI.
    #[sysml::part]
    #[sysml::specializes(AcDcPowerSupply)]
    #[derive(Debug, Clone, Copy)]
    pub struct ControlPsu {
        #[sysml::attr(redefines = "ratedPower", default = 25.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(redefines = "outputVoltage", default = 5.0, unit = "V")]
        pub output_voltage: Quantity<f64>,
        #[sysml::attr(redefines = "efficiency", default = 0.85)]
        pub efficiency: f64,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
    }

    /// High-power relay / triac stage switching AC mains to the magnetron HV transformer and AC auxiliaries.
    #[sysml::part]
    #[sysml::specializes(PowerSwitchStage)]
    #[derive(Debug, Clone, Copy)]
    pub struct HvPowerRelay {
        #[sysml::attr(redefines = "ratedPower", default = 1600.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(redefines = "outputVoltage", default = 230.0, unit = "V")]
        pub output_voltage: Quantity<f64>,
        #[sysml::attr(default = 0.008, unit = "s")]
        pub switching_cutoff_latency: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
        #[sysml::port(redefines)]
        pub cmd_in: Conjugated<CommandPort>,
    }

    /// Step-up high-voltage magnetron transformer with 50/60 Hz lamination hum and core heat loss.
    #[sysml::part]
    #[sysml::specializes(HighVoltageTransformer, ElectroThermalAcousticUnit)]
    #[derive(Debug, Clone, Copy)]
    pub struct MagnetronHvTransformer {
        #[sysml::attr(redefines = "mass", default = 3.8, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "ratedPower", default = 1550.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(redefines = "efficiency", default = 0.90)]
        pub efficiency: f64,
        #[sysml::attr(redefines = "wasteHeat", default = 140.0, unit = "W")]
        pub waste_heat: Quantity<f64>,
        #[sysml::attr(redefines = "outputVoltage", default = 2100.0, unit = "V")]
        pub output_voltage: Quantity<f64>,
        #[sysml::attr(redefines = "humNoiseDb", default = 47.0)]
        pub hum_noise_db: f64,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub power_out: ElectricalPowerPort,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
    }

    /// AC mains input, low-voltage control PSU, HV relay switch stage, and step-up HV transformer.
    #[sysml::part]
    #[sysml::connect(interface = ElectricalLink, name = "mainsToControlPsu", from = mains_supply.power_out, to = control_psu.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "mainsToHvRelay", from = mains_supply.power_out, to = hv_relay.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "hvRelayToTransformer", from = hv_relay.power_out, to = hv_transformer.power_in)]
    #[derive(Debug, Clone, Copy)]
    pub struct PowerHvSubsystem {
        #[sysml::part]
        pub mains_supply: MicrowaveAcMainsInput,
        #[sysml::part]
        pub control_psu: ControlPsu,
        #[sysml::part]
        pub hv_relay: HvPowerRelay,
        #[sysml::part]
        pub hv_transformer: MagnetronHvTransformer,
    }

    // =========================================================================
    // 6. Subsystem 2: RfCookingSubsystem (Magnetron, Waveguide, Cavity, Turntable, Fan)
    // =========================================================================

    /// 2.45 GHz vacuum magnetron converting 1400 W HV electrical power into 900 W RF and 500 W waste heat.
    #[sysml::part]
    #[sysml::specializes(RadiativeEmitterLoad)]
    #[sysml::satisfy(req = MagnetronRfOutputRequirement, id = "MW-001")]
    #[sysml::perform(generateRf: GenerateRfEnergy)]
    #[derive(Debug, Clone, Copy)]
    pub struct MagnetronTube {
        #[sysml::attr(redefines = "mass", default = 0.95, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "ratedPower", default = 1400.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(redefines = "efficiency", default = 0.642857)]
        pub efficiency: f64,
        #[sysml::attr(redefines = "wasteHeat", default = 500.0, unit = "W")]
        pub waste_heat: Quantity<f64>,
        #[sysml::attr(redefines = "emissionFrequency", default = 2450000000.0, unit = "Hz")]
        pub emission_frequency: Quantity<f64>,
        #[sysml::attr(redefines = "radiativeOutputPower", default = 900.0, unit = "W")]
        pub radiative_output_power: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub radiation_out: RadiativePort,
    }

    /// Rectangular metallic waveguide launching 2.45 GHz RF waves from the magnetron into the cavity.
    #[sysml::part]
    #[sysml::specializes(RadiativeConduit)]
    #[derive(Debug, Clone, Copy)]
    pub struct Waveguide {
        #[sysml::attr(redefines = "mass", default = 0.25, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "transmissionEfficiency", default = 0.98)]
        pub transmission_efficiency: f64,
        #[sysml::port(redefines)]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub radiation_out: RadiativePort,
    }

    /// Shielded stainless-steel cooking cavity receiving RF energy, heating food load, and rejecting cabinet heat.
    #[sysml::part]
    #[sysml::specializes(ThermalEnclosure)]
    #[sysml::perform(radiateAndHeat: RadiateAndHeatLoad)]
    #[derive(Debug, Clone, Copy)]
    pub struct CookingCavity {
        #[sysml::attr(redefines = "mass", default = 5.2, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "heatDissipation", default = 45.0, unit = "W")]
        pub heat_dissipation: Quantity<f64>,
        #[sysml::attr(redefines = "thermalConductanceWPerK", default = 1.5)]
        pub thermal_conductance_w_per_k: f64,
        #[sysml::attr(redefines = "acousticAttenuationDb", default = 14.0)]
        pub acoustic_attenuation_db: f64,
        #[sysml::attr(default = 0.88)]
        pub rf_coupling_efficiency: f64,
        #[sysml::attr(default = 0.50, unit = "kg")]
        pub nominal_food_mass: Quantity<f64>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port]
        pub rf_in: Conjugated<RadiativePort>,
        #[sysml::port]
        pub turntable_drive: Conjugated<MechanicalPowerPort>,
    }

    /// Synchronous low-speed turntable motor rotating food load for uniform microwave field exposure.
    #[sysml::part]
    #[sysml::specializes(ElectricMotor)]
    #[derive(Debug, Clone, Copy)]
    pub struct TurntableMotor {
        #[sysml::attr(redefines = "mass", default = 0.18, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "ratedPower", default = 4.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(redefines = "efficiency", default = 0.55)]
        pub efficiency: f64,
        #[sysml::attr(redefines = "ratedRotationFreq", default = 0.10, unit = "Hz")]
        pub rated_rotation_freq: Quantity<f64>,
        #[sysml::attr(redefines = "noiseLevelDb", default = 34.0)]
        pub noise_level_db: f64,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub cmd_in: Conjugated<CommandPort>,
        #[sysml::port(redefines)]
        pub mech_out: MechanicalPowerPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
    }

    /// Forced-air axial cooling fan removing 500 W magnetron waste heat with 0.015 kg/s airflow.
    #[sysml::part]
    #[sysml::specializes(CoolingFan, ElectroThermalAcousticUnit)]
    #[sysml::satisfy(req = MagnetronCoolingTempRequirement, id = "MW-002")]
    #[sysml::satisfy(req = AcousticNoiseLimitRequirement, id = "MW-003")]
    #[sysml::perform(coolMagnetron: CoolMagnetronAndHv)]
    #[derive(Debug, Clone, Copy)]
    pub struct MagnetronCoolingFan {
        #[sysml::attr(redefines = "mass", default = 0.35, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "airflowRate", default = 0.015, unit = "kg/s")]
        pub airflow_rate: Quantity<f64>,
        #[sysml::attr(redefines = "ratedPower", default = 18.0, unit = "W")]
        pub rated_power: Quantity<f64>,
        #[sysml::attr(redefines = "noiseLevelDb", default = 52.0)]
        pub noise_level_db: f64,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub heat_in: Conjugated<ThermalPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub noise_out: AcousticPort,
    }

    /// RF generation, waveguide propagation, cavity heating, turntable drive, and forced-air cooling.
    #[sysml::part]
    #[sysml::connect(interface = RadiativeLink, name = "magnetronToWaveguideRf", from = magnetron.radiation_out, to = waveguide.radiation_in)]
    #[sysml::connect(interface = RadiativeLink, name = "waveguideToCavityRf", from = waveguide.radiation_out, to = cavity.rf_in)]
    #[sysml::connect(interface = ThermalLink, name = "magnetronToCoolingFan", from = magnetron.heat_out, to = cooling_fan.heat_in)]
    #[sysml::connect(interface = MechanicalLink, name = "turntableDriveLink", from = turntable.mech_out, to = cavity.turntable_drive)]
    #[derive(Debug, Clone, Copy)]
    pub struct RfCookingSubsystem {
        #[sysml::part]
        pub magnetron: MagnetronTube,
        #[sysml::part]
        pub waveguide: Waveguide,
        #[sysml::part]
        pub cavity: CookingCavity,
        #[sysml::part]
        pub turntable: TurntableMotor,
        #[sysml::part]
        pub cooling_fan: MagnetronCoolingFan,
    }

    // =========================================================================
    // 7. Subsystem 3: ControlSafetySubsystem (MCU, Temp Sensor, Dual Interlocks)
    // =========================================================================

    /// Primary mechanical door-latch interlock switch directly sensing door latch state.
    ///
    /// FMEA: contacts welded closed -> the switch keeps reporting "door closed" when the door
    /// opens, so HV cutoff relies on `MonitorDoorInterlock` alone (failure mode named in the
    /// `MonitorDoorInterlock` description). `severity`/`occurrence`/`detection` = 10/10/10 are
    /// PLACEHOLDER — no source (worst-case stand-ins; the model has no FMEA ratings).
    #[sysml::part]
    #[sysml::specializes(SafetyInterlockSwitch)]
    #[sysml::satisfy(req = DoorInterlockCutoffRequirement, id = "MW-004")]
    #[sysml::fmea(
        failure_mode = "ContactsWeldedClosed",
        effect = "DoorOpenNotReportedByPrimaryInterlock",
        severity = 10,
        occurrence = 10,
        detection = 10
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct PrimaryDoorInterlock {
        #[sysml::attr(redefines = "mass", default = 0.03, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "cutoffLatency", default = 0.005, unit = "s")]
        pub cutoff_latency: Quantity<f64>,
        #[sysml::port(redefines)]
        pub mechanical_actuation: Conjugated<HmiInputPort>,
        #[sysml::port(redefines)]
        pub interlock_status: TelemetryPort,
    }

    /// Secondary redundant monitor interlock switch detecting welded contacts or unlatched door.
    ///
    /// FMEA: contacts welded closed -> loss of the redundant door-open detection; combined
    /// with the same fault on `PrimaryDoorInterlock` the HV is never cut off (MW-004 violated).
    /// `severity`/`occurrence`/`detection` = 10/10/10 are PLACEHOLDER — no source (worst-case
    /// stand-ins; the model has no FMEA ratings).
    #[sysml::part]
    #[sysml::specializes(SafetyInterlockSwitch)]
    #[sysml::satisfy(req = DoorInterlockCutoffRequirement, id = "MW-004")]
    #[sysml::fmea(
        failure_mode = "ContactsWeldedClosed",
        effect = "RedundantDoorOpenDetectionLost",
        severity = 10,
        occurrence = 10,
        detection = 10
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct MonitorDoorInterlock {
        #[sysml::attr(redefines = "mass", default = 0.03, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "cutoffLatency", default = 0.008, unit = "s")]
        pub cutoff_latency: Quantity<f64>,
        #[sysml::port(redefines)]
        pub mechanical_actuation: Conjugated<HmiInputPort>,
        #[sysml::port(redefines)]
        pub interlock_status: TelemetryPort,
    }

    /// Cavity/magnetron thermal cutout sensor measuring exhaust/cavity temperature.
    #[sysml::part]
    #[sysml::specializes(TemperatureSensor)]
    #[derive(Debug, Clone, Copy)]
    pub struct CavityTempSensor {
        #[sysml::attr(redefines = "maxMeasurableTemp", default = 423.15, unit = "K")]
        pub max_measurable_temp: Quantity<f64>,
        #[sysml::attr(redefines = "sampleRate", default = 20.0, unit = "Hz")]
        pub sample_rate: Quantity<f64>,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub thermal_sense: Conjugated<ThermalPort>,
        #[sysml::port(redefines)]
        pub reading_out: TelemetryPort,
    }

    /// Embedded microwave MCU supervising cooking timing, door interlocks, and over-temperature faults.
    #[sysml::part]
    #[sysml::specializes(Microcontroller)]
    #[sysml::perform(supervise: SuperviseSafetyAndCooking)]
    #[derive(Debug, Clone, Copy)]
    pub struct MicrowaveMcu {
        #[sysml::attr(redefines = "clockFreq", default = 16000000.0, unit = "Hz")]
        pub clock_freq: Quantity<f64>,
        #[sysml::attr(redefines = "powerDraw", default = 1.5, unit = "W")]
        pub power_draw: Quantity<f64>,
        #[sysml::attr(default = 373.15, unit = "K")]
        pub max_safe_cavity_temp_k: Quantity<f64>,
        #[sysml::attr(default = 0.5)]
        pub door_open_trip_factor: f64,
        #[sysml::port(redefines)]
        pub power_in: Conjugated<ElectricalPowerPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
        #[sysml::port(redefines)]
        pub sensor_in: Conjugated<TelemetryPort>,
        #[sysml::port(redefines)]
        pub cmd_out: CommandPort,
        #[sysml::port]
        pub monitor_interlock_in: Conjugated<TelemetryPort>,
        #[sysml::port]
        pub temp_sensor_in: Conjugated<TelemetryPort>,
        #[sysml::port]
        pub turntable_cmd_out: CommandPort,
        #[sysml::port]
        pub hmi_cmd_in: Conjugated<CommandPort>,
        #[sysml::port]
        pub hmi_status_out: TelemetryPort,
    }

    /// Embedded control electronics, temperature sensing, and dual door safety interlocks.
    #[sysml::part]
    #[sysml::connect(interface = TelemetryChannel, name = "primaryInterlockToMcu", from = primary_interlock.interlock_status, to = mcu.sensor_in)]
    #[sysml::connect(interface = TelemetryChannel, name = "monitorInterlockToMcu", from = monitor_interlock.interlock_status, to = mcu.monitor_interlock_in)]
    #[sysml::connect(interface = TelemetryChannel, name = "cavityTempToMcu", from = cavity_temp_sensor.reading_out, to = mcu.temp_sensor_in)]
    #[derive(Debug, Clone, Copy)]
    pub struct ControlSafetySubsystem {
        #[sysml::part]
        pub mcu: MicrowaveMcu,
        #[sysml::part]
        pub cavity_temp_sensor: CavityTempSensor,
        #[sysml::part]
        pub primary_interlock: PrimaryDoorInterlock,
        #[sysml::part]
        pub monitor_interlock: MonitorDoorInterlock,
    }

    // =========================================================================
    // 8. Subsystem 4: HmiSubsystem (Keypad, Timer Display, Completion Buzzer)
    // =========================================================================

    /// Membrane numeric keypad, 7-segment timer display, and piezoelectric completion buzzer.
    #[sysml::part]
    #[sysml::specializes(HmiPanel)]
    #[sysml::perform(manageHmi: ManageUserInterface)]
    #[derive(Debug, Clone, Copy)]
    pub struct MicrowaveKeypadDisplay {
        #[sysml::attr(redefines = "powerDraw", default = 2.5, unit = "W")]
        pub power_draw: Quantity<f64>,
        #[sysml::attr(redefines = "chimeLevelDb", default = 68.0)]
        pub chime_level_db: f64,
        #[sysml::attr(default = 12)]
        pub keypad_button_count: i64,
        #[sysml::attr(default = 2700.0, unit = "Hz")]
        pub buzzer_freq: Quantity<f64>,
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
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
    }

    /// User interface panel subsystem bridging operator input, status display, and buzzer feedback.
    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct HmiSubsystem {
        #[sysml::part]
        pub keypad_display: MicrowaveKeypadDisplay,
    }

    // =========================================================================
    // 9. Cooking & Safety State Machine (#[sysml::state])
    // =========================================================================

    /// Microwave cooking & safety interlock state machine.
    ///
    /// Guarantees that any door-open event (`event.door_open > subject.door_open_trip_factor`)
    /// immediately forces `DoorOpenCutoff` and prevents starting cooking while the door is open.
    #[sysml::state(subject = MicrowaveMcu)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum MicrowaveState {
        #[sysml::initial]
        #[sysml::transition(
            on = CookStartCommand,
            guard = (event.door_closed > 0.5) && (event.requested_rf_power_w > 0.0),
            to = Cooking
        )]
        #[sysml::transition(
            on = DoorInterlockEvent,
            guard = event.door_open > subject.door_open_trip_factor,
            to = DoorOpenCutoff
        )]
        Standby,

        #[sysml::transition(
            on = DoorInterlockEvent,
            guard = event.door_open > subject.door_open_trip_factor,
            to = DoorOpenCutoff
        )]
        #[sysml::transition(
            on = ThermalMonitorEvent,
            guard = event.measured_temp_k > subject.max_safe_cavity_temp_k,
            to = OvertempFault
        )]
        #[sysml::transition(
            on = CookStopEvent,
            guard = event.cycle_done > 0.5,
            to = Standby
        )]
        Cooking,

        #[sysml::transition(
            on = DoorInterlockEvent,
            guard = (event.door_open <= subject.door_open_trip_factor) && (event.door_closed > 0.5),
            to = Standby
        )]
        DoorOpenCutoff,

        #[sysml::transition(
            on = ThermalMonitorEvent,
            guard = event.measured_temp_k <= subject.max_safe_cavity_temp_k,
            to = Standby
        )]
        OvertempFault,
    }

    // =========================================================================
    // 10. Top-Level Oven Assembly & Multi-Domain Context
    // =========================================================================

    /// Complete Microwave Oven product integrating the 4 domain subsystems and cross-domain wiring.
    #[sysml::part]
    #[sysml::exhibit(name = "operatingState", state = MicrowaveState)]
    // Electrical distribution from PowerHvSubsystem to RF, Control/Safety, and HMI subsystems
    #[sysml::connect(interface = ElectricalLink, name = "hvTransformerToMagnetron", from = power_hv.hv_transformer.power_out, to = rf_cooking.magnetron.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "hvRelayToCoolingFan", from = power_hv.hv_relay.power_out, to = rf_cooking.cooling_fan.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "hvRelayToTurntable", from = power_hv.hv_relay.power_out, to = rf_cooking.turntable.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "psuToMcu", from = power_hv.control_psu.power_out, to = control_safety.mcu.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "psuToTempSensor", from = power_hv.control_psu.power_out, to = control_safety.cavity_temp_sensor.power_in)]
    #[sysml::connect(interface = ElectricalLink, name = "psuToHmi", from = power_hv.control_psu.power_out, to = hmi.keypad_display.power_in)]
    // Control & telemetry channels across subsystems
    #[sysml::connect(interface = CommandChannel, name = "mcuToHvRelay", from = control_safety.mcu.cmd_out, to = power_hv.hv_relay.cmd_in)]
    #[sysml::connect(interface = CommandChannel, name = "mcuToTurntable", from = control_safety.mcu.turntable_cmd_out, to = rf_cooking.turntable.cmd_in)]
    #[sysml::connect(interface = CommandChannel, name = "hmiToMcuCmd", from = hmi.keypad_display.cmd_to_logic, to = control_safety.mcu.hmi_cmd_in)]
    #[sysml::connect(interface = TelemetryChannel, name = "mcuToHmiStatus", from = control_safety.mcu.hmi_status_out, to = hmi.keypad_display.status_from_logic)]
    // Thermal sensing from cavity to sensor
    #[sysml::connect(interface = ThermalLink, name = "cavityHeatToTempSensor", from = rf_cooking.cavity.heat_out, to = control_safety.cavity_temp_sensor.thermal_sense)]
    #[derive(Debug, Clone, Copy)]
    pub struct MicrowaveOven {
        #[sysml::part]
        pub power_hv: PowerHvSubsystem,
        #[sysml::part]
        pub rf_cooking: RfCookingSubsystem,
        #[sysml::part]
        pub control_safety: ControlSafetySubsystem,
        #[sysml::part]
        pub hmi: HmiSubsystem,
    }

    /// Top-level multi-domain system context closing HMI, door-interlock actuation, thermal, and acoustic boundaries.
    #[sysml::part]
    // HMI channels (HumanOperator <-> MicrowaveOven HMI)
    #[sysml::connect(interface = HmiInputChannel, name = "operatorToKeypad", from = operator.user_input, to = oven.hmi.keypad_display.user_in)]
    #[sysml::connect(interface = HmiInputChannel, name = "operatorToPrimaryInterlock", from = operator.user_input, to = oven.control_safety.primary_interlock.mechanical_actuation)]
    #[sysml::connect(interface = HmiInputChannel, name = "operatorToMonitorInterlock", from = operator.user_input, to = oven.control_safety.monitor_interlock.mechanical_actuation)]
    #[sysml::connect(interface = HmiFeedbackChannel, name = "displayToOperator", from = oven.hmi.keypad_display.user_out, to = operator.user_feedback)]
    // Thermal boundary closure (CoolingFan, Transformer, ControlPSU, MCU, HMI -> AmbientEnvironment)
    #[sysml::connect(interface = ThermalLink, name = "coolingFanToAmbient", from = oven.rf_cooking.cooling_fan.heat_out, to = ambient.heat_sink)]
    #[sysml::connect(interface = ThermalLink, name = "hvTransformerToAmbient", from = oven.power_hv.hv_transformer.heat_out, to = ambient.heat_sink)]
    #[sysml::connect(interface = ThermalLink, name = "controlPsuToAmbient", from = oven.power_hv.control_psu.heat_out, to = ambient.heat_sink)]
    #[sysml::connect(interface = ThermalLink, name = "mcuToAmbient", from = oven.control_safety.mcu.heat_out, to = ambient.heat_sink)]
    #[sysml::connect(interface = ThermalLink, name = "hmiToAmbient", from = oven.hmi.keypad_display.heat_out, to = ambient.heat_sink)]
    // Acoustic boundary closure (CoolingFan, HV Transformer, Turntable, Buzzer -> AmbientEnvironment)
    #[sysml::connect(interface = AcousticLink, name = "coolingFanNoiseToAmbient", from = oven.rf_cooking.cooling_fan.noise_out, to = ambient.noise_sink)]
    #[sysml::connect(interface = AcousticLink, name = "hvTransformerHumToAmbient", from = oven.power_hv.hv_transformer.noise_out, to = ambient.noise_sink)]
    #[sysml::connect(interface = AcousticLink, name = "turntableNoiseToAmbient", from = oven.rf_cooking.turntable.noise_out, to = ambient.noise_sink)]
    #[sysml::connect(interface = AcousticLink, name = "buzzerChimeToAmbient", from = oven.hmi.keypad_display.chime_out, to = ambient.noise_sink)]
    #[derive(Debug, Clone, Copy)]
    pub struct MicrowaveContext {
        #[sysml::part]
        pub operator: HumanOperator,
        #[sysml::part]
        pub oven: MicrowaveOven,
        #[sysml::part]
        pub ambient: AmbientEnvironment,
    }
}

/// Build the complete Microwave Oven `sysml_ir::Model` including all `sysml-std` packages
/// and the ISO 26262 metadata package (for `@SafetyGoal` / `@FMEA`).
pub fn build_model() -> Model {
    let mut pkgs = sysml_std::std_packages();
    pkgs.push(sysml_emit::iso26262_metadata_package());
    pkgs.push(microwave_model::sysml_package());
    Model::new(pkgs)
}

#[cfg(test)]
mod tests {
    use super::microwave_model::*;
    use super::*;
    use sysml_emit::{audit_model_with_stdlib, emit_model, lint_model};
    use sysml_std::prelude::{
        attenuated_noise_db, forced_convection_temp_rise_k, heating_time_s, sensible_heat_energy_j,
        useful_power_watts, waste_heat_watts,
    };

    #[test]
    fn microwave_model_audits_zero_errors_zero_warnings_zero_orphans() {
        let model = build_model();
        let sysml = emit_model(&model);
        let audit = audit_model_with_stdlib("microwave", &model, None, None, true);
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
            "microwave audit failed: {audit:#?}\n--- SYSML ---\n{sysml}"
        );
        println!(
            "AUDIT_OK: stdlib={}, errors={}, warnings={}, reqs={}/{}, orphans={}, calcs={}",
            audit.stdlib_loaded,
            audit.validation_errors,
            audit.validation_warnings,
            audit.satisfied_requirements,
            audit.total_requirements,
            audit.orphan_requirements.len(),
            audit.total_calcs
        );
    }

    #[test]
    fn microwave_model_lints_clean() {
        let model = build_model();
        let report = lint_model("microwave", &model);
        assert!(
            report.clean && report.summary.warning_count == 0,
            "lint report had warnings: {report:#?}"
        );
        assert_eq!(report.summary.info_count, 0, "lint report had infos: {report:#?}");
        println!(
            "LINT_OK: clean={}, warnings={}, infos={}, parts={}, specializations={}, redefinitions={}, reuse_pct={:.1}",
            report.clean,
            report.summary.warning_count,
            report.summary.info_count,
            report.summary.total_parts,
            report.summary.specializations,
            report.summary.redefinitions,
            report.summary.reuse_ratio_pct
        );
    }

    #[test]
    fn microwave_physics_calcs_execute_accurately() {
        let elec_in_w = 1400.0;
        let rf_eff = 900.0 / 1400.0;
        let rf_w = useful_power_watts(elec_in_w, rf_eff);
        let waste_w = waste_heat_watts(elec_in_w, rf_eff);
        assert!((rf_w - 900.0).abs() < 1e-9);
        assert!((waste_w - 500.0).abs() < 1e-9);

        // Heating 0.5 kg water by 60 K at 900 W * 0.88 coupling efficiency:
        // E = 0.5 * 4184 * 60 = 125520 J; P_abs = 792 W; t = 158.4848... s
        let energy_j = sensible_heat_energy_j(0.5, 4184.0, 60.0);
        assert!((energy_j - 125520.0).abs() < 1e-9);
        let t_s = heating_time_s(energy_j, rf_w, 0.88);
        assert!((t_s - (125520.0 / 792.0)).abs() < 1e-9);

        // Magnetron cooling temperature rise with 500 W waste heat and 0.015 kg/s airflow (c_p_air = 1005 J/(kg*K)):
        // dT = 500 / (0.015 * 1005) = 33.167495854... K <= 45 K
        let dt_k = forced_convection_temp_rise_k(waste_w, 0.015, 1005.0);
        assert!((dt_k - (500.0 / (0.015 * 1005.0))).abs() < 1e-9);
        assert!(dt_k <= 45.0);

        // Acoustic attenuation through CookingCavity thermal/acoustic enclosure:
        let cavity = CookingCavity::default();
        let fan = MagnetronCoolingFan::default();
        let attenuated_db = attenuated_noise_db(fan.noise_level_db, cavity.acoustic_attenuation_db);
        assert!((attenuated_db - 38.0).abs() < 1e-9);
    }

    #[test]
    fn microwave_requirements_evaluate_pass_and_fail_bounds() {
        assert!(MagnetronRfOutputRequirement::default().evaluate());
        assert!(!MagnetronRfOutputRequirement {
            rf_output_power_w: Quantity::new(750.0, "W"),
            ..MagnetronRfOutputRequirement::default()
        }
        .evaluate());

        let mcu_waste_rise = forced_convection_temp_rise_k(500.0, 0.015, 1005.0);
        assert!(MagnetronCoolingTempRequirement {
            actual_temp_rise_k: Quantity::new(mcu_waste_rise, "K"),
            ..MagnetronCoolingTempRequirement::default()
        }
        .evaluate());
        assert!(!MagnetronCoolingTempRequirement {
            actual_temp_rise_k: Quantity::new(52.0, "K"),
            ..MagnetronCoolingTempRequirement::default()
        }
        .evaluate());

        assert!(AcousticNoiseLimitRequirement::default().evaluate());
        assert!(!AcousticNoiseLimitRequirement {
            operating_noise_db: 61.0,
            ..AcousticNoiseLimitRequirement::default()
        }
        .evaluate());

        assert!(DoorInterlockCutoffRequirement::default().evaluate());
        assert!(!DoorInterlockCutoffRequirement {
            actual_cutoff_latency_s: Quantity::new(0.035, "s"),
            ..DoorInterlockCutoffRequirement::default()
        }
        .evaluate());
    }

    #[test]
    fn microwave_state_machine_enforces_door_open_cutoff_and_overtemp_fault() {
        let mcu = MicrowaveMcu::default();
        let mut state = MicrowaveState::initial_state();
        assert_eq!(state, MicrowaveState::Standby);

        // Door open blocks starting cooking
        let start_while_open = CookStartCommand {
            door_closed: 0.0,
            requested_rf_power_w: 900.0,
        };
        assert!(state.step_with(&start_while_open, &mcu).is_err());
        assert_eq!(state, MicrowaveState::Standby);
        // `dispatch_with` reports the same unmatched event as the normal SysML no-op.
        assert_eq!(
            state.dispatch_with(&start_while_open, &mcu),
            StepOutcome::Ignored
        );
        assert_eq!(state, MicrowaveState::Standby);

        // Door closed allows entering Cooking
        let start_valid = CookStartCommand {
            door_closed: 1.0,
            requested_rf_power_w: 900.0,
        };
        assert_eq!(
            state.step_with(&start_valid, &mcu).unwrap(),
            MicrowaveState::Cooking
        );

        // Door opening immediately forces DoorOpenCutoff (HV power cut)
        let door_opened = DoorInterlockEvent {
            door_open: 1.0,
            door_closed: 0.0,
        };
        assert_eq!(
            state.step_with(&door_opened, &mcu).unwrap(),
            MicrowaveState::DoorOpenCutoff
        );

        // Cannot start cooking directly from DoorOpenCutoff even with closed flag
        assert!(state.step_with(&start_valid, &mcu).is_err());
        assert_eq!(state, MicrowaveState::DoorOpenCutoff);

        // Closing door resets back to Standby
        let door_reclosed = DoorInterlockEvent {
            door_open: 0.0,
            door_closed: 1.0,
        };
        assert_eq!(
            state.step_with(&door_reclosed, &mcu).unwrap(),
            MicrowaveState::Standby
        );

        // Re-enter Cooking and trigger OvertempFault
        assert_eq!(
            state.step_with(&start_valid, &mcu).unwrap(),
            MicrowaveState::Cooking
        );
        let overtemp = ThermalMonitorEvent {
            measured_temp_k: 390.0,
        };
        assert_eq!(
            state.step_with(&overtemp, &mcu).unwrap(),
            MicrowaveState::OvertempFault
        );

        // Cooldown below max_safe_cavity_temp_k returns to Standby
        let cooled = ThermalMonitorEvent {
            measured_temp_k: 315.0,
        };
        assert_eq!(
            state.step_with(&cooled, &mcu).unwrap(),
            MicrowaveState::Standby
        );

        // Normal cycle completion
        assert_eq!(
            state.step_with(&start_valid, &mcu).unwrap(),
            MicrowaveState::Cooking
        );
        let done = CookStopEvent { cycle_done: 1.0 };
        assert_eq!(
            state.step_with(&done, &mcu).unwrap(),
            MicrowaveState::Standby
        );
    }
}
