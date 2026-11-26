//! ISO 26262-9 compile-time ASIL types, safety goals, decomposition rules,
//! and freedom-from-interference independence witnesses (`#![no_std]`).
//!
//! Enforces ISO 26262-9 Clause 5 ASIL decomposition strictly at `rustc` compile time:
//! - `AsilD` decomposes into `(AsilB, AsilB)`, `(AsilC, AsilA)`, `(AsilA, AsilC)`, `(AsilD, Qm)`,
//!   or any stronger combination (`left_rank + right_rank >= parent_rank`).
//! - Insufficient decompositions (e.g. `AsilD -> (AsilB, AsilA)`) fail `rustc` trait resolution.
//! - Decomposed architectural elements must additionally witness mutual independence via
//!   `IndependentFrom<Other>` (declared with `#[sysml::independent(...)]`).

use crate::traits::{SysmlPartCore, SysmlRequirementCore};

/// Runtime enum representing ISO 26262 Automotive Safety Integrity Levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Asil {
    Qm = 0,
    A = 1,
    B = 2,
    C = 3,
    D = 4,
}

impl Asil {
    pub const fn as_str(self) -> &'static str {
        match self {
            Asil::Qm => "QM",
            Asil::A => "ASIL_A",
            Asil::B => "ASIL_B",
            Asil::C => "ASIL_C",
            Asil::D => "ASIL_D",
        }
    }
}

/// Type-level ASIL markers for compile-time ISO 26262 checking.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Qm;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AsilA;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AsilB;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AsilC;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AsilD;

/// Trait implemented by the 5 ISO 26262 ASIL marker types.
pub trait AsilLevel: 'static {
    const ASIL: Asil;
    const RANK: u8;
    const LABEL: &'static str;
}

impl AsilLevel for Qm {
    const ASIL: Asil = Asil::Qm;
    const RANK: u8 = 0;
    const LABEL: &'static str = "QM";
}
impl AsilLevel for AsilA {
    const ASIL: Asil = Asil::A;
    const RANK: u8 = 1;
    const LABEL: &'static str = "ASIL_A";
}
impl AsilLevel for AsilB {
    const ASIL: Asil = Asil::B;
    const RANK: u8 = 2;
    const LABEL: &'static str = "ASIL_B";
}
impl AsilLevel for AsilC {
    const ASIL: Asil = Asil::C;
    const RANK: u8 = 3;
    const LABEL: &'static str = "ASIL_C";
}
impl AsilLevel for AsilD {
    const ASIL: Asil = Asil::D;
    const RANK: u8 = 4;
    const LABEL: &'static str = "ASIL_D";
}

/// Trait implemented on requirements annotated with `#[sysml::safety_goal(...)]`.
pub trait SafetyGoal: SysmlRequirementCore {
    type Asil: AsilLevel;
    const SAFE_STATE: &'static str;
    const FTTI_MS: u32;
}

/// Witness trait that two architectural parts satisfy ISO 26262-9 freedom from interference /
/// sufficient independence for ASIL decomposition.
pub trait IndependentFrom<Other: SysmlPartCore>: SysmlPartCore {}

/// Compile-time ISO 26262-9 Table 1 ASIL decomposition witness:
/// `Self` (parent ASIL) can be legally decomposed into `(Left, Right)`.
pub trait AsilDecomposable<Left: AsilLevel, Right: AsilLevel>: AsilLevel {}

// QM (rank 0): any pair
impl<L: AsilLevel, R: AsilLevel> AsilDecomposable<L, R> for Qm {}

// ASIL A (rank 1): sum >= 1
impl AsilDecomposable<AsilA, Qm> for AsilA {}
impl AsilDecomposable<Qm, AsilA> for AsilA {}
impl AsilDecomposable<AsilA, AsilA> for AsilA {}
impl AsilDecomposable<AsilB, Qm> for AsilA {}
impl AsilDecomposable<Qm, AsilB> for AsilA {}
impl AsilDecomposable<AsilB, AsilA> for AsilA {}
impl AsilDecomposable<AsilA, AsilB> for AsilA {}
impl AsilDecomposable<AsilB, AsilB> for AsilA {}
impl AsilDecomposable<AsilC, Qm> for AsilA {}
impl AsilDecomposable<Qm, AsilC> for AsilA {}
impl AsilDecomposable<AsilD, Qm> for AsilA {}
impl AsilDecomposable<Qm, AsilD> for AsilA {}

