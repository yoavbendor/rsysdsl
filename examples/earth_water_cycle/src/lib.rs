//! Earth Water Cycle hydrologic mass & thermal energy balance model.
//!
//! Models solar radiative forcing, the five primary terrestrial water reservoirs
//! (oceans, atmosphere, cryosphere, land surface water, and groundwater aquifers),
//! closed-loop fluid and latent-heat links, phase transitions of a `ThermodynamicParcel`,
//! hydrologic process allocations, mass/energy conservation requirements, and
//! physical calculations built on `sysml-std`.

use sysml_dsl::prelude::*;
use sysml_std::prelude::*;

#[sysml::package(name = "EarthWaterCycleModel")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
#[sysml::import("SysmlStdNatural::*")]
pub mod earth_water_cycle_model {
    use super::*;

    /// Solar radiative forcing driving global evaporation and atmospheric circulation.
    #[sysml::part]
    #[sysml::specializes(StarBody)]
    #[derive(Debug, Clone, Copy)]
    pub struct SolarInsolation {
        #[sysml::attr(redefines = "mass", default = 1.9885e30, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 6.9634e8, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 5772.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "luminosity", default = 3.828e26, unit = "W")]
        pub luminosity: Quantity<f64>,
        #[sysml::attr(default = 1.734e17, unit = "W")]
        pub earth_toa_intercept_power: Quantity<f64>,
        #[sysml::attr(default = 1.214e17, unit = "W")]
        pub surface_absorbed_power: Quantity<f64>,
        #[sysml::port(redefines)]
        pub radiation_out: RadiativePort,
    }

    /// Global ocean water reservoir (~96.5% of Earth's liquid/frozen hydrosphere).
    #[sysml::part]
    #[sysml::specializes(HydrologicReservoir)]
    #[sysml::perform(evaporate: EvaporateWater)]
    #[derive(Debug, Clone, Copy)]
    pub struct OceanReservoir {
        #[sysml::attr(redefines = "storedMass", default = 1.338e21, unit = "kg")]
        pub stored_mass: Quantity<f64>,
        #[sysml::attr(redefines = "temperature", default = 289.15, unit = "K")]
        pub temperature: Quantity<f64>,
        #[sysml::attr(redefines = "evaporationRate", default = 1.348e10, unit = "kg/s")]
        pub evaporation_rate: Quantity<f64>,
        #[sysml::attr(redefines = "precipitationRate", default = 1.221e10, unit = "kg/s")]
        pub precipitation_rate: Quantity<f64>,
        #[sysml::attr(redefines = "runoffRate", default = 1.27e9, unit = "kg/s")]
        pub runoff_rate: Quantity<f64>,
        #[sysml::attr(redefines = "latentHeatFlux", default = 3.371e16, unit = "W")]
        pub latent_heat_flux: Quantity<f64>,
        #[sysml::port]
        pub solar_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub inflow: Conjugated<FluidPort>,
        #[sysml::port]
        pub runoff_in: Conjugated<FluidPort>,
        #[sysml::port]
        pub baseflow_in: Conjugated<FluidPort>,
        #[sysml::port(redefines)]
        pub outflow: FluidPort,
        #[sysml::port(redefines)]
        pub heat_in: Conjugated<ThermalPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
    }

    /// Atmospheric water vapor and cloud reservoir (~1.29e16 kg, ~9.5 day mean residence time).
    #[sysml::part]
    #[sysml::specializes(HydrologicReservoir)]
    #[sysml::perform(condense: CondenseClouds)]
    #[sysml::perform(precipitate: Precipitate)]
    #[derive(Debug, Clone, Copy)]
    pub struct AtmosphereVapor {
        #[sysml::attr(redefines = "storedMass", default = 1.29e16, unit = "kg")]
        pub stored_mass: Quantity<f64>,
        #[sysml::attr(redefines = "temperature", default = 255.0, unit = "K")]
        pub temperature: Quantity<f64>,
        #[sysml::attr(redefines = "evaporationRate", default = 1.573e10, unit = "kg/s")]
        pub evaporation_rate: Quantity<f64>,
        #[sysml::attr(redefines = "precipitationRate", default = 1.573e10, unit = "kg/s")]
        pub precipitation_rate: Quantity<f64>,
        #[sysml::attr(redefines = "runoffRate", default = 0.0, unit = "kg/s")]
        pub runoff_rate: Quantity<f64>,
        #[sysml::attr(redefines = "latentHeatFlux", default = 3.934e16, unit = "W")]
        pub latent_heat_flux: Quantity<f64>,
        #[sysml::port(redefines)]
        pub inflow: Conjugated<FluidPort>,
        #[sysml::port(redefines)]
        pub outflow: FluidPort,
        #[sysml::port(redefines)]
        pub heat_in: Conjugated<ThermalPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
    }

