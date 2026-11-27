//! Deterministic SysML v2 textual emitter, Open-MBEE normative validator,
//! traceability audit, and ASPICE bidirectional traceability matrix generator.

#[cfg(feature = "validate")]
pub mod audit;
pub mod emit;
pub mod emit_cli;
pub mod evidence;
pub mod lint;
pub mod resolve;
pub mod safety_report;
pub mod trace;
#[cfg(feature = "validate")]
pub mod validate;
pub mod viewer;

#[cfg(feature = "validate")]
pub use audit::{
    audit_model, audit_model_with_lib, audit_model_with_stdlib, audit_units_with_lib, AuditFinding,
    AuditReport,
};
pub use emit_cli::{emit_model_split_std, is_sysml_std_package};
pub use emit::{emit_model, emit_package, escape_sysml_name, iso26262_metadata_package};
pub use resolve::qualify_requirement_refs;
pub use lint::{
    lint_model, render_lint_markdown, LintFinding, LintReport, LintSeverity, LintSummary,
};
pub use safety_report::{
    build_safety_report, render_safety_report_markdown, AsilDecompositionSummary, FaultCutSet,
    FmeaSummary, SafetyGoalSummary, SafetyReport,
};
pub use trace::{
    build_trace_matrix, render_trace_matrix_csv, render_trace_matrix_markdown, TraceMatrix,
    TraceMatrixSummary, TraceRow,
};
#[cfg(feature = "validate")]
pub use validate::{
    discover_omg_stdlib, validate_sysml_source, validate_sysml_source_with_stdlib,
    validate_sysml_units, validate_sysml_units_with_lib, ValidationDiagnostic, ValidationReport,
};
#[cfg(feature = "viewer")]
pub use viewer::render_standalone_html;
pub use viewer::{
    build_viewer_snapshot, ViewerElement, ViewerOptions, ViewerRelationship, ViewerSnapshot,
    ViewerTheme, ViewerView,
};
