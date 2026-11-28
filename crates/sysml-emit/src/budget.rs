//! `budget` — cross-domain static budget rollup, DAG critical-path latency (including LLM
//! "thinking" time), pipeline FPS bottleneck, and top-3 hot-spot attribution over `sysml_ir::Model`.
//!
//! Runs in sub-millisecond time directly on the in-memory IR with zero external dependencies.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sysml_ir::{AttributeUsage, Model, PackageMember, PartDef};

use crate::emit_cli::is_sysml_std_package;

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

/// Additive rollup for one physical/economic domain (mass, power, waste heat, cost).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainRollup {
    pub domain: String,
    pub unit: String,
    pub total: f64,
    pub limit: Option<f64>,
    pub margin: Option<f64>,
    pub pass: bool,
    pub hot_spots: Vec<HotSpot>,
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
}

/// Complete cross-domain budget report produced by [`analyze_budgets`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetReport {
    pub ok: bool,
    pub mass: DomainRollup,
    pub power: DomainRollup,
    pub waste_heat: DomainRollup,
    pub cost: DomainRollup,
    pub timing: TimingBudget,
}

/// Analyze cross-domain budgets, DAG critical-path latency (including LLM thinking time),
/// pipeline FPS bottleneck, and top-3 hot spots on a [`Model`].
pub fn analyze_budgets(model: &Model, limits: &BudgetLimits) -> BudgetReport {
    let mut part_defs: BTreeMap<String, &PartDef> = BTreeMap::new();
    for pkg in &model.packages {
        if is_sysml_std_package(pkg) {
            continue;
        }
        for m in &pkg.members {
            if let PackageMember::PartDef(p) = m {
                part_defs.insert(p.name.clone(), p);
            }
        }
    }

    let mut mass_items: Vec<(String, f64)> = Vec::new();
    let mut power_items: Vec<(String, f64)> = Vec::new();
    let mut heat_items: Vec<(String, f64)> = Vec::new();
    let mut cost_items: Vec<(String, f64)> = Vec::new();
    let mut stage_latency_ms: BTreeMap<String, f64> = BTreeMap::new();

    // Leaf/component contributions (skip pure container assemblies when summing if they only compose parts)
    for (name, p) in &part_defs {
        if p.is_abstract {
            continue;
        }
        if let Some(m) = attr_number(&p.attributes, &["mass", "massKg", "dryMass", "waterMass"]) {
            if m > 0.0 {
                mass_items.push((name.clone(), m));
            }
        }
        if let Some(pw) = attr_number(&p.attributes, &["ratedPower", "powerDraw", "powerW"]) {
            if pw > 0.0 {
                power_items.push((name.clone(), pw));
                if let Some(eta) = attr_number(&p.attributes, &["efficiency"]) {
                    if (0.0..1.0).contains(&eta) {
                        heat_items.push((name.clone(), pw * (1.0 - eta)));
                    }
                }
            }
        }
        if let Some(h) = attr_number(&p.attributes, &["heatDissipationW", "wasteHeatW"]) {
            if h > 0.0 {
                heat_items.push((name.clone(), h));
            }
        }
        if let Some(c) = attr_number(&p.attributes, &["unitCost", "costUsd", "bomCost"]) {
            if c > 0.0 {
                cost_items.push((name.clone(), c));
            }
        }

        // Compute stage latency (ms): explicit latency/wcet OR LLM thinking time formula
        if let Some(lat) = compute_stage_latency_ms(&p.attributes) {
            stage_latency_ms.insert(name.clone(), lat);
        }
    }

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

    let mass = build_rollup("mass", "kg", &mass_items, limits.max_mass_kg);
    let power = build_rollup("power", "W", &power_items, limits.max_power_w);
    let waste_heat = build_rollup("waste_heat", "W", &heat_items, None);
    let cost = build_rollup("cost", "USD", &cost_items, limits.max_cost_usd);

    let ok = mass.pass && power.pass && waste_heat.pass && cost.pass && latency_pass && fps_pass;

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
        },
    }
}

/// Render a [`BudgetReport`] as clean Markdown.
pub fn render_budget_markdown(r: &BudgetReport) -> String {
    let mut out = String::new();
    out.push_str("# Cross-Domain System Budget & Critical-Path Report\n\n");
    out.push_str(&format!(
        "- **Overall Status:** {}\n",
        if r.ok { "PASS" } else { "FAIL" }
    ));
    out.push_str(&format!(
        "- **Critical Path Latency:** {:.2} ms ({}){}\n",
        r.timing.critical_path_ms,
        if r.timing.critical_path_nodes.is_empty() {
            "none".to_string()
        } else {
            r.timing.critical_path_nodes.join(" -> ")
        },
        r.timing
            .latency_margin_ms
            .map(|m| format!(" | margin: {m:+.2} ms"))
            .unwrap_or_default()
    ));
    if let Some(fps) = r.timing.pipeline_fps {
        out.push_str(&format!(
            "- **Pipeline Throughput:** {:.2} FPS (bottleneck: `{}`)\n",
            fps,
            r.timing.fps_bottleneck_stage.as_deref().unwrap_or("—")
        ));
    }
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
            "- **{}:** {:.2} {}{} [{}]{}\n",
            d.domain,
            d.total,
            d.unit,
            lim,
            if d.pass { "PASS" } else { "FAIL" },
            if top.is_empty() {
                String::new()
            } else {
                format!(" — hot spots: {top}")
            }
        ));
    }
    out
}

