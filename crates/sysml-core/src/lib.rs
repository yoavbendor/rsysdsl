//! Zero-allocation `#![no_std]` SysML v2 compile-time contracts & ISO 26262 safety witnesses.
//!
//! Designed for safety-critical embedded targets (including Ferrocene bare-metal builds):
//! carries zero `std` or `alloc` dependencies while enforcing port polarity, specialization
//! chains, requirement IDs, modeled calc purity, and ISO 26262 ASIL decomposition at `rustc`
//! compile time.

#![no_std]

pub mod quantity;
pub mod safety;
pub mod traits;
pub mod units;

pub use quantity::Quantity;
pub use safety::{
    assert_asil_decomposition, assert_independent, assert_safety_goal, Asil, AsilA, AsilB, AsilC,
    AsilD, AsilDecomposable, AsilLevel, IndependentFrom, Qm, SafetyGoal,
};
pub use traits::{
    assert_action_core, assert_calc_core, assert_interface_connect_core, assert_interface_core,
    assert_interface_ends_core, assert_item_core, assert_part_core, assert_port_core,
    assert_ports_connect_core, assert_requirement_core, assert_specializes, assert_state_core,
    const_str_eq, Conjugated, EvaluableRequirement, NoStdTransitionError, Performs,
    PortConnectable, Satisfies, Specializes, SymmetricPort, SysmlActionCore, SysmlCalcCore,
    SysmlEnumCore, SysmlInterfaceCore, SysmlItemCore, SysmlPartCore, SysmlPortCore,
    SysmlRequirementCore, SysmlStateCore, ToScalar, Verifies,
};
