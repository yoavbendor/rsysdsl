//! `budget` — cross-domain static budget rollup, DAG critical-path latency (including LLM
//! "thinking" time), pipeline FPS bottleneck, and top-3 hot-spot attribution over `sysml_ir::Model`.
//!
//! Contributions come only from budget roles (C1 / D1, see [`crate::roles`]): never from
//! attribute names. Attributes whose dimension matches a domain but carry no role are reported
//! as `untracked`, and a limited domain with untracked candidates is `incomplete` (D2), never a
//! silent pass.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sysml_ir::{BudgetRole, Model, PartDef};

use crate::roles::{concrete_user_parts, AttrValue, DimClass, EffectiveAttr, RoleIndex};

/// Optional user-specified budget ceilings for `--strict` pass/fail checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BudgetLimits {
    pub max_latency_ms: Option<f64>,
    pub min_fps: Option<f64>,
    pub max_power_w: Option<f64>,
    pub max_mass_kg: Option<f64>,
    pub max_cost_usd: Option<f64>,
}

/// One contributor to a budget domain, ranked for hot-spot detection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HotSpot {
    pub element: String,
    pub value: f64,
    pub pct_of_total: f64,
}

/// Verdict of one budget domain (D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetStatus {
    /// Within the limit, every candidate attribute has a role, at least one contributor
    /// (also reported for a domain without a limit: nothing to check).
    Pass,
    /// The tracked total already exceeds the limit.
    Fail,
    /// A limit is set but the total cannot be trusted: untracked candidates, role-tagged
    /// values that cannot be summed (`unresolved`), or zero contributors.
    Incomplete,
}

impl BudgetStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BudgetStatus::Pass => "pass",
            BudgetStatus::Fail => "fail",
            BudgetStatus::Incomplete => "incomplete",
        }
    }
}

/// An attribute the rollup could not use (see [`DomainRollup::untracked`] / `unresolved`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UntrackedAttr {
    pub element: String,
    pub attribute: String,
    pub unit: String,
    pub value: Option<f64>,
}

/// Additive rollup for one physical/economic domain (mass, power, waste heat, cost).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainRollup {
    pub domain: String,
    pub unit: String,
    pub total: f64,
    pub limit: Option<f64>,
    pub margin: Option<f64>,
    /// `status == "pass"` (kept for compatibility).
    pub pass: bool,
    pub status: BudgetStatus,
    /// Number of attribute values summed into `total`.
    pub contributors: usize,
    pub hot_spots: Vec<HotSpot>,
    /// Attributes whose dimension matches this domain but that carry no budget role
    /// (e.g. an untagged `ratedPowerW [W]`): not summed, and they make a limited domain
    /// `incomplete`. Tag them (`budget = "..."`) or opt them out (`budget = "none"`).
    pub untracked: Vec<UntrackedAttr>,
    /// Role-tagged attributes whose value cannot be summed: `tbd_value`, a non-literal
    /// expression, or a unit whose scale is unknown. They make a limited domain `incomplete`.
    pub unresolved: Vec<UntrackedAttr>,
}

/// End-to-end DAG critical path & pipeline FPS bottleneck summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimingBudget {
    pub critical_path_ms: f64,
    pub critical_path_nodes: Vec<String>,
    pub latency_limit_ms: Option<f64>,
    pub latency_margin_ms: Option<f64>,
    pub latency_pass: bool,
    pub pipeline_fps: Option<f64>,
    pub fps_bottleneck_stage: Option<String>,
    pub min_fps_limit: Option<f64>,
    pub fps_pass: bool,
    pub stage_hot_spots: Vec<HotSpot>,
    /// Verdict over `--max-latency-ms` / `--min-fps` (same rules as [`DomainRollup::status`]).
    pub status: BudgetStatus,
    /// Time-dimension attributes (`s`) of parts without a role.
    pub untracked: Vec<UntrackedAttr>,
    /// Latency-role attributes whose value cannot be used.
    pub unresolved: Vec<UntrackedAttr>,
}

