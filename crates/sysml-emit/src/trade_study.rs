//! Phase B2 — `trade-study` (automatic `#[sysml::variation]` Cartesian expansion +
//! Pareto frontier) and `what-if` (Tornado parameter sensitivity ranking).
//!
//! Pure in-memory IR analysis with zero external dependencies; runs in < 1 ms.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sysml_ir::{AttributeUsage, Model, PackageMember, PartDef};

use crate::budget::{analyze_budgets, BudgetLimits};
use crate::emit_cli::is_sysml_std_package;

/// One concrete combination of `#[sysml::variation]` choices evaluated in a trade study.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradePoint {
    pub id: String,
    /// Map of variation name -> chosen variant type name (e.g. `BatteryPackVariant -> Battery18V5Ah`).
    pub choices: BTreeMap<String, String>,
    pub mass_kg: f64,
    pub rated_power_w: f64,
    pub energy_capacity_j: f64,
    pub estimated_runtime_s: Option<f64>,
    pub max_noise_db: Option<f64>,
    pub unit_cost_usd: f64,
    pub feasible: bool,
    pub pareto_optimal: bool,
    pub failed_checks: Vec<String>,
}

/// Complete trade-study report over all `#[sysml::variation]` combinations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeStudyReport {
    pub variation_count: usize,
    pub total_combinations: usize,
    pub feasible_count: usize,
    pub pareto_count: usize,
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