    /// Cryosphere ice sheets, glaciers, and seasonal snowpack (~2.406e19 kg).
    #[sysml::part]
    #[sysml::specializes(HydrologicReservoir)]
    #[derive(Debug, Clone, Copy)]
    pub struct CryosphereIce {
        #[sysml::attr(redefines = "storedMass", default = 2.406e19, unit = "kg")]
        pub stored_mass: Quantity<f64>,
        #[sysml::attr(redefines = "temperature", default = 260.0, unit = "K")]
        pub temperature: Quantity<f64>,
        #[sysml::attr(redefines = "evaporationRate", default = 1.0e7, unit = "kg/s")]
        pub evaporation_rate: Quantity<f64>,
        #[sysml::attr(redefines = "precipitationRate", default = 7.0e7, unit = "kg/s")]
        pub precipitation_rate: Quantity<f64>,
        #[sysml::attr(redefines = "runoffRate", default = 6.0e7, unit = "kg/s")]
        pub runoff_rate: Quantity<f64>,
        #[sysml::attr(redefines = "latentHeatFlux", default = 2.834e13, unit = "W")]
        pub latent_heat_flux: Quantity<f64>,
        #[sysml::port(redefines)]
        pub inflow: Conjugated<FluidPort>,
        #[sysml::port(redefines)]
        pub outflow: FluidPort,
        #[sysml::port(redefines)]
        pub heat_in: Conjugated<ThermalPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
    }

    /// Continental land surface water (rivers, lakes, soil moisture, wetlands ~2.0e17 kg).
    #[sysml::part]
    #[sysml::specializes(HydrologicReservoir)]
    #[sysml::perform(runoff: SurfaceRunoff)]
    #[derive(Debug, Clone, Copy)]
    pub struct LandSurfaceWater {
        #[sysml::attr(redefines = "storedMass", default = 2.0e17, unit = "kg")]
        pub stored_mass: Quantity<f64>,
        #[sysml::attr(redefines = "temperature", default = 287.15, unit = "K")]
        pub temperature: Quantity<f64>,
        #[sysml::attr(redefines = "evaporationRate", default = 2.25e9, unit = "kg/s")]
        pub evaporation_rate: Quantity<f64>,
        #[sysml::attr(redefines = "precipitationRate", default = 3.52e9, unit = "kg/s")]
        pub precipitation_rate: Quantity<f64>,
        #[sysml::attr(redefines = "runoffRate", default = 1.20e9, unit = "kg/s")]
        pub runoff_rate: Quantity<f64>,
        #[sysml::attr(redefines = "latentHeatFlux", default = 5.627e15, unit = "W")]
        pub latent_heat_flux: Quantity<f64>,
        #[sysml::port]
        pub solar_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub inflow: Conjugated<FluidPort>,
        #[sysml::port]
        pub melt_in: Conjugated<FluidPort>,
        #[sysml::port(redefines)]
        pub outflow: FluidPort,
        #[sysml::port]
        pub snowpack_out: FluidPort,
        #[sysml::port]
        pub recharge_out: FluidPort,
        #[sysml::port]
        pub river_runoff_out: FluidPort,
        #[sysml::port(redefines)]
        pub heat_in: Conjugated<ThermalPort>,
        #[sysml::port(redefines)]
        pub heat_out: ThermalPort,
    }

