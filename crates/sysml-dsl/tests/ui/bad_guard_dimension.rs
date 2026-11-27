use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Default)]
pub struct OvenReading {
    #[sysml::attr(unit = "K")]
    pub cavity_temp: Quantity<f64>,
}

#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct Oven {
    #[sysml::attr(default = 900.0, unit = "W")]
    pub max_rf_power: Quantity<f64>,
}

// BUG: Guard compares the event's temperature [K] with the subject's power limit [W]!
#[sysml::state(subject = Oven)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OvenMode {
    #[sysml::initial]
    #[sysml::transition(
        on = OvenReading,
        guard = event.cavity_temp > subject.max_rf_power,
        to = Tripped
    )]
    Heating,
    Tripped,
}

fn main() {}
