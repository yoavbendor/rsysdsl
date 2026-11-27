use sysml_dsl::prelude::*;

// BUG: every escape hatch costs a written reason; an empty `tbd_unit` is rejected.
#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct Heater {
    #[sysml::attr(tbd_unit = "")]
    pub rated_power: f64,
}

fn main() {}
