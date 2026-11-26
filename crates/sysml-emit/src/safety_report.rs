//! ISO 26262 Safety, FMEA Ranking & Minimal Cut-Set Analysis (`rsysdsl safety-report`).
//!
//! Extracts safety goals (`@SafetyGoal`), verified ASIL decompositions (`@AsilDecomposition`),
//! FMEA entries (`@FMEA`, sorted descending by RPN = S * O * D), and computes fault cut sets
//! (distinguishing 1-element Single-Point Faults from 2-element decomposed dual-channel cut sets).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use sysml_ir::{Model, Package, PackageMember};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsilDecompositionSummary {
    pub owner_part: String,
    pub goal: String,
    pub left_part: String,
    pub left_asil: String,
    pub right_part: String,
    pub right_asil: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyGoalSummary {
    pub req_id: String,
    pub req_name: String,
    pub package: String,
    pub asil: String,
    pub safe_state: String,
    pub ftti_ms: u32,
    pub satisfied_by: Vec<String>,
    pub decompositions: Vec<AsilDecompositionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FmeaSummary {
    pub part_name: String,
    pub package: String,
    pub failure_mode: String,
    pub effect: String,
    pub severity: u8,
    pub occurrence: u8,
    pub detection: u8,
    pub rpn: u16,
    pub asil: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FaultCutSet {
    pub order: usize,
    pub kind: String,
    pub target: String,
    pub elements: Vec<String>,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyReport {
    pub model_name: String,
    pub total_safety_goals: usize,
    pub total_decompositions: usize,
    pub total_fmea_entries: usize,
    pub max_rpn: u16,
    pub single_point_fault_count: usize,
    pub dual_channel_cut_set_count: usize,
    pub safety_goals: Vec<SafetyGoalSummary>,
    pub fmea_entries: Vec<FmeaSummary>,
    pub cut_sets: Vec<FaultCutSet>,
}

pub fn build_safety_report(model_name: impl Into<String>, model: &Model) -> SafetyReport {
    let mut goals: Vec<SafetyGoalSummary> = Vec::new();
    let mut decomps: Vec<AsilDecompositionSummary> = Vec::new();
    let mut fmea_entries: Vec<FmeaSummary> = Vec::new();
    let mut satisfied_by_map: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for pkg in &model.packages {
        collect_pkg(
            pkg,
            "",
            &mut goals,
            &mut decomps,
            &mut fmea_entries,
            &mut satisfied_by_map,
        );
    }

    // Attach satisfying parts and ASIL decompositions to each safety goal
    let mut decomposed_parts: BTreeSet<String> = BTreeSet::new();
    for d in &decomps {
        decomposed_parts.insert(d.left_part.clone());
        decomposed_parts.insert(d.right_part.clone());
    }

    for g in &mut goals {
        let mut sats = BTreeSet::new();
        if let Some(s) = satisfied_by_map.get(&g.req_name) {
            sats.extend(s.iter().cloned());
        }
        if let Some(s) = satisfied_by_map.get(&g.req_id) {
            sats.extend(s.iter().cloned());
        }
        g.satisfied_by = sats.into_iter().collect();
        g.decompositions = decomps
            .iter()
            .filter(|d| d.goal == g.req_id || d.goal == g.req_name)
            .cloned()
            .collect();
    }

    // Sort FMEA entries descending by RPN, then severity, then part_name
    fmea_entries.sort_by(|a, b| {
        b.rpn
            .cmp(&a.rpn)
            .then_with(|| b.severity.cmp(&a.severity))
            .then_with(|| a.part_name.cmp(&b.part_name))
            .then_with(|| a.failure_mode.cmp(&b.failure_mode))
    });

    // Compute minimal fault cut sets
    let mut cut_sets: Vec<FaultCutSet> = Vec::new();
    let mut dual_channel_cut_set_count = 0usize;
    let mut single_point_fault_count = 0usize;

    for d in &decomps {
        let left_modes: Vec<String> = fmea_entries
            .iter()
            .filter(|f| f.part_name == d.left_part)
            .map(|f| format!("{}:{}", f.part_name, f.failure_mode))
            .collect();
        let right_modes: Vec<String> = fmea_entries
            .iter()
            .filter(|f| f.part_name == d.right_part)
            .map(|f| format!("{}:{}", f.part_name, f.failure_mode))
            .collect();

        if !left_modes.is_empty() && !right_modes.is_empty() {
            for lm in &left_modes {
                for rm in &right_modes {
                    dual_channel_cut_set_count += 1;
                    cut_sets.push(FaultCutSet {
                        order: 2,
                        kind: "dual_channel_decomposed".to_string(),
                        target: d.goal.clone(),
                        elements: vec![lm.clone(), rm.clone()],
                        note: format!(
                            "Independent ASIL decomposition ({} [{}] + {} [{}]) inside {}",
                            d.left_part, d.left_asil, d.right_part, d.right_asil, d.owner_part
                        ),
                    });
                }
            }
        } else {
            dual_channel_cut_set_count += 1;
            cut_sets.push(FaultCutSet {
                order: 2,
                kind: "dual_channel_decomposed".to_string(),
                target: d.goal.clone(),
                elements: vec![d.left_part.clone(), d.right_part.clone()],
                note: format!(
                    "Independent ASIL decomposition ({} [{}] + {} [{}]) inside {}",
                    d.left_part, d.left_asil, d.right_part, d.right_asil, d.owner_part
                ),
            });
        }
    }

    for f in &fmea_entries {
        if !decomposed_parts.contains(&f.part_name) {
            single_point_fault_count += 1;
            cut_sets.push(FaultCutSet {
                order: 1,
                kind: "single_point_fault".to_string(),
                target: f.effect.clone(),
                elements: vec![format!("{}:{}", f.part_name, f.failure_mode)],
                note: format!(
                    "Unmitigated single-point fault (RPN={}, S={}, O={}, D={})",
                    f.rpn, f.severity, f.occurrence, f.detection
                ),
            });
        }
    }

    let max_rpn = fmea_entries.iter().map(|f| f.rpn).max().unwrap_or(0);

    SafetyReport {
        model_name: model_name.into(),
        total_safety_goals: goals.len(),
        total_decompositions: decomps.len(),
        total_fmea_entries: fmea_entries.len(),
        max_rpn,
        single_point_fault_count,
        dual_channel_cut_set_count,
        safety_goals: goals,
        fmea_entries,
        cut_sets,
    }
}

pub fn render_safety_report_markdown(report: &SafetyReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# ISO 26262 Safety, FMEA & Cut-Set Report — `{}`\n", report.model_name);
    let _ = writeln!(
        out,
        "- **Safety Goals:** {} | **ASIL Decompositions:** {} | **FMEA Entries:** {} (max RPN: {})",
        report.total_safety_goals,
        report.total_decompositions,
        report.total_fmea_entries,
        report.max_rpn
    );
    let _ = writeln!(
        out,
        "- **Fault Cut Sets:** {} single-point fault(s) (order 1) | {} dual-channel cut set(s) (order 2)\n",
        report.single_point_fault_count, report.dual_channel_cut_set_count
    );

    if !report.safety_goals.is_empty() {
        let _ = writeln!(out, "## Safety Goals (`@SafetyGoal`)\n");
        let _ = writeln!(out, "| Goal ID | Name | ASIL | Safe State | FTTI (ms) | Satisfied By | Decomposition |");
        let _ = writeln!(out, "| --- | --- | --- | --- | ---: | --- | --- |");
        for g in &report.safety_goals {
            let sats = if g.satisfied_by.is_empty() {
                "—".to_string()
            } else {
                g.satisfied_by.join(", ")
            };
            let decomp = if g.decompositions.is_empty() {
                "—".to_string()
            } else {
                g.decompositions
                    .iter()
                    .map(|d| format!("{} ({}) + {} ({})", d.left_part, d.left_asil, d.right_part, d.right_asil))
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            let _ = writeln!(
                out,
                "| `{}` | `{}` | **{}** | `{}` | {} | {} | {} |",
                g.req_id, g.req_name, g.asil, g.safe_state, g.ftti_ms, sats, decomp
            );
        }
        out.push('\n');
    }

    if !report.fmea_entries.is_empty() {
        let _ = writeln!(out, "## FMEA Table (`@FMEA`, Ranked by RPN)\n");
        let _ = writeln!(out, "| Part | Failure Mode | Effect | S | O | D | RPN | ASIL |");
        let _ = writeln!(out, "| --- | --- | --- | ---: | ---: | ---: | ---: | --- |");
        for f in &report.fmea_entries {
            let _ = writeln!(
                out,
                "| `{}` | `{}` | `{}` | {} | {} | {} | **{}** | {} |",
                f.part_name,
                f.failure_mode,
                f.effect,
                f.severity,
                f.occurrence,
                f.detection,
                f.rpn,
                f.asil.as_deref().unwrap_or("—")
            );
        }
        out.push('\n');
    }

    if !report.cut_sets.is_empty() {
        let _ = writeln!(out, "## Minimal Fault Cut Sets\n");
        let _ = writeln!(out, "| Order | Kind | Target | Cut-Set Elements | Notes |");
        let _ = writeln!(out, "| ---: | --- | --- | --- | --- |");
        for cs in &report.cut_sets {
            let _ = writeln!(
                out,
                "| {} | `{}` | `{}` | `{}` | {} |",
                cs.order,
                cs.kind,
                cs.target,
                cs.elements.join(" ∧ "),
                cs.note
            );
        }
    }

    out
}

fn collect_pkg(
    pkg: &Package,
    prefix: &str,
    goals: &mut Vec<SafetyGoalSummary>,
    decomps: &mut Vec<AsilDecompositionSummary>,
    fmea_entries: &mut Vec<FmeaSummary>,
    satisfied_by_map: &mut BTreeMap<String, BTreeSet<String>>,
) {
    let qpkg = if prefix.is_empty() {
        pkg.name.clone()
    } else {
        format!("{prefix}::{}", pkg.name)
    };
    for m in &pkg.members {
        match m {
            PackageMember::Package(sub) => {
                collect_pkg(sub, &qpkg, goals, decomps, fmea_entries, satisfied_by_map);
            }
            PackageMember::RequirementDef(r) => {
                if let Some(sg) = &r.safety_goal {
                    goals.push(SafetyGoalSummary {
                        req_id: r.short_name.clone().unwrap_or_else(|| r.name.clone()),
                        req_name: r.name.clone(),
                        package: qpkg.clone(),
                        asil: sg.asil.clone(),
                        safe_state: sg.safe_state.clone(),
                        ftti_ms: sg.ftti_ms,
                        satisfied_by: Vec::new(),
                        decompositions: Vec::new(),
                    });
                }
            }
            PackageMember::PartDef(p) => {
                for sat in &p.satisfies {
                    let key = sat
                        .requirement
                        .rsplit("::")
                        .next()
                        .unwrap_or(&sat.requirement)
                        .trim_matches('\'')
                        .to_string();
                    satisfied_by_map
                        .entry(key)
                        .or_default()
                        .insert(p.name.clone());
                }
                for f in &p.fmea {
                    fmea_entries.push(FmeaSummary {
                        part_name: p.name.clone(),
                        package: qpkg.clone(),
                        failure_mode: f.failure_mode.clone(),
                        effect: f.effect.clone(),
                        severity: f.severity,
                        occurrence: f.occurrence,
                        detection: f.detection,
                        rpn: f.rpn(),
                        asil: f.asil.clone(),
                    });
                }
                for d in &p.asil_decompositions {
                    decomps.push(AsilDecompositionSummary {
                        owner_part: p.name.clone(),
                        goal: d.goal.clone(),
                        left_part: d.left_part.clone(),
                        left_asil: d.left_asil.clone(),
                        right_part: d.right_part.clone(),
                        right_asil: d.right_asil.clone(),
                    });
                }
            }
            _ => {}
        }
    }
}
