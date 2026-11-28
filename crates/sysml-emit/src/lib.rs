//! Deterministic SysML v2 textual emitter, Open-MBEE normative validator,
//! traceability audit, and ASPICE bidirectional traceability matrix generator.

#[cfg(feature = "validate")]
pub mod audit;
pub mod budget;
pub mod emit;
pub mod emit_cli;
pub mod evidence;
pub mod ir_validate;
pub mod lint;
pub mod reconcile;
pub mod refactor;
pub mod refute;
pub mod resolve;
pub mod safety_report;
pub mod tbd;
pub mod trace;
pub mod trade_study;
#[cfg(feature = "validate")]
pub mod validate;
pub mod viewer;

#[cfg(feature = "validate")]
pub use audit::{
    apply_maturity, audit_model, audit_model_with_lib, audit_model_with_stdlib,
    audit_units_with_lib, AuditFinding, AuditReport,
};
pub use budget::{
    analyze_budgets, render_budget_markdown, BudgetLimits, BudgetReport, DomainRollup, HotSpot,
    TimingBudget,
};
pub use emit_cli::{emit_model_split_std, is_sysml_std_package};
pub use emit::{
    budget_metadata_package, emit_model, emit_package, escape_sysml_name, iso26262_metadata_package,
    model_has_budget_roles, tbd_metadata_package,
};
pub use ir_validate::{
    validate_ir, validate_model_ir, IrDiagnostic, IrSeverity, IrValidationReport,
};
pub use reconcile::{
    reconcile_model, render_reconcile_markdown, KgConflict, KgEntity, KgExport, ReconcileFinding,
    ReconcileReport, ReconcileSeverity, ReconcileSummary,
};
pub use refactor::{
    elevate_model, render_elevate_markdown, slice_model, ElevateProposal, ElevateReport,
    ModelAspect,
};
pub use refute::{
    refute_requirements, render_refute_markdown, ActiveConstraint, RefuteLimits, RefuteReport,
    RelaxationSuggestion,
};
pub use trade_study::{
    render_trade_study_markdown, render_what_if_markdown, run_trade_study, run_what_if,
    SensitivityDriver, TradePoint, TradeStudyReport, WhatIfReport,
};
pub use tbd::{
    collect_tbd_ledger, count_by_kind, model_has_tbd, overdue_at, render_tbd_markdown, tbd_ledger_report,
    check_tbd_ratchet, TbdEntry, TbdLedger, TbdRatchet,
};
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
