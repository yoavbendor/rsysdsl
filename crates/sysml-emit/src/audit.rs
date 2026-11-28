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
    Model, Package, PackageMember, RequirementDef, RequirementUsage, TbdPhase, VerificationReport,
    VerificationStatus,
};

use crate::tbd::{collect_tbd_ledger, overdue_at, TbdEntry};

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
    /// Requirements whose constraint compares two attributes holding the same model value
    /// (a "measured" value copied from the limit). Reported as `hollow-requirement` warnings;
    /// they do not fail the audit (inventing a different number would be worse): mark the
    /// measured attribute `evidence` and supply it from a verification case.
    #[serde(default)]
    pub hollow_requirements: Vec<String>,
    pub total_calcs: usize,
    pub findings: Vec<AuditFinding>,
    /// Number of TBD markers (escape hatches) in the model.
    #[serde(default)]
    pub tbd_count: usize,
    /// Every TBD marker: element, kind (unit|value|element|channel|custom_unit), reason,
    /// owner, resolve_by.
    #[serde(default)]
    pub tbd_ledger: Vec<TbdEntry>,
    /// Design maturity the model is audited at (`--maturity`, default `concept`).
    #[serde(default = "default_maturity")]
    pub maturity: TbdPhase,
    /// TBDs due at or before `maturity` (`resolve_by <= maturity`); fail the audit in `--strict`.
    #[serde(default)]
    pub overdue_tbds: Vec<TbdEntry>,
    /// C2.1: every requirement with `require constraint`s, evaluated by the IR expression
    /// evaluator (`expr_eval`) on the requirement's own attribute values: `pass` / `fail` when
    /// every operand has a value, `unknown` (with reasons) when an operand is a TBD, unmeasured
    /// evidence or has no value, `error` for a dimension mismatch, unresolved reference or
    /// parse failure. Informational: it does not change `ok` (gating comes later).
    #[serde(default)]
    pub requirement_evaluations: Vec<RequirementEvaluation>,
}

/// IR evaluation of one requirement's constraints (see [`AuditReport::requirement_evaluations`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequirementEvaluation {
    /// `"<id> (<name>)"`, as in the other requirement lists.
    pub requirement: String,
    pub verdict: crate::expr_eval::Verdict,
    pub constraints: Vec<crate::expr_eval::ConstraintOutcome>,
}

/// Evaluate every requirement definition's `require constraint`s on its own values.
pub fn evaluate_requirements(model: &Model) -> Vec<RequirementEvaluation> {
    use crate::expr_eval::{combine, evaluate_constraint, Evaluator, RequirementScope};
    let mut req_defs: BTreeMap<String, &RequirementDef> = BTreeMap::new();
    let (mut sat, mut calcs) = (BTreeSet::new(), 0usize);
    for pkg in &model.packages {
        collect_pkg(pkg, &mut req_defs, &mut sat, &mut calcs);
    }
    let ev = Evaluator::new(model);
    req_defs
        .iter()
        .filter(|(_, r)| !r.require_constraints.is_empty())
        .map(|(name, r)| {
            let scope = RequirementScope::new(model, r);
            let constraints: Vec<_> = r
                .require_constraints
                .iter()
                .map(|c| evaluate_constraint(&ev, c, &scope))
                .collect();
            let id = r.short_name.as_deref().unwrap_or(name.as_str());
            RequirementEvaluation {
                requirement: format!("{id} ({name})"),
                verdict: combine(&constraints),
                constraints,
            }
        })
        .collect()
}

fn default_maturity() -> TbdPhase {
    TbdPhase::Concept
}