/// Complete cross-domain budget report produced by [`analyze_budgets`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetReport {
    /// `false` unless every domain with a limit is `pass`.
    pub ok: bool,
    pub mass: DomainRollup,
    pub power: DomainRollup,
    pub waste_heat: DomainRollup,
    pub cost: DomainRollup,
    pub timing: TimingBudget,
}

#[derive(Default)]
struct DomainAcc {
    items: Vec<(String, f64)>,
    untracked: Vec<UntrackedAttr>,
    unresolved: Vec<UntrackedAttr>,
}

fn entry(element: &str, a: &EffectiveAttr) -> UntrackedAttr {
    UntrackedAttr {
        element: element.to_string(),
        attribute: a.key.clone(),
        unit: a.unit.clone().unwrap_or_default(),
        value: a.number(),
    }
}

/// Analyze cross-domain budgets, DAG critical-path latency, pipeline FPS bottleneck, and top-3
/// hot spots on a [`Model`].
///
/// Contributions come only from budget roles (D1): an attribute that is, or redefines, a
/// role-tagged attribute (`#[sysml::attr(budget = "...")]` or a sysml-std role attribute such
/// as `PowerElement::ratedPower`). Every non-abstract part definition of the model's own
/// packages counts once, with its inherited values. Attributes of requirement definitions,
/// evidence attributes and `tbd_value` attributes are never contributions nor `untracked`
/// (role-tagged `tbd_value` ones are listed as `unresolved`).
pub fn analyze_budgets(model: &Model, limits: &BudgetLimits) -> BudgetReport {
    let idx = RoleIndex::new(model);
    let mut mass = DomainAcc::default();
    let mut power = DomainAcc::default();
    let mut heat = DomainAcc::default();
    let mut cost = DomainAcc::default();
    let mut time = DomainAcc::default();
    let mut stage_latency_ms: BTreeMap<String, f64> = BTreeMap::new();

    for p in concrete_user_parts(model) {
        let name = p.name.clone();
        let mut part_power = 0.0;
        let mut part_heat: Option<f64> = None;
        let mut part_eff: Option<f64> = None;
        let mut part_latency: Option<f64> = None;
        for a in idx.effective_attributes(p) {
            if a.is_evidence {
                continue;
            }
            match a.role {
                Some(BudgetRole::None) => {}
                Some(role) => {
                    let acc = match role {
                        BudgetRole::Mass => &mut mass,
                        BudgetRole::Power => &mut power,
                        BudgetRole::WasteHeat => &mut heat,
                        BudgetRole::Cost => &mut cost,
                        BudgetRole::Latency => &mut time,
                        BudgetRole::Efficiency | BudgetRole::None => {
                            if let Some(v) = a.number() {
                                part_eff = Some(v);
                            }
                            continue;
                        }
                    };
                    let scaled = match role {
                        BudgetRole::Cost => a.number(),
                        _ => a.base_number(),
                    };
                    match (&a.value, scaled) {
                        (AttrValue::Absent, _) => {}
                        (_, Some(v)) => match role {
                            BudgetRole::Power => part_power += v,
                            BudgetRole::WasteHeat => *part_heat.get_or_insert(0.0) += v,
                            BudgetRole::Latency => *part_latency.get_or_insert(0.0) += v,
                            _ => acc.items.push((name.clone(), v)),
                        },
                        _ => acc.unresolved.push(entry(&name, &a)),
                    }
                }
                None => {
                    if matches!(a.value, AttrValue::Tbd) {
                        continue;
                    }
                    match a.dim() {
                        Some(DimClass::Mass) => mass.untracked.push(entry(&name, &a)),
                        Some(DimClass::Power) => {
                            power.untracked.push(entry(&name, &a));
                            heat.untracked.push(entry(&name, &a));
                        }
                        Some(DimClass::Time) => time.untracked.push(entry(&name, &a)),
                        _ => {}
                    }
                }
            }
        }
        if part_power > 0.0 {
            power.items.push((name.clone(), part_power));
        }
        match (part_heat, part_eff) {
            (Some(h), _) => heat.items.push((name.clone(), h)),
            (None, Some(eta)) if part_power > 0.0 && (0.0..1.0).contains(&eta) => {
                heat.items.push((name.clone(), part_power * (1.0 - eta)));
            }
            _ => {}
        }
        if let Some(lat_s) = part_latency {
            stage_latency_ms.insert(name.clone(), lat_s * 1000.0);
        }
    }

    let part_defs: BTreeMap<String, &PartDef> = concrete_user_parts(model)
        .into_iter()
        .map(|p| (p.name.clone(), p))
        .collect();

    // Build directed stage graph from assembly connections (field.port -> field.port)
    let mut nodes: BTreeSet<String> = BTreeSet::new();
    let mut node_lat: BTreeMap<String, f64> = BTreeMap::new();
    let mut edges: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for p in part_defs.values() {
        let mut field_to_type: BTreeMap<String, String> = BTreeMap::new();
        for child in &p.parts {
            if let (Some(fname), Some(ty)) = (child.name.as_deref(), child.type_names.first()) {
                field_to_type.insert(fname.to_string(), ty.clone());
                if let Some(&lat) = stage_latency_ms.get(ty) {
                    nodes.insert(fname.to_string());
                    node_lat.insert(fname.to_string(), lat);
                }
            }
        }
        for conn in &p.connections {
            let (Some(from_s), Some(to_s)) = (conn.from.as_deref(), conn.to.as_deref()) else {
                continue;
            };
            let src = from_s.split('.').next().unwrap_or(from_s);
            let dst = to_s.split('.').next().unwrap_or(to_s);
            if src != dst && field_to_type.contains_key(src) && field_to_type.contains_key(dst) {
                nodes.insert(src.to_string());
                nodes.insert(dst.to_string());
                edges
                    .entry(src.to_string())
                    .or_default()
                    .push(dst.to_string());
            }
        }
    }

    // Fallback when no assembly connections exist: treat timed PartDefs as independent stages
    if nodes.is_empty() {
        for (k, &v) in &stage_latency_ms {
            nodes.insert(k.clone());
            node_lat.insert(k.clone(), v);
        }
    }

    // Longest-path (critical path) via bounded DFS with cycle guard
    let (crit_ms, crit_path) = longest_dag_path(&nodes, &edges, &node_lat);

    // Pipeline FPS = min(1000 / stage_latency_ms) across stages with latency > 0
    let mut bottleneck_stage: Option<String> = None;
    let mut bottleneck_ms: f64 = 0.0;
    for (n, &lat) in &node_lat {
        if lat > bottleneck_ms {
            bottleneck_ms = lat;
            bottleneck_stage = Some(n.clone());
        }
    }
    let pipeline_fps = if bottleneck_ms > 0.0 {
        Some((1000.0 / bottleneck_ms * 100.0).round() / 100.0)
    } else {
        None
    };

    let stage_hot_spots = build_hot_spots(
        &crit_path
            .iter()
            .filter_map(|n| node_lat.get(n).map(|&v| (n.clone(), v)))
            .collect::<Vec<_>>(),
        crit_ms,
    );

    let (latency_margin_ms, latency_pass) = match limits.max_latency_ms {
        Some(lim) => (Some(round2(lim - crit_ms)), crit_ms <= lim + 1e-9),
        None => (None, true),
    };
    let fps_pass = match (limits.min_fps, pipeline_fps) {
        (Some(min_f), Some(actual_f)) => actual_f + 1e-9 >= min_f,
        (Some(_), None) => false,
        (None, _) => true,
    };
    let timing_limited = limits.max_latency_ms.is_some() || limits.min_fps.is_some();
    let timing_status = if !timing_limited {
        BudgetStatus::Pass
    } else if (limits.max_latency_ms.is_some() && !latency_pass)
        || (limits.min_fps.is_some() && pipeline_fps.is_some() && !fps_pass)
    {
        BudgetStatus::Fail
    } else if !time.untracked.is_empty() || !time.unresolved.is_empty() || node_lat.is_empty() {
        BudgetStatus::Incomplete
    } else {
        BudgetStatus::Pass
    };

    let mass = build_rollup("mass", "kg", mass, limits.max_mass_kg);
    let power = build_rollup("power", "W", power, limits.max_power_w);
    let waste_heat = build_rollup("waste_heat", "W", heat, None);
    let cost = build_rollup("cost", "USD", cost, limits.max_cost_usd);

    let ok = [&mass, &power, &waste_heat, &cost]
        .iter()
        .all(|d| d.limit.is_none() || d.status == BudgetStatus::Pass)
        && (!timing_limited || timing_status == BudgetStatus::Pass);

    BudgetReport {
        ok,
        mass,
        power,
        waste_heat,
        cost,
        timing: TimingBudget {
            critical_path_ms: round2(crit_ms),
            critical_path_nodes: crit_path,
            latency_limit_ms: limits.max_latency_ms,
            latency_margin_ms,
            latency_pass,
            pipeline_fps,
            fps_bottleneck_stage: bottleneck_stage,
            min_fps_limit: limits.min_fps,
            fps_pass,
            stage_hot_spots,
            status: timing_status,
            untracked: time.untracked,
            unresolved: time.unresolved,
        },
    }
}

