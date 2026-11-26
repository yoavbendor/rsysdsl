use sysml_dsl::prelude::*;

#[sysml::requirement(id = "SG-BAD")]
#[sysml::safety_goal(asil = AsilD, safe_state = "SafeStop", ftti_ms = 50)]
pub struct TopSafetyGoal;

#[sysml::part]
#[sysml::independent(ChannelB)]
#[derive(Default)]
pub struct ChannelA;

#[sysml::part]
#[sysml::independent(ChannelA)]
#[derive(Default)]
pub struct ChannelB;

// MUST FAIL: ISO 26262-9 forbids decomposing ASIL D into ASIL B + ASIL A (rank 2 + 1 < 4).
#[sysml::part]
#[sysml::asil_decompose(
    goal = TopSafetyGoal,
    left = ChannelA,
    left_asil = AsilB,
    right = ChannelB,
    right_asil = AsilA
)]
pub struct IllegalDecompositionAssembly {
    #[sysml::part]
    pub a: ChannelA,
    #[sysml::part]
    pub b: ChannelB,
}

fn main() {}
