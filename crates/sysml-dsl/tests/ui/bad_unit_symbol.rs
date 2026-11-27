use sysml_dsl::prelude::*;

// BUG: `kgs` is not a unit symbol in the closed SI table (typo for `kg` / `kg/s`)!
#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct FuelTank {
    #[sysml::attr(default = 120.0, unit = "kgs")]
    pub fuel_mass: Quantity<f64>,
}

fn main() {}
