use sysml_dsl::prelude::*;

// BUG: actions cannot carry a TBD marker; it must not be dropped silently.
#[sysml::action]
#[sysml::tbd(reason = "steps unknown")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Measure {}

fn main() {}
