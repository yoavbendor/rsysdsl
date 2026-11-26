use sysml_dsl::prelude::*;

#[sysml::requirement(id = "HLR-R002")]
#[derive(Default)]
pub struct LunarLanderSoftLandingRequirement;

// BUG: Wrong requirement ID string ("HLR-9999" != "HLR-R002") must fail at compile time!
#[sysml::part]
#[sysml::satisfy(req = LunarLanderSoftLandingRequirement, id = "HLR-9999")]
#[derive(Default)]
pub struct LunarModule;

fn main() {}
