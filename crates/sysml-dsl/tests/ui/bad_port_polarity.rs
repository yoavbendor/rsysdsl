use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Default)]
pub struct CommandSignal;

#[sysml::port]
#[derive(Default)]
pub struct StagingPort {
    #[sysml::out]
    pub cmd: CommandSignal,
}

#[sysml::part]
#[derive(Default)]
pub struct StageA {
    #[sysml::port]
    pub p: StagingPort,
}

#[sysml::part]
#[derive(Default)]
pub struct StageB {
    // BUG: directional port is NOT conjugated (`Conjugated<StagingPort>`), so out -> out!
    #[sysml::port]
    pub p: StagingPort,
}

#[sysml::part]
#[sysml::connect(from = a.p, to = b.p)]
#[derive(Default)]
pub struct BrokenStack {
    #[sysml::part]
    pub a: StageA,
    #[sysml::part]
    pub b: StageB,
}

fn main() {}