/// Expand all `#[sysml::variation]` enums in `model`, evaluate physical metrics and
/// model/user limits, and compute the Pareto frontier.
pub fn run_trade_study(model: &Model, limits: &BudgetLimits) -> TradeStudyReport {
    let mut part_defs: BTreeMap<String, &PartDef> = BTreeMap::new();
    let mut variations: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let mut req_min_runtime_s: Option<f64> = None;
    let mut req_max_noise_db: Option<f64> = None;

    for pkg in &model.packages {
        if is_sysml_std_package(pkg) {
            continue;
        }
        for m in &pkg.members {
            match m {
                PackageMember::PartDef(p) => {
                    part_defs.insert(p.name.clone(), p);
                    if p.is_variation && !p.variants.is_empty() {
                        let opts = p
                            .variants
                            .iter()
                            .map(|v| (v.name.clone(), v.type_name.clone()))
                            .collect();
                        variations.push((p.name.clone(), opts));
                    }
                }
                PackageMember::RequirementDef(r) => {
                    if let Some(v) = attr_number(&r.attributes, &["minRuntimeS", "minRuntime"]) {
                        req_min_runtime_s = Some(req_min_runtime_s.map_or(v, |c| c.max(v)));
                    }
                    if let Some(v) = attr_number(&r.attributes, &["maxNoiseDb"]) {
                        req_max_noise_db = Some(req_max_noise_db.map_or(v, |c| c.min(v)));
                    }
                }
                _ => {}
            }
        }
    }

    if variations.is_empty() {
        return TradeStudyReport {
            variation_count: 0,
            total_combinations: 0,
            feasible_count: 0,
            pareto_count: 0,
            points: Vec::new(),
        };
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

    let mut points = Vec::with_capacity(combos.len());
    for combo in combos {
        let id = combo
            .iter()
            .map(|(_, label, _)| label.as_str())
            .collect::<Vec<_>>()
            .join(" + ");
        let mut choices = BTreeMap::new();
        let mut mass_kg = 0.0;
        let mut rated_power_w = 0.0;
        let mut energy_capacity_j = 0.0;
        let mut max_noise_db: Option<f64> = None;
        let mut unit_cost_usd = 0.0;

        for (var_name, _, ty_name) in &combo {
            choices.insert(var_name.clone(), ty_name.clone());
            if let Some(p) = part_defs.get(ty_name) {
                if let Some(m) = attr_number(&p.attributes, &["mass", "massKg", "dryMass"]) {
                    mass_kg += m;
                }
                if let Some(pw) = attr_number(&p.attributes, &["ratedPower", "powerDraw", "powerW"]) {
                    rated_power_w += pw;
                }
                if let Some(e) = attr_number(&p.attributes, &["energyCapacity", "energyCapacityJ"]) {
                    energy_capacity_j += e;
                }
                if let Some(db) = attr_number(&p.attributes, &["maxNoiseDb", "noiseLevelDb"]) {
                    max_noise_db = Some(max_noise_db.map_or(db, |cur| cur.max(db)));
                }
                if let Some(c) = attr_number(&p.attributes, &["unitCost", "costUsd"]) {
                    unit_cost_usd += c;
                }
            }
        }

        let estimated_runtime_s = if energy_capacity_j > 0.0 && rated_power_w > 0.0 {
            Some(round2(energy_capacity_j / rated_power_w))
        } else {
            None
        };

        let mut failed_checks = Vec::new();
        if let (Some(min_rt), Some(rt)) = (req_min_runtime_s, estimated_runtime_s) {
            if rt < min_rt {
                failed_checks.push(format!("runtime {rt:.1}s < {min_rt:.1}s"));
            }
        }
        if let (Some(max_db), Some(db)) = (req_max_noise_db, max_noise_db) {
            if db > max_db {
                failed_checks.push(format!("noise {db:.1}dB > {max_db:.1}dB"));
            }
        }
        if let Some(max_m) = limits.max_mass_kg {
            if mass_kg > max_m + 1e-9 {
                failed_checks.push(format!("mass {mass_kg:.2}kg > {max_m:.2}kg"));
            }
        }
        if let Some(max_p) = limits.max_power_w {
            if rated_power_w > max_p + 1e-9 {
                failed_checks.push(format!("power {rated_power_w:.1}W > {max_p:.1}W"));
            }
        }
        if let Some(max_c) = limits.max_cost_usd {
            if unit_cost_usd > max_c + 1e-9 {
                failed_checks.push(format!("cost ${unit_cost_usd:.2} > ${max_c:.2}"));
            }
        }

        points.push(TradePoint {
            id,
            choices,
            mass_kg: round2(mass_kg),
            rated_power_w: round2(rated_power_w),
            energy_capacity_j: round2(energy_capacity_j),
            estimated_runtime_s,
            max_noise_db: max_noise_db.map(round2),
            unit_cost_usd: round2(unit_cost_usd),
            feasible: failed_checks.is_empty(),
            pareto_optimal: false,
            failed_checks,
        });
    }

    // Mark Pareto-optimal feasible points:
    // minimize mass_kg, maximize runtime_s, minimize noise_db, minimize cost_usd
    let n = points.len();
    for i in 0..n {
        if !points[i].feasible {
            continue;
        }
        let mut dominated = false;
        for j in 0..n {
            if i == j || !points[j].feasible {
                continue;
            }
            if dominates(&points[j], &points[i]) {
                dominated = true;
                break;
            }
        }
        points[i].pareto_optimal = !dominated;
    }

    let feasible_count = points.iter().filter(|p| p.feasible).count();
    let pareto_count = points.iter().filter(|p| p.pareto_optimal).count();

    TradeStudyReport {
        variation_count: variations.len(),
        total_combinations: points.len(),
        feasible_count,
        pareto_count,
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
    out.push_str(&format!(
        "- **Variations:** {} | **Combinations:** {} | **Feasible:** {} | **Pareto-Optimal:** {}\n\n",
        r.variation_count, r.total_combinations, r.feasible_count, r.pareto_count
    ));
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
        let db = p
            .max_noise_db
            .map(|v| format!("{v:.1} dB"))
            .unwrap_or_else(|| "—".into());
        let fails = if p.failed_checks.is_empty() {
            String::new()
        } else {
            format!(" — fails: {}", p.failed_checks.join(", "))
        };
        out.push_str(&format!(
            "- **`[{}]` {}**: mass {:.2} kg, power {:.0} W, runtime {}, noise {}{}\n",
            tag, p.id, p.mass_kg, p.rated_power_w, rt, db, fails
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
    let a_db = a.max_noise_db.unwrap_or(0.0);
    let b_db = b.max_noise_db.unwrap_or(0.0);

    let no_worse = a.mass_kg <= b.mass_kg + 1e-9
        && a_rt + 1e-9 >= b_rt
        && a_db <= b_db + 1e-9
        && a.unit_cost_usd <= b.unit_cost_usd + 1e-9;
    let strictly_better = a.mass_kg < b.mass_kg - 1e-9
        || a_rt > b_rt + 1e-9
        || a_db < b_db - 1e-9
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

fn attr_number(attrs: &[AttributeUsage], names: &[&str]) -> Option<f64> {
    for a in attrs {
        let Some(aname) = attr_effective_name(a) else {
            continue;
        };
        if names.iter().any(|n| n.eq_ignore_ascii_case(aname)) {
            if let Some(v) = parse_f64(a.value.as_deref()) {
                return Some(v);
            }
        }
    }
    None
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
    use sysml_ir::{Package, RequirementDef, VariantPart};

    fn attr(name: &str, val: &str) -> AttributeUsage {
        AttributeUsage {
            name: Some(name.into()),
            value: Some(val.into()),
            ..Default::default()
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
            name: "Saw".into(),
            attributes: vec![attr("ratedPower", "1100.0"), attr("mass", "3.10"), attr("maxNoiseDb", "90.0")],
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
        assert_eq!(rep.variation_count, 2);
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
