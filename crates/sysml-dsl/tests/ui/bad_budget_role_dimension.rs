use sysml_dsl::prelude::*;

// BUG: a power budget role on a mass-dimension attribute would put kilograms into the watt sum.
#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct Heater {
    #[sysml::attr(budget = "power", default = 1.5, unit = "kg")]
    pub rated_power: Quantity<f64>,
}

fn main() {}
