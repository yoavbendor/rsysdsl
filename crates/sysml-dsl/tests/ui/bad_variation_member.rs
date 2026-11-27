use sysml_dsl::prelude::*;

#[sysml::part(abstract)]
#[derive(Default)]
pub struct BatteryBase {}

#[sysml::part]
#[derive(Default)]
pub struct UnrelatedPart {}

// BUG: UnrelatedPart does not specialize BatteryBase!
#[sysml::variation(of = BatteryBase)]
pub enum BadVar {
    Wrong(UnrelatedPart),
}

fn main() {}
