use sysml_dsl::prelude::*;

// BUG: `resolve_by` must be one of concept, preliminary, detailed, release.
#[sysml::part]
#[sysml::tbd(reason = "supplier not selected", resolve_by = "someday")]
#[derive(Debug, Clone, Copy, Default)]
pub struct Sensor {}

fn main() {}