fn render_attr_list(label: &str, list: &[UntrackedAttr], out: &mut String) {
    if list.is_empty() {
        return;
    }
    let items = list
        .iter()
        .map(|u| {
            let v = u.value.map(|v| format!(" = {v}")).unwrap_or_default();
            let unit = if u.unit.is_empty() { String::new() } else { format!(" [{}]", u.unit) };
            format!("`{}.{}`{v}{unit}", u.element, u.attribute)
        })
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!("  - {label}: {items}\n"));
}

/// Render a [`BudgetReport`] as clean Markdown.
pub fn render_budget_markdown(r: &BudgetReport) -> String {
    let mut out = String::new();
    out.push_str("# Cross-Domain System Budget & Critical-Path Report\n\n");
    out.push_str(&format!(
        "- **Overall Status:** {}\n",
        if r.ok { "PASS" } else { "NOT OK" }
    ));
    out.push_str(&format!(
        "- **Critical Path Latency:** {:.2} ms ({}){} [{}]\n",
        r.timing.critical_path_ms,
        if r.timing.critical_path_nodes.is_empty() {
            "none".to_string()
        } else {
            r.timing.critical_path_nodes.join(" -> ")
        },
        r.timing
            .latency_margin_ms
            .map(|m| format!(" | margin: {m:+.2} ms"))
            .unwrap_or_default(),
        r.timing.status.as_str().to_uppercase()
    ));
    if let Some(fps) = r.timing.pipeline_fps {
        out.push_str(&format!(
            "- **Pipeline Throughput:** {:.2} FPS (bottleneck: `{}`)\n",
            fps,
            r.timing.fps_bottleneck_stage.as_deref().unwrap_or("—")
        ));
    }
    render_attr_list("untracked (no budget role)", &r.timing.untracked, &mut out);
    render_attr_list("unresolved (role, no usable value)", &r.timing.unresolved, &mut out);
    for d in [&r.mass, &r.power, &r.waste_heat, &r.cost] {
        let lim = d
            .limit
            .map(|l| format!(" / limit {:.2} {} (margin {:+.2})", l, d.unit, d.margin.unwrap_or(0.0)))
            .unwrap_or_default();
        let top = d
            .hot_spots
            .iter()
            .map(|h| format!("`{}` ({:.1}%)", h.element, h.pct_of_total))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "- **{}:** {:.2} {}{} [{}] ({} contributors){}\n",
            d.domain,
            d.total,
            d.unit,
            lim,
            d.status.as_str().to_uppercase(),
            d.contributors,
            if top.is_empty() {
                String::new()
            } else {
                format!(" — hot spots: {top}")
            }
        ));
        render_attr_list("untracked (no budget role)", &d.untracked, &mut out);
        render_attr_list("unresolved (role, no usable value)", &d.unresolved, &mut out);
    }
    out
}

