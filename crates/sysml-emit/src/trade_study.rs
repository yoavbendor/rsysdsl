//! Phase B2 — `trade-study` (automatic `#[sysml::variation]` Cartesian expansion +
//! Pareto frontier) and `what-if` (Tornado parameter sensitivity ranking).
//!
//! Pure in-memory IR analysis with zero external dependencies; runs in < 1 ms.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sysml_ir::{AttributeUsage, BudgetRole, Model, PackageMember, RequirementDef};

use crate::budget::{analyze_budgets, BudgetLimits, UntrackedAttr};
use crate::emit_cli::is_sysml_std_package;
use crate::roles::{DimClass, RoleIndex};

/// One concrete combination of `#[sysml::variation]` choices evaluated in a trade study.
///
/// Metrics come from budget roles (C1 / D1) of the chosen variant types, never from attribute
/// names: `mass_kg` (mass role), `rated_power_w` (power role), `unit_cost_usd` (cost role),
/// `energy_capacity_j` (attributes of dimension energy, `J`), and the derived
/// `estimated_runtime_s = energy / power`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePoint {
    pub id: String,
    /// Map of variation name -> chosen variant type name (e.g. `BatteryPackVariant -> Battery18V5Ah`).
    pub choices: BTreeMap<String, String>,
    pub mass_kg: f64,
    pub rated_power_w: f64,
    pub energy_capacity_j: f64,
    pub estimated_runtime_s: Option<f64>,
    pub unit_cost_usd: f64,
    pub feasible: bool,
    pub pareto_optimal: bool,
    pub failed_checks: Vec<String>,
}

/// One bound checked on every design point (`metric op limit`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveConstraint {
    pub id: String,
    pub metric: String,
    pub op: String,
    pub limit: f64,
    pub unit: String,
}

/// A requirement constraint (or CLI bound) the operator could not map onto a design-point
/// metric. Listed, never silently dropped; C2/C3 evaluate `require_constraint` expressions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnevaluatedConstraint {
    pub id: String,
    pub constraint: String,
    pub reason: String,
}

/// `ok` when there is a design space to expand, `not_applicable` when the model has no
/// `#[sysml::variation]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeStudyStatus {
    Ok,
    NotApplicable,
}

/// Complete trade-study report over all `#[sysml::variation]` combinations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeStudyReport {
    pub status: TradeStudyStatus,
    pub variation_count: usize,
    pub total_combinations: usize,
    pub feasible_count: usize,
    pub pareto_count: usize,
    /// Model requirement constraints and CLI limits applied to every point.
    pub constraints: Vec<ActiveConstraint>,
    /// Requirement constraints that map onto no metric (e.g. temperature, noise).
    pub unevaluated: Vec<UnevaluatedConstraint>,
    /// Attributes of variant types whose dimension is mass or power but that carry no budget
    /// role: they are not in the metrics.
    pub untracked: Vec<UntrackedAttr>,
    pub points: Vec<TradePoint>,
}

/// One ranked Tornado sensitivity entry from [`run_what_if`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensitivityDriver {
    pub element: String,
    pub attribute: String,
    pub base_value: f64,
    pub delta_pct: f64,
    pub affected_metric: String,
    pub low_total: f64,
    pub high_total: f64,
    pub swing: f64,
}

/// Complete `what-if` Tornado sensitivity report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhatIfReport {
    pub delta_pct: f64,
    pub parameters_evaluated: usize,
    pub top_drivers: Vec<SensitivityDriver>,
}

/// User `#[sysml::variation]` definitions: (variation name, [(variant label, variant type)]).
pub(crate) fn collect_variations(model: &Model) -> Vec<(String, Vec<(String, String)>)> {
    let mut out = Vec::new();
    for pkg in model.packages.iter().filter(|p| !is_sysml_std_package(p)) {
        for m in &pkg.members {
            if let PackageMember::PartDef(p) = m {
                if p.is_variation && !p.variants.is_empty() {
                    let opts = p
                        .variants
                        .iter()
                        .map(|v| (v.name.clone(), v.type_name.clone()))
                        .collect();
                    out.push((p.name.clone(), opts));
                }
            }
        }
    }
    out
}