    /// Subsurface groundwater aquifers (~1.053e19 kg fresh groundwater, slow baseflow discharge).
    #[sysml::part]
    #[sysml::specializes(HydrologicReservoir)]
    #[derive(Debug, Clone, Copy)]
    pub struct GroundwaterAquifer {
        #[sysml::attr(redefines = "storedMass", default = 1.053e19, unit = "kg")]
        pub stored_mass: Quantity<f64>,
        #[sysml::attr(redefines = "temperature", default = 285.15, unit = "K")]
        pub temperature: Quantity<f64>,
        #[sysml::attr(redefines = "evaporationRate", default = 0.0, unit = "kg/s")]
        pub evaporation_rate: Quantity<f64>,
        #[sysml::attr(redefines = "precipitationRate", default = 0.0, unit = "kg/s")]
        pub precipitation_rate: Quantity<f64>,
        #[sysml::attr(redefines = "runoffRate", default = 7.0e7, unit = "kg/s")]
        pub runoff_rate: Quantity<f64>,
        #[sysml::attr(redefines = "latentHeatFlux", default = 0.0, unit = "W")]
        pub latent_heat_flux: Quantity<f64>,
        #[sysml::port(redefines)]
        pub inflow: Conjugated<FluidPort>,
        #[sysml::port(redefines)]
        pub outflow: FluidPort,
    }

    // --- Phase-Transition Trigger Events ---