fn build_rollup(domain: &str, unit: &str, acc: DomainAcc, limit: Option<f64>) -> DomainRollup {
    let total: f64 = acc.items.iter().map(|(_, v)| *v).sum();
    let status = match limit {
        None => BudgetStatus::Pass,
        Some(l) if total > l + 1e-9 => BudgetStatus::Fail,
        Some(_) if !acc.untracked.is_empty() || !acc.unresolved.is_empty() || acc.items.is_empty() => {
            BudgetStatus::Incomplete
        }
        Some(_) => BudgetStatus::Pass,
    };
    DomainRollup {
        domain: domain.to_string(),
        unit: unit.to_string(),
        total: round2(total),
        limit,
        margin: limit.map(|l| round2(l - total)),
        pass: status == BudgetStatus::Pass,
        status,
        contributors: acc.items.len(),
        hot_spots: build_hot_spots(&acc.items, total),
        untracked: acc.untracked,
        unresolved: acc.unresolved,
    }
}

fn build_hot_spots(items: &[(String, f64)], total: f64) -> Vec<HotSpot> {
    if total <= 0.0 {
        return Vec::new();
    }
    let mut sorted = items.to_vec();
    sorted.sort_by(|a, b| b.1.total_cmp(&a.1));
    sorted
        .into_iter()
        .take(3)
        .map(|(element, value)| HotSpot {
            element,
            value: round2(value),
            pct_of_total: ((value / total) * 1000.0).round() / 10.0,
        })
        .collect()
}

