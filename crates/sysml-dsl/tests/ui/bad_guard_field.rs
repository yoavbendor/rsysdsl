use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Default)]
pub struct TouchdownEvent {
    #[sysml::attr(unit = "m/s")]
    pub vertical_velocity: Quantity<f64>,
}

// BUG: Guard references `event.nonexistent_metric` which does not exist on `TouchdownEvent`!
#[sysml::state]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandingMachine {
    #[sysml::initial]
    #[sysml::transition(
        on = TouchdownEvent,
        guard = event.nonexistent_metric <= 2.0,
        to = Landed
    )]
    Descending,
    Landed,
}

fn main() {}