// ASIL B (rank 2): sum >= 2 -> (A,A), (B,QM), (QM,B), and stronger
impl AsilDecomposable<AsilA, AsilA> for AsilB {}
impl AsilDecomposable<AsilB, Qm> for AsilB {}
impl AsilDecomposable<Qm, AsilB> for AsilB {}
impl AsilDecomposable<AsilB, AsilA> for AsilB {}
impl AsilDecomposable<AsilA, AsilB> for AsilB {}
impl AsilDecomposable<AsilB, AsilB> for AsilB {}
impl AsilDecomposable<AsilC, Qm> for AsilB {}
impl AsilDecomposable<Qm, AsilC> for AsilB {}
impl AsilDecomposable<AsilC, AsilA> for AsilB {}
impl AsilDecomposable<AsilA, AsilC> for AsilB {}
impl AsilDecomposable<AsilD, Qm> for AsilB {}
impl AsilDecomposable<Qm, AsilD> for AsilB {}

// ASIL C (rank 3): sum >= 3 -> (B,A), (A,B), (C,QM), (QM,C), and stronger
impl AsilDecomposable<AsilB, AsilA> for AsilC {}
impl AsilDecomposable<AsilA, AsilB> for AsilC {}
impl AsilDecomposable<AsilB, AsilB> for AsilC {}
impl AsilDecomposable<AsilC, Qm> for AsilC {}
impl AsilDecomposable<Qm, AsilC> for AsilC {}
impl AsilDecomposable<AsilC, AsilA> for AsilC {}
impl AsilDecomposable<AsilA, AsilC> for AsilC {}
impl AsilDecomposable<AsilC, AsilB> for AsilC {}
impl AsilDecomposable<AsilB, AsilC> for AsilC {}
impl AsilDecomposable<AsilC, AsilC> for AsilC {}
impl AsilDecomposable<AsilD, Qm> for AsilC {}
impl AsilDecomposable<Qm, AsilD> for AsilC {}

// ASIL D (rank 4): sum >= 4 -> (B,B), (C,A), (A,C), (D,QM), (QM,D), and stronger
impl AsilDecomposable<AsilB, AsilB> for AsilD {}
impl AsilDecomposable<AsilC, AsilA> for AsilD {}
impl AsilDecomposable<AsilA, AsilC> for AsilD {}
impl AsilDecomposable<AsilC, AsilB> for AsilD {}
impl AsilDecomposable<AsilB, AsilC> for AsilD {}
impl AsilDecomposable<AsilC, AsilC> for AsilD {}
impl AsilDecomposable<AsilD, Qm> for AsilD {}
impl AsilDecomposable<Qm, AsilD> for AsilD {}
impl AsilDecomposable<AsilD, AsilA> for AsilD {}
impl AsilDecomposable<AsilA, AsilD> for AsilD {}
impl AsilDecomposable<AsilD, AsilB> for AsilD {}
impl AsilDecomposable<AsilB, AsilD> for AsilD {}
impl AsilDecomposable<AsilD, AsilC> for AsilD {}
impl AsilDecomposable<AsilC, AsilD> for AsilD {}
impl AsilDecomposable<AsilD, AsilD> for AsilD {}

/// Compile-time check that `G` is a valid ISO 26262 safety goal.
pub const fn assert_safety_goal<G: SafetyGoal>() {}

/// Compile-time check that parts `A` and `B` are witnessed mutually independent.
pub const fn assert_independent<A: IndependentFrom<B>, B: SysmlPartCore>() {}

/// Compile-time check that decomposing safety goal `G` into `(LeftPart: LeftAsil, RightPart: RightAsil)`
/// satisfies BOTH ISO 26262-9 Table 1 ASIL decomposition AND freedom-from-interference independence.
pub const fn assert_asil_decomposition<G, LeftPart, LeftAsil, RightPart, RightAsil>()
where
    G: SafetyGoal,
    LeftPart: SysmlPartCore + IndependentFrom<RightPart>,
    RightPart: SysmlPartCore,
    LeftAsil: AsilLevel,
    RightAsil: AsilLevel,
    G::Asil: AsilDecomposable<LeftAsil, RightAsil>,
{
}
