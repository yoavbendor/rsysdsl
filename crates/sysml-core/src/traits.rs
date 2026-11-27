//! `#![no_std]` zero-allocation compile-time MBSE contract traits.

use core::fmt;
use core::marker::PhantomData;

use crate::quantity::Quantity;

/// Zero-alloc compile-time marker trait for SysML `part def` types.
pub trait SysmlPartCore: 'static {
    const SYSML_NAME: &'static str;
}

/// Zero-alloc compile-time marker trait for SysML `port def` types.
pub trait SysmlPortCore: 'static {
    type Conjugate: SysmlPortCore;
    const SYSML_NAME: &'static str;
    const IS_CONJUGATED: bool = false;
}

/// Conjugated port wrapper representing `~P` in SysML v2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Conjugated<P: SysmlPortCore>(pub PhantomData<P>);

impl<P: SysmlPortCore> Conjugated<P> {
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<P: SysmlPortCore> SysmlPortCore for Conjugated<P> {
    type Conjugate = P;
    const SYSML_NAME: &'static str = P::SYSML_NAME;
    const IS_CONJUGATED: bool = !P::IS_CONJUGATED;
}

/// Compile-time port compatibility relation.
pub trait PortConnectable<Other: SysmlPortCore>: SysmlPortCore {}

/// Marker trait for symmetric ports whose items are all bidirectional (`inout`).
pub trait SymmetricPort: SysmlPortCore {}

impl<P: SysmlPortCore> PortConnectable<P> for Conjugated<P> {}
impl<P: SymmetricPort> PortConnectable<Conjugated<P>> for Conjugated<P> {}

/// Zero-alloc compile-time marker trait for SysML `interface def` types.
pub trait SysmlInterfaceCore: 'static {
    type EndA: SysmlPortCore;
    type EndB: SysmlPortCore;
    const SYSML_NAME: &'static str;
    /// `false` for an abstract classification interface that declares no `end` ports
    /// (its `EndA`/`EndB` are [`NoInterfaceEnds`]). Concrete specializations must declare ends.
    const HAS_ENDS: bool = true;
}

/// Placeholder end type of an abstract `#[sysml::interface(abstract)]` that declares no `end`
/// fields (a pure taxonomy base). It connects to nothing, so such an interface can only be
/// used through a specialization that declares its own two `#[sysml::end]` ports.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct NoInterfaceEnds;

impl SysmlPortCore for NoInterfaceEnds {
    type Conjugate = NoInterfaceEnds;
    const SYSML_NAME: &'static str = "NoInterfaceEnds";
}

/// Zero-alloc compile-time marker trait for SysML `item def` types.
pub trait SysmlItemCore: 'static {
    const SYSML_NAME: &'static str;
}

/// Zero-alloc compile-time marker trait for SysML `enum def` types.
pub trait SysmlEnumCore: 'static {
    const SYSML_NAME: &'static str;
}

/// Zero-alloc compile-time marker trait for SysML `action def` types.
pub trait SysmlActionCore: 'static {
    const SYSML_NAME: &'static str;
}

/// Zero-alloc compile-time marker trait for SysML `state def` types.
pub trait SysmlStateCore: 'static {
    const SYSML_NAME: &'static str;
}

/// Zero-alloc compile-time marker trait for SysML `requirement def` types.
pub trait SysmlRequirementCore: 'static {
    const REQ_ID: &'static str;
    const SYSML_NAME: &'static str;
}

/// Executable requirement evaluation generated from `#[sysml::require_constraint(...)]`.
/// Works identically in `#![no_std]` bare-metal firmware and host simulation runners!
pub trait EvaluableRequirement: SysmlRequirementCore {
    fn evaluate(&self) -> bool;
}

/// Zero-alloc compile-time marker trait for modeled `#[sysml::calc]` functions.
pub trait SysmlCalcCore: 'static {
    const SYSML_NAME: &'static str;
}

/// Compile-time specialization witness (`Sub :> Base`).
pub trait Specializes<Base: ?Sized> {}

/// Compile-time functional allocation witness (`Part` performs `Action`).
pub trait Performs<A: SysmlActionCore> {}

/// Compile-time requirement satisfaction witness (`Element` satisfies `Req`).
pub trait Satisfies<R: SysmlRequirementCore> {}

