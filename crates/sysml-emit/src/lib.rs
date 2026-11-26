//! Deterministic SysML v2 textual emitter and Open-MBEE normative validator.

pub mod emit;
#[cfg(feature = "validate")]
pub mod validate;

pub use emit::{emit_model, emit_package, escape_sysml_name};
#[cfg(feature = "validate")]
pub use validate::{
    validate_sysml_source, validate_sysml_units, ValidationDiagnostic, ValidationReport,
};
