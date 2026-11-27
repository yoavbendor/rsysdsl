//! Solar System hierarchical astrophysical & radiative-thermal energy balance model.
//!
//! Models the Sun (`StarBody`), all 8 planets (`Mercury`, `Venus`, `Earth`, `Mars`,
//! `Jupiter`, `Saturn`, `Uranus`, `Neptune`), representative natural satellites (`Moon`,
//! `Europa`), a `PlanetClass` product-line variation over `PlanetaryBody`, `DeepSpaceSink`
//! (`2.725 K` cosmic microwave background from `sysml-std`), and the top-level `SolarSystem`
//! context wired with `RadiativeLink` (Sun -> each planet) and `ThermalLink`
//! (planets -> DeepSpaceSink), plus orbital mechanics, radiative irradiance, and planetary
//! thermal equilibrium requirements and `sysml-std` physics calculations.

use sysml_dsl::prelude::*;
use sysml_std::prelude::*;

#[sysml::package(name = "SolarSystemModel")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
#[sysml::import("SysmlStdNatural::*")]
pub mod solar_system_model {
    use super::*;

    // =========================================================================
    // 1. Star, Planets & Natural Satellites (Specializing SysmlStdNatural)
    // =========================================================================

    /// The Sun (G2V main-sequence star at the center of the Solar System).
    #[sysml::part]
    #[sysml::specializes(StarBody)]
    #[derive(Debug, Clone, Copy)]
    pub struct Sun {
        #[sysml::attr(redefines = "mass", default = 1.98847e30, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 6.9634e8, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 5772.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "luminosity", default = 3.828e26, unit = "W")]
        pub luminosity: Quantity<f64>,
        #[sysml::port(redefines)]
        pub radiation_out: RadiativePort,
    }

    /// Earth's Moon (Luna), tidally locked rocky natural satellite.
    #[sysml::part]
    #[sysml::specializes(NaturalSatellite)]
    #[derive(Debug, Clone, Copy)]
    pub struct Moon {
        #[sysml::attr(redefines = "mass", default = 7.342e22, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 1.7374e6, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 250.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalRadius", default = 3.844e8, unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalPeriod", default = 2.36059e6, unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(redefines = "bondAlbedo", default = 0.11)]
        pub bond_albedo: f64,
    }

    /// Europa (Jupiter II), icy Galilean moon with a subsurface liquid ocean.
    #[sysml::part]
    #[sysml::specializes(NaturalSatellite)]
    #[derive(Debug, Clone, Copy)]
    pub struct Europa {
        #[sysml::attr(redefines = "mass", default = 4.79984e22, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 1.5608e6, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 102.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalRadius", default = 6.709e8, unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalPeriod", default = 3.06822e5, unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(redefines = "bondAlbedo", default = 0.67)]
        pub bond_albedo: f64,
    }

    /// Mercury (innermost terrestrial planet, 0.387 AU).
    #[sysml::part]
    #[sysml::specializes(RockyPlanet)]
    #[derive(Debug, Clone, Copy)]
    pub struct Mercury {
        #[sysml::attr(redefines = "mass", default = 3.3011e23, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 2.4397e6, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 440.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalRadius", default = 5.7909e10, unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalPeriod", default = 7.60052e6, unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(redefines = "bondAlbedo", default = 0.088)]
        pub bond_albedo: f64,
        #[sysml::port(redefines)]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub thermal_emission: ThermalPort,
    }

    /// Venus (dense CO2-atmosphere rocky planet, 0.723 AU).
    #[sysml::part]
    #[sysml::specializes(RockyPlanet)]
    #[derive(Debug, Clone, Copy)]
    pub struct Venus {
        #[sysml::attr(redefines = "mass", default = 4.8675e24, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 6.0518e6, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 737.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalRadius", default = 1.08208e11, unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalPeriod", default = 1.94142e7, unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(redefines = "bondAlbedo", default = 0.76)]
        pub bond_albedo: f64,
        #[sysml::port(redefines)]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub thermal_emission: ThermalPort,
    }