fn is_ident(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Split `lhs OP rhs` for a single comparison of two attribute references.
fn split_comparison(expr: &str) -> Option<(&str, &'static str, &str)> {
    for op in [">=", "<=", ">", "<"] {
        if let Some(i) = expr.find(op) {
            let (l, r) = (expr[..i].trim(), expr[i + op.len()..].trim());
            if is_ident(l) && is_ident(r) {
                return Some((l, op, r));
            }
            return None;
        }
    }
    None
}

fn flip(op: &str) -> &'static str {
    match op {
        ">=" => "<=",
        "<=" => ">=",
        ">" => "<",
        _ => ">",
    }
}

/// Map the model's requirement constraints onto design-point metrics, structurally.
///
/// A constraint is used when it is a single comparison of two attributes of its requirement
/// (`actual OP bound`). The bound is the side that is not `evidence` (or, when neither is, the
/// right-hand side, the `measured <= limit` convention of the transpiler subset). The bound's
/// unit dimension picks the metric: mass with `<=` -> `mass_kg`, power with `<=` -> `power_w`,
/// time with `>=` -> `runtime_s` (the only time metric a design point has: energy / power).
/// Everything else is reported in the second list.
pub(crate) fn requirement_constraints(
    model: &Model,
) -> (Vec<ActiveConstraint>, Vec<UnevaluatedConstraint>) {
    let mut active = Vec::new();
    let mut unevaluated = Vec::new();
    let mut reqs: Vec<&RequirementDef> = Vec::new();
    for pkg in model.packages.iter().filter(|p| !is_sysml_std_package(p)) {
        for m in &pkg.members {
            if let PackageMember::RequirementDef(r) = m {
                reqs.push(r);
            }
        }
    }
    for r in reqs {
        let id = r.short_name.clone().unwrap_or_else(|| r.name.clone());
        let find = |n: &str| -> Option<&AttributeUsage> {
            r.attributes
                .iter()
                .find(|a| crate::roles::attr_key(a).as_deref() == Some(n))
        };
        for c in &r.require_constraints {
            let skip = |reason: &str, out: &mut Vec<UnevaluatedConstraint>| {
                out.push(UnevaluatedConstraint {
                    id: id.clone(),
                    constraint: c.clone(),
                    reason: reason.to_string(),
                })
            };
            let Some((l, op, rhs)) = split_comparison(c) else {
                skip("not a single comparison of two requirement attributes", &mut unevaluated);
                continue;
            };
            let (Some(la), Some(ra)) = (find(l), find(rhs)) else {
                skip("operand is not an attribute of the requirement", &mut unevaluated);
                continue;
            };
            // `actual OP bound`; flip when the evidence (measured) side is on the right.
            let (bound, op) = if ra.is_evidence && !la.is_evidence {
                (la, flip(op))
            } else {
                (ra, op)
            };
            let dim = crate::roles::dim_class(bound.unit.as_deref());
            let metric = match (dim, op) {
                (Some(DimClass::Mass), "<=" | "<") => ("mass_kg", "kg"),
                (Some(DimClass::Power), "<=" | "<") => ("power_w", "W"),
                (Some(DimClass::Time), ">=" | ">") => ("runtime_s", "s"),
                _ => {
                    skip(
                        "bound's dimension/direction maps to no design-point metric (mass <=, power <=, runtime >=)",
                        &mut unevaluated,
                    );
                    continue;
                }
            };
            let value = bound
                .value
                .as_deref()
                .and_then(|v| v.split_whitespace().next()?.parse::<f64>().ok())
                .zip(crate::roles::base_scale(bound.unit.as_deref()))
                .map(|(v, k)| v * k);
            let Some(limit) = value else {
                skip("bound has no numeric value", &mut unevaluated);
                continue;
            };
            active.push(ActiveConstraint {
                id: id.clone(),
                metric: metric.0.to_string(),
                op: if op.starts_with('>') { ">=" } else { "<=" }.to_string(),
                limit,
                unit: metric.1.to_string(),
            });
        }
    }
    (active, unevaluated)
}

