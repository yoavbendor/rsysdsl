use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Default)]
pub struct Signal;

#[sysml::port]
#[derive(Default)]
pub struct StagingPort {
    #[sysml::out]
    pub cmd: Signal,
}

#[sysml::interface]
#[derive(Default)]
pub struct StagingInterface {
    #[sysml::end]
    pub lower: StagingPort,
    #[sysml::end]
    pub upper: Conjugated<StagingPort>,
}

#[sysml::action]
#[derive(Default)]
pub struct ExecuteBurn;

#[sysml::requirement(id = "HLR-R002")]
#[derive(Default)]
pub struct SoftLandingReq;

#[sysml::part(abstract)]
#[derive(Default)]
pub struct HardwareComponent;

#[sysml::part]
#[sysml::specializes(HardwareComponent)]
#[derive(Default)]
pub struct DescentStage {
    #[sysml::port]
    pub upper_port: StagingPort,
}

#[sysml::part]
#[sysml::specializes(HardwareComponent)]
#[derive(Default)]
pub struct AscentStage {
    #[sysml::port]
    pub lower_port: Conjugated<StagingPort>,
}

#[sysml::part]
#[sysml::specializes(HardwareComponent)]
#[sysml::perform(burn: ExecuteBurn)]
#[sysml::satisfy(req = SoftLandingReq, id = "HLR-R002")]
#[sysml::connect(interface = StagingInterface, name = "interstage", from = ds.upper_port, to = as_.lower_port)]
#[derive(Default)]
pub struct LunarModule {
    #[sysml::part]
    pub ds: DescentStage,
    #[sysml::part]
    pub as_: AscentStage,
}

#[sysml::state]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseMachine {
    #[sysml::initial]
    #[sysml::transition(on = Signal, to = Done)]
    Ready,
    Done,
}

fn main() {
    let mut sm = PhaseMachine::initial_state();
    assert_eq!(sm.step_on::<Signal>().unwrap(), PhaseMachine::Done);
}