    /// Earth (terrestrial ocean-bearing planet at 1.000 AU, composing `Moon`).
    #[sysml::part]
    #[sysml::specializes(RockyPlanet)]
    #[sysml::satisfy(req = EarthHabitableSurfaceTempReq, id = "SOL-001")]
    #[sysml::satisfy(req = EarthOrbitalIrradianceReq, id = "SOL-002")]
    #[derive(Debug, Clone, Copy)]
    pub struct Earth {
        #[sysml::attr(redefines = "mass", default = 5.9722e24, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 6.371e6, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 288.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalRadius", default = 1.49598e11, unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalPeriod", default = 3.15581e7, unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(redefines = "bondAlbedo", default = 0.306)]
        pub bond_albedo: f64,
        #[sysml::port(redefines)]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub thermal_emission: ThermalPort,
        #[sysml::part]
        pub moon: Moon,
    }

    /// Mars (cold desert rocky planet, 1.524 AU).
    #[sysml::part]
    #[sysml::specializes(RockyPlanet)]
    #[derive(Debug, Clone, Copy)]
    pub struct Mars {
        #[sysml::attr(redefines = "mass", default = 6.4171e23, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 3.3895e6, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 210.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalRadius", default = 2.27939e11, unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalPeriod", default = 5.93550e7, unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(redefines = "bondAlbedo", default = 0.25)]
        pub bond_albedo: f64,
        #[sysml::port(redefines)]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub thermal_emission: ThermalPort,
    }

    /// Jupiter (largest gas giant at 5.204 AU, composing `Europa`).
    #[sysml::part]
    #[sysml::specializes(GasGiant)]
    #[derive(Debug, Clone, Copy)]
    pub struct Jupiter {
        #[sysml::attr(redefines = "mass", default = 1.89813e27, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 6.9911e7, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 165.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalRadius", default = 7.78479e11, unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalPeriod", default = 3.74356e8, unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(redefines = "bondAlbedo", default = 0.503)]
        pub bond_albedo: f64,
        #[sysml::port(redefines)]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub thermal_emission: ThermalPort,
        #[sysml::part]
        pub europa: Europa,
    }

    /// Saturn (ringed gas giant, 9.583 AU).
    #[sysml::part]
    #[sysml::specializes(GasGiant)]
    #[derive(Debug, Clone, Copy)]
    pub struct Saturn {
        #[sysml::attr(redefines = "mass", default = 5.6834e26, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 5.8232e7, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 134.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalRadius", default = 1.43353e12, unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalPeriod", default = 9.29292e8, unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(redefines = "bondAlbedo", default = 0.342)]
        pub bond_albedo: f64,
        #[sysml::port(redefines)]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub thermal_emission: ThermalPort,
    }

    /// Uranus (oblique ice giant, 19.191 AU).
    #[sysml::part]
    #[sysml::specializes(IceGiant)]
    #[derive(Debug, Clone, Copy)]
    pub struct Uranus {
        #[sysml::attr(redefines = "mass", default = 8.6810e25, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 2.5362e7, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 76.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalRadius", default = 2.87097e12, unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalPeriod", default = 2.65137e9, unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(redefines = "bondAlbedo", default = 0.300)]
        pub bond_albedo: f64,
        #[sysml::port(redefines)]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub thermal_emission: ThermalPort,
    }

    /// Neptune (outermost ice giant, 30.07 AU).
    #[sysml::part]
    #[sysml::specializes(IceGiant)]
    #[derive(Debug, Clone, Copy)]
    pub struct Neptune {
        #[sysml::attr(redefines = "mass", default = 1.02409e26, unit = "kg")]
        pub mass: Quantity<f64>,
        #[sysml::attr(redefines = "radius", default = 2.4622e7, unit = "m")]
        pub radius: Quantity<f64>,
        #[sysml::attr(redefines = "surfaceTemperature", default = 72.0, unit = "K")]
        pub surface_temperature: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalRadius", default = 4.49825e12, unit = "m")]
        pub orbital_radius: Quantity<f64>,
        #[sysml::attr(redefines = "orbitalPeriod", default = 5.20042e9, unit = "s")]
        pub orbital_period: Quantity<f64>,
        #[sysml::attr(redefines = "bondAlbedo", default = 0.290)]
        pub bond_albedo: f64,
        #[sysml::port(redefines)]
        pub radiation_in: Conjugated<RadiativePort>,
        #[sysml::port(redefines)]
        pub thermal_emission: ThermalPort,
    }