/// Value of a named metric on a design point.
pub(crate) fn metric_value(p: &TradePoint, metric: &str) -> Option<f64> {
    match metric {
        "runtime_s" => p.estimated_runtime_s,
        "mass_kg" => Some(p.mass_kg),
        "power_w" => Some(p.rated_power_w),
        "cost_usd" => Some(p.unit_cost_usd),
        _ => None,
    }
}

pub(crate) fn satisfies(p: &TradePoint, c: &ActiveConstraint) -> bool {
    let Some(val) = metric_value(p, &c.metric) else {
        return false;
    };
    match c.op.as_str() {
        ">=" => val + 1e-9 >= c.limit,
        "<=" => val <= c.limit + 1e-9,
        _ => true,
    }
}

/// Move constraints whose metric no design point has (e.g. `runtime_s` without any energy or
/// power role) to `unevaluated`: they can neither pass nor refute anything.
pub(crate) fn drop_unmeasurable(
    points: &[TradePoint],
    constraints: Vec<ActiveConstraint>,
    unevaluated: &mut Vec<UnevaluatedConstraint>,
) -> Vec<ActiveConstraint> {
    if points.is_empty() {
        return constraints;
    }
    constraints
        .into_iter()
        .filter(|c| {
            let measurable = points.iter().any(|p| metric_value(p, &c.metric).is_some());
            if !measurable {
                unevaluated.push(UnevaluatedConstraint {
                    id: c.id.clone(),
                    constraint: format!("{} {} {}", c.metric, c.op, c.limit),
                    reason: "no design point has this metric (missing energy / power / mass / cost roles)".into(),
                });
            }
            measurable
        })
        .collect()
}

fn cli_constraints(limits: &BudgetLimits) -> Vec<ActiveConstraint> {
    let mut out = Vec::new();
    let mut push = |id: &str, metric: &str, v: Option<f64>, unit: &str| {
        if let Some(limit) = v {
            out.push(ActiveConstraint {
                id: id.into(),
                metric: metric.into(),
                op: "<=".into(),
                limit,
                unit: unit.into(),
            });
        }
    };
    push("CLI-MAX-MASS", "mass_kg", limits.max_mass_kg, "kg");
    push("CLI-MAX-POWER", "power_w", limits.max_power_w, "W");
    push("CLI-MAX-COST", "cost_usd", limits.max_cost_usd, "USD");
    out
}

