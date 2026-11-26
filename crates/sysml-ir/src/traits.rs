//! Compile-time MBSE type-system traits.
//!
//! Proc-macros in `sysml-macros` implement these traits on user structs and
//! enums and generate `const` / dead-code-safe compile-time assertions against
//! them. Broken port polarity, mismatched port types, invalid specialization
//! (`:>`) targets, unknown requirement IDs, or non-action `perform` targets
//! therefore fail at `rustc` compile time.

use std::marker::PhantomData;

use crate::ir::{
    ActionDef, EnumDef, InterfaceDef, ItemDef, PartDef, PortDef, RequirementDef, StateDef,
};

/// Marker and IR reflection trait for SysML `part def` types.
pub trait SysmlPart: 'static {
    const SYSML_NAME: &'static str;
    fn part_def() -> PartDef;
}

/// Marker and IR reflection trait for SysML `port def` types.
pub trait SysmlPort: 'static {
    /// The conjugated counterpart (`~Port` in SysML v2).
    type Conjugate: SysmlPort;
    const SYSML_NAME: &'static str;
    const IS_CONJUGATED: bool = false;
    fn port_def() -> PortDef;
}

/// Conjugated port wrapper representing `~P` in SysML v2.
///
/// Reverses `in`/`out` directionality at the Rust type level so connecting two
/// identically polarized directional ports fails `rustc` trait resolution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Conjugated<P: SysmlPort>(pub PhantomData<P>);

impl<P: SysmlPort> Conjugated<P> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<P: SysmlPort> SysmlPort for Conjugated<P> {
    type Conjugate = P;
    const SYSML_NAME: &'static str = P::SYSML_NAME;
    const IS_CONJUGATED: bool = !P::IS_CONJUGATED;

    fn port_def() -> PortDef {
        P::port_def()
    }
}

/// Compile-time port compatibility relation.
///
/// - Directional ports (`#[sysml::port]`) implement `PortConnectable<Conjugated<P>>`
///   and `Conjugated<P>` implements `PortConnectable<P>`.
/// - Symmetric ports (`#[sysml::port(symmetric)]` or all `inout` items) also
///   implement `SymmetricPort` + `PortConnectable<P>` so identical symmetric ports connect.
pub trait PortConnectable<Other: SysmlPort>: SysmlPort {}

/// Marker trait for symmetric ports whose items are all bidirectional (`inout`).
pub trait SymmetricPort: SysmlPort {}

impl<P: SysmlPort> PortConnectable<P> for Conjugated<P> {}
impl<P: SymmetricPort> PortConnectable<Conjugated<P>> for Conjugated<P> {}

/// Marker and IR reflection trait for SysML `interface def` types.
pub trait SysmlInterface: 'static {
    type EndA: SysmlPort;
    type EndB: SysmlPort;
    const SYSML_NAME: &'static str;
    fn interface_def() -> InterfaceDef;
}

/// Marker and IR reflection trait for SysML `item def` types.
pub trait SysmlItem: 'static {
    const SYSML_NAME: &'static str;
    fn item_def() -> ItemDef;
}

/// Marker and IR reflection trait for SysML `enum def` types.
pub trait SysmlEnum: 'static {
    const SYSML_NAME: &'static str;
    fn enum_def() -> EnumDef;
}

/// Marker and IR reflection trait for SysML `action def` types.
pub trait SysmlAction: 'static {
    const SYSML_NAME: &'static str;
    fn action_def() -> ActionDef;
}

/// Marker and IR reflection trait for SysML `state def` types.
pub trait SysmlState: 'static {
    const SYSML_NAME: &'static str;
    fn state_def() -> StateDef;
}

/// Marker and IR reflection trait for SysML `requirement def` types.
pub trait SysmlRequirement: 'static {
    /// Short requirement identifier (e.g. `"HLR-R002"`, `"CLR-R050"`).
    const REQ_ID: &'static str;
    const SYSML_NAME: &'static str;
    fn requirement_def() -> RequirementDef;
}

/// Compile-time specialization witness (`Sub :> Base`).
pub trait Specializes<Base: ?Sized> {}

/// Compile-time functional allocation witness (`Part` performs `Action`).
pub trait Performs<A: SysmlAction> {}

/// Compile-time requirement satisfaction witness (`Element` satisfies `Req`).
pub trait Satisfies<R: SysmlRequirement> {}

/// Compile-time requirement verification witness (`Case` verifies `Req`).
pub trait Verifies<R: SysmlRequirement> {}

/// Compile-time assertion helpers invoked from macro-generated check blocks.
pub fn assert_part<T: SysmlPart>() {}
pub fn assert_port<T: SysmlPort>() {}
pub fn assert_item<T: SysmlItem>() {}
pub fn assert_action<T: SysmlAction>() {}
pub fn assert_state<T: SysmlState>() {}
pub fn assert_requirement<T: SysmlRequirement>() {}
pub fn assert_specializes<Sub: Specializes<Base>, Base>() {}

/// Compile-time check that two port fields are compatible (`A: PortConnectable<B>`).
pub fn assert_ports_connect<A, B>(_a: &A, _b: &B)
where
    A: SysmlPort + PortConnectable<B>,
    B: SysmlPort,
{
}

/// Compile-time check that two port fields match an interface contract `I`
/// and satisfy `PortConnectable`.
pub fn assert_interface_connect<I>(_a: &I::EndA, _b: &I::EndB)
where
    I: SysmlInterface,
    I::EndA: PortConnectable<I::EndB>,
{
}

/// Compile-time check that an interface's two end port types are compatible.
pub fn assert_interface_ends<A, B>()
where
    A: SysmlPort + PortConnectable<B>,
    B: SysmlPort,
{
}

/// `const`-evaluable string equality check used by `#[sysml::satisfy(..., id = "...")]`
/// to reject mismatched requirement IDs at compile time.
pub const fn const_str_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
