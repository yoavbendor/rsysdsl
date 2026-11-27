//! Universal physical flows, ports, energy/matter links, and core physics `#[sysml::calc]` laws.

use sysml_dsl::prelude::*;

/// sysml-std library package defining universal physical flows, ports, links, and physics calcs.
/// Flow items and generic physical links declare units without numeric placeholder defaults so
/// specializing models never inherit fictional numbers.
#[sysml::package(library, name = "SysmlStdFlows")]
#[sysml::import("ScalarValues::*")]
pub mod sysml_std_flows {
    use super::*;

    /// Root abstract item for any conserved physical or informational transfer.
    #[sysml::item(abstract)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct PhysicalFlow {}

    /// Abstract energy transfer carrying power in Watts (`W`).
    #[sysml::item(abstract)]
    #[sysml::specializes(PhysicalFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct EnergyFlow {
        #[sysml::attr(unit = "W")]
        pub power: Quantity<f64>,
    }

    /// Electrical power transfer (`V`, `A`, `W`).
    #[sysml::item]
    #[sysml::specializes(EnergyFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ElectricalEnergy {
        #[sysml::attr(unit = "V")]
        pub voltage: Quantity<f64>,
        #[sysml::attr(unit = "A")]
        pub current: Quantity<f64>,
    }

    /// Mechanical power transfer (`N`, `m/s`, `W`).
    #[sysml::item]
    #[sysml::specializes(EnergyFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MechanicalEnergy {
        #[sysml::attr(unit = "N")]
        pub force: Quantity<f64>,
        #[sysml::attr(unit = "m/s")]
        pub velocity: Quantity<f64>,
    }

    /// Conductive/convective thermal energy flow (`K`, `W`).
    #[sysml::item]
    #[sysml::specializes(EnergyFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ThermalEnergy {
        #[sysml::attr(unit = "K")]
        pub temperature: Quantity<f64>,
    }

    /// Electromagnetic / radiative energy transfer (sunlight, RF microwave, infrared).
    #[sysml::item]
    #[sysml::specializes(EnergyFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct RadiativeEnergy {
        #[sysml::attr(unit = "Hz")]
        pub frequency: Quantity<f64>,
    }

    /// Acoustic pressure wave / mechanical noise emission (`Pa`, `Hz`).
    #[sysml::item]
    #[sysml::specializes(EnergyFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct AcousticEmission {
        #[sysml::attr(unit = "Pa")]
        pub sound_pressure: Quantity<f64>,
        #[sysml::attr(unit = "Hz")]
        pub dominant_freq: Quantity<f64>,
    }

    /// Abstract material / mass transfer (`kg/s`).
    #[sysml::item(abstract)]
    #[sysml::specializes(PhysicalFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MaterialFlow {
        #[sysml::attr(unit = "kg/s")]
        pub mass_rate: Quantity<f64>,
    }

    /// Liquid or gas fluid flow (`kg/s`, `Pa`, `K`).
    #[sysml::item]
    #[sysml::specializes(MaterialFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct FluidFlow {
        #[sysml::attr(unit = "Pa")]
        pub pressure: Quantity<f64>,
        #[sysml::attr(unit = "K")]
        pub temperature: Quantity<f64>,
    }

    /// Abstract information / signal transfer.
    #[sysml::item(abstract)]
    #[sysml::specializes(PhysicalFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct InformationFlow {}

    /// Actuator / subsystem control command payload.
    #[sysml::item]
    #[sysml::specializes(InformationFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ControlCommand {}

    /// Sensor measurement or status telemetry payload.
    #[sysml::item]
    #[sysml::specializes(InformationFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct TelemetrySignal {}

    /// Human operator input action (press, turn, squeeze, pull).
    #[sysml::item]
    #[sysml::specializes(InformationFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct HumanInputSignal {}

    /// Visual or tactile feedback presented to a human operator.
    #[sysml::item]
    #[sysml::specializes(InformationFlow)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct HumanFeedbackSignal {}

    // --- Directed & Symmetric Standard Ports ---

    /// Directed electrical power source port (connect to `Conjugated<ElectricalPowerPort>`).
    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ElectricalPowerPort {
        #[sysml::out_item]
        pub electrical: ElectricalEnergy,
    }

    /// Directed mechanical power output port (connect to `Conjugated<MechanicalPowerPort>`).
    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MechanicalPowerPort {
        #[sysml::out_item]
        pub mechanical: MechanicalEnergy,
    }

    /// Directed thermal heat output port (connect to `Conjugated<ThermalPort>`).
    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ThermalPort {
        #[sysml::out_item]
        pub thermal: ThermalEnergy,
    }

    /// Directed electromagnetic / radiative flux output port.
    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct RadiativePort {
        #[sysml::out_item]
        pub radiation: RadiativeEnergy,
    }

    /// Directed fluid mass-flow outlet port.
    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct FluidPort {
        #[sysml::out_item]
        pub fluid: FluidFlow,
    }

    /// Directed acoustic noise emission port.
    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct AcousticPort {
        #[sysml::out_item]
        pub acoustic: AcousticEmission,
    }

    /// Abstract directed information output port.
    #[sysml::port(abstract)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct InfoPort {
        #[sysml::out_item]
        pub info: InformationFlow,
    }

    /// Directed control command output port.
    #[sysml::port]
    #[sysml::specializes(InfoPort)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct CommandPort {
        #[sysml::out_item]
        pub cmd: ControlCommand,
    }

    /// Directed sensor/status telemetry output port.
    #[sysml::port]
    #[sysml::specializes(InfoPort)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct TelemetryPort {
        #[sysml::out_item]
        pub telemetry: TelemetrySignal,
    }

    /// Directed human operator input port (button, dial, trigger, lever).
    #[sysml::port]
    #[sysml::specializes(InfoPort)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct HmiInputPort {
        #[sysml::out_item]
        pub input: HumanInputSignal,
    }

    /// Directed human operator visual/haptic feedback port.
    #[sysml::port]
    #[sysml::specializes(InfoPort)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct HmiFeedbackPort {
        #[sysml::out_item]
        pub feedback: HumanFeedbackSignal,
    }

    /// Bidirectional symmetric digital data bus port (CAN, I2C, Ethernet).
    #[sysml::port(symmetric)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct DataBusPort {
        #[sysml::inout_item]
        pub packet: InformationFlow,
    }

    // --- Standard Physical & Energy Interfaces ---

    /// Electrical power distribution wire / busbar link.
    #[sysml::interface]
    #[derive(Debug, Clone, Copy)]
    pub struct ElectricalLink {
        #[sysml::attr(dimensionless)]
        pub transmission_efficiency: f64,
        #[sysml::end]
        pub supply: ElectricalPowerPort,
        #[sysml::end]
        pub load: Conjugated<ElectricalPowerPort>,
    }

    /// Mechanical drive coupling (shaft, chain, gear mesh, direct drive).
    #[sysml::interface]
    #[derive(Debug, Clone, Copy)]
    pub struct MechanicalLink {
        #[sysml::attr(dimensionless)]
        pub transmission_efficiency: f64,
        #[sysml::end]
        pub driver: MechanicalPowerPort,
        #[sysml::end]
        pub driven: Conjugated<MechanicalPowerPort>,
    }

    /// Thermal conduction or convection path.
    #[sysml::interface]
    #[derive(Debug, Clone, Copy)]
    pub struct ThermalLink {
        #[sysml::end]
        pub source: ThermalPort,
        #[sysml::end]
        pub sink: Conjugated<ThermalPort>,
    }

    /// Radiative / electromagnetic energy transfer path (solar, RF waveguide, IR).
    #[sysml::interface]
    #[derive(Debug, Clone, Copy)]
    pub struct RadiativeLink {
        #[sysml::end]
        pub emitter: RadiativePort,
        #[sysml::end]
        pub receiver: Conjugated<RadiativePort>,
    }

    /// Fluid conduit (hose, pipe, atmospheric moisture transport, waterway).
    #[sysml::interface]
    #[derive(Debug, Clone, Copy)]
    pub struct FluidLink {
        #[sysml::end]
        pub outlet: FluidPort,
        #[sysml::end]
        pub inlet: Conjugated<FluidPort>,
    }

    /// Acoustic noise propagation path into ambient space.
    #[sysml::interface]
    #[derive(Debug, Clone, Copy)]
    pub struct AcousticLink {
        #[sysml::end]
        pub emitter: AcousticPort,
        #[sysml::end]
        pub sink: Conjugated<AcousticPort>,
    }

    // --- Universal Physics Calculations ---

    /// Useful output power given input power and conversion efficiency (0..1).
    #[sysml::calc]
    pub fn useful_power_watts(input_power_w: f64, efficiency: f64) -> f64 {
        input_power_w * efficiency
    }

    /// Waste heat dissipated given input power and efficiency: `P_loss = P_in * (1 - eta)`.
    #[sysml::calc]
    pub fn waste_heat_watts(input_power_w: f64, efficiency: f64) -> f64 {
        input_power_w * (1.0 - efficiency)
    }

    /// Electrical power law: `P = V * I`.
    #[sysml::calc]
    pub fn electrical_power_watts(voltage_v: f64, current_a: f64) -> f64 {
        voltage_v * current_a
    }

    /// Linear mechanical power law: `P = F * v`.
    #[sysml::calc]
    pub fn mechanical_power_watts(force_n: f64, velocity_m_s: f64) -> f64 {
        force_n * velocity_m_s
    }

    /// Steady-state temperature rise across thermal resistance: `dT = Q * R_th`.
    #[sysml::calc]
    pub fn thermal_rise_k(heat_watts: f64, thermal_resistance_k_per_w: f64) -> f64 {
        heat_watts * thermal_resistance_k_per_w
    }

    /// Net reservoir mass balance rate: `dm/dt = inflow - outflow`.
    #[sysml::calc]
    pub fn mass_balance_rate(inflow_kg_s: f64, outflow_kg_s: f64) -> f64 {
        inflow_kg_s - outflow_kg_s
    }

    /// Sensible heating energy: `E = m * c_p * dT`.
    #[sysml::calc]
    pub fn sensible_heat_energy_j(mass_kg: f64, specific_heat_j_kg_k: f64, delta_temp_k: f64) -> f64 {
        mass_kg * specific_heat_j_kg_k * delta_temp_k
    }

    /// Latent phase-change heat power: `Q_dot = m_dot * L`.
    #[sysml::calc]
    pub fn latent_heat_power_w(mass_rate_kg_s: f64, latent_heat_j_per_kg: f64) -> f64 {
        mass_rate_kg_s * latent_heat_j_per_kg
    }

    /// Time required to deliver thermal energy `E` at power `P` and coupling efficiency `eta`.
    #[sysml::calc]
    pub fn heating_time_s(energy_j: f64, power_w: f64, efficiency: f64) -> f64 {
        energy_j / (power_w * efficiency)
    }

    /// Forced-air / fluid convective temperature rise: `dT = Q / (m_dot * c_p)`.
    #[sysml::calc]
    pub fn forced_convection_temp_rise_k(
        heat_watts: f64,
        mass_flow_kg_s: f64,
        specific_heat_j_kg_k: f64,
    ) -> f64 {
        heat_watts / (mass_flow_kg_s * specific_heat_j_kg_k)
    }

    /// Enclosure heat loss across thermal conductance `G` (`W/K`): `Q_loss = G * (T_in - T_amb)`.
    #[sysml::calc]
    pub fn convective_heat_loss_w(
        conductance_w_per_k: f64,
        t_internal_k: f64,
        t_ambient_k: f64,
    ) -> f64 {
        conductance_w_per_k * (t_internal_k - t_ambient_k)
    }

    /// Total waste heat across a 3-stage cascaded powertrain (e.g. inverter -> motor -> gearbox).
    #[sysml::calc]
    pub fn cascaded_waste_heat_w(
        input_power_w: f64,
        eff_stage1: f64,
        eff_stage2: f64,
        eff_stage3: f64,
    ) -> f64 {
        input_power_w * (1.0 - eff_stage1 * eff_stage2 * eff_stage3)
    }

    /// Battery runtime at constant load power: `t = E / P`.
    #[sysml::calc]
    pub fn battery_runtime_s(energy_j: f64, load_power_w: f64) -> f64 {
        energy_j / load_power_w
    }

    /// Mean reservoir residence time: `tau = M / outflow`.
    #[sysml::calc]
    pub fn reservoir_residence_time_s(stored_mass_kg: f64, outflow_rate_kg_s: f64) -> f64 {
        stored_mass_kg / outflow_rate_kg_s
    }

    /// Inverse-square radiative flux at distance `r`: `F = L / (4 * pi * r^2)`.
    #[sysml::calc]
    pub fn inverse_square_flux_w_m2(luminosity_w: f64, distance_m: f64) -> f64 {
        luminosity_w / (4.0 * 3.141592653589793 * distance_m * distance_m)
    }

    /// Radiative power absorbed by a spherical body of radius `R` and Bond albedo `a`.
    #[sysml::calc]
    pub fn radiative_intercept_power_w(flux_w_m2: f64, radius_m: f64, bond_albedo: f64) -> f64 {
        flux_w_m2 * 3.141592653589793 * radius_m * radius_m * (1.0 - bond_albedo)
    }

    /// Circular orbital speed: `v = 2 * pi * r / T`.
    #[sysml::calc]
    pub fn orbital_speed_m_s(orbital_radius_m: f64, orbital_period_s: f64) -> f64 {
        2.0 * 3.141592653589793 * orbital_radius_m / orbital_period_s
    }

    /// Newtonian gravitational attraction force: `F = G * m1 * m2 / r^2`.
    #[sysml::calc]
    pub fn gravitational_force_n(m1_kg: f64, m2_kg: f64, distance_m: f64) -> f64 {
        6.67430e-11 * m1_kg * m2_kg / (distance_m * distance_m)
    }

    /// Aerodynamic drag power: `P_aero = 0.5 * rho * CdA * v^3`.
    #[sysml::calc]
    pub fn aero_drag_power_w(air_density_kg_m3: f64, cda_m2: f64, speed_m_s: f64) -> f64 {
        0.5 * air_density_kg_m3 * cda_m2 * speed_m_s * speed_m_s * speed_m_s
    }

    /// Rolling resistance power: `P_roll = C_rr * m * g * v`.
    #[sysml::calc]
    pub fn rolling_resistance_power_w(
        crr: f64,
        total_mass_kg: f64,
        gravity_m_s2: f64,
        speed_m_s: f64,
    ) -> f64 {
        crr * total_mass_kg * gravity_m_s2 * speed_m_s
    }

    /// Instantaneous braking thermal dissipation power: `P_brake = m * a * v`.
    #[sysml::calc]
    pub fn braking_heat_power_w(total_mass_kg: f64, decel_m_s2: f64, speed_m_s: f64) -> f64 {
        total_mass_kg * decel_m_s2 * speed_m_s
    }

    /// Acoustic noise level after enclosure attenuation: `L_out = L_src - attenuation`.
    #[sysml::calc]
    pub fn attenuated_noise_db(source_db: f64, attenuation_db: f64) -> f64 {
        source_db - attenuation_db
    }
}
