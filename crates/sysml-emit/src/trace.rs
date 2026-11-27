//! ASPICE Bidirectional Traceability Matrix generator (`rsysdsl trace-matrix`).
//!
//! Builds a deterministic, audit-ready traceability matrix linking:
//! `Upstream Refinement -> Requirement (ID + Constraints) -> Satisfying Architecture Parts
//!  -> Allocated Functions (perform action) -> Verification Status + Evidence`
//! with JSON, Markdown, and RFC-4180 CSV renderers.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use sysml_ir::{
    Model, Package, PackageMember, PartDef, RequirementDef, RequirementUsage, VerificationReport,
    VerificationStatus,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceRow {
    pub req_id: String,
    pub req_name: String,
    pub package: String,
    pub rationale: Option<String>,
    pub constraints: Vec<String>,
    pub refines: Vec<String>,
    pub satisfied_by: Vec<String>,
    pub allocated_functions: Vec<String>,
    pub verification_status: String,
    pub verification_scenario: Option<String>,
    pub evidence: Option<serde_json::Value>,
    /// Attributes marked `#[sysml::attr(..., evidence)]` (measured values, not limits).
    #[serde(default)]
    pub evidence_attributes: Vec<String>,
    /// Where the requirement's evidence comes from: `verification` (a verdict exists),
    /// `model default (placeholder)` (evidence attributes only hold model values),
    /// `unmarked` (constrained, but no attribute is marked `evidence`), or `n/a`.
    #[serde(default)]
    pub evidence_source: String,
    /// Constraint comparisons whose two operands hold the same model value (limit copied
    /// into the "measured" value).
    #[serde(default)]
    pub hollow_comparisons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceMatrixSummary {
    pub total_requirements: usize,
    pub satisfied_requirements: usize,
    pub satisfaction_coverage_pct: f64,
    pub constrained_requirements: usize,
    pub verified_pass: usize,
    pub verified_fail: usize,
    pub unverified: usize,
    pub verification_coverage_pct: f64,
    pub total_parts: usize,
    pub total_actions: usize,
    pub total_perform_allocations: usize,
    pub total_calcs: usize,
    pub total_state_defs: usize,
    /// Constrained requirements with no verdict whose evidence is only a model value.
    #[serde(default)]
    pub placeholder_evidence: usize,
    /// Requirements with at least one hollow comparison (see [`TraceRow::hollow_comparisons`]).
    #[serde(default)]
    pub hollow_requirements: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceMatrix {
    pub model: String,
    pub summary: TraceMatrixSummary,
    pub modeled_calcs: Vec<String>,
    pub rows: Vec<TraceRow>,
}

/// Build an ASPICE bidirectional traceability matrix from any `sysml_ir::Model`
/// and optional executable `VerificationReport`.
pub fn build_trace_matrix(
    model_name: impl Into<String>,
    model: &Model,
    verification: Option<&VerificationReport>,
) -> TraceMatrix {
    let mut req_defs: Vec<(String, &RequirementDef)> = Vec::new();
    let mut part_defs: BTreeMap<String, &PartDef> = BTreeMap::new();
    // req_name_or_id -> set of satisfying part/path labels
    let mut req_satisfiers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut modeled_calcs: Vec<String> = Vec::new();
    let mut total_actions = 0usize;
    let mut total_performs = 0usize;
    let mut total_states = 0usize;

    for pkg in &model.packages {
        index_pkg(
            pkg,
            &mut req_defs,
            &mut part_defs,
            &mut req_satisfiers,
            &mut modeled_calcs,
            &mut total_actions,
            &mut total_performs,
            &mut total_states,
        );
    }

    // Build part -> transitive `perform action` allocations via `:>` specialization & sub-parts
    let part_performs = build_part_performs(&part_defs);

    // Index verification verdicts by req_id and req_name
    let mut verdicts_by_key: BTreeMap<&str, &sysml_ir::VerificationVerdict> = BTreeMap::new();
    if let Some(vrep) = verification {
        for v in &vrep.verdicts {
            verdicts_by_key.insert(v.req_id.as_str(), v);
            verdicts_by_key.insert(v.req_name.as_str(), v);
        }
    }

    let mut rows = Vec::with_capacity(req_defs.len());
    let mut satisfied_count = 0usize;
    let mut constrained_count = 0usize;
    let mut verified_pass = 0usize;
    let mut verified_fail = 0usize;
    let mut unverified = 0usize;
    let mut placeholder_evidence = 0usize;
    let mut hollow_requirements = 0usize;

    for (pkg_name, rdef) in &req_defs {
        let req_id = rdef
            .short_name
            .clone()
            .unwrap_or_else(|| rdef.name.clone());
        if !rdef.require_constraints.is_empty() {
            constrained_count += 1;
        }

        let mut sat_set: BTreeSet<String> = BTreeSet::new();
        if let Some(s) = req_satisfiers.get(&rdef.name) {
            sat_set.extend(s.iter().cloned());
        }
        if let Some(s) = req_satisfiers.get(&req_id) {
            sat_set.extend(s.iter().cloned());
        }
        if !sat_set.is_empty() {
            satisfied_count += 1;
        }
        let satisfied_by: Vec<String> = sat_set.into_iter().collect();

        // Collect allocated functions (`perform action`) reachable from satisfying parts/paths,
        // walking from the most specific leaf part up to its enclosing logical owner.
        let mut alloc_fns: BTreeSet<String> = BTreeSet::new();
        for sat in &satisfied_by {
            for seg in sat.split('.').rev() {
                let pascal = upper_first(seg);
                let mut matched = false;
                for key in [seg, pascal.as_str()] {
                    if let Some(fns) = part_performs.get(key) {
                        if !fns.is_empty() {
                            alloc_fns.extend(fns.iter().cloned());
                            matched = true;
                        }
                    }
                }
                if matched {
                    break;
                }
            }
        }

        let verdict = verdicts_by_key
            .get(req_id.as_str())
            .or_else(|| verdicts_by_key.get(rdef.name.as_str()))
            .copied();

        let (verification_status, verification_scenario, evidence) = match verdict {
            Some(v) => {
                let st = match v.status {
                    VerificationStatus::Pass => {
                        verified_pass += 1;
                        "pass"
                    }
                    VerificationStatus::Fail => {
                        verified_fail += 1;
                        "fail"
                    }
                    VerificationStatus::Skipped => {
                        unverified += 1;
                        "skipped"
                    }
                };
                (
                    st.to_string(),
                    Some(v.scenario.clone()),
                    Some(v.evidence.clone()),
                )
            }
            None => {
                unverified += 1;
                ("unverified".to_string(), None, None)
            }
        };

        let ev = crate::evidence::analyze_requirement(rdef);
        let evidence_source = if verdict.is_some() {
            "verification"
        } else if rdef.require_constraints.is_empty() {
            "n/a"
        } else if ev.evidence_attributes.is_empty() {
            placeholder_evidence += 1;
            "unmarked"
        } else {
            placeholder_evidence += 1;
            "model default (placeholder)"
        };
        if !ev.hollow_comparisons.is_empty() {
            hollow_requirements += 1;
        }

        let refines: Vec<String> = rdef
            .refinements
            .iter()
            .map(|r| r.supplier.clone())
            .collect();

        rows.push(TraceRow {
            req_id,
            req_name: rdef.name.clone(),
            package: pkg_name.clone(),
            rationale: rdef.rationale.clone(),
            constraints: rdef.require_constraints.clone(),
            refines,
            satisfied_by,
            allocated_functions: alloc_fns.into_iter().collect(),
            verification_status,
            verification_scenario,
            evidence,
            evidence_attributes: ev.evidence_attributes,
            evidence_source: evidence_source.to_string(),
            hollow_comparisons: ev.hollow_comparisons,
        });
    }

    rows.sort_by(|a, b| a.req_id.cmp(&b.req_id).then(a.req_name.cmp(&b.req_name)));

    let total_requirements = rows.len();
    let satisfaction_coverage_pct = if total_requirements == 0 {
        100.0
    } else {
        round1((satisfied_count as f64 / total_requirements as f64) * 100.0)
    };
    let verification_coverage_pct = if constrained_count == 0 {
        100.0
    } else {
        round1((verified_pass as f64 / constrained_count as f64) * 100.0)
    };

    TraceMatrix {
        model: model_name.into(),
        summary: TraceMatrixSummary {
            total_requirements,
            satisfied_requirements: satisfied_count,
            satisfaction_coverage_pct,
            constrained_requirements: constrained_count,
            verified_pass,
            verified_fail,
            unverified,
            verification_coverage_pct,
            total_parts: part_defs.len(),
            total_actions,
            total_perform_allocations: total_performs,
            total_calcs: modeled_calcs.len(),
            total_state_defs: total_states,
            placeholder_evidence,
            hollow_requirements,
        },
        modeled_calcs,
        rows,
    }
}

/// Render a `TraceMatrix` as a GitHub-Flavored Markdown table suitable for PRs and ASPICE artifacts.
pub fn render_trace_matrix_markdown(matrix: &TraceMatrix) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# ASPICE Bidirectional Traceability Matrix — `{}`\n", matrix.model);
    let s = &matrix.summary;
    let _ = writeln!(
        out,
        "- **Requirements:** {} total | {} satisfied ({:.1}%) | {} constrained | {} verified PASS / {} FAIL / {} unverified ({:.1}% constrained coverage)",
        s.total_requirements,
        s.satisfied_requirements,
        s.satisfaction_coverage_pct,
        s.constrained_requirements,
        s.verified_pass,
        s.verified_fail,
        s.unverified,
        s.verification_coverage_pct,
    );
    let _ = writeln!(
        out,
        "- **Architecture:** {} part defs | {} action defs | {} perform allocations | {} modeled calcs | {} state defs\n",
        s.total_parts, s.total_actions, s.total_perform_allocations, s.total_calcs, s.total_state_defs
    );
    if s.placeholder_evidence > 0 || s.hollow_requirements > 0 {
        let _ = writeln!(
            out,
            "- **Evidence:** {} constrained requirement(s) rely only on model values (not verification evidence) | {} hollow (measured value equals the limit)\n",
            s.placeholder_evidence, s.hollow_requirements
        );
    }
    let _ = writeln!(
        out,
        "| Req ID | Requirement | Package | Satisfied By | Allocated Functions | Constraints | Verification |"
    );
    let _ = writeln!(
        out,
        "| --- | --- | --- | --- | --- | --- | --- |"
    );
    for row in &matrix.rows {
        let sat = if row.satisfied_by.is_empty() {
            "—".to_string()
        } else {
            row.satisfied_by.join(", ")
        };
        let fns = if row.allocated_functions.is_empty() {
            "—".to_string()
        } else {
            row.allocated_functions.join(", ")
        };
        let cons = if row.constraints.is_empty() {
            "—".to_string()
        } else {
            row.constraints
                .iter()
                .map(|c| format!("`{}`", c.replace('|', "\\|")))
                .collect::<Vec<_>>()
                .join("; ")
        };
        let status = match row.verification_status.as_str() {
            "pass" => "PASS",
            "fail" => "FAIL",
            other => other,
        };
        let mut scenario = match &row.verification_scenario {
            Some(s) if !s.is_empty() => format!(" (scenario: {})", s.replace('|', "\\|")),
            _ => String::new(),
        };
        match row.evidence_source.as_str() {
            "model default (placeholder)" => scenario.push_str(" — evidence is a model placeholder"),
            "unmarked" => scenario.push_str(" — no `evidence` attribute"),
            _ => {}
        }
        if !row.hollow_comparisons.is_empty() {
            scenario.push_str(&format!(
                " — HOLLOW: {}",
                row.hollow_comparisons.join("; ").replace('|', "\\|")
            ));
        }
        let _ = writeln!(
            out,
            "| `{}` | `{}` | `{}` | {} | {} | {} | **{}**{} |",
            row.req_id, row.req_name, row.package, sat, fns, cons, status, scenario
        );
    }
    out
}

/// Render a `TraceMatrix` as RFC-4180 CSV for DOORS / Polarion / spreadsheet import.
pub fn render_trace_matrix_csv(matrix: &TraceMatrix) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "req_id,req_name,package,satisfied_by,allocated_functions,constraints,refines,verification_status,verification_scenario,evidence_source"
    );
    for row in &matrix.rows {
        let fields = [
            row.req_id.clone(),
            row.req_name.clone(),
            row.package.clone(),
            row.satisfied_by.join("; "),
            row.allocated_functions.join("; "),
            row.constraints.join("; "),
            row.refines.join("; "),
            row.verification_status.clone(),
            row.verification_scenario.clone().unwrap_or_default(),
            row.evidence_source.clone(),
        ];
        let line = fields
            .iter()
            .map(|f| csv_escape(f))
            .collect::<Vec<_>>()
            .join(",");
        let _ = writeln!(out, "{line}");
    }
    out
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

