//! SysML v2 -> Rust DSL lifter (`sysml-lift`).
//!
//! Powered by `Open-MBEE/sysml-toolkit`'s normative `sysmlv2-syntax` parser:
//! - `lift`: lifts `.sysml` ASTs into `sysml_ir::Model` while explicitly
//!   logging every unsupported construct (zero silent drops).
//! - `codegen`: generates clean, idiomatic, compilable Rust DSL source code
//!   using `#[sysml::*]` proc-macros.
//! - `compare`: compares parsed semantic graphs of original vs. re-emitted
//!   `.sysml` files to verify round-trip fidelity.

pub mod codegen;
pub mod compare;
pub mod lift;

pub use codegen::{generate_rust_dsl, CodegenOptions};
pub use compare::{compare_sysml_roundtrip, extract_semantic_graph, RoundTripDiff, SemanticGraph};
pub use lift::{lift_sysml_source, lift_sysml_units, LiftReport, LiftStats, UnsupportedConstruct};
