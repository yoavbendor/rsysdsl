use sysml_dsl::prelude::*;

#[sysml::package(name = "LeakyPackage")]
pub mod leaky_package {
    use sysml_dsl::prelude::*;

    #[sysml::part]
    #[derive(Default)]
    pub struct Thruster;

    // BUG: An AI agent trying to bury an unmodeled safety check inside #[sysml::package]!
    pub fn hidden_safety_override(pressure: f64) -> bool {
        if pressure > 42.0 {
            return false;
        }
        true
    }
}

fn main() {}
