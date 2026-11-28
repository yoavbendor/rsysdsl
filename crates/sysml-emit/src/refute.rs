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
use sysml_ir::Model;

pub use crate::trade_study::{ActiveConstraint, UnevaluatedConstraint};
use crate::trade_study::{
    design_points, drop_unmeasurable, metric_value, requirement_constraints, satisfies, TradePoint,
};

/// Candidate stakeholder / requirement bounds to test for feasibility.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RefuteLimits {
    pub min_runtime_s: Option<f64>,
    pub max_mass_kg: Option<f64>,
    pub max_power_w: Option<f64>,
    /// No structural noise source exists until C3 evaluates `require_constraint`s; a bound
    /// given here is listed under `unevaluated` and never decides feasibility.
    pub max_noise_db: Option<f64>,
    pub max_cost_usd: Option<f64>,
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

/// Three-valued refutation verdict (D3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefuteStatus {
    /// At least one design point satisfies every active constraint.
    Feasible,
    /// No design point does: see the minimal unsatisfiable core.
    Refuted,
    /// Nothing to decide: the model has no `#[sysml::variation]` design space, or no
    /// constraint could be applied.
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefuteReport {
    pub status: RefuteStatus,
    /// `true` (feasible) / `false` (refuted, UNSAT); `null` when `not_applicable`.
    pub feasible: Option<bool>,
    pub total_variants_searched: usize,
    pub witness_variants: Vec<String>,
    pub active_constraints: Vec<ActiveConstraint>,
    /// Requirement constraints / CLI bounds that map onto no design-point metric.
    pub unevaluated: Vec<UnevaluatedConstraint>,
    /// Minimal Unsatisfiable Core (non-empty only when `status == "refuted"`).
    pub minimal_unsat_core: Vec<ActiveConstraint>,
    /// Minimum single-constraint relaxations that restore feasibility (when refuted).
    pub relaxations: Vec<RelaxationSuggestion>,
}