    /// Solar heating or surface warming pulse carrying parcel enthalpy condition.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Evaporate {
        #[sysml::attr(default = 373.15, unit = "K")]
        pub parcel_temp_k: f64,
        #[sysml::attr(default = 101325.0, unit = "Pa")]
        pub parcel_pressure_pa: f64,
    }

    /// Adiabatic cooling trigger in upper troposphere leading to cloud condensation.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Condense {
        #[sysml::attr(default = 273.15, unit = "K")]
        pub dewpoint_temp_k: f64,
    }

    /// Sub-zero cooling event driving crystallization of liquid water into ice.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Freeze {
        #[sysml::attr(default = 273.15, unit = "K")]
        pub freezing_temp_k: f64,
    }

    /// Warming above freezing point melting glacial ice or snowpack into liquid water.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Melt {
        #[sysml::attr(default = 273.15, unit = "K")]
        pub melting_temp_k: f64,
    }

    /// Direct solid-to-vapor phase transition under low vapor pressure and solar radiation.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Sublimate {
        #[sysml::attr(default = 265.0, unit = "K")]
        pub sublimation_temp_k: f64,
        #[sysml::attr(default = 611.657, unit = "Pa")]
        pub vapor_pressure_pa: f64,
    }

    /// Executable thermodynamic phase state machine over `ThermodynamicParcel`.
    #[sysml::state(subject = ThermodynamicParcel)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum WaterPhase {
        #[sysml::initial]
        #[sysml::transition(
            on = Evaporate,
            guard = event.parcel_temp_k >= subject.temperature_k && event.parcel_pressure_pa <= subject.pressure_pa,
            to = Vapor
        )]
        #[sysml::transition(
            on = Freeze,
            guard = event.freezing_temp_k <= 273.15 && subject.pressure_pa >= 611.657,
            to = SolidIce
        )]
        Liquid,

        #[sysml::transition(
            on = Condense,
            guard = event.dewpoint_temp_k < subject.temperature_k,
            to = Liquid
        )]
        Vapor,

        #[sysml::transition(
            on = Melt,
            guard = event.melting_temp_k > 273.15 && subject.pressure_pa >= 611.657,
            to = Liquid
        )]
        #[sysml::transition(
            on = Sublimate,
            guard = event.vapor_pressure_pa <= 611.657 && event.sublimation_temp_k >= 220.0,
            to = Vapor
        )]
        SolidIce,
    }

    // --- Hydrologic Processes (`#[sysml::action]`) ---

    /// Solar-driven evaporation transferring liquid surface water into atmospheric vapor.
    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct EvaporateWater {
        #[sysml::in_param]
        pub liquid_in: FluidFlow,
        #[sysml::out_param]
        pub vapor_out: FluidFlow,
    }

    /// Tropospheric cooling and nucleation converting water vapor into cloud droplets/ice.
    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct CondenseClouds {
        #[sysml::in_param]
        pub vapor_in: FluidFlow,
        #[sysml::out_param]
        pub latent_heat_out: ThermalEnergy,
    }

    /// Gravitational fallout of rain and snow back to ocean and continental surfaces.
    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Precipitate {
        #[sysml::in_param]
        pub cloud_moisture_in: FluidFlow,
        #[sysml::out_param]
        pub surface_flux_out: FluidFlow,
    }

    /// Continental overland and riverine drainage returning freshwater to the oceans.
    #[sysml::action]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct SurfaceRunoff {
        #[sysml::in_param]
        pub catchment_in: FluidFlow,
        #[sysml::out_param]
        pub river_discharge_out: FluidFlow,
    }

    // --- Top-Level Closed Hydrologic System Assembly ---

    /// Top-level Earth hydrologic system composing solar forcing, all 5 reservoirs,
    /// and a representative thermodynamic water parcel wired in a closed mass-and-energy loop.
    #[sysml::part]
    #[sysml::satisfy(req = GlobalWaterMassConservationReq, id = "HYD-001")]
    #[sysml::satisfy(req = OceanSteadyStateBalanceReq, id = "HYD-002")]
    #[sysml::satisfy(req = HydrologicLatentHeatBalanceReq, id = "HYD-003")]
    #[sysml::exhibit(name = "waterPhaseCycle", state = WaterPhase)]
    #[sysml::connect(interface = RadiativeLink, name = "sunToOcean", from = sun.radiation_out, to = ocean.solar_in)]
    #[sysml::connect(interface = RadiativeLink, name = "sunToLand", from = sun.radiation_out, to = land.solar_in)]
    #[sysml::connect(interface = FluidLink, name = "oceanEvaporationToAtmosphere", from = ocean.outflow, to = atmosphere.inflow)]
    #[sysml::connect(interface = FluidLink, name = "atmospherePrecipitationToLand", from = atmosphere.outflow, to = land.inflow)]
    #[sysml::connect(interface = FluidLink, name = "atmospherePrecipitationToOcean", from = atmosphere.outflow, to = ocean.inflow)]
    #[sysml::connect(interface = FluidLink, name = "landEvaporationToAtmosphere", from = land.outflow, to = atmosphere.inflow)]
    #[sysml::connect(interface = FluidLink, name = "landToCryosphere", from = land.snowpack_out, to = cryosphere.inflow)]
    #[sysml::connect(interface = FluidLink, name = "cryosphereMeltToLand", from = cryosphere.outflow, to = land.melt_in)]
    #[sysml::connect(interface = FluidLink, name = "landRechargeToGroundwater", from = land.recharge_out, to = groundwater.inflow)]
    #[sysml::connect(interface = FluidLink, name = "landRunoffToOcean", from = land.river_runoff_out, to = ocean.runoff_in)]
    #[sysml::connect(interface = FluidLink, name = "groundwaterBaseflowToOcean", from = groundwater.outflow, to = ocean.baseflow_in)]
    #[sysml::connect(interface = ThermalLink, name = "oceanLatentHeatToAtmosphere", from = ocean.heat_out, to = atmosphere.heat_in)]
    #[sysml::connect(interface = ThermalLink, name = "landLatentHeatToAtmosphere", from = land.heat_out, to = atmosphere.heat_in)]
    #[sysml::connect(interface = ThermalLink, name = "atmosphereCondensationHeatToCryosphere", from = atmosphere.heat_out, to = cryosphere.heat_in)]
    #[sysml::connect(interface = ThermalLink, name = "atmosphereCondensationHeatToLand", from = atmosphere.heat_out, to = land.heat_in)]
    #[sysml::connect(interface = ThermalLink, name = "cryosphereHeatExchangeToOcean", from = cryosphere.heat_out, to = ocean.heat_in)]
    #[derive(Debug, Clone, Copy)]
    pub struct EarthHydrologicSystem {
        #[sysml::part]
        pub sun: SolarInsolation,
        #[sysml::part]
        pub ocean: OceanReservoir,
        #[sysml::part]
        pub atmosphere: AtmosphereVapor,
        #[sysml::part]
        pub cryosphere: CryosphereIce,
        #[sysml::part]
        pub land: LandSurfaceWater,
        #[sysml::part]
        pub groundwater: GroundwaterAquifer,
        #[sysml::part]
        pub parcel: ThermodynamicParcel,
    }

    // --- Hydrologic & Thermodynamic Requirements ---

    /// Global planetary water mass shall be conserved across evaporation and precipitation fluxes.
    #[sysml::requirement(id = "HYD-001")]
    #[sysml::require_constraint(
        (total_inflow_kg_s - total_outflow_kg_s <= max_mass_imbalance_kg_s)
            && (total_outflow_kg_s - total_inflow_kg_s <= max_mass_imbalance_kg_s)
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct GlobalWaterMassConservationReq {
        #[sysml::attr(default = 1.573e10, unit = "kg/s")]
        pub total_inflow_kg_s: Quantity<f64>,
        #[sysml::attr(default = 1.573e10, unit = "kg/s")]
        pub total_outflow_kg_s: Quantity<f64>,
        #[sysml::attr(default = 1.0e6, unit = "kg/s")]
        pub max_mass_imbalance_kg_s: Quantity<f64>,
    }

    /// Global ocean mass balance (precipitation + surface runoff + baseflow - evaporation)
    /// shall remain in steady-state equilibrium within tolerance.
    #[sysml::requirement(id = "HYD-002")]
    #[sysml::require_constraint(
        ((precip_ocean_kg_s + runoff_kg_s + baseflow_kg_s) - evap_ocean_kg_s <= max_ocean_imbalance_kg_s)
            && (evap_ocean_kg_s - (precip_ocean_kg_s + runoff_kg_s + baseflow_kg_s) <= max_ocean_imbalance_kg_s)
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct OceanSteadyStateBalanceReq {
        #[sysml::attr(default = 1.221e10, unit = "kg/s")]
        pub precip_ocean_kg_s: Quantity<f64>,
        #[sysml::attr(default = 1.20e9, unit = "kg/s")]
        pub runoff_kg_s: Quantity<f64>,
        #[sysml::attr(default = 7.0e7, unit = "kg/s")]
        pub baseflow_kg_s: Quantity<f64>,
        #[sysml::attr(default = 1.348e10, unit = "kg/s")]
        pub evap_ocean_kg_s: Quantity<f64>,
        #[sysml::attr(default = 1.0e6, unit = "kg/s")]
        pub max_ocean_imbalance_kg_s: Quantity<f64>,
    }

    /// Surface-to-atmosphere latent heat flux shall balance global latent heat release
    /// and remain bounded by absorbed solar radiation power.
    #[sysml::requirement(id = "HYD-003")]
    #[sysml::require_constraint(
        (surface_latent_heat_w - atmosphere_latent_release_w <= max_thermal_imbalance_w)
            && (atmosphere_latent_release_w - surface_latent_heat_w <= max_thermal_imbalance_w)
            && (surface_latent_heat_w <= absorbed_solar_power_w)
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct HydrologicLatentHeatBalanceReq {
        #[sysml::attr(default = 3.934e16, unit = "W")]
        pub surface_latent_heat_w: Quantity<f64>,
        #[sysml::attr(default = 3.934e16, unit = "W")]
        pub atmosphere_latent_release_w: Quantity<f64>,
        #[sysml::attr(default = 1.214e17, unit = "W")]
        pub absorbed_solar_power_w: Quantity<f64>,
        #[sysml::attr(default = 1.0e14, unit = "W")]
        pub max_thermal_imbalance_w: Quantity<f64>,
    }

    // --- Domain-Specific Modeled Physics Calculation ---

    /// Net ocean mass rate [kg/s]: `dm_ocean/dt = P_ocean + R_surface + Q_baseflow - E_ocean`.
    #[sysml::calc]
    pub fn net_ocean_balance_kg_s(
        precip_ocean: f64,
        runoff: f64,
        baseflow: f64,
        evap_ocean: f64,
    ) -> f64 {
        (precip_ocean + runoff + baseflow) - evap_ocean
    }
}

