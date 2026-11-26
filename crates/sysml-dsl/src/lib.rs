//! Ergonomic `#![no_std]`-capable Rust DSL facade for authoring strongly typed SysML v2 models.
//!
//! - On host builds (default feature `reflect`): re-exports `sysml-core`, `sysml-ir`, and `sysml-macros`.
//! - On bare-metal / Ferrocene `#![no_std]` targets (`default-features = false`): depends ONLY on
//!   zero-alloc `sysml-core` + `sysml-macros` while enforcing all compile-time MBSE & ISO 26262 checks.

#![no_std]

pub use sysml_core as core_contracts;
#[cfg(not(feature = "reflect"))]
pub use sysml_core::*;
#[cfg(feature = "reflect")]
pub use sysml_ir as ir;
#[cfg(feature = "reflect")]
pub use sysml_ir::*;
pub use sysml_macros as sysml;

pub mod prelude {
    pub use sysml_core::{
        Asil, AsilA, AsilB, AsilC, AsilD, AsilDecomposable, AsilLevel, Conjugated,
        EvaluableRequirement, IndependentFrom, NoStdTransitionError, Performs, PortConnectable,
        Qm, Quantity, SafetyGoal, Satisfies, Specializes, SymmetricPort, SysmlActionCore,
        SysmlCalcCore, SysmlEnumCore, SysmlInterfaceCore, SysmlItemCore, SysmlPartCore,
        SysmlPortCore, SysmlRequirementCore, SysmlStateCore, ToScalar, Verifies,
    };
    #[cfg(feature = "reflect")]
    pub use sysml_ir::{
        Model, Package, SysmlAction, SysmlCalc, SysmlEnum, SysmlInterface, SysmlItem, SysmlPart,
        SysmlPort, SysmlRequirement, SysmlState, TransitionError, VerificationReport,
        VerificationStatus, VerificationVerdict,
    };
    pub use sysml_macros as sysml;
}
