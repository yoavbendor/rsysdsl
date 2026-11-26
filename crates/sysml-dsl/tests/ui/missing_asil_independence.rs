use sysml_dsl::prelude::*;

#[sysml::requirement(id = "SG-IND")]
#[sysml::safety_goal(asil = AsilD, safe_state = "SafeStop", ftti_ms = 50)]
pub struct TopSafetyGoal;

// Missing #[sysml::independent(ChannelB)]!
#[sysml::part]
#[derive(Default)]
pub struct ChannelA;

#[sysml::part]
#[derive(Default)]
pub struct ChannelB;

// MUST FAIL: Even though ASIL B + ASIL B satisfies rank sum >= 4,
// ISO 26262-9 requires freedom-from-interference independence between ChannelA and ChannelB.
#[sysml::part]
#[sysml::asil_decompose(
    goal = TopSafetyGoal,
    left = ChannelA,
    left_asil = AsilB,
    right = ChannelB,
    right_asil = AsilB
)]
pub struct NonIndependentDecompositionAssembly {
    #[sysml::part]
    pub a: ChannelA,
    #[sysml::part]
    pub b: ChannelB,
}

fn main() {}
