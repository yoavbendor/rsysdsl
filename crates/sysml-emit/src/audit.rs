//! Traceability completeness and semantic validation audit (`rsysdsl audit`).
//!
//! Enforces that a Rust-authored SysML v2 model has:
//! 1. Zero Open-MBEE `sysml-toolkit` syntax, body-context, or semantic errors.
//! 2. Zero orphan `requirement def`s (every declared requirement must be satisfied
//!    by at least one part or specification `satisfy` link).
//! 3. Full verification coverage (when a `VerificationReport` is supplied, every
//!    constrained `requirement def` must have a recorded PASS verdict linked by ID).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use sysml_ir::{
    Model, Package, PackageMember, RequirementDef, RequirementUsage, VerificationReport,
    VerificationStatus,
};

use crate::emit::emit_model;
use crate::validate::validate_sysml_source;

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
    pub validation_ok: bool,
    pub validation_errors: usize,
    pub total_requirements: usize,
    pub satisfied_requirements: usize,
    pub orphan_requirements: Vec<String>,
    pub unverified_constrained_requirements: Vec<String>,
    pub total_calcs: usize,
    pub findings: Vec<AuditFinding>,
}

/// Audit a `sysml_ir::Model` for semantic validity and traceability completeness.
pub fn audit_model(
    model_name: &str,
    model: &Model,
    verification: Option<&VerificationReport>,
) -> AuditReport {
    let emitted = emit_model(model);
    let val = validate_sysml_source(&format!("{model_name}.sysml"), &emitted);

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

    let mut orphan_requirements = Vec::new();
    for (req_name, rdef) in &req_defs {
        let req_id = rdef.short_name.as_deref().unwrap_or(req_name.as_str());
        let is_satisfied = satisfied_req_names.contains(req_name)
            || satisfied_req_names.contains(req_id);
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
    let ok = val.valid
        && orphan_requirements.is_empty()
        && unverified_constrained_requirements.is_empty();

    AuditReport {
        ok,
        validation_ok: val.valid,
        validation_errors: val.error_count,
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
    // Map nested requirement usage names (`hlr-R002`) -> their RequirementDef type (`LunarLanderSoftLandingRequirement`)
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