/// Every design point of the model's variation space with role-derived metrics (no checks).
/// Also returns the variation count and the untracked attributes of variant types.
pub(crate) fn design_points(model: &Model) -> (usize, Vec<TradePoint>, Vec<UntrackedAttr>) {
    let variations = collect_variations(model);
    let idx = RoleIndex::new(model);
    let mut untracked: Vec<UntrackedAttr> = Vec::new();
    // Per variant type: (mass, power, energy, cost)
    let mut metrics: BTreeMap<String, (f64, f64, f64, f64)> = BTreeMap::new();
    for (_, opts) in &variations {
        for (_, ty) in opts {
            if metrics.contains_key(ty) {
                continue;
            }
            let mut m = (0.0, 0.0, 0.0, 0.0);
            if let Some(def) = idx.get(ty) {
                for a in idx.effective_attributes(def) {
                    if a.is_evidence {
                        continue;
                    }
                    match a.role {
                        Some(BudgetRole::Mass) => m.0 += a.base_number().unwrap_or(0.0),
                        Some(BudgetRole::Power) => m.1 += a.base_number().unwrap_or(0.0),
                        Some(BudgetRole::Cost) => m.3 += a.number().unwrap_or(0.0),
                        Some(_) => {}
                        None => match a.dim() {
                            Some(DimClass::Energy) => m.2 += a.base_number().unwrap_or(0.0),
                            Some(DimClass::Mass | DimClass::Power)
                                if !matches!(a.value, crate::roles::AttrValue::Tbd) =>
                            {
                                untracked.push(UntrackedAttr {
                                    element: def.name.clone(),
                                    attribute: a.key.clone(),
                                    unit: a.unit.clone().unwrap_or_default(),
                                    value: a.number(),
                                });
                            }
                            _ => {}
                        },
                    }
                }
            }
            metrics.insert(ty.clone(), m);
        }
    }
    if variations.is_empty() {
        return (0, Vec::new(), untracked);
    }

    // Cartesian product of all variation enums
    let mut combos: Vec<Vec<(String, String, String)>> = vec![Vec::new()];
    for (var_name, opts) in &variations {
        let mut next = Vec::with_capacity(combos.len() * opts.len());
        for prefix in &combos {
            for (v_label, v_type) in opts {
                let mut row = prefix.clone();
                row.push((var_name.clone(), v_label.clone(), v_type.clone()));
                next.push(row);
            }
        }
        combos = next;
    }

    let points = combos
        .into_iter()
        .map(|combo| {
            let id = combo
                .iter()
                .map(|(_, label, _)| label.as_str())
                .collect::<Vec<_>>()
                .join(" + ");
            let mut choices = BTreeMap::new();
            let (mut mass, mut power, mut energy, mut cost) = (0.0, 0.0, 0.0, 0.0);
            for (var_name, _, ty) in &combo {
                choices.insert(var_name.clone(), ty.clone());
                let m = metrics[ty];
                mass += m.0;
                power += m.1;
                energy += m.2;
                cost += m.3;
            }
            TradePoint {
                id,
                choices,
                mass_kg: round2(mass),
                rated_power_w: round2(power),
                energy_capacity_j: round2(energy),
                estimated_runtime_s: (energy > 0.0 && power > 0.0)
                    .then(|| round2(energy / power)),
                unit_cost_usd: round2(cost),
                feasible: true,
                pareto_optimal: false,
                failed_checks: Vec::new(),
            }
        })
        .collect();
    (variations.len(), points, untracked)
}

/// Expand all `#[sysml::variation]` enums in `model`, evaluate role-derived metrics against
/// the model's (structurally recognised) requirement constraints and the CLI limits, and
/// compute the Pareto frontier (minimise mass and cost, maximise runtime).
pub fn run_trade_study(model: &Model, limits: &BudgetLimits) -> TradeStudyReport {
    let (variation_count, mut points, untracked) = design_points(model);
    let (mut constraints, mut unevaluated) = requirement_constraints(model);
    constraints.extend(cli_constraints(limits));
    let constraints = drop_unmeasurable(&points, constraints, &mut unevaluated);

    if variation_count == 0 {
        return TradeStudyReport {
            status: TradeStudyStatus::NotApplicable,
            variation_count: 0,
            total_combinations: 0,
            feasible_count: 0,
            pareto_count: 0,
            constraints,
            unevaluated,
            untracked,
            points: Vec::new(),
        };
    }

    for p in &mut points {
        for c in &constraints {
            if !satisfies(p, c) {
                let shown = metric_value(p, &c.metric)
                    .map(|v| format!("{v:.2}"))
                    .unwrap_or_else(|| "n/a".into());
                p.failed_checks.push(format!(
                    "{} {} {shown} {} {:.2} {}",
                    c.id, c.metric, if c.op == ">=" { "<" } else { ">" }, c.limit, c.unit
                ));
            }
        }
        p.feasible = p.failed_checks.is_empty();
    }

    let n = points.len();
    for i in 0..n {
        if !points[i].feasible {
            continue;
        }
        let dominated = (0..n).any(|j| j != i && points[j].feasible && dominates(&points[j], &points[i]));
        points[i].pareto_optimal = !dominated;
    }

    let feasible_count = points.iter().filter(|p| p.feasible).count();
    let pareto_count = points.iter().filter(|p| p.pareto_optimal).count();

    TradeStudyReport {
        status: TradeStudyStatus::Ok,
        variation_count,
        total_combinations: points.len(),
        feasible_count,
        pareto_count,
        constraints,
        unevaluated,
        untracked,
        points,
    }
}