    /// Planetary taxonomy variation classifying `PlanetaryBody` into terrestrial rocky,
    /// hydrogen/helium gas giant, and volatile-rich ice giant planet classes.
    #[sysml::variation(of = PlanetaryBody)]
    #[derive(Debug, Clone, Copy)]
    pub enum PlanetClass {
        Rocky(RockyPlanet),
        Gas(GasGiant),
        Ice(IceGiant),
    }

    // =========================================================================
    // 2. Top-Level Solar System Assembly (Radiative & Thermal Energy Web)
    // =========================================================================

    /// Top-level Solar System assembly composing the Sun, all 8 planets, and the `2.725 K`
    /// cosmic microwave background `DeepSpaceSink`, wired with `RadiativeLink` (Sun -> each planet)
    /// and `ThermalLink` (planets -> DeepSpaceSink).
    #[sysml::part]
    #[sysml::satisfy(req = PlanetaryThermalEquilibriumReq, id = "SOL-003")]
    // Solar irradiance links: Sun -> each planet
    #[sysml::connect(interface = RadiativeLink, name = "sunToMercury", from = sun.radiation_out, to = mercury.radiation_in)]
    #[sysml::connect(interface = RadiativeLink, name = "sunToVenus", from = sun.radiation_out, to = venus.radiation_in)]
    #[sysml::connect(interface = RadiativeLink, name = "sunToEarth", from = sun.radiation_out, to = earth.radiation_in)]
    #[sysml::connect(interface = RadiativeLink, name = "sunToMars", from = sun.radiation_out, to = mars.radiation_in)]
    #[sysml::connect(interface = RadiativeLink, name = "sunToJupiter", from = sun.radiation_out, to = jupiter.radiation_in)]
    #[sysml::connect(interface = RadiativeLink, name = "sunToSaturn", from = sun.radiation_out, to = saturn.radiation_in)]
    #[sysml::connect(interface = RadiativeLink, name = "sunToUranus", from = sun.radiation_out, to = uranus.radiation_in)]
    #[sysml::connect(interface = RadiativeLink, name = "sunToNeptune", from = sun.radiation_out, to = neptune.radiation_in)]
    // Planetary infrared thermal emission links: each planet -> DeepSpaceSink
    #[sysml::connect(interface = ThermalLink, name = "mercuryToDeepSpace", from = mercury.thermal_emission, to = deep_space.thermal_sink)]
    #[sysml::connect(interface = ThermalLink, name = "venusToDeepSpace", from = venus.thermal_emission, to = deep_space.thermal_sink)]
    #[sysml::connect(interface = ThermalLink, name = "earthToDeepSpace", from = earth.thermal_emission, to = deep_space.thermal_sink)]
    #[sysml::connect(interface = ThermalLink, name = "marsToDeepSpace", from = mars.thermal_emission, to = deep_space.thermal_sink)]
    #[sysml::connect(interface = ThermalLink, name = "jupiterToDeepSpace", from = jupiter.thermal_emission, to = deep_space.thermal_sink)]
    #[sysml::connect(interface = ThermalLink, name = "saturnToDeepSpace", from = saturn.thermal_emission, to = deep_space.thermal_sink)]
    #[sysml::connect(interface = ThermalLink, name = "uranusToDeepSpace", from = uranus.thermal_emission, to = deep_space.thermal_sink)]
    #[sysml::connect(interface = ThermalLink, name = "neptuneToDeepSpace", from = neptune.thermal_emission, to = deep_space.thermal_sink)]
    #[derive(Debug, Clone, Copy)]
    pub struct SolarSystem {
        #[sysml::part]
        pub sun: Sun,
        #[sysml::part]
        pub mercury: Mercury,
        #[sysml::part]
        pub venus: Venus,
        #[sysml::part]
        pub earth: Earth,
        #[sysml::part]
        pub mars: Mars,
        #[sysml::part]
        pub jupiter: Jupiter,
        #[sysml::part]
        pub saturn: Saturn,
        #[sysml::part]
        pub uranus: Uranus,
        #[sysml::part]
        pub neptune: Neptune,
        #[sysml::part]
        pub deep_space: DeepSpaceSink,
    }

