use sysml_dsl::prelude::*;

// BUG: `tbd_value` says the value is not in the source, so it cannot also carry a default.
#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct Heater {
    #[sysml::attr(default = 1500.0, unit = "W", tbd_value = "datasheet pending")]
    pub rated_power: Quantity<f64>,
}

fn main() {}
