use sysml_dsl::prelude::*;

// BUG: cost has no SI unit: it is `dimensionless` (an amount in the model's currency) or a
// `custom_unit` currency symbol, never a physical unit.
#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct Board {
    #[sysml::attr(budget = "cost", default = 12.0, unit = "kg")]
    pub unit_cost: Quantity<f64>,
}

fn main() {}