    // =========================================================================
    // 3. Astrophysical & Habitability Requirements
    // =========================================================================

    /// Earth's global mean surface temperature shall remain within liquid-water habitable bounds (250 K .. 330 K).
    #[sysml::requirement(id = "SOL-001")]
    #[sysml::require_constraint(min_surface_temp <= surface_temp && surface_temp <= max_surface_temp)]
    #[derive(Debug, Clone, Copy)]
    pub struct EarthHabitableSurfaceTempReq {
        #[sysml::attr(default = 250.0, unit = "K")]
        pub min_surface_temp: Quantity<f64>,
        #[sysml::attr(default = 288.0, unit = "K")]
        pub surface_temp: Quantity<f64>,
        #[sysml::attr(default = 330.0, unit = "K")]
        pub max_surface_temp: Quantity<f64>,
    }

    /// Total solar irradiance at Earth's 1 AU orbit shall lie within nominal solar constant bounds (1340 W/m^2 .. 1385 W/m^2).
    #[sysml::requirement(id = "SOL-002")]
    #[sysml::require_constraint(min_irradiance <= earth_irradiance && earth_irradiance <= max_irradiance)]
    #[derive(Debug, Clone, Copy)]
    pub struct EarthOrbitalIrradianceReq {
        #[sysml::attr(default = 1340.0)]
        pub min_irradiance: f64,
        #[sysml::attr(default = 1361.2)]
        pub earth_irradiance: f64,
        #[sysml::attr(default = 1385.0)]
        pub max_irradiance: f64,
    }

    /// Global planetary radiative thermal equilibrium: absorbed solar power and outgoing infrared
    /// power into `DeepSpaceSink` (`2.725 K`) shall balance within the allowed thermal residual.
    #[sysml::requirement(id = "SOL-003")]
    #[sysml::require_constraint(
        (absorbed_power_w - emitted_ir_power_w <= max_thermal_imbalance_w)
            && (emitted_ir_power_w - absorbed_power_w <= max_thermal_imbalance_w)
            && (sink_temp_k <= min_planet_temp_k)
    )]
    #[derive(Debug, Clone, Copy)]
    pub struct PlanetaryThermalEquilibriumReq {
        #[sysml::attr(default = 1.204e17, unit = "W")]
        pub absorbed_power_w: Quantity<f64>,
        #[sysml::attr(default = 1.204e17, unit = "W")]
        pub emitted_ir_power_w: Quantity<f64>,
        #[sysml::attr(default = 1.0e15, unit = "W")]
        pub max_thermal_imbalance_w: Quantity<f64>,
        #[sysml::attr(default = 2.725, unit = "K")]
        pub sink_temp_k: Quantity<f64>,
        #[sysml::attr(default = 72.0, unit = "K")]
        pub min_planet_temp_k: Quantity<f64>,
    }
}