/// Compile-time requirement verification witness (`Case` verifies `Req`).
pub trait Verifies<R: SysmlRequirementCore> {}

/// Zero-alloc state transition error usable in `#![no_std]` firmware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoStdTransitionError {
    pub state_machine: &'static str,
    pub from: &'static str,
    pub event: &'static str,
}

impl fmt::Display for NoStdTransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "illegal transition in `{}` from `{}` on event `{}`",
            self.state_machine, self.from, self.event
        )
    }
}

/// Outcome of dispatching an event to a state machine with `dispatch_with` / `dispatch_event`.
///
/// In SysML v2 an event that matches no transition from the current state (wrong trigger, or
/// every guard false) is simply not consumed: the machine stays where it is. That is a normal
/// outcome, so it is `Ignored` here rather than an error. (`step_with` / `step_event` keep
/// returning `Err` for it, for callers that treat an unmatched event as a test failure.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome<S> {
    /// A transition fired; the payload is the new current state (may equal the old one for a
    /// self-transition).
    Moved(S),
    /// No transition matched; the state is unchanged.
    Ignored,
}

impl<S> StepOutcome<S> {
    /// `true` if a transition fired.
    pub fn moved(&self) -> bool {
        matches!(self, StepOutcome::Moved(_))
    }

    /// `true` if no transition matched.
    pub fn is_ignored(&self) -> bool {
        matches!(self, StepOutcome::Ignored)
    }
}

/// Compile-time assertion helpers usable in `#![no_std]` crates.
pub const fn assert_part_core<T: SysmlPartCore>() {}
pub const fn assert_port_core<T: SysmlPortCore>() {}
pub const fn assert_interface_core<T: SysmlInterfaceCore>() {}
pub const fn assert_item_core<T: SysmlItemCore>() {}
pub const fn assert_action_core<T: SysmlActionCore>() {}
pub const fn assert_state_core<T: SysmlStateCore>() {}
pub const fn assert_requirement_core<T: SysmlRequirementCore>() {}
pub const fn assert_calc_core<T: SysmlCalcCore>() {}
pub const fn assert_specializes<Sub: Specializes<Base>, Base>() {}

pub const fn assert_ports_connect_core<A, B>(_a: &A, _b: &B)
where
    A: SysmlPortCore + PortConnectable<B>,
    B: SysmlPortCore,
{
}

pub const fn assert_interface_connect_core<I>(_a: &I::EndA, _b: &I::EndB)
where
    I: SysmlInterfaceCore,
    I::EndA: PortConnectable<I::EndB>,
{
}

pub const fn assert_interface_ends_core<A, B>()
where
    A: SysmlPortCore + PortConnectable<B>,
    B: SysmlPortCore,
{
}

/// Extracts a scalar `f64` from primitive numbers or `Quantity<f64>` (`#![no_std]`).
pub trait ToScalar {
    fn to_scalar(self) -> f64;
}

impl ToScalar for f64 {
    #[inline]
    fn to_scalar(self) -> f64 {
        self
    }
}

impl ToScalar for f32 {
    #[inline]
    fn to_scalar(self) -> f64 {
        self as f64
    }
}

impl ToScalar for i64 {
    #[inline]
    fn to_scalar(self) -> f64 {
        self as f64
    }
}

impl ToScalar for i32 {
    #[inline]
    fn to_scalar(self) -> f64 {
        self as f64
    }
}

impl ToScalar for u64 {
    #[inline]
    fn to_scalar(self) -> f64 {
        self as f64
    }
}

impl ToScalar for u32 {
    #[inline]
    fn to_scalar(self) -> f64 {
        self as f64
    }
}

impl ToScalar for usize {
    #[inline]
    fn to_scalar(self) -> f64 {
        self as f64
    }
}

impl ToScalar for Quantity<f64> {
    #[inline]
    fn to_scalar(self) -> f64 {
        self.value
    }
}

impl<T: Copy + ToScalar> ToScalar for &T {
    #[inline]
    fn to_scalar(self) -> f64 {
        (*self).to_scalar()
    }
}

/// `const`-evaluable string equality check used by `#[sysml::satisfy(..., id = "...")]`.
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
