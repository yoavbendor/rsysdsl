use sysml_dsl::prelude::*;

// BUG: every TBD marker needs a written reason.
#[sysml::part]
#[sysml::tbd(owner = "systems")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Sensor {}

fn main() {}
