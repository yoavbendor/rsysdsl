//! Compile-time MBSE type-system traits extending `#![no_std]` `sysml-core` contracts
//! with host-side IR reflection (`part_def()`, `port_def()`, etc.).

use crate::ir::{
    ActionDef, CalcDef, EnumDef, InterfaceDef, ItemDef, PartDef, PortDef, RequirementDef, StateDef,
};

pub use sysml_core::{
    assert_action_core, assert_asil_decomposition, assert_calc_core, assert_independent,
    assert_interface_connect_core, assert_interface_ends_core, assert_item_core, assert_part_core,
    assert_port_core, assert_ports_connect_core, assert_requirement_core, assert_safety_goal,
    assert_specializes, assert_state_core, const_str_eq, Asil, AsilA, AsilB, AsilC, AsilD,
    AsilDecomposable, AsilLevel, Conjugated, EvaluableRequirement, IndependentFrom,
    NoStdTransitionError, Performs, PortConnectable, Qm, SafetyGoal, Satisfies, Specializes,
    SymmetricPort, SysmlActionCore, SysmlCalcCore, SysmlEnumCore, SysmlInterfaceCore,
    SysmlItemCore, SysmlPartCore, SysmlPortCore, SysmlRequirementCore, SysmlStateCore, ToScalar,
    Verifies,
};

/// IR reflection trait for SysML `part def` types (extends `#![no_std]` `SysmlPartCore`).
pub trait SysmlPart: SysmlPartCore {
    fn part_def() -> PartDef;
}

/// IR reflection trait for SysML `port def` types (extends `#![no_std]` `SysmlPortCore`).
pub trait SysmlPort: SysmlPortCore {
    fn port_def() -> PortDef;
}

impl<P: SysmlPort> SysmlPort for Conjugated<P> {
    fn port_def() -> PortDef {
        P::port_def()
    }
}

/// IR reflection trait for SysML `interface def` types.
pub trait SysmlInterface: SysmlInterfaceCore {
    fn interface_def() -> InterfaceDef;
}

/// IR reflection trait for SysML `item def` types.
pub trait SysmlItem: SysmlItemCore {
    fn item_def() -> ItemDef;
}

/// IR reflection trait for SysML `enum def` types.
pub trait SysmlEnum: SysmlEnumCore {
    fn enum_def() -> EnumDef;
}

/// IR reflection trait for SysML `action def` types.
pub trait SysmlAction: SysmlActionCore {
    fn action_def() -> ActionDef;
}

/// IR reflection trait for SysML `state def` types.
pub trait SysmlState: SysmlStateCore {
    fn state_def() -> StateDef;
}

/// IR reflection trait for SysML `requirement def` types.
pub trait SysmlRequirement: SysmlRequirementCore {
    fn requirement_def() -> RequirementDef;
}

/// IR reflection trait for modeled `#[sysml::calc]` functions and structs.
pub trait SysmlCalc: SysmlCalcCore {
    fn calc_def() -> CalcDef;
}

/// Compile-time assertion helpers (delegate to `#![no_std]` core traits).
pub fn assert_part<T: SysmlPartCore>() {}
pub fn assert_port<T: SysmlPortCore>() {}
pub fn assert_item<T: SysmlItemCore>() {}
pub fn assert_action<T: SysmlActionCore>() {}
pub fn assert_state<T: SysmlStateCore>() {}
pub fn assert_requirement<T: SysmlRequirementCore>() {}
pub fn assert_calc<T: SysmlCalcCore>() {}

/// Compile-time check that two port fields are compatible (`A: PortConnectable<B>`).
pub fn assert_ports_connect<A, B>(_a: &A, _b: &B)
where
    A: SysmlPortCore + PortConnectable<B>,
    B: SysmlPortCore,
{
}

/// Compile-time check that two port fields match an interface contract `I`
/// and satisfy `PortConnectable`.
pub fn assert_interface_connect<I>(_a: &I::EndA, _b: &I::EndB)
where
    I: SysmlInterfaceCore,
    I::EndA: PortConnectable<I::EndB>,
{
}

/// Compile-time check that an interface's two end port types are compatible.
pub fn assert_interface_ends<A, B>()
where
    A: SysmlPortCore + PortConnectable<B>,
    B: SysmlPortCore,
{
}
