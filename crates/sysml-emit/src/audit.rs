//! Traceability completeness and semantic validation audit (`rsysdsl audit`).
//!
//! Enforces that a Rust-authored SysML v2 model has:
//! 1. Zero Open-MBEE `sysml-toolkit` syntax, body-context, or semantic errors
//!    (and optionally zero warnings against the OMG Standard Library in `--strict` mode).
//! 2. Zero orphan `requirement def`s (every declared requirement must be satisfied
//!    by at least one part or specification `satisfy` link).
//! 3. Full verification coverage (when a `VerificationReport` is supplied, every
//!    constrained `requirement def` must have a recorded PASS verdict linked by ID).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use sysml_ir::{
    Model, Package, PackageMember, RequirementDef, RequirementUsage, VerificationReport,
    VerificationStatus,
};

use crate::emit::emit_model;
use crate::validate::{discover_omg_stdlib, validate_sysml_units_with_lib};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditFinding {
    pub rule: String,
    pub severity: String,
    pub element: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditReport {
    pub ok: bool,
    pub stdlib_loaded: bool,
    pub validation_ok: bool,
    pub validation_errors: usize,
    pub validation_warnings: usize,
    pub total_requirements: usize,
    pub satisfied_requirements: usize,
    pub orphan_requirements: Vec<String>,
    pub unverified_constrained_requirements: Vec<String>,
    pub total_calcs: usize,
    pub findings: Vec<AuditFinding>,
}

/// Audit a `sysml_ir::Model` without loading the OMG Standard Library (fast path).
pub fn audit_model(
    model_name: &str,
    model: &Model,
    verification: Option<&VerificationReport>,
) -> AuditReport {
    audit_model_with_lib(model_name, model, verification, None, false)
}

/// Audit a `sysml_ir::Model` with an optional explicit OMG Standard Library path.
/// Unlike [`audit_model_with_stdlib`], `lib_dir = None` skips stdlib loading.
pub fn audit_model_with_lib(
    model_name: &str,
    model: &Model,
    verification: Option<&VerificationReport>,
    lib_dir: Option<&Path>,
    strict: bool,
) -> AuditReport {
    let emitted = emit_model(model);
    let val = validate_sysml_units_with_lib(&[(format!("{model_name}.sysml"), emitted)], lib_dir);
    build_audit_report(model, verification, val, strict)
}

/// Audit a `sysml_ir::Model` against the official OMG Standard Library (`sysml.library`),
/// using `lib_dir` if provided or auto-discovering `sysml.library` otherwise.
/// When `strict` is `true`, any validation warning also fails the audit.
pub fn audit_model_with_stdlib(
    model_name: &str,
    model: &Model,
    verification: Option<&VerificationReport>,
    lib_dir: Option<&Path>,
    strict: bool,
) -> AuditReport {
    let discovered;
    let resolved = match lib_dir {
        Some(p) => Some(p),
        None => {
            discovered = discover_omg_stdlib();
            discovered.as_deref()
        }
    };
    audit_model_with_lib(model_name, model, verification, resolved, strict)
}

fn build_audit_report(
    model: &Model,
    verification: Option<&VerificationReport>,
    val: crate::validate::ValidationReport,
    strict: bool,
) -> AuditReport {
    let mut req_defs: BTreeMap<String, &RequirementDef> = BTreeMap::new();
    let mut satisfied_req_names: BTreeSet<String> = BTreeSet::new();
    let mut total_calcs = 0usize;

    for pkg in &model.packages {
        collect_pkg(pkg, &mut req_defs, &mut satisfied_req_names, &mut total_calcs);
    }

    let mut findings = Vec::new();
    if !val.valid {
        for err in val
            .syntax_errors
            .iter()
            .chain(val.context_errors.iter())
            .chain(val.model_errors.iter())
        {
            findings.push(AuditFinding {
                rule: format!("openmbee:{}", err.stage),
                severity: "error".to_string(),
                element: format!("{}:{}:{}", err.unit, err.line, err.column),
                message: err.message.clone(),
            });
        }
    }
    if strict {
        for warn in &val.warnings {
            findings.push(AuditFinding {
                rule: format!("openmbee:strict-warning:{}", warn.stage),
                severity: "error".to_string(),
                element: format!("{}:{}:{}", warn.unit, warn.line, warn.column),
                message: warn.message.clone(),
            });
        }
    }

    let mut orphan_requirements = Vec::new();
    for (req_name, rdef) in &req_defs {
        let req_id = rdef.short_name.as_deref().unwrap_or(req_name.as_str());
        let is_satisfied =
            satisfied_req_names.contains(req_name) || satisfied_req_names.contains(req_id);
        if !is_satisfied {
            let label = format!("{req_id} ({req_name})");
            orphan_requirements.push(label.clone());
            findings.push(AuditFinding {
                rule: "orphan-requirement".to_string(),
                severity: "error".to_string(),
                element: label,
                message: format!(
                    "requirement `{req_name}` has no `satisfy` link in any part or specification"
                ),
            });
        }
    }

    let mut unverified_constrained_requirements = Vec::new();
    if let Some(vrep) = verification {
        let mut passed_ids: BTreeSet<&str> = BTreeSet::new();
        for v in &vrep.verdicts {
            if v.status == VerificationStatus::Pass {
                passed_ids.insert(v.req_id.as_str());
                passed_ids.insert(v.req_name.as_str());
            }
        }
        for (req_name, rdef) in &req_defs {
            if !rdef.require_constraints.is_empty() {
                let req_id = rdef.short_name.as_deref().unwrap_or(req_name.as_str());
                if !passed_ids.contains(req_id) && !passed_ids.contains(req_name.as_str()) {
                    let label = format!("{req_id} ({req_name})");
                    unverified_constrained_requirements.push(label.clone());
                    findings.push(AuditFinding {
                        rule: "unverified-constrained-requirement".to_string(),
                        severity: "error".to_string(),
                        element: label,
                        message: format!(
                            "constrained requirement `{req_id}` has no PASS verdict in verification report"
                        ),
                    });
                }
            }
        }
    }

    let total_requirements = req_defs.len();
    let satisfied_requirements = total_requirements.saturating_sub(orphan_requirements.len());
    let strict_ok = !strict || val.warning_count == 0;
    let ok = val.valid
        && strict_ok
        && orphan_requirements.is_empty()
        && unverified_constrained_requirements.is_empty();

    AuditReport {
        ok,
        stdlib_loaded: val.stdlib_loaded,
        validation_ok: val.valid,
        validation_errors: val.error_count,
        validation_warnings: val.warning_count,
        total_requirements,
        satisfied_requirements,
        orphan_requirements,
        unverified_constrained_requirements,
        total_calcs,
        findings,
    }
}

fn collect_pkg<'a>(
    pkg: &'a Package,
    req_defs: &mut BTreeMap<String, &'a RequirementDef>,
    satisfied_req_names: &mut BTreeSet<String>,
    total_calcs: &mut usize,
) {
    for member in &pkg.members {
        match member {
            PackageMember::Package(sub) => {
                collect_pkg(sub, req_defs, satisfied_req_names, total_calcs);
            }
            PackageMember::RequirementDef(r) => {
                req_defs.insert(r.name.clone(), r);
            }
            PackageMember::CalcDef(_) => {
                *total_calcs += 1;
            }
            PackageMember::PartDef(p) => {
                for s in &p.satisfies {
                    satisfied_req_names.insert(simple_name(&s.requirement));
                }
                for ru in &p.requirements {
                    collect_req_usage(ru, satisfied_req_names);
                }
            }
            PackageMember::RequirementUsage(ru) => {
                collect_req_usage(ru, satisfied_req_names);
            }
            _ => {}
        }
    }
}

fn collect_req_usage(ru: &RequirementUsage, satisfied_req_names: &mut BTreeSet<String>) {
    let mut usage_to_def: BTreeMap<String, String> = BTreeMap::new();
    for nested in &ru.nested_requirements {
        if let Some(ty) = &nested.type_name {
            usage_to_def.insert(nested.name.clone(), simple_name(ty));
        }
        collect_req_usage(nested, satisfied_req_names);
    }
    for s in &ru.satisfies {
        let key = simple_name(&s.requirement);
        if let Some(def_name) = usage_to_def.get(&key) {
            satisfied_req_names.insert(def_name.clone());
        }
        satisfied_req_names.insert(key);
    }
}

fn simple_name(raw: &str) -> String {
    raw.rsplit("::")
        .next()
        .unwrap_or(raw)
        .trim()
        .trim_matches('\'')
        .to_string()
}
