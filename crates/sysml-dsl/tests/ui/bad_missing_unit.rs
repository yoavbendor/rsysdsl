use sysml_dsl::prelude::*;

// BUG: a numeric attribute must declare `unit`, `dimensionless`, `tbd_unit` or `custom_unit`;
// omitting all of them used to skip the dimension checks silently.
#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct Heater {
    #[sysml::attr(default = 1500.0)]
    pub rated_power: f64,
}

fn main() {}
