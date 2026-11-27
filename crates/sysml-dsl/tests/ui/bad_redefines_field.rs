use sysml_dsl::prelude::*;

#[sysml::part(abstract)]
pub struct Base {
    #[sysml::attr(default = 10.0, unit = "W")]
    pub power: Quantity<f64>,
}

// BUG: Base has no field `nonexistent_field` (`nonexistentField` in SysML)!
#[sysml::part]
#[sysml::specializes(Base)]
pub struct Child {
    #[sysml::attr(redefines = "nonexistentField", default = 5.0, unit = "W")]
    pub nonexistent_field: Quantity<f64>,
}

fn main() {}
