//! Natural physical system abstractions: celestial bodies, planetary hierarchy, and fluid reservoirs.

use sysml_dsl::prelude::*;

/// sysml-std library package. Abstract natural bases declare units with no placeholder defaults;
/// concrete physical reference bodies (`StarBody`, `DeepSpaceSink`, `ThermodynamicParcel`) carry
/// physical reference defaults (`rsysdsl lint` reports inherited ones as `inherited_std_default`).
#[sysml::package(library, name = "SysmlStdNatural")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
pub mod sysml_std_natural {
    use super::*;
    use crate::flows::sysml_std_flows::{FluidPort, RadiativePort, ThermalPort};

    /// Abstract macroscopic natural body with mass, radius, and surface temperature.
    #[sysml::part(abstract)]
    #[derive(Debug, Clone, Copy)]
    pub struct NaturalBody {
        #[sysml::attr(unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(unit = "K")]
        pub surface_temperature: Quantity<f64>,
    }

    /// Radiating star emitting electromagnetic luminosity (`W`).
    #[sysml::part]
    #[sysml::specializes(NaturalBody)]
    #[derive(Debug, Clone, Copy)]
    pub struct StarBody {
        #[sysml::attr(default = 3.828e26, unit = "W")]
        pub luminosity: Quantity<f64>,
        #[sysml::port]
        pub radiation_out: RadiativePort,
    }

    /// Abstract orbiting celestial body receiving stellar radiation and re-radiating thermal energy.
    #[sysml::part(abstract)]
    #[sysml::specializes(NaturalBody)]
    #[derive(Debug, Clone, Copy)]
    pub struct PlanetaryBody {
        #[sysml::attr(unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(dimensionless)]
        pub bond_albedo: f64,
        #[sysml::port]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port]
        pub thermal_emission: ThermalPort,
    }

    /// Terrestrial rocky planet (Mercury, Venus, Earth, Mars).
    #[sysml::part]
    #[sysml::specializes(PlanetaryBody)]
    #[derive(Debug, Clone, Copy)]
    pub struct RockyPlanet {}

    /// Hydrogen/helium gas giant (Jupiter, Saturn).
    #[sysml::part]
    #[sysml::specializes(PlanetaryBody)]
    #[derive(Debug, Clone, Copy)]
    pub struct GasGiant {}

    /// Volatile-rich ice giant (Uranus, Neptune).
    #[sysml::part]
    #[sysml::specializes(PlanetaryBody)]
    #[derive(Debug, Clone, Copy)]
    pub struct IceGiant {}

    /// Natural satellite orbiting a host planet.
    #[sysml::part]
    #[sysml::specializes(PlanetaryBody)]
    #[derive(Debug, Clone, Copy)]
    pub struct NaturalSatellite {}

    /// Abstract physical fluid reservoir conserving water/fluid mass and thermal energy.
    #[sysml::part(abstract)]
    #[derive(Debug, Clone, Copy)]
    pub struct FluidReservoir {
        #[sysml::attr(unit = "kg")]
        pub stored_mass: Quantity<f64>,
        #[sysml::attr(unit = "K")]
        pub temperature: Quantity<f64>,
        #[sysml::port]
        pub inflow: Conjugated<FluidPort>,
        #[sysml::port]
        pub outflow: FluidPort,
        #[sysml::port]
        pub heat_in: Conjugated<ThermalPort>,
        #[sysml::port]
        pub heat_out: ThermalPort,
    }

    /// Abstract hydrologic reservoir adding evaporation, precipitation, runoff, and latent heat fluxes.
    #[sysml::part(abstract)]
    #[sysml::specializes(FluidReservoir)]
    #[derive(Debug, Clone, Copy)]
    pub struct HydrologicReservoir {
        #[sysml::attr(unit = "kg/s")]
        pub evaporation_rate: Quantity<f64>,
        #[sysml::attr(unit = "kg/s")]
        pub precipitation_rate: Quantity<f64>,
        #[sysml::attr(unit = "kg/s")]
        pub runoff_rate: Quantity<f64>,
        #[sysml::attr(unit = "W")]
        pub latent_heat_flux: Quantity<f64>,
    }

    /// Deep space cosmic-microwave-background thermal/radiative sink (`2.725 K`).
    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct DeepSpaceSink {
        #[sysml::attr(default = 2.725, unit = "K")]
        pub cmb_temperature: Quantity<f64>,
        #[sysml::port]
        pub thermal_sink: Conjugated<ThermalPort>,
        #[sysml::port]
        pub radiation_sink: Conjugated<RadiativePort>,
    }

    /// Thermodynamic state parcel for phase-change state machines (solid/liquid/vapor).
    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct ThermodynamicParcel {
        #[sysml::attr(default = 293.15, unit = "K")]
        pub temperature_k: Quantity<f64>,
        #[sysml::attr(default = 101325.0, unit = "Pa")]
        pub pressure_pa: Quantity<f64>,
    }
}
