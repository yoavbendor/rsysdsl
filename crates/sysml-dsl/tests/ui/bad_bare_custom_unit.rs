use sysml_dsl::prelude::*;

// BUG: `custom_unit` opts out of the SI table and must say why.
#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct Pump {
    #[sysml::attr(default = 3000.0, unit = "rpm", custom_unit)]
    pub speed: f64,
}

fn main() {}
