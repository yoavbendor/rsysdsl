use sysml_dsl::prelude::*;

#[sysml::interface(abstract)]
pub struct EnergyChannel {}

// BUG: a concrete interface must declare two `end` ports; the abstract base has none to inherit.
#[sysml::interface]
#[sysml::specializes(EnergyChannel)]
pub struct HeatLink {}

fn main() {}