/// Run a `±delta_pct` Tornado sensitivity sweep over every numeric attribute in `model`,
/// ranking which parameters move budget totals (power, mass, waste heat, cost, latency) most.
pub fn run_what_if(model: &Model, delta_pct: f64, limits: &BudgetLimits) -> WhatIfReport {
    let mut drivers = Vec::new();
    let mut evaluated = 0usize;
    let factor_lo = 1.0 - (delta_pct / 100.0);
    let factor_hi = 1.0 + (delta_pct / 100.0);

    for (pkg_idx, pkg) in model.packages.iter().enumerate() {
        if is_sysml_std_package(pkg) {
            continue;
        }
        for (mem_idx, mem) in pkg.members.iter().enumerate() {
            let PackageMember::PartDef(p) = mem else {
                continue;
            };
            if p.is_abstract {
                continue;
            }
            for (attr_idx, a) in p.attributes.iter().enumerate() {
                let Some(aname) = attr_effective_name(a) else {
                    continue;
                };
                let Some(base_val) = parse_f64(a.value.as_deref()) else {
                    continue;
                };
                if base_val == 0.0 {
                    continue;
                }
                evaluated += 1;

                let mut m_lo = model.clone();
                let mut m_hi = model.clone();
                set_attr_val(&mut m_lo, pkg_idx, mem_idx, attr_idx, base_val * factor_lo);
                set_attr_val(&mut m_hi, pkg_idx, mem_idx, attr_idx, base_val * factor_hi);

                let b_lo = analyze_budgets(&m_lo, limits);
                let b_hi = analyze_budgets(&m_hi, limits);

                let candidates = [
                    ("power_w", b_lo.power.total, b_hi.power.total),
                    ("waste_heat_w", b_lo.waste_heat.total, b_hi.waste_heat.total),
                    ("mass_kg", b_lo.mass.total, b_hi.mass.total),
                    ("cost_usd", b_lo.cost.total, b_hi.cost.total),
                    (
                        "critical_path_ms",
                        b_lo.timing.critical_path_ms,
                        b_hi.timing.critical_path_ms,
                    ),
                ];

                for (metric, lo, hi) in candidates {
                    let swing = (hi - lo).abs();
                    if swing > 1e-6 {
                        drivers.push(SensitivityDriver {
                            element: p.name.clone(),
                            attribute: aname.to_string(),
                            base_value: round2(base_val),
                            delta_pct,
                            affected_metric: metric.to_string(),
                            low_total: round2(lo),
                            high_total: round2(hi),
                            swing: round2(swing),
                        });
                    }
                }
            }
        }
    }

    drivers.sort_by(|a, b| b.swing.total_cmp(&a.swing));
    drivers.truncate(15);

    WhatIfReport {
        delta_pct,
        parameters_evaluated: evaluated,
        top_drivers: drivers,
    }
}

/// Render [`TradeStudyReport`] as clean Markdown.
pub fn render_trade_study_markdown(r: &TradeStudyReport) -> String {
    let mut out = String::new();
    out.push_str("# Product-Line Trade Study & Pareto Frontier\n\n");
    if r.status == TradeStudyStatus::NotApplicable {
        out.push_str("- **Status:** NOT APPLICABLE (the model has no `#[sysml::variation]`)\n");
    }
    out.push_str(&format!(
        "- **Variations:** {} | **Combinations:** {} | **Feasible:** {} | **Pareto-Optimal:** {}\n",
        r.variation_count, r.total_combinations, r.feasible_count, r.pareto_count
    ));
    for c in &r.constraints {
        out.push_str(&format!("- **Constraint `{}`:** `{} {} {:.2} {}`\n", c.id, c.metric, c.op, c.limit, c.unit));
    }
    for u in &r.unevaluated {
        out.push_str(&format!("- **Not evaluated `{}`:** `{}` ({})\n", u.id, u.constraint, u.reason));
    }
    for u in &r.untracked {
        out.push_str(&format!("- **Untracked (no budget role):** `{}.{}` [{}]\n", u.element, u.attribute, u.unit));
    }
    out.push('\n');
    for p in &r.points {
        let tag = if p.pareto_optimal {
            "PARETO"
        } else if p.feasible {
            "FEASIBLE"
        } else {
            "INFEASIBLE"
        };
        let rt = p
            .estimated_runtime_s
            .map(|v| format!("{v:.1} s"))
            .unwrap_or_else(|| "—".into());
        let fails = if p.failed_checks.is_empty() {
            String::new()
        } else {
            format!(" — fails: {}", p.failed_checks.join(", "))
        };
        out.push_str(&format!(
            "- **`[{}]` {}**: mass {:.2} kg, power {:.0} W, runtime {}, cost {:.2}{}\n",
            tag, p.id, p.mass_kg, p.rated_power_w, rt, p.unit_cost_usd, fails
        ));
    }
    out
}

