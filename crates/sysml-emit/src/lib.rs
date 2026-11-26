//! Deterministic SysML v2 textual emitter, Open-MBEE normative validator, and traceability audit.

#[cfg(feature = "validate")]
pub mod audit;
pub mod emit;
#[cfg(feature = "validate")]
pub mod validate;

#[cfg(feature = "validate")]
pub use audit::{audit_model, AuditFinding, AuditReport};
pub use emit::{emit_model, emit_package, escape_sysml_name};
#[cfg(feature = "validate")]
pub use validate::{
    validate_sysml_source, validate_sysml_units, ValidationDiagnostic, ValidationReport,
};