/// Apply the S7 maturity rule: every TBD whose `resolve_by` is at or before `maturity` is
/// overdue. Overdue TBDs are `tbd-overdue` findings (errors that fail the audit in `strict`
/// mode, warnings otherwise). `release` therefore requires zero TBDs; the default `concept`
/// only rejects TBDs that had to be resolved by the concept phase.
pub fn apply_maturity(report: &mut AuditReport, maturity: TbdPhase, strict: bool) {
    report.findings.retain(|f| f.rule != "tbd-overdue");
    report.maturity = maturity;
    report.overdue_tbds = overdue_at(&report.tbd_ledger, maturity);
    for e in &report.overdue_tbds {
        report.findings.push(AuditFinding {
            rule: "tbd-overdue".to_string(),
            severity: if strict { "error" } else { "warning" }.to_string(),
            element: e.element.clone(),
            message: format!(
                "{} TBD due by `{}` is still open at maturity `{}`: {}",
                e.kind.as_str(),
                e.resolve_by.as_str(),
                maturity.as_str(),
                e.reason
            ),
        });
    }
    if strict && !report.overdue_tbds.is_empty() {
        report.ok = false;
    }
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

/// Audit raw `.sysml` source units alongside their lifted IR `Model` against an optional
/// OMG Standard Library path (preserving exact source file names and line numbers in findings).
pub fn audit_units_with_lib(
    units: &[(String, String)],
    model: &Model,
    verification: Option<&VerificationReport>,
    lib_dir: Option<&Path>,
    strict: bool,
) -> AuditReport {
    let val = validate_sysml_units_with_lib(units, lib_dir);
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

    let mut hollow_requirements = Vec::new();
    for (req_name, rdef) in &req_defs {
        let ev = crate::evidence::analyze_requirement(rdef);
        if ev.hollow_comparisons.is_empty() {
            continue;
        }
        let req_id = rdef.short_name.as_deref().unwrap_or(req_name.as_str());
        let label = format!("{req_id} ({req_name})");
        hollow_requirements.push(label.clone());
        findings.push(AuditFinding {
            rule: "hollow-requirement".to_string(),
            severity: "warning".to_string(),
            element: label,
            message: format!(
                "constraint operands hold the same model value ({}): the measured value looks copied from the limit. Mark it `#[sysml::attr(..., evidence)]` without a default and let a verification case supply it",
                ev.hollow_comparisons.join("; ")
            ),
        });
    }

    let total_requirements = req_defs.len();
    let satisfied_requirements = total_requirements.saturating_sub(orphan_requirements.len());
    let strict_ok = !strict || val.warning_count == 0;
    let ok = val.valid
        && strict_ok
        && orphan_requirements.is_empty()
        && unverified_constrained_requirements.is_empty();

    let tbd_ledger = collect_tbd_ledger(model);
    let mut report = AuditReport {
        ok,
        stdlib_loaded: val.stdlib_loaded,
        validation_ok: val.valid,
        validation_errors: val.error_count,
        validation_warnings: val.warning_count,
        total_requirements,
        satisfied_requirements,
        orphan_requirements,
        unverified_constrained_requirements,
        hollow_requirements,
        total_calcs,
        findings,
        tbd_count: tbd_ledger.len(),
        tbd_ledger,
        maturity: TbdPhase::Concept,
        overdue_tbds: Vec::new(),
        requirement_evaluations: evaluate_requirements(model),
    };
    apply_maturity(&mut report, TbdPhase::Concept, strict);
    report
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expr_eval::Verdict;
    use sysml_ir::{AttributeUsage, PackageMember};

    fn attr(name: &str, value: Option<&str>, unit: &str, evidence: bool) -> AttributeUsage {
        AttributeUsage {
            name: Some(name.into()),
            value: value.map(String::from),
            unit: Some(unit.into()),
            is_evidence: evidence,
            ..Default::default()
        }
    }

    fn req(name: &str, attributes: Vec<AttributeUsage>, constraint: &str) -> PackageMember {
        PackageMember::RequirementDef(RequirementDef {
            name: name.into(),
            short_name: Some(format!("{name}-ID")),
            attributes,
            require_constraints: vec![constraint.into()],
            ..Default::default()
        })
    }

    /// C2.1: requirement constraints are evaluated and reported per requirement, without
    /// changing the audit's pass/fail gating.
    #[test]
    fn audit_reports_requirement_evaluations_without_gating() {
        let model = Model::new(vec![Package {
            name: "Reqs".into(),
            members: vec![
                req(
                    "Fits",
                    vec![attr("limit", Some("100.0"), "W", false), attr("draw", Some("80.0"), "W", false)],
                    "draw <= limit",
                ),
                req(
                    "Exceeds",
                    vec![attr("limit", Some("100.0"), "W", false), attr("draw", Some("120.0"), "W", false)],
                    "draw <= limit",
                ),
                req(
                    "Unmeasured",
                    vec![attr("limit", Some("100.0"), "W", false), attr("draw", None, "W", true)],
                    "draw <= limit",
                ),
                req(
                    "Mismatched",
                    vec![attr("limit", Some("100.0"), "W", false), attr("time", Some("2.0"), "s", false)],
                    "time <= limit",
                ),
            ],
            ..Default::default()
        }]);
        let evals = evaluate_requirements(&model);
        let verdict = |n: &str| {
            evals
                .iter()
                .find(|e| e.requirement == format!("{n}-ID ({n})"))
                .map(|e| e.verdict)
        };
        assert_eq!(verdict("Fits"), Some(Verdict::Pass));
        assert_eq!(verdict("Exceeds"), Some(Verdict::Fail));
        assert_eq!(verdict("Unmeasured"), Some(Verdict::Unknown));
        assert_eq!(verdict("Mismatched"), Some(Verdict::Error));
        let unmeasured = evals.iter().find(|e| e.requirement.contains("Unmeasured")).unwrap();
        assert!(unmeasured.constraints[0].reasons[0].contains("evidence"));

        let report = audit_model("reqs", &model, None);
        assert_eq!(report.requirement_evaluations, evals);
        // Gating unchanged: the failing requirement is reported, not (yet) an audit error.
        // (The only findings on it are the orphan check's: none of these has a `satisfy`.)
        assert!(!report
            .findings
            .iter()
            .any(|f| f.element.contains("Exceeds") && f.rule != "orphan-requirement"));
    }
}