fn longest_dag_path(
    nodes: &BTreeSet<String>,
    edges: &BTreeMap<String, Vec<String>>,
    weights: &BTreeMap<String, f64>,
) -> (f64, Vec<String>) {
    let mut best_cost = 0.0;
    let mut best_path = Vec::new();

    for start in nodes {
        let mut visited = BTreeSet::new();
        let (c, p) = dfs_longest(start, edges, weights, &mut visited);
        if c > best_cost {
            best_cost = c;
            best_path = p;
        }
    }
    (best_cost, best_path)
}

fn dfs_longest(
    cur: &str,
    edges: &BTreeMap<String, Vec<String>>,
    weights: &BTreeMap<String, f64>,
    visited: &mut BTreeSet<String>,
) -> (f64, Vec<String>) {
    if !visited.insert(cur.to_string()) {
        return (0.0, Vec::new());
    }
    let w = weights.get(cur).copied().unwrap_or(0.0);
    let mut sub_best = 0.0;
    let mut sub_path = Vec::new();
    if let Some(nexts) = edges.get(cur) {
        for nxt in nexts {
            let (c, p) = dfs_longest(nxt, edges, weights, visited);
            if c > sub_best {
                sub_best = c;
                sub_path = p;
            }
        }
    }
    visited.remove(cur);
    let mut path = vec![cur.to_string()];
    path.extend(sub_path);
    (w + sub_best, path)
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
        AttributeUsage, ConnectionKind, ConnectionUsage, Package, PackageMember, PartUsage,
    };

    fn attr(name: &str, val: &str, unit: Option<&str>, role: Option<BudgetRole>) -> AttributeUsage {
        AttributeUsage {
            name: Some(name.into()),
            value: Some(val.into()),
            unit: unit.map(String::from),
            budget_role: role,
            ..Default::default()
        }
    }

    fn part(name: &str, attributes: Vec<AttributeUsage>) -> PackageMember {
        PackageMember::PartDef(PartDef {
            name: name.into(),
            attributes,
            ..Default::default()
        })
    }


    #[test]
    fn edge_ai_robot_critical_path_fps_and_hot_spots_from_roles() {
        let mut pkg = Package {
            name: "EdgeAiRobot".into(),
            ..Default::default()
        };
        pkg.members.push(part(
            "Camera",
            vec![
                attr("frameLatency", "0.016", Some("s"), Some(BudgetRole::Latency)),
                attr("ratedPower", "3.0", Some("W"), Some(BudgetRole::Power)),
                attr("mass", "0.05", Some("kg"), Some(BudgetRole::Mass)),
                attr("unitCost", "45.0", None, Some(BudgetRole::Cost)),
            ],
        ));
        pkg.members.push(part(
            "EdgeLlmAccelerator",
            vec![
                // LLM thinking time = time-to-first-token + decode time (40 tokens / 80 tok/s),
                // each a latency-role attribute: 120 ms + 500 ms = 620 ms.
                attr("ttft", "0.120", Some("s"), Some(BudgetRole::Latency)),
                attr("decodeTime", "0.5", Some("s"), Some(BudgetRole::Latency)),
                attr("ratedPower", "25.0", Some("W"), Some(BudgetRole::Power)),
                attr("eta", "0.75", None, Some(BudgetRole::Efficiency)),
                attr("mass", "80", Some("g"), Some(BudgetRole::Mass)),
                attr("unitCost", "149.0", None, Some(BudgetRole::Cost)),
            ],
        ));
        pkg.members.push(part(
            "MotorCtrl",
            vec![
                attr("loopLatency", "0.005", Some("s"), Some(BudgetRole::Latency)),
                attr("ratedPower", "12.0", Some("W"), Some(BudgetRole::Power)),
                attr("mass", "0.12", Some("kg"), Some(BudgetRole::Mass)),
                attr("unitCost", "35.0", None, Some(BudgetRole::Cost)),
            ],
        ));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "SmartRobot".into(),
            parts: vec![
                PartUsage { name: Some("cam".into()), type_names: vec!["Camera".into()], ..Default::default() },
                PartUsage { name: Some("llm".into()), type_names: vec!["EdgeLlmAccelerator".into()], ..Default::default() },
                PartUsage { name: Some("ctrl".into()), type_names: vec!["MotorCtrl".into()], ..Default::default() },
            ],
            connections: vec![
                ConnectionUsage { kind: ConnectionKind::Connection, from: Some("cam.out".into()), to: Some("llm.in".into()), ..Default::default() },
                ConnectionUsage { kind: ConnectionKind::Connection, from: Some("llm.out".into()), to: Some("ctrl.in".into()), ..Default::default() },
            ],
            ..Default::default()
        }));

        let model = Model::new(vec![pkg]);
        let limits = BudgetLimits {
            max_latency_ms: Some(500.0), // 641 ms > 500 ms -> FAIL!
            min_fps: Some(2.0),          // 1.61 FPS < 2.0 -> FAIL!
            max_power_w: Some(50.0),     // 40 W <= 50 W -> PASS
            max_mass_kg: Some(0.50),     // 0.25 kg <= 0.50 -> PASS
            max_cost_usd: Some(250.0),   // 229 <= 250 -> PASS
        };

        let report = analyze_budgets(&model, &limits);
        assert!(!report.ok);
        assert!((report.timing.critical_path_ms - 641.0).abs() < 1e-6);
        assert_eq!(report.timing.critical_path_nodes, vec!["cam", "llm", "ctrl"]);
        assert!(!report.timing.latency_pass);
        assert_eq!(report.timing.status, BudgetStatus::Fail);
        assert_eq!(report.timing.fps_bottleneck_stage.as_deref(), Some("llm"));
        assert!(!report.timing.fps_pass);
        assert_eq!(report.timing.stage_hot_spots[0].element, "llm");
        assert_eq!(report.power.status, BudgetStatus::Pass);
        assert!(report.power.pass && (report.power.total - 40.0).abs() < 1e-6);
        assert!(report.mass.pass && (report.mass.total - 0.25).abs() < 1e-6);
        assert!(report.cost.pass && (report.cost.total - 229.0).abs() < 1e-6);
        assert!((report.waste_heat.total - 6.25).abs() < 1e-6);
    }

    /// P2 regression: a 2000 W element named `ratedPowerW` without a role used to be
    /// invisible (total 0 W, `pass: true`). It is now `untracked` and the limited power domain
    /// is `incomplete`, so `ok` is false.
    #[test]
    fn untagged_watts_are_untracked_and_never_a_silent_pass() {
        let pkg = Package {
            name: "WaterHeater".into(),
            members: vec![part(
                "HeatingElement",
                vec![
                    attr("ratedPowerW", "2000.0", Some("W"), None),
                    attr("supplyVoltage", "230.0", Some("V"), None),
                ],
            )],
            ..Default::default()
        };
        let model = Model::new(vec![pkg]);
        let r = analyze_budgets(
            &model,
            &BudgetLimits {
                max_power_w: Some(100.0),
                ..Default::default()
            },
        );
        assert!(!r.ok);
        assert_eq!(r.power.status, BudgetStatus::Incomplete);
        assert_eq!(r.power.contributors, 0);
        assert_eq!(r.power.untracked.len(), 1);
        assert_eq!(r.power.untracked[0].attribute, "ratedPowerW");
        assert_eq!(r.power.untracked[0].value, Some(2000.0));
        // No limit on mass: nothing to check.
        assert_eq!(r.mass.status, BudgetStatus::Pass);

        // Tagging it makes the budget decidable: 2000 W > 100 W -> fail.
        let mut tagged = model.clone();
        if let PackageMember::PartDef(p) = &mut tagged.packages[0].members[0] {
            p.attributes[0].budget_role = Some(BudgetRole::Power);
        }
        let r = analyze_budgets(
            &tagged,
            &BudgetLimits {
                max_power_w: Some(100.0),
                ..Default::default()
            },
        );
        assert_eq!(r.power.status, BudgetStatus::Fail);
        assert!((r.power.total - 2000.0).abs() < 1e-9);
        assert!(r.power.untracked.is_empty());
    }

    #[test]
    fn tbd_and_none_roles_and_inherited_std_roles() {
        let base = PartDef {
            name: "PowerElement".into(),
            is_abstract: true,
            attributes: vec![AttributeUsage {
                name: Some("ratedPower".into()),
                unit: Some("W".into()),
                budget_role: Some(BudgetRole::Power),
                ..Default::default()
            }],
            ..Default::default()
        };
        let fan = PartDef {
            name: "Fan".into(),
            specializes: vec!["PowerElement".into()],
            attributes: vec![
                AttributeUsage {
                    redefines: vec!["ratedPower".into()],
                    value: Some("170.0".into()),
                    unit: Some("W".into()),
                    ..Default::default()
                },
                attr("supplyCapacity", "500.0", Some("W"), Some(BudgetRole::None)),
            ],
            ..Default::default()
        };
        let pump = PartDef {
            name: "Pump".into(),
            specializes: vec!["PowerElement".into()],
            attributes: vec![AttributeUsage {
                redefines: vec!["ratedPower".into()],
                unit: Some("W".into()),
                tbd: vec![sysml_ir::TbdMark::new(sysml_ir::TbdKind::Value, "datasheet pending")],
                ..Default::default()
            }],
            ..Default::default()
        };
        let model = Model::new(vec![Package {
            name: "Vent".into(),
            members: vec![
                PackageMember::PartDef(base),
                PackageMember::PartDef(fan),
                PackageMember::PartDef(pump),
            ],
            ..Default::default()
        }]);
        let r = analyze_budgets(
            &model,
            &BudgetLimits {
                max_power_w: Some(1000.0),
                ..Default::default()
            },
        );
        assert!((r.power.total - 170.0).abs() < 1e-9);
        assert!(r.power.untracked.is_empty(), "{:?}", r.power.untracked);
        assert_eq!(r.power.unresolved.len(), 1);
        assert_eq!(r.power.unresolved[0].element, "Pump");
        assert_eq!(r.power.status, BudgetStatus::Incomplete);
        assert!(!r.ok);
    }
}
