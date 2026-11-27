use sysml_dsl::prelude::*;

// BUG: Compares a temperature [K] against a power [W] — dimensionally meaningless!
#[sysml::requirement(id = "REQ-THERM-001")]
#[sysml::require_constraint(temp_k <= power_w)]
#[derive(Debug, Clone)]
pub struct ThermalLimitReq {
    #[sysml::attr(default = 350.0, unit = "K")]
    pub temp_k: Quantity<f64>,
    #[sysml::attr(default = 250.0, unit = "W")]
    pub power_w: Quantity<f64>,
}

fn main() {}