/// Test whether `model` + `extra` limits has a feasible design; if UNSAT, compute the
/// Minimal Unsatisfiable Core and exact minimum relaxations.
///
/// Design points and their metrics come from budget roles (see [`crate::trade_study`]); model
/// requirements are recognised structurally from their `require_constraint` (a comparison of
/// two requirement attributes whose bound dimension maps onto a metric), never by name.
pub fn refute_requirements(model: &Model, extra: &RefuteLimits) -> RefuteReport {
    let (_, points, _) = design_points(model);
    let (model_constraints, mut unevaluated) = requirement_constraints(model);

    let cli = |id: &str, metric: &str, op: &str, limit: f64, unit: &str| ActiveConstraint {
        id: id.into(),
        metric: metric.into(),
        op: op.into(),
        limit,
        unit: unit.into(),
    };
    let mut constraints: Vec<ActiveConstraint> = Vec::new();
    for c in model_constraints {
        // A CLI runtime bound replaces the model's runtime requirement (candidate what-if).
        if c.metric == "runtime_s" && extra.min_runtime_s.is_some() {
            continue;
        }
        constraints.push(c);
    }
    if let Some(v) = extra.min_runtime_s {
        constraints.push(cli("CLI-MIN-RUNTIME", "runtime_s", ">=", v, "s"));
    }
    if let Some(v) = extra.max_mass_kg {
        constraints.push(cli("CLI-MAX-MASS", "mass_kg", "<=", v, "kg"));
    }
    if let Some(v) = extra.max_power_w {
        constraints.push(cli("CLI-MAX-POWER", "power_w", "<=", v, "W"));
    }
    if let Some(v) = extra.max_cost_usd {
        constraints.push(cli("CLI-MAX-COST", "cost_usd", "<=", v, "USD"));
    }
    if let Some(v) = extra.max_noise_db {
        unevaluated.push(UnevaluatedConstraint {
            id: "CLI-MAX-NOISE".into(),
            constraint: format!("noise_db <= {v}"),
            reason: "no structural noise metric on design points (C3 evaluates require_constraint expressions)".into(),
        });
    }

    let constraints = drop_unmeasurable(&points, constraints, &mut unevaluated);
    if points.is_empty() || constraints.is_empty() {
        return RefuteReport {
            status: RefuteStatus::NotApplicable,
            feasible: None,
            total_variants_searched: points.len(),
            witness_variants: Vec::new(),
            active_constraints: constraints,
            unevaluated,
            minimal_unsat_core: Vec::new(),
            relaxations: Vec::new(),
        };
    }

    let witnesses = feasible_witnesses(&points, &constraints);
    if !witnesses.is_empty() {
        return RefuteReport {
            status: RefuteStatus::Feasible,
            feasible: Some(true),
            total_variants_searched: points.len(),
            witness_variants: witnesses,
            active_constraints: constraints,
            unevaluated,
            minimal_unsat_core: Vec::new(),
            relaxations: Vec::new(),
        };
    }

    // Minimal Unsatisfiable Core (MUC): greedily drop any constraint that is not needed to
    // keep the remaining set UNSAT.
    let mut muc = constraints.clone();
    let mut i = 0;
    while i < muc.len() {
        let mut trial = muc.clone();
        trial.remove(i);
        if !trial.is_empty() && feasible_witnesses(&points, &trial).is_empty() {
            muc = trial;
        } else {
            i += 1;
        }
    }

    // For each constraint in the MUC, the exact minimum relaxation that restores feasibility
    // (all other constraints fixed).
    let mut relaxations = Vec::new();
    for (idx, c) in muc.iter().enumerate() {
        let others: Vec<ActiveConstraint> = muc
            .iter()
            .enumerate()
            .filter(|(j, _)| *j != idx)
            .map(|(_, x)| x.clone())
            .collect();
        let mut best: Option<(f64, f64, String)> = None;
        for p in &points {
            if !others.iter().all(|o| satisfies(p, o)) {
                continue;
            }
            if let Some(val) = metric_value(p, &c.metric) {
                let abs_d = (val - c.limit).abs();
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
        status: RefuteStatus::Refuted,
        feasible: Some(false),
        total_variants_searched: points.len(),
        witness_variants: Vec::new(),
        active_constraints: constraints,
        unevaluated,
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
        match r.status {
            RefuteStatus::Feasible => "FEASIBLE (SAT)",
            RefuteStatus::Refuted => "REFUTED (UNSAT — No Feasible Design)",
            RefuteStatus::NotApplicable =>
                "NOT APPLICABLE (no variation design space or no applicable constraint)",
        },
        r.total_variants_searched
    ));
    for u in &r.unevaluated {
        out.push_str(&format!("- **Not evaluated `{}`:** `{}` ({})\n", u.id, u.constraint, u.reason));
    }
    match r.status {
        RefuteStatus::Feasible => out.push_str(&format!(
            "- **Feasible Witness Variants:** {}\n",
            r.witness_variants.join(", ")
        )),
        RefuteStatus::NotApplicable => {}
        RefuteStatus::Refuted => {
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
    }
    out
}

fn feasible_witnesses(points: &[TradePoint], constraints: &[ActiveConstraint]) -> Vec<String> {
    points
        .iter()
        .filter(|p| constraints.iter().all(|c| satisfies(p, c)))
        .map(|p| p.id.clone())
        .collect()
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
    use sysml_ir::{
        AttributeUsage, BudgetRole, Package, PackageMember, PartDef, RequirementDef, VariantPart,
    };

    fn attr(name: &str, val: &str, unit: &str) -> AttributeUsage {
        AttributeUsage {
            name: Some(name.into()),
            value: Some(val.into()),
            unit: (!unit.is_empty()).then(|| unit.into()),
            ..Default::default()
        }
    }

    fn role(name: &str, val: &str, unit: &str, r: BudgetRole) -> AttributeUsage {
        AttributeUsage {
            budget_role: Some(r),
            ..attr(name, val, unit)
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
            attributes: vec![
                attr("minRuntimeS", "300.0", "s"),
                attr("actualRuntimeS", "589.0", "s"),
            ],
            require_constraints: vec!["actualRuntimeS >= minRuntimeS".into()],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "Batt2Ah".into(),
            attributes: vec![attr("energyCapacity", "129600.0", "J"), role("mass", "0.40", "kg", BudgetRole::Mass)],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "Batt5Ah".into(),
            attributes: vec![attr("energyCapacity", "324000.0", "J"), role("mass", "0.68", "kg", BudgetRole::Mass)],
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
            attributes: vec![role("ratedPower", "550.0", "W", BudgetRole::Power), role("mass", "1.95", "kg", BudgetRole::Mass)],
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
        assert_eq!(rep.status, RefuteStatus::Refuted);
        assert_eq!(rep.feasible, Some(false));
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

    /// P3 regression: no variations and no recognised requirement used to report
    /// `"feasible": false` with an empty core.
    #[test]
    fn no_design_space_is_not_applicable_not_infeasible() {
        let model = Model::new(vec![Package {
            name: "WaterHeater".into(),
            members: vec![PackageMember::PartDef(PartDef {
                name: "Heater".into(),
                attributes: vec![attr("ratedPowerW", "2000.0", "W")],
                ..Default::default()
            })],
            ..Default::default()
        }]);
        for limits in [
            RefuteLimits::default(),
            RefuteLimits {
                max_power_w: Some(100.0),
                ..Default::default()
            },
        ] {
            let rep = refute_requirements(&model, &limits);
            assert_eq!(rep.status, RefuteStatus::NotApplicable);
            assert_eq!(rep.feasible, None);
            assert!(rep.minimal_unsat_core.is_empty());
            let json = serde_json::to_value(&rep).unwrap();
            assert_eq!(json["status"], "not_applicable");
            assert!(json["feasible"].is_null());
        }
    }
}
