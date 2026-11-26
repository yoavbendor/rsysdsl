use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Default)]
pub struct Power;

#[sysml::port]
#[derive(Default)]
pub struct PortA {
    #[sysml::out]
    pub power: Power,
}

#[sysml::port]
#[derive(Default)]
pub struct PortB {
    #[sysml::out]
    pub power: Power,
}

#[sysml::part]
#[derive(Default)]
pub struct CompA {
    #[sysml::port]
    pub p: PortA,
}

#[sysml::part]
#[derive(Default)]
pub struct CompB {
    #[sysml::port]
    pub p: Conjugated<PortB>,
}

// BUG: PortA cannot connect to Conjugated<PortB>!
#[sysml::part]
#[sysml::connect(from = a.p, to = b.p)]
#[derive(Default)]
pub struct MismatchedAssembly {
    #[sysml::part]
    pub a: CompA,
    #[sysml::part]
    pub b: CompB,
}

fn main() {}
