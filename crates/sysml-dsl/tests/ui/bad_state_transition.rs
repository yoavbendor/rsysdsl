use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Default)]
pub struct TriggerEvent;

// BUG: Transition target `NonExistentPhase` is not a variant of this state machine enum!
#[sysml::state]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokenMissionMachine {
    #[sysml::initial]
    #[sysml::transition(on = TriggerEvent, to = NonExistentPhase)]
    Launch,
    LunarOrbit,
}

fn main() {}
