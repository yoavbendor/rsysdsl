//! Normative SysML v2 validation using Open-MBEE `sysmlv2-syntax` and `sysmlv2-model`,
//! with automatic OMG Standard Library (`sysml.library`) resolution for 0-warning validation.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use sysmlv2_model::check::{validate_model_with, validate_semantics_with};
use sysmlv2_model::json::ResolvedModel;
use sysmlv2_model::model::Model as ToolkitModel;
use sysmlv2_model::prepared::load_library_with_cache;
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
    pub stdlib_loaded: bool,
    pub stdlib_units: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub syntax_errors: Vec<ValidationDiagnostic>,
    pub context_errors: Vec<ValidationDiagnostic>,
    pub model_errors: Vec<ValidationDiagnostic>,
    pub warnings: Vec<ValidationDiagnostic>,
}

/// Auto-discover the official OMG SysML v2 Standard Library (`sysml.library`) directory.
///
/// Search order:
/// 1. `SYSML_LIBRARY_DIR` environment variable
/// 2. Bundled repository library (`third_party/omg/sysml.library`) relative to workspace root,
///    current working directory ancestors, or executable ancestors
pub fn discover_omg_stdlib() -> Option<PathBuf> {
    if let Ok(env_dir) = std::env::var("SYSML_LIBRARY_DIR") {
        let p = PathBuf::from(env_dir);
        if p.is_dir() {
            return Some(p);
        }
    }

    let manifest_candidate = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../third_party/omg/sysml.library");
    if manifest_candidate.is_dir() {
        return Some(manifest_candidate);
    }

    for start in [
        std::env::current_dir().ok(),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf)),
    ]
    .into_iter()
    .flatten()
    {
        for ancestor in start.ancestors() {
            let candidate = ancestor.join("third_party/omg/sysml.library");
            if candidate.is_dir() {
                return Some(candidate);
            }
        }
    }
    None
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

/// Validate a single `.sysml` source unit without loading the OMG standard library (fast path).
pub fn validate_sysml_source(name: &str, source: &str) -> ValidationReport {
    validate_sysml_units_with_lib(&[(name.to_string(), source.to_string())], None)
}

/// Validate a single `.sysml` source unit against the OMG Standard Library (explicit path or auto-discovered).
pub fn validate_sysml_source_with_stdlib(
    name: &str,
    source: &str,
    lib_dir: Option<&Path>,
) -> ValidationReport {
    let discovered;
    let resolved_lib = match lib_dir {
        Some(p) => Some(p),
        None => {
            discovered = discover_omg_stdlib();
            discovered.as_deref()
        }
    };
    validate_sysml_units_with_lib(&[(name.to_string(), source.to_string())], resolved_lib)
}

/// Validate multiple `.sysml` units together in one shared global resolution namespace (fast path).
pub fn validate_sysml_units(units: &[(String, String)]) -> ValidationReport {
    validate_sysml_units_with_lib(units, None)
}

/// Validate multiple `.sysml` units with optional OMG Standard Library loading (`load_library_with_cache`).
pub fn validate_sysml_units_with_lib(
    units: &[(String, String)],
    lib_dir: Option<&Path>,
) -> ValidationReport {
    let mut syntax_errors = Vec::new();
    let mut context_errors = Vec::new();
    let mut model_errors = Vec::new();
    let mut warnings = Vec::new();

    let mut model = ToolkitModel::new();
    let mut stdlib_loaded = false;
    let mut stdlib_units = 0usize;

    if let Some(dir) = lib_dir {
        if load_library_with_cache(&mut model, dir).is_ok() {
            stdlib_loaded = true;
            stdlib_units = model.loaded_library_unit_count();
        }
    }

    let user_unit_offset = model.units().len();
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
            if unit_idx < user_unit_offset {
                continue;
            }
            let (uname, lines) = &line_indices[unit_idx - user_unit_offset];
            let vd = to_diag(uname, "model_ref", lines, &d);
            if d.severity == Severity::Error {
                model_errors.push(vd);
            } else {
                warnings.push(vd);
            }
        }
        let sem_diags = validate_semantics_with(&mut resolved, &model);
        for (unit_idx, d) in sem_diags {
            if unit_idx < user_unit_offset {
                continue;
            }
            let (uname, lines) = &line_indices[unit_idx - user_unit_offset];
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
        stdlib_loaded,
        stdlib_units,
        error_count,
        warning_count,
        syntax_errors,
        context_errors,
        model_errors,
        warnings,
    }
}