/// Build the complete `EarthWaterCycleModel` alongside `sysml-std` foundation packages.
pub fn build_model() -> Model {
    let mut pkgs = sysml_std::std_packages();
    pkgs.push(earth_water_cycle_model::sysml_package());
    Model::new(pkgs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use earth_water_cycle_model::*;
    use sysml_emit::{audit_model_with_stdlib, emit_model, lint_model};

    #[test]
    fn earth_water_cycle_audits_zero_errors_zero_warnings_zero_orphans() {
        let model = build_model();
        let sysml = emit_model(&model);
        let audit = audit_model_with_stdlib("earth_water_cycle", &model, None, None, true);
        assert!(audit.stdlib_loaded, "OMG sysml.library must be loaded");
        assert!(
            audit.ok,
            "earth_water_cycle audit failed: {audit:#?}\n--- SYSML ---\n{sysml}"
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
    fn earth_water_cycle_lints_clean() {
        let model = build_model();
        let lint = lint_model("earth_water_cycle", &model);
        assert!(lint.clean, "earth_water_cycle lint failed: {lint:#?}");
        assert_eq!(
            lint.summary.info_count, 0,
            "earth_water_cycle lint infos: {:#?}",
            lint.findings
        );
    }

    #[test]
    fn earth_water_cycle_state_transitions_execute() {
        let parcel = ThermodynamicParcel::default();

        // Guard rejection when parcel temperature is below subject threshold (293.15 K)
        let mut phase = WaterPhase::initial_state();
        assert_eq!(phase, WaterPhase::Liquid);
        let too_cool = Evaporate {
            parcel_temp_k: 280.0,
            parcel_pressure_pa: 101325.0,
        };
        assert!(phase.step_with(&too_cool, &parcel).is_err());
        assert_eq!(phase, WaterPhase::Liquid);

        // Liquid -> Vapor via Evaporate
        let evap = Evaporate {
            parcel_temp_k: 300.0,
            parcel_pressure_pa: 101325.0,
        };
        assert_eq!(phase.step_with(&evap, &parcel).unwrap(), WaterPhase::Vapor);

        // Vapor -> Liquid via Condense
        let cond = Condense {
            dewpoint_temp_k: 278.15,
        };
        assert_eq!(phase.step_with(&cond, &parcel).unwrap(), WaterPhase::Liquid);

        // Liquid -> SolidIce via Freeze
        let freeze = Freeze {
            freezing_temp_k: 268.0,
        };
        assert_eq!(
            phase.step_with(&freeze, &parcel).unwrap(),
            WaterPhase::SolidIce
        );

        // SolidIce -> Vapor via Sublimate (below water triple-point pressure 611.657 Pa)
        let subl = Sublimate {
            sublimation_temp_k: 265.0,
            vapor_pressure_pa: 400.0,
        };
        assert_eq!(phase.step_with(&subl, &parcel).unwrap(), WaterPhase::Vapor);

        // Vapor -> Liquid -> SolidIce -> Liquid via Melt
        assert_eq!(phase.step_with(&cond, &parcel).unwrap(), WaterPhase::Liquid);
        assert_eq!(
            phase.step_with(&freeze, &parcel).unwrap(),
            WaterPhase::SolidIce
        );
        let melt = Melt {
            melting_temp_k: 276.0,
        };
        assert_eq!(phase.step_with(&melt, &parcel).unwrap(), WaterPhase::Liquid);
    }

    #[test]
    fn earth_water_cycle_physics_calcs_execute_accurately() {
        // Reused from sysml-std: latent heat power (2.501e6 J/kg) & reservoir residence time
        let q_lat = latent_heat_power_w(1.348e10, 2.501e6);
        assert!((q_lat - 3.371348e16).abs() < 1e9);

        let tau_atm = reservoir_residence_time_s(1.29e16, 1.573e10);
        assert!((tau_atm - 820_089.0019).abs() < 1.0);

        // Local calc: net ocean balance 1.221e10 + 1.20e9 + 7.0e7 - 1.348e10 == 0
        let net_ocean = net_ocean_balance_kg_s(1.221e10, 1.20e9, 7.0e7, 1.348e10);
        assert!(net_ocean.abs() < 1e-3);
    }

    #[test]
    fn earth_water_cycle_requirements_evaluate_nominal_and_violated() {
        let req1_ok = GlobalWaterMassConservationReq::default();
        assert!(req1_ok.evaluate());
        let req1_bad = GlobalWaterMassConservationReq {
            total_outflow_kg_s: Quantity::new(1.580e10, "kg/s"),
            ..GlobalWaterMassConservationReq::default()
        };
        assert!(!req1_bad.evaluate());

        let req2_ok = OceanSteadyStateBalanceReq::default();
        assert!(req2_ok.evaluate());
        let req2_bad = OceanSteadyStateBalanceReq {
            runoff_kg_s: Quantity::new(1.00e9, "kg/s"),
            ..OceanSteadyStateBalanceReq::default()
        };
        assert!(!req2_bad.evaluate());

        let req3_ok = HydrologicLatentHeatBalanceReq::default();
        assert!(req3_ok.evaluate());
        let req3_bad = HydrologicLatentHeatBalanceReq {
            atmosphere_latent_release_w: Quantity::new(4.10e16, "W"),
            ..HydrologicLatentHeatBalanceReq::default()
        };
        assert!(!req3_bad.evaluate());
    }
}