/// Render [`WhatIfReport`] as clean Markdown.
pub fn render_what_if_markdown(r: &WhatIfReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# What-If Tornado Sensitivity (±{:.1}% Perturbation)\n\n",
        r.delta_pct
    ));
    out.push_str(&format!(
        "- **Parameters Evaluated:** {} | **Top Drivers Shown:** {}\n\n",
        r.parameters_evaluated,
        r.top_drivers.len()
    ));
    for d in &r.top_drivers {
        out.push_str(&format!(
            "- **`{}.{}`** (base {}): moves `{}` from {} to {} (**swing {:.2}**)\n",
            d.element, d.attribute, d.base_value, d.affected_metric, d.low_total, d.high_total, d.swing
        ));
    }
    out
}

fn dominates(a: &TradePoint, b: &TradePoint) -> bool {
    let a_rt = a.estimated_runtime_s.unwrap_or(0.0);
    let b_rt = b.estimated_runtime_s.unwrap_or(0.0);
    let no_worse = a.mass_kg <= b.mass_kg + 1e-9
        && a_rt + 1e-9 >= b_rt
        && a.unit_cost_usd <= b.unit_cost_usd + 1e-9;
    let strictly_better = a.mass_kg < b.mass_kg - 1e-9
        || a_rt > b_rt + 1e-9
        || a.unit_cost_usd < b.unit_cost_usd - 1e-9;
    no_worse && strictly_better
}

fn set_attr_val(model: &mut Model, pkg_idx: usize, mem_idx: usize, attr_idx: usize, val: f64) {
    if let Some(PackageMember::PartDef(p)) = model.packages[pkg_idx].members.get_mut(mem_idx) {
        if let Some(a) = p.attributes.get_mut(attr_idx) {
            a.value = Some(format!("{val:.6}"));
        }
    }
}

fn attr_effective_name(a: &AttributeUsage) -> Option<&str> {
    a.name
        .as_deref()
        .or_else(|| a.redefines.first().map(String::as_str))
}