#[allow(clippy::too_many_arguments)]
fn index_pkg<'a>(
    pkg: &'a Package,
    req_defs: &mut Vec<(String, &'a RequirementDef)>,
    part_defs: &mut BTreeMap<String, &'a PartDef>,
    req_satisfiers: &mut BTreeMap<String, BTreeSet<String>>,
    modeled_calcs: &mut Vec<String>,
    total_actions: &mut usize,
    total_performs: &mut usize,
    total_states: &mut usize,
) {
    for member in &pkg.members {
        match member {
            PackageMember::Package(sub) => index_pkg(
                sub,
                req_defs,
                part_defs,
                req_satisfiers,
                modeled_calcs,
                total_actions,
                total_performs,
                total_states,
            ),
            PackageMember::RequirementDef(r) => {
                req_defs.push((pkg.name.clone(), r));
            }
            PackageMember::PartDef(p) => {
                part_defs.insert(p.name.clone(), p);
                *total_performs += p.performs.len();
                for s in &p.satisfies {
                    let target = s.by.clone().unwrap_or_else(|| p.name.clone());
                    req_satisfiers
                        .entry(simple_name(&s.requirement))
                        .or_default()
                        .insert(target);
                }
                for ru in &p.requirements {
                    index_req_usage(ru, req_satisfiers);
                }
            }
            PackageMember::RequirementUsage(ru) => {
                index_req_usage(ru, req_satisfiers);
            }
            PackageMember::ActionDef(_) => {
                *total_actions += 1;
            }
            PackageMember::StateDef(_) => {
                *total_states += 1;
            }
            PackageMember::CalcDef(c) => {
                modeled_calcs.push(format!("{}::{}", pkg.name, c.name));
            }
            _ => {}
        }
    }
}

