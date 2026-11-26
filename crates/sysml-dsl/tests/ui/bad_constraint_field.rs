use sysml_dsl::prelude::*;

// BUG: Constraint references `max_veloctiy` (typo!) which is not a field on the requirement struct!
#[sysml::requirement(id = "REQ-001")]
#[sysml::require_constraint(actual_velocity <= max_veloctiy)]
#[derive(Debug, Clone)]
pub struct VelocityReq {
    #[sysml::attr(unit = "m/s")]
    pub actual_velocity: Quantity<f64>,
    #[sysml::attr(value = "2.0", unit = "m/s")]
    pub max_velocity: Quantity<f64>,
}

fn main() {}
