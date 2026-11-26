//! Strongly typed Intermediate Representation (IR), compile-time MBSE marker
//! traits, physical quantities, and verification report types for `rsysdsl`.
//!
//! This crate forms the zero-macro foundation shared by:
//! - `sysml-macros` (generates IR builders and compile-time `rustc` checks)
//! - `sysml-emit` (deterministically renders IR into OMG-valid `.sysml`)
//! - `sysml-lift` (lifts parsed `.sysml` ASTs into IR and Rust DSL code)
//! - `sysml-cli` (agent-native CLI tooling)

pub mod ir;
pub mod quantity;
pub mod traits;
pub mod verify;

pub use ir::*;
pub use quantity::Quantity;
pub use traits::*;
pub use verify::*;