fn parse_f64(raw: Option<&str>) -> Option<f64> {
    let s = raw?.trim();
    let token = s.split_whitespace().next()?;
    token.parse::<f64>().ok().filter(|v| v.is_finite())
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

    fn role(name: &str, val: &str, unit: &str, role: sysml_ir::BudgetRole) -> AttributeUsage {
        AttributeUsage {
            unit: Some(unit.into()),
            budget_role: Some(role),
            ..attr(name, val)
        }
    }

    #[test]
    fn trade_study_expands_variations_and_computes_pareto_frontier() {
        let mut pkg = Package {
            name: "ToolTrade".into(),
            ..Default::default()
        };
        pkg.members.push(PackageMember::RequirementDef(RequirementDef {
            name: "RuntimeReq".into(),
            short_name: Some("TOOL-001".into()),
            attributes: vec![
                AttributeUsage { unit: Some("s".into()), ..attr("minRuntimeS", "300.0") },
                AttributeUsage { unit: Some("s".into()), ..attr("actualRuntimeS", "0.0") },
                // Dimensionless (noise): maps to no metric, so it is listed as unevaluated.
                attr("maxNoiseDb", "85.0"),
                attr("noiseDb", "80.0"),
            ],
            require_constraints: vec![
                "actualRuntimeS >= minRuntimeS".into(),
                "noiseDb <= maxNoiseDb".into(),
            ],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "Batt2Ah".into(),
            attributes: vec![AttributeUsage { unit: Some("J".into()), ..attr("energyCapacity", "129600.0") }, role("mass", "0.40", "kg", sysml_ir::BudgetRole::Mass)],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "Batt5Ah".into(),
            attributes: vec![AttributeUsage { unit: Some("J".into()), ..attr("energyCapacity", "324000.0") }, role("mass", "0.68", "kg", sysml_ir::BudgetRole::Mass)],
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
            attributes: vec![role("ratedPower", "550.0", "W", sysml_ir::BudgetRole::Power), role("mass", "1.95", "kg", sysml_ir::BudgetRole::Mass), attr("maxNoiseDb", "79.0"), attr("ratedPowerW", "9.0")],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "Saw".into(),
            attributes: vec![role("ratedPower", "1100.0", "W", sysml_ir::BudgetRole::Power), role("mass", "3.10", "kg", sysml_ir::BudgetRole::Mass), attr("maxNoiseDb", "90.0")],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "ToolVar".into(),
            is_variation: true,
            variants: vec![
                VariantPart { name: "Drill".into(), type_name: "Drill".into(), doc: None },
                VariantPart { name: "Saw".into(), type_name: "Saw".into(), doc: None },
            ],
            ..Default::default()
        }));

        let model = Model::new(vec![pkg]);
        let rep = run_trade_study(&model, &BudgetLimits::default());
        assert_eq!(rep.status, TradeStudyStatus::Ok);
        assert_eq!(rep.variation_count, 2);
        assert_eq!(rep.constraints.len(), 1);
        assert_eq!(rep.constraints[0].metric, "runtime_s");
        assert_eq!(rep.unevaluated.len(), 1);
        // Untagged attributes without a unit are not candidates; nothing untracked here.
        assert!(rep.untracked.is_empty(), "{:?}", rep.untracked);
        assert_eq!(rep.total_combinations, 4);
        // Only Pro5Ah + Drill (324000 / 550 = 589.1s >= 300s) is feasible!
        // Compact2Ah + Drill = 235.6s < 300s; Pro5Ah + Saw = 294.5s < 300s; Compact2Ah + Saw = 117.8s < 300s
        assert_eq!(rep.feasible_count, 1);
        assert_eq!(rep.pareto_count, 1);
        let winner = rep.points.iter().find(|p| p.pareto_optimal).unwrap();
        assert_eq!(winner.id, "Pro5Ah + Drill");

        // What-if tornado sensitivity
        let wi = run_what_if(&model, 10.0, &BudgetLimits::default());
        assert!(wi.parameters_evaluated >= 6);
        assert!(!wi.top_drivers.is_empty());
        assert_eq!(wi.top_drivers[0].element, "Saw");
        assert_eq!(wi.top_drivers[0].attribute, "ratedPower");
    }
}

#[cfg(test)]
mod status_tests {
    use super::*;
    use sysml_ir::{Package, PartDef};

    #[test]
    fn no_variations_is_not_applicable() {
        let model = Model::new(vec![Package {
            name: "Flat".into(),
            members: vec![PackageMember::PartDef(PartDef {
                name: "Heater".into(),
                attributes: vec![AttributeUsage {
                    name: Some("ratedPowerW".into()),
                    value: Some("2000.0".into()),
                    unit: Some("W".into()),
                    ..Default::default()
                }],
                ..Default::default()
            })],
            ..Default::default()
        }]);
        let r = run_trade_study(&model, &BudgetLimits::default());
        assert_eq!(r.status, TradeStudyStatus::NotApplicable);
        assert!(r.points.is_empty());
        let json = serde_json::to_value(&r).unwrap();
        assert_eq!(json["status"], "not_applicable");
        assert!(json["untracked"].is_array());
    }
}