fn index_req_usage(ru: &RequirementUsage, req_satisfiers: &mut BTreeMap<String, BTreeSet<String>>) {
    let mut usage_to_def: BTreeMap<String, String> = BTreeMap::new();
    for nested in &ru.nested_requirements {
        if let Some(ty) = &nested.type_name {
            usage_to_def.insert(nested.name.clone(), simple_name(ty));
        }
        index_req_usage(nested, req_satisfiers);
    }
    for s in &ru.satisfies {
        let key = simple_name(&s.requirement);
        let def_key = usage_to_def.get(&key).cloned().unwrap_or(key);
        let target = s
            .by
            .clone()
            .unwrap_or_else(|| ru.name.clone());
        req_satisfiers.entry(def_key).or_default().insert(target);
    }
}

fn build_part_performs(part_defs: &BTreeMap<String, &PartDef>) -> BTreeMap<String, BTreeSet<String>> {
    let mut out = BTreeMap::new();
    for name in part_defs.keys() {
        let mut visited = BTreeSet::new();
        let mut fns = BTreeSet::new();
        collect_performs_recursive(name, part_defs, &mut visited, &mut fns);
        out.insert(name.clone(), fns);
    }
    out
}

fn collect_performs_recursive(
    name: &str,
    part_defs: &BTreeMap<String, &PartDef>,
    visited: &mut BTreeSet<String>,
    out: &mut BTreeSet<String>,
) {
    let simple = simple_name(name);
    if !visited.insert(simple.clone()) {
        return;
    }
    let Some(pd) = part_defs.get(&simple) else {
        return;
    };
    for perf in &pd.performs {
        if let Some(ty) = &perf.action_type {
            out.insert(simple_name(ty));
        } else if let Some(n) = &perf.name {
            out.insert(n.clone());
        }
    }
    for base in &pd.specializes {
        collect_performs_recursive(base, part_defs, visited, out);
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

fn upper_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
    }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
