//! Deterministic SysML v2 textual emitter, Open-MBEE normative validator,
//! traceability audit, and ASPICE bidirectional traceability matrix generator.

#[cfg(feature = "validate")]
pub mod audit;
pub mod emit;
pub mod trace;
#[cfg(feature = "validate")]
pub mod validate;

#[cfg(feature = "validate")]
pub use audit::{
    audit_model, audit_model_with_lib, audit_model_with_stdlib, audit_units_with_lib, AuditFinding,
    AuditReport,
};
pub use emit::{emit_model, emit_package, escape_sysml_name, iso26262_metadata_package};
pub use trace::{
    build_trace_matrix, render_trace_matrix_csv, render_trace_matrix_markdown, TraceMatrix,
    TraceMatrixSummary, TraceRow,
};
#[cfg(feature = "validate")]
pub use validate::{
    discover_omg_stdlib, validate_sysml_source, validate_sysml_source_with_stdlib,
    validate_sysml_units, validate_sysml_units_with_lib, ValidationDiagnostic, ValidationReport,
};
