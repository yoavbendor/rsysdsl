//! Phase B3 — `refute`: Requirement Contradiction & Feasibility Prover.
//!
//! Given a model's `#[sysml::variation]` design space and a set of active requirements
//! (from model `RequirementDef`s + candidate stakeholder limits), proves whether *any*
//! feasible design exists. When **UNSAT (Refuted)**, computes:
//! 1. **Minimal Unsatisfiable Core (MUC)** — the smallest subset of constraints that
//!    cannot be satisfied simultaneously.
//! 2. **Exact Minimum Relaxations** — the smallest single-requirement relaxation that
//!    restores feasibility, along with the winning witness variant.

use serde::{Deserialize, Serialize};
use sysml_ir::{AttributeUsage, Model, PackageMember};

use crate::budget::BudgetLimits;
use crate::emit_cli::is_sysml_std_package;
use crate::trade_study::{run_trade_study, TradePoint};

/// Candidate stakeholder / requirement bounds to test for feasibility.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefuteLimits {
    pub min_runtime_s: Option<f64>,
    pub max_mass_kg: Option<f64>,
    pub max_power_w: Option<f64>,
    pub max_noise_db: Option<f64>,
    pub max_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveConstraint {
    pub id: String,
    pub metric: String,
    pub op: String,
    pub limit: f64,
    pub unit: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelaxationSuggestion {
    pub constraint_id: String,
    pub metric: String,
    pub current_limit: f64,
    pub required_limit: f64,
    pub delta: f64,
    pub unit: String,
    pub witness_variant: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefuteReport {
    /// `true` when at least one feasible variant exists; `false` when the requirement set is REFUTED (UNSAT).
    pub feasible: bool,
    pub total_variants_searched: usize,
    pub witness_variants: Vec<String>,
    pub active_constraints: Vec<ActiveConstraint>,
    /// Minimal Unsatisfiable Core (non-empty only when `feasible == false`).
    pub minimal_unsat_core: Vec<ActiveConstraint>,
    /// Minimum single-constraint relaxations that restore feasibility (non-empty when `feasible == false`).
    pub relaxations: Vec<RelaxationSuggestion>,
}

/// Test whether `model` + `extra_limits` has a feasible design; if UNSAT, compute the
/// Minimal Unsatisfiable Core and exact minimum relaxations.
pub fn refute_requirements(model: &Model, extra: &RefuteLimits) -> RefuteReport {
    // 1. Collect all design points (unconstrained trade-study) + model requirement bounds
    let base_study = run_trade_study(model, &BudgetLimits::default());
    let points = &base_study.points;

    let mut constraints: Vec<ActiveConstraint> = Vec::new();
    let mut model_min_rt: Option<(String, f64)> = None;
    let mut model_max_db: Option<(String, f64)> = None;

    for pkg in &model.packages {
        if is_sysml_std_package(pkg) {
            continue;
        }
        for m in &pkg.members {
            if let PackageMember::RequirementDef(r) = m {
                let rid = r.short_name.clone().unwrap_or_else(|| r.name.clone());
                if let Some(v) = attr_number(&r.attributes, &["minRuntimeS", "minRuntime"]) {
                    if model_min_rt.as_ref().is_none_or(|(_, cur)| v > *cur) {
                        model_min_rt = Some((rid.clone(), v));
                    }
                }
                if let Some(v) = attr_number(&r.attributes, &["maxNoiseDb"]) {
                    if model_max_db.as_ref().is_none_or(|(_, cur)| v < *cur) {
                        model_max_db = Some((rid.clone(), v));
                    }
                }
            }
        }
    }

    if let Some(v) = extra.min_runtime_s {
        constraints.push(ActiveConstraint {
            id: "CLI-MIN-RUNTIME".into(),
            metric: "runtime_s".into(),
            op: ">=".into(),
            limit: v,
            unit: "s".into(),
        });
    } else if let Some((rid, v)) = model_min_rt {
        constraints.push(ActiveConstraint {
            id: rid,
            metric: "runtime_s".into(),
            op: ">=".into(),
            limit: v,
            unit: "s".into(),
        });
    }

    if let Some(v) = extra.max_noise_db {
        constraints.push(ActiveConstraint {
            id: "CLI-MAX-NOISE".into(),
            metric: "noise_db".into(),
            op: "<=".into(),
            limit: v,
            unit: "dB".into(),
        });
    } else if let Some((rid, v)) = model_max_db {
        constraints.push(ActiveConstraint {
            id: rid,
            metric: "noise_db".into(),
            op: "<=".into(),
            limit: v,
            unit: "dB".into(),
        });
    }

    if let Some(v) = extra.max_mass_kg {
        constraints.push(ActiveConstraint {
            id: "CLI-MAX-MASS".into(),
            metric: "mass_kg".into(),
            op: "<=".into(),
            limit: v,
            unit: "kg".into(),
        });
    }
    if let Some(v) = extra.max_power_w {
        constraints.push(ActiveConstraint {
            id: "CLI-MAX-POWER".into(),
            metric: "power_w".into(),
            op: "<=".into(),
            limit: v,
            unit: "W".into(),
        });
    }
    if let Some(v) = extra.max_cost_usd {
        constraints.push(ActiveConstraint {
            id: "CLI-MAX-COST".into(),
            metric: "cost_usd".into(),
            op: "<=".into(),
            limit: v,
            unit: "USD".into(),
        });
    }

    let witnesses = feasible_witnesses(points, &constraints);
    if !witnesses.is_empty() || points.is_empty() {
        return RefuteReport {
            feasible: !witnesses.is_empty(),
            total_variants_searched: points.len(),
            witness_variants: witnesses,
            active_constraints: constraints,
            minimal_unsat_core: Vec::new(),
            relaxations: Vec::new(),
        };
    }

    // 2. Compute Minimal Unsatisfiable Core (MUC): greedily drop any constraint that
    //    is not needed to keep the remaining set UNSAT.
    let mut muc = constraints.clone();
    let mut i = 0;
    while i < muc.len() {
        let mut trial = muc.clone();
        trial.remove(i);
        if !trial.is_empty() && feasible_witnesses(points, &trial).is_empty() {
            muc = trial;
        } else {
            i += 1;
        }
    }

    // 3. For each constraint in the MUC, compute the exact minimum relaxation that
    //    restores feasibility (assuming all other constraints remain fixed).
    let mut relaxations = Vec::new();
    for (idx, c) in muc.iter().enumerate() {
        let others: Vec<ActiveConstraint> = muc
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != idx)
            .map(|(_, x)| x.clone())
            .collect();

        let mut best: Option<(f64, f64, String)> = None; // (abs_delta, required_limit, witness_id)
        for p in points {
            if !satisfies_all(p, &others) {
                continue;
            }
            if let Some(val) = metric_value(p, &c.metric) {
                let delta = val - c.limit;
                let abs_d = delta.abs();
                if best.as_ref().is_none_or(|(bd, _, _)| abs_d < *bd) {
                    best = Some((abs_d, val, p.id.clone()));
                }
            }
        }
        if let Some((_, req_lim, witness)) = best {
            relaxations.push(RelaxationSuggestion {
                constraint_id: c.id.clone(),
                metric: c.metric.clone(),
                current_limit: round2(c.limit),
                required_limit: round2(req_lim),
                delta: round2(req_lim - c.limit),
                unit: c.unit.clone(),
                witness_variant: witness,
            });
        }
    }

    RefuteReport {
        feasible: false,
        total_variants_searched: points.len(),
        witness_variants: Vec::new(),
        active_constraints: constraints,
        minimal_unsat_core: muc,
        relaxations,
    }
}

/// Render [`RefuteReport`] as clean Markdown.
pub fn render_refute_markdown(r: &RefuteReport) -> String {
    let mut out = String::new();
    out.push_str("# Requirement Feasibility & Refutation Report\n\n");
    out.push_str(&format!(
        "- **Status:** {} ({} variants searched)\n",
        if r.feasible {
            "FEASIBLE (SAT)"
        } else {
            "REFUTED (UNSAT — No Feasible Design)"
        },
        r.total_variants_searched
    ));
    if r.feasible {
        out.push_str(&format!(
            "- **Feasible Witness Variants:** {}\n",
            r.witness_variants.join(", ")
        ));
    } else {
        out.push_str("\n## Minimal Unsatisfiable Core (Conflicting Requirements)\n\n");
        for c in &r.minimal_unsat_core {
            out.push_str(&format!(
                "- **`{}`**: `{} {} {:.2} {}`\n",
                c.id, c.metric, c.op, c.limit, c.unit
            ));
        }
        out.push_str("\n## Minimum Relaxations to Restore Feasibility\n\n");
        for rx in &r.relaxations {
            out.push_str(&format!(
                "- Relax **`{}`** (`{}`) from `{:.2} {}` to **`{:.2} {}`** (`{:+.2} {}`) -> feasible via **`{}`**\n",
                rx.constraint_id,
                rx.metric,
                rx.current_limit,
                rx.unit,
                rx.required_limit,
                rx.unit,
                rx.delta,
                rx.unit,
                rx.witness_variant
            ));
        }
    }
    out
}

fn feasible_witnesses(points: &[TradePoint], constraints: &[ActiveConstraint]) -> Vec<String> {
    points
        .iter()
        .filter(|p| satisfies_all(p, constraints))
        .map(|p| p.id.clone())
        .collect()
}

fn satisfies_all(p: &TradePoint, constraints: &[ActiveConstraint]) -> bool {
    for c in constraints {
        let Some(val) = metric_value(p, &c.metric) else {
            return false;
        };
        let ok = match c.op.as_str() {
            ">=" => val + 1e-9 >= c.limit,
            "<=" => val <= c.limit + 1e-9,
            _ => true,
        };
        if !ok {
            return false;
        }
    }
    true
}

fn metric_value(p: &TradePoint, metric: &str) -> Option<f64> {
    match metric {
        "runtime_s" => p.estimated_runtime_s,
        "mass_kg" => Some(p.mass_kg),
        "power_w" => Some(p.rated_power_w),
        "noise_db" => p.max_noise_db,
        "cost_usd" => Some(p.unit_cost_usd),
        _ => None,
    }
}

fn attr_number(attrs: &[AttributeUsage], names: &[&str]) -> Option<f64> {
    for a in attrs {
        let aname = a
            .name
            .as_deref()
            .or_else(|| a.redefines.first().map(String::as_str))?;
        if names.iter().any(|n| n.eq_ignore_ascii_case(aname)) {
            let s = a.value.as_deref()?.trim();
            let tok = s.split_whitespace().next()?;
            if let Ok(v) = tok.parse::<f64>() {
                if v.is_finite() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn round2(v: f64) -> f64 {
    let r = (v * 100.0).round() / 100.0;
    if r == 0.0 {
        0.0
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysml_ir::{Package, PartDef, RequirementDef, VariantPart};

    fn attr(name: &str, val: &str) -> AttributeUsage {
        AttributeUsage {
            name: Some(name.into()),
            value: Some(val.into()),
            ..Default::default()
        }
    }

    #[test]
    fn refutes_impossible_mass_and_runtime_and_computes_exact_muc_and_relaxation() {
        let mut pkg = Package {
            name: "RefuteDemo".into(),
            ..Default::default()
        };
        pkg.members.push(PackageMember::RequirementDef(RequirementDef {
            name: "RuntimeReq".into(),
            short_name: Some("TOOL-001".into()),
            attributes: vec![attr("minRuntimeS", "300.0")],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "Batt2Ah".into(),
            attributes: vec![attr("energyCapacity", "129600.0"), attr("mass", "0.40")],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "Batt5Ah".into(),
            attributes: vec![attr("energyCapacity", "324000.0"), attr("mass", "0.68")],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "BatteryVar".into(),
            is_variation: true,
            variants: vec![
                VariantPart { name: "Compact2Ah".into(), type_name: "Batt2Ah".into(), doc: None },
                VariantPart { name: "Pro5Ah".into(), type_name: "Batt5Ah".into(), doc: None },
            ],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "Drill".into(),
            attributes: vec![attr("ratedPower", "550.0"), attr("mass", "1.95"), attr("maxNoiseDb", "79.0")],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "ToolVar".into(),
            is_variation: true,
            variants: vec![VariantPart { name: "Drill".into(), type_name: "Drill".into(), doc: None }],
            ..Default::default()
        }));

        let model = Model::new(vec![pkg]);

        // Demand BOTH runtime >= 300s (needs Pro5Ah -> 2.63 kg) AND mass <= 2.40 kg (needs Compact2Ah -> 235.6 s)
        let rep = refute_requirements(
            &model,
            &RefuteLimits {
                max_mass_kg: Some(2.40),
                ..Default::default()
            },
        );
        assert!(!rep.feasible);
        assert_eq!(rep.minimal_unsat_core.len(), 2);
        assert!(rep.relaxations.iter().any(|rx| {
            rx.constraint_id == "CLI-MAX-MASS"
                && (rx.required_limit - 2.63).abs() < 1e-6
                && rx.witness_variant == "Pro5Ah + Drill"
        }));
        assert!(rep.relaxations.iter().any(|rx| {
            rx.constraint_id == "TOOL-001"
                && (rx.required_limit - 235.64).abs() < 0.1
                && rx.witness_variant == "Compact2Ah + Drill"
        }));
    }
}
