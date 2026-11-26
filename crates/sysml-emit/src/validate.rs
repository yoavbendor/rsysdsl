//! Normative SysML v2 validation using Open-MBEE `sysmlv2-syntax` and `sysmlv2-model`.

use serde::{Deserialize, Serialize};
use sysmlv2_model::check::{validate_model_with, validate_semantics_with};
use sysmlv2_model::json::ResolvedModel;
use sysmlv2_model::model::Model as ToolkitModel;
use sysmlv2_syntax::check::validate as validate_syntax_context;
use sysmlv2_syntax::diag::{Diagnostic, Severity};
use sysmlv2_syntax::parser::parse_source;
use sysmlv2_syntax::span::LineIndex;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationDiagnostic {
    pub unit: String,
    pub stage: String,
    pub severity: String,
    pub line: usize,
    pub column: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub syntax_errors: Vec<ValidationDiagnostic>,
    pub context_errors: Vec<ValidationDiagnostic>,
    pub model_errors: Vec<ValidationDiagnostic>,
    pub warnings: Vec<ValidationDiagnostic>,
}

fn to_diag(unit: &str, stage: &str, lines: &LineIndex, d: &Diagnostic) -> ValidationDiagnostic {
    let lc = lines.line_col(d.span.start);
    let severity = match d.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    ValidationDiagnostic {
        unit: unit.to_string(),
        stage: stage.to_string(),
        severity: severity.to_string(),
        line: lc.line as usize,
        column: lc.col as usize,
        message: d.message.clone(),
    }
}

/// Validate a single `.sysml` source unit through:
/// 1. `sysmlv2-syntax` parser (OMG 2025 SysML v2 textual grammar)
/// 2. `sysmlv2-syntax` body-context legality validator
/// 3. `sysmlv2-model` referential + semantic validator
pub fn validate_sysml_source(name: &str, source: &str) -> ValidationReport {
    validate_sysml_units(&[(name.to_string(), source.to_string())])
}

/// Validate multiple `.sysml` units together in one shared global resolution namespace.
pub fn validate_sysml_units(units: &[(String, String)]) -> ValidationReport {
    let mut syntax_errors = Vec::new();
    let mut context_errors = Vec::new();
    let mut model_errors = Vec::new();
    let mut warnings = Vec::new();

    let mut model = ToolkitModel::new();
    let mut line_indices = Vec::with_capacity(units.len());

    for (name, src) in units {
        let lines = LineIndex::new(src);
        let parsed = parse_source(src);
        for d in &parsed.diagnostics {
            let vd = to_diag(name, "parse", &lines, d);
            if d.severity == Severity::Error {
                syntax_errors.push(vd);
            } else {
                warnings.push(vd);
            }
        }
        let ctx_diags = validate_syntax_context(&parsed.unit);
        for d in &ctx_diags {
            let vd = to_diag(name, "body_context", &lines, d);
            if d.severity == Severity::Error {
                context_errors.push(vd);
            } else {
                warnings.push(vd);
            }
        }
        line_indices.push((name.clone(), lines));
        model.add_source(name.clone(), src);
    }

    if syntax_errors.is_empty() && context_errors.is_empty() {
        let mut resolved = ResolvedModel::build(&model);
        let ref_diags = validate_model_with(&mut resolved, &model);
        for (unit_idx, d) in ref_diags {
            let (uname, lines) = &line_indices[unit_idx];
            let vd = to_diag(uname, "model_ref", lines, &d);
            if d.severity == Severity::Error {
                model_errors.push(vd);
            } else {
                warnings.push(vd);
            }
        }
        let sem_diags = validate_semantics_with(&mut resolved, &model);
        for (unit_idx, d) in sem_diags {
            let (uname, lines) = &line_indices[unit_idx];
            let vd = to_diag(uname, "semantics", lines, &d);
            if d.severity == Severity::Error {
                model_errors.push(vd);
            } else {
                warnings.push(vd);
            }
        }
    }

    let error_count = syntax_errors.len() + context_errors.len() + model_errors.len();
    let warning_count = warnings.len();
    ValidationReport {
        valid: error_count == 0,
        error_count,
        warning_count,
        syntax_errors,
        context_errors,
        model_errors,
        warnings,
    }
}