/// Build the complete `SolarSystemModel` alongside `sysml-std` foundation packages.
pub fn build_model() -> Model {
    let mut pkgs = sysml_std::std_packages();
    pkgs.push(solar_system_model::sysml_package());
    Model::new(pkgs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use solar_system_model::*;
    use sysml_emit::{audit_model_with_stdlib, emit_model, lint_model};

    #[test]
    fn solar_system_audits_zero_errors_zero_warnings_zero_orphans() {
        let model = build_model();
        let sysml = emit_model(&model);
        let audit = audit_model_with_stdlib("solar-system", &model, None, None, true);
        assert!(audit.stdlib_loaded, "OMG sysml.library must be loaded");
        assert!(
            audit.validation_ok,
            "OMG stdlib validation failed: {audit:#?}\n--- SYSML ---\n{sysml}"
        );
        assert_eq!(
            audit.validation_warnings, 0,
            "OMG stdlib warnings: {audit:#?}\n--- SYSML ---\n{sysml}"
        );
        assert!(
            audit.findings.is_empty(),
            "audit findings not empty: {:#?}\n--- SYSML ---\n{sysml}",
            audit.findings
        );
        assert!(audit.ok);
    }

    #[test]
    fn solar_system_lints_clean() {
        let model = build_model();
        let lint = lint_model("solar-system", &model);
        assert!(
            lint.clean,
            "solar-system lint failed: {:#?}",
            lint.findings
        );
    }

    #[test]
    fn solar_system_physics_calcs_and_requirements_pass_for_earth_and_sun() {
        let sun = Sun::default();
        let earth = Earth::default();
        let deep_space = DeepSpaceSink::default();

        // 1. Solar irradiance at 1 AU (~1361.2 W/m^2) via sysml-std inverse_square_flux_w_m2
        let irradiance = inverse_square_flux_w_m2(sun.luminosity.value, earth.orbital_radius.value);
        assert!(
            (irradiance - 1361.2).abs() < 2.0,
            "unexpected 1 AU irradiance: {irradiance}"
        );

        // 2. Absorbed solar power on Earth (~1.204e17 W) via sysml-std radiative_intercept_power_w
        let absorbed =
            radiative_intercept_power_w(irradiance, earth.radius.value, earth.bond_albedo);
        assert!(
            (absorbed - 1.204e17).abs() < 2.0e15,
            "unexpected Earth absorbed power: {absorbed}"
        );

        // 3. Earth orbital speed (~29,785 m/s) via sysml-std orbital_speed_m_s
        let speed = orbital_speed_m_s(earth.orbital_radius.value, earth.orbital_period.value);
        assert!(
            (speed - 29_785.0).abs() < 50.0,
            "unexpected Earth orbital speed: {speed}"
        );

        // 4. Sun-Earth gravitational force (~3.542e22 N) via sysml-std gravitational_force_n
        let force = gravitational_force_n(
            sun.mass.value,
            earth.mass.value,
            earth.orbital_radius.value,
        );
        assert!(
            (force - 3.542e22).abs() < 1.0e20,
            "unexpected Sun-Earth gravitational force: {force}"
        );

        // 5. Requirements evaluation (SOL-001, SOL-002, SOL-003)
        let req_temp_ok = EarthHabitableSurfaceTempReq {
            surface_temp: earth.surface_temperature,
            ..EarthHabitableSurfaceTempReq::default()
        };
        assert!(req_temp_ok.evaluate());
        let req_temp_cold = EarthHabitableSurfaceTempReq {
            surface_temp: Quantity::new(Mars::default().surface_temperature.value, "K"),
            ..EarthHabitableSurfaceTempReq::default()
        };
        assert!(!req_temp_cold.evaluate());

        let req_irrad_ok = EarthOrbitalIrradianceReq {
            earth_irradiance: irradiance,
            ..EarthOrbitalIrradianceReq::default()
        };
        assert!(req_irrad_ok.evaluate());
        let req_irrad_mars = EarthOrbitalIrradianceReq {
            earth_irradiance: inverse_square_flux_w_m2(
                sun.luminosity.value,
                Mars::default().orbital_radius.value,
            ),
            ..EarthOrbitalIrradianceReq::default()
        };
        assert!(!req_irrad_mars.evaluate());

        let req_eq_ok = PlanetaryThermalEquilibriumReq {
            absorbed_power_w: Quantity::new(absorbed, "W"),
            emitted_ir_power_w: Quantity::new(absorbed, "W"),
            sink_temp_k: deep_space.cmb_temperature,
            min_planet_temp_k: Neptune::default().surface_temperature,
            ..PlanetaryThermalEquilibriumReq::default()
        };
        assert!(req_eq_ok.evaluate());
        let req_eq_imbalanced = PlanetaryThermalEquilibriumReq {
            absorbed_power_w: Quantity::new(absorbed, "W"),
            emitted_ir_power_w: Quantity::new(absorbed * 0.8, "W"),
            ..PlanetaryThermalEquilibriumReq::default()
        };
        assert!(!req_eq_imbalanced.evaluate());
    }
}
