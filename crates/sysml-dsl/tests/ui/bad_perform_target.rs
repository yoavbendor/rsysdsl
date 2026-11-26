use sysml_dsl::prelude::*;

#[sysml::part]
#[derive(Default)]
pub struct NotAnAction;

// BUG: `perform` target must implement `SysmlAction`, not `SysmlPart`!
#[sysml::part]
#[sysml::perform(do_something: NotAnAction)]
#[derive(Default)]
pub struct Spacecraft;

fn main() {}