fn compute_stage_latency_ms(attrs: &[AttributeUsage]) -> Option<f64> {
    // 1. LLM thinking time: ttft + tokens_out / tok_per_s
    let ttft_ms = attr_time_ms(attrs, &["ttftMs", "ttft"]);
    let tokens_out = attr_number(attrs, &["tokensOut", "outputTokens"]);
    let tok_per_s = attr_number(attrs, &["tokPerS", "tokensPerSecond"]);
    if let (Some(ttft), Some(toks), Some(rate)) = (ttft_ms, tokens_out, tok_per_s) {
        if rate > 0.0 {
            return Some(ttft + (toks / rate) * 1000.0);
        }
    }
    // 2. Direct stage latency / WCET / cutoff latency
    attr_time_ms(
        attrs,
        &["latencyMs", "latency", "wcetMs", "cutoffLatency", "switchingCutoffLatency"],
    )
}

fn attr_effective_name(a: &AttributeUsage) -> Option<&str> {
    a.name
        .as_deref()
        .or_else(|| a.redefines.first().map(String::as_str))
}

fn attr_time_ms(attrs: &[AttributeUsage], names: &[&str]) -> Option<f64> {
    for a in attrs {
        let Some(aname) = attr_effective_name(a) else {
            continue;
        };
        if names.iter().any(|n| n.eq_ignore_ascii_case(aname)) {
            if let Some(v) = parse_f64(a.value.as_deref()) {
                let unit = a.unit.as_deref().unwrap_or("");
                if unit == "s"
                    || (!aname.to_lowercase().ends_with("ms")
                        && unit != "ms"
                        && v < 1.0
                        && aname.to_lowercase().contains("latency"))
                {
                    return Some(v * 1000.0);
                }
                return Some(v);
            }
        }
    }
    None
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
    // Handle `12.5 [W]` or plain `12.5`
    let token = s.split_whitespace().next()?;
    token.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn build_rollup(
    domain: &str,
    unit: &str,
    items: &[(String, f64)],
    limit: Option<f64>,
) -> DomainRollup {
    let total: f64 = items.iter().map(|(_, v)| *v).sum();
    let (margin, pass) = match limit {
        Some(l) => (Some(round2(l - total)), total <= l + 1e-9),
        None => (None, true),
    };
    DomainRollup {
        domain: domain.to_string(),
        unit: unit.to_string(),
        total: round2(total),
        limit,
        margin,
        pass,
        hot_spots: build_hot_spots(items, total),
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
    use sysml_ir::{ConnectionKind, ConnectionUsage, Package, PartUsage};

    fn attr(name: &str, val: &str, unit: Option<&str>) -> AttributeUsage {
        AttributeUsage {
            name: Some(name.into()),
            value: Some(val.into()),
            unit: unit.map(String::from),
            ..Default::default()
        }
    }

    #[test]
    fn edge_ai_robot_critical_path_llm_thinking_time_fps_and_hot_spots() {
        let mut pkg = Package {
            name: "EdgeAiRobot".into(),
            ..Default::default()
        };
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "Camera".into(),
            attributes: vec![
                attr("latencyMs", "16.0", Some("ms")),
                attr("ratedPower", "3.0", Some("W")),
                attr("mass", "0.05", Some("kg")),
                attr("unitCost", "45.0", Some("USD")),
            ],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "EdgeLlmAccelerator".into(),
            attributes: vec![
                // 120 ms TTFT + 40 tokens / 80 tok/s = 620 ms total!
                attr("ttft", "0.120", Some("s")),
                attr("tokensOut", "40.0", None),
                attr("tokPerS", "80.0", Some("Hz")),
                attr("ratedPower", "25.0", Some("W")),
                attr("efficiency", "0.75", None),
                attr("mass", "0.08", Some("kg")),
                attr("unitCost", "149.0", Some("USD")),
            ],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "MotorCtrl".into(),
            attributes: vec![
                attr("latencyMs", "5.0", Some("ms")),
                attr("ratedPower", "12.0", Some("W")),
                attr("mass", "0.12", Some("kg")),
                attr("unitCost", "35.0", Some("USD")),
            ],
            ..Default::default()
        }));
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
            max_cost_usd: Some(250.0),   // $229 <= $250 -> PASS
        };

        let report = analyze_budgets(&model, &limits);
        assert!(!report.ok);
        assert!((report.timing.critical_path_ms - 641.0).abs() < 1e-6);
        assert_eq!(report.timing.critical_path_nodes, vec!["cam", "llm", "ctrl"]);
        assert!(!report.timing.latency_pass);
        assert_eq!(report.timing.fps_bottleneck_stage.as_deref(), Some("llm"));
        assert!(!report.timing.fps_pass);
        assert_eq!(report.timing.stage_hot_spots[0].element, "llm");
        assert!(report.power.pass && (report.power.total - 40.0).abs() < 1e-6);
        assert!(report.mass.pass && (report.mass.total - 0.25).abs() < 1e-6);
        assert!(report.cost.pass && (report.cost.total - 229.0).abs() < 1e-6);
        assert!((report.waste_heat.total - 6.25).abs() < 1e-6);
    }
}
