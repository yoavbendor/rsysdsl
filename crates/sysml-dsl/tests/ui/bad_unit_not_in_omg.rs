use sysml_dsl::prelude::*;

// BUG: `kN` is dimensionally fine but is not defined in the OMG SysML v2 `SI` library,
// so the emitted `.sysml` would fail the strict audit with `unresolved reference`.
#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct Engine {
    #[sysml::attr(default = 45.0, unit = "kN")]
    pub thrust: Quantity<f64>,
}

fn main() {}
