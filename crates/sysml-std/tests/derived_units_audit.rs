//! Derived SI units written in natural ASCII (`m^3/s`, `m/s^2`, `N*m`, `kg/m^3`, `W/K`,
//! `J/(kg*K)`, `1/h`, `°C`) must compile, emit as OMG-resolvable unit expressions and pass a
//! strict (0 errors / 0 warnings) audit against the OMG SysML v2 Standard Library.

use sysml_dsl::prelude::*;
use sysml_emit::{audit_model_with_stdlib, emit_model};

#[sysml::package(name = "DerivedUnitsRoundTrip")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod derived_units {
    use super::*;

    /// Pump with attributes in derived SI units.
    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Pump {
        #[sysml::attr(default = 0.002, unit = "m^3/s")]
        pub volume_flow: Quantity<f64>,
        #[sysml::attr(default = 9.81, unit = "m/s^2")]
        pub accel: Quantity<f64>,
        #[sysml::attr(default = 1.5, unit = "N*m")]
        pub torque: Quantity<f64>,
        #[sysml::attr(default = 998.0, unit = "kg/m^3")]
        pub density: Quantity<f64>,
        #[sysml::attr(default = 4.2, unit = "W/K")]
        pub conductance: Quantity<f64>,
        #[sysml::attr(default = 4186.0, unit = "J/(kg*K)")]
        pub specific_heat: Quantity<f64>,
        #[sysml::attr(default = 0.5, unit = "1/h")]
        pub cycle_rate: Quantity<f64>,
        #[sysml::attr(default = 25.0, unit = "°C")]
        pub ambient: Quantity<f64>,
    }
}

#[test]
fn derived_units_round_trip_to_strict_zero_warning_audit() {
    let model = Model::new(vec![derived_units::sysml_package()]);
    let sysml = emit_model(&model);
    for frag in [
        "[m^3/s]",
        "[m/s^2]",
        "[N*m]",
        "[kg/m^3]",
        "[W/K]",
        "[J/(kg*K)]",
        "[1/h]",
        "['°C']",
    ] {
        assert!(sysml.contains(frag), "missing `{frag}` in:\n{sysml}");
    }
    let audit = audit_model_with_stdlib("derived_units", &model, None, None, true);
    assert!(audit.stdlib_loaded, "OMG sysml.library must be loaded");
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "derived-unit audit failed: {audit:#?}\n--- SYSML ---\n{sysml}"
    );
}
