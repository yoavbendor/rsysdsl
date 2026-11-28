//! Fast, in-memory IR validator producing crisp, actionable diagnostics for both
//! humans and AI agents (human-readable `Display` + `--json` serialization).
//!
//! Runs in sub-millisecond to low-millisecond time even on 2,000-element models,
//! avoiding slow or cryptic `rustc` trait-bound diagnostics for cross-element
//! architectural checks.
//!
//! Diagnostic codes:
//! - `E001`: Broken port connection (missing partition/port, direction mismatch, or payload type mismatch).
//! - `E002`: Dangling `satisfy` / `verify` requirement reference.
//! - `E003`: Timing sanity violation (`wcet_ms > deadline_ms`, `deadline_ms > period_ms`, or
//!           summed `wcet_ms / period_ms` on a `ComputeNode` exceeding `max_utilization`).
//! - `E004`: ISO 26262 / ISO/PAS 8800 guardrail violation — high-ASIL partition (`ASIL_C`/`ASIL_D`)
//!           consuming data from a `QM` or `is_ai_component = true` partition without an explicit
//!           safety monitor/guardrail or fallback state.
//! - `W001`: Shallow-model warning — safety-critical `SwPartition` (`ASIL_A..D`) has no
//!           `ComputeNode` allocation yet (warning only, so shallow models pass cleanly).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use sysml_ir::swhw::Asil;
use sysml_ir::{Model, PackageMember, SwHwSystem, SwPartition, SwPortDirection};

/// Severity of an [`IrDiagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IrSeverity {
    Error,
    Warning,
}

impl fmt::Display for IrSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IrSeverity::Error => write!(f, "error"),
            IrSeverity::Warning => write!(f, "warning"),
        }
    }
}

/// A single structured, actionable IR diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrDiagnostic {
    pub code: String,
    pub severity: IrSeverity,
    pub element: String,
    pub source_hint: String,
    pub message: String,
    pub suggested_fix: String,
}

impl fmt::Display for IrDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}[{}]: {} ({})\n  element: {}\n  fix: {}",
            self.severity,
            self.code,
            self.message,
            self.source_hint,
            self.element,
            self.suggested_fix
        )
    }
}

/// Complete report returned by [`validate_ir`] / [`validate_swhw_ir`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrValidationReport {
    pub ok: bool,
    pub error_count: usize,
    pub warning_count: usize,
    pub diagnostics: Vec<IrDiagnostic>,
}

impl IrValidationReport {
    pub fn from_diagnostics(diagnostics: Vec<IrDiagnostic>) -> Self {
        let error_count = diagnostics
            .iter()
            .filter(|d| d.severity == IrSeverity::Error)
            .count();
        let warning_count = diagnostics
            .iter()
            .filter(|d| d.severity == IrSeverity::Warning)
            .count();
        Self {
            ok: error_count == 0,
            error_count,
            warning_count,
            diagnostics,
        }
    }

    pub fn to_json_pretty(&self) -> String {
        serde_json::to_string_pretty(self).expect("IrValidationReport serializes")
    }

    pub fn render_human(&self) -> String {
        if self.diagnostics.is_empty() {
            return "OK: 0 errors, 0 warnings".to_string();
        }
        let mut lines = Vec::with_capacity(self.diagnostics.len() + 1);
        for d in &self.diagnostics {
            lines.push(d.to_string());
        }
        lines.push(format!(
            "Summary: {} error(s), {} warning(s) [{}]",
            self.error_count,
            self.warning_count,
            if self.ok { "PASS" } else { "FAIL" }
        ));
        lines.join("\n\n")
    }
}

/// Validate a shallow-first [`SwHwSystem`] in memory and emit crisp diagnostics (`E001`–`E004`, `W001`).
pub fn validate_ir(system: &SwHwSystem) -> IrValidationReport {
    let mut diags = Vec::new();

    let partitions: BTreeMap<&str, &SwPartition> = system
        .partitions
        .iter()
        .map(|p| (p.name.as_str(), p))
        .collect();
    let nodes: BTreeMap<&str, &sysml_ir::ComputeNode> = system
        .compute_nodes
        .iter()
        .map(|n| (n.name.as_str(), n))
        .collect();
    let buses: BTreeSet<&str> = system.buses.iter().map(|b| b.name.as_str()).collect();
    let req_ids: BTreeSet<&str> = system.requirements.iter().map(|r| r.id.as_str()).collect();
    let req_names: BTreeSet<&str> = system.requirements.iter().map(|r| r.name.as_str()).collect();

    // -------------------------------------------------------------------------
    // E001: Broken port connections, direction mismatch, payload type mismatch,
    //       or unknown bus binding.
    // -------------------------------------------------------------------------
    for conn in &system.connections {
        let src_part = partitions.get(conn.from_partition.as_str());
        let dst_part = partitions.get(conn.to_partition.as_str());

        if src_part.is_none() {
            diags.push(IrDiagnostic {
                code: "E001".into(),
                severity: IrSeverity::Error,
                element: conn.name.clone(),
                source_hint: format!("{}.{}", system.package_name, conn.name),
                message: format!(
                    "connection `{}` references unknown source partition `{}`",
                    conn.name, conn.from_partition
                ),
                suggested_fix: format!(
                    "add SwPartition::new(\"{}\") or fix `from_partition` on `{}`",
                    conn.from_partition, conn.name
                ),
            });
            continue;
        }
        if dst_part.is_none() {
            diags.push(IrDiagnostic {
                code: "E001".into(),
                severity: IrSeverity::Error,
                element: conn.name.clone(),
                source_hint: format!("{}.{}", system.package_name, conn.name),
                message: format!(
                    "connection `{}` references unknown target partition `{}`",
                    conn.name, conn.to_partition
                ),
                suggested_fix: format!(
                    "add SwPartition::new(\"{}\") or fix `to_partition` on `{}`",
                    conn.to_partition, conn.name
                ),
            });
            continue;
        }

        let src_p = src_part.unwrap();
        let dst_p = dst_part.unwrap();
        let src_port = src_p.ports.iter().find(|p| p.name == conn.from_port);
        let dst_port = dst_p.ports.iter().find(|p| p.name == conn.to_port);

        if src_port.is_none() {
            diags.push(IrDiagnostic {
                code: "E001".into(),
                severity: IrSeverity::Error,
                element: conn.name.clone(),
                source_hint: format!("{}.{}", src_p.name, conn.from_port),
                message: format!(
                    "connection `{}` references missing port `{}` on partition `{}`",
                    conn.name, conn.from_port, src_p.name
                ),
                suggested_fix: format!(
                    "add `.port(SwPort::output(\"{}\", \"<PayloadType>\"))` to `{}`",
                    conn.from_port, src_p.name
                ),
            });
        }
        if dst_port.is_none() {
            diags.push(IrDiagnostic {
                code: "E001".into(),
                severity: IrSeverity::Error,
                element: conn.name.clone(),
                source_hint: format!("{}.{}", dst_p.name, conn.to_port),
                message: format!(
                    "connection `{}` references missing port `{}` on partition `{}`",
                    conn.name, conn.to_port, dst_p.name
                ),
                suggested_fix: format!(
                    "add `.port(SwPort::input(\"{}\", \"<PayloadType>\"))` to `{}`",
                    conn.to_port, dst_p.name
                ),
            });
        }

        if let (Some(sp), Some(dp)) = (src_port, dst_port) {
            if sp.direction != SwPortDirection::Out || dp.direction != SwPortDirection::In {
                diags.push(IrDiagnostic {
                    code: "E001".into(),
                    severity: IrSeverity::Error,
                    element: conn.name.clone(),
                    source_hint: format!(
                        "{}.{} -> {}.{}",
                        src_p.name, sp.name, dst_p.name, dp.name
                    ),
                    message: format!(
                        "port direction mismatch on `{}`: `{}.{}` is {:?}, `{}.{}` is {:?} (expected Out -> In)",
                        conn.name, src_p.name, sp.name, sp.direction, dst_p.name, dp.name, dp.direction
                    ),
                    suggested_fix: format!(
                        "make `{}.{}` an output port and `{}.{}` an input port",
                        src_p.name, sp.name, dst_p.name, dp.name
                    ),
                });
            }
            if sp.payload_type != dp.payload_type {
                diags.push(IrDiagnostic {
                    code: "E001".into(),
                    severity: IrSeverity::Error,
                    element: conn.name.clone(),
                    source_hint: format!(
                        "{}.{} ({}) -> {}.{} ({})",
                        src_p.name, sp.name, sp.payload_type, dst_p.name, dp.name, dp.payload_type
                    ),
                    message: format!(
                        "port payload type mismatch on `{}`: `{}` emits `{}` but `{}` expects `{}`",
                        conn.name, src_p.name, sp.payload_type, dst_p.name, dp.payload_type
                    ),
                    suggested_fix: format!(
                        "align payload types on `{}.{}` and `{}.{}` or insert an explicit adapter partition",
                        src_p.name, sp.name, dst_p.name, dp.name
                    ),
                });
            }
        }

        if let Some(bus_name) = &conn.bus {
            if !buses.contains(bus_name.as_str()) {
                diags.push(IrDiagnostic {
                    code: "E001".into(),
                    severity: IrSeverity::Error,
                    element: conn.name.clone(),
                    source_hint: format!("{}.bus", conn.name),
                    message: format!(
                        "connection `{}` references unknown bus `{bus_name}`",
                        conn.name
                    ),
                    suggested_fix: format!("add `BusChannel::new(\"{bus_name}\")` to the system"),
                });
            }
        }

        // ---------------------------------------------------------------------
        // E004: ISO 26262 / ISO/PAS 8800 guardrail check
        // High-ASIL consumer (ASIL_C / ASIL_D) receiving data from QM or AI
        // partition without an explicit safety guardrail or fallback state.
        // ---------------------------------------------------------------------
        let dst_high_asil = dst_p.asil.is_some_and(Asil::is_high_integrity);
        let src_untrusted = src_p.is_ai_component || matches!(src_p.asil, Some(Asil::QM) | None);
        if dst_high_asil && src_untrusted && !src_p.is_safety_guardrail {
            let dst_has_protection = dst_p.is_safety_guardrail || dst_p.fallback_state.is_some();
            if !dst_has_protection {
                diags.push(IrDiagnostic {
                    code: "E004".into(),
                    severity: IrSeverity::Error,
                    element: conn.name.clone(),
                    source_hint: format!("{} -> {}", src_p.name, dst_p.name),
                    message: format!(
                        "high-integrity partition `{}` ({}) directly consumes data from `{}` ({}{}) without a safety guardrail or fallback state",
                        dst_p.name,
                        dst_p.asil.unwrap().as_str(),
                        src_p.name,
                        src_p.asil.map_or("unspecified ASIL", |a| a.as_str()),
                        if src_p.is_ai_component { ", AI component" } else { "" }
                    ),
                    suggested_fix: format!(
                        "route `{}` through an explicit `.safety_guardrail(true)` monitor partition or declare `.fallback_state(\"SafeStop\")` on `{}`",
                        conn.name, dst_p.name
                    ),
                });
            }
        }
    }

    // -------------------------------------------------------------------------
    // E002: Dangling satisfy / verify requirement references
    // -------------------------------------------------------------------------
    for p in &system.partitions {
        for req_ref in &p.satisfies {
            if !req_ids.contains(req_ref.as_str()) && !req_names.contains(req_ref.as_str()) {
                diags.push(IrDiagnostic {
                    code: "E002".into(),
                    severity: IrSeverity::Error,
                    element: p.name.clone(),
                    source_hint: format!("{}.satisfies", p.name),
                    message: format!(
                        "partition `{}` satisfies unknown requirement `{req_ref}`",
                        p.name
                    ),
                    suggested_fix: format!(
                        "declare `SwRequirement::new(\"{req_ref}\", ...)` or fix the requirement ID on `{}`",
                        p.name
                    ),
                });
            }
        }
    }
    for r in &system.requirements {
        for verifier in &r.verified_by {
            if !partitions.contains_key(verifier.as_str())
                && !nodes.contains_key(verifier.as_str())
            {
                diags.push(IrDiagnostic {
                    code: "E002".into(),
                    severity: IrSeverity::Error,
                    element: r.id.clone(),
                    source_hint: format!("{}.verified_by", r.id),
                    message: format!(
                        "requirement `{}` references unknown verifier element `{verifier}`",
                        r.id
                    ),
                    suggested_fix: format!(
                        "add partition/node `{verifier}` or update `verified_by` on `{}`",
                        r.id
                    ),
                });
            }
        }
    }

    // -------------------------------------------------------------------------
    // E003: Task timing sanity & ComputeNode utilization check
    // -------------------------------------------------------------------------
    for p in &system.partitions {
        for t in &p.tasks {
            let elem = format!("{}.{}", p.name, t.name);
            if let (Some(wcet), Some(deadline)) = (t.wcet_ms, t.deadline_ms) {
                if wcet > deadline {
                    diags.push(IrDiagnostic {
                        code: "E003".into(),
                        severity: IrSeverity::Error,
                        element: elem.clone(),
                        source_hint: format!("{elem} (wcet={wcet} ms, deadline={deadline} ms)"),
                        message: format!(
                            "task `{elem}` has wcet_ms ({wcet} ms) > deadline_ms ({deadline} ms)"
                        ),
                        suggested_fix: format!(
                            "reduce `wcet_ms` to <= {deadline} ms or relax `deadline_ms` on `{elem}`"
                        ),
                    });
                }
            }
            if let (Some(deadline), Some(period)) = (t.deadline_ms, t.period_ms) {
                if deadline > period {
                    diags.push(IrDiagnostic {
                        code: "E003".into(),
                        severity: IrSeverity::Error,
                        element: elem.clone(),
                        source_hint: format!("{elem} (deadline={deadline} ms, period={period} ms)"),
                        message: format!(
                            "task `{elem}` has deadline_ms ({deadline} ms) > period_ms ({period} ms)"
                        ),
                        suggested_fix: format!(
                            "set `deadline_ms` <= {period} ms or increase `period_ms` on `{elem}`"
                        ),
                    });
                }
            }
            if let (Some(wcet), Some(period)) = (t.wcet_ms, t.period_ms) {
                if period <= 0.0 || wcet < 0.0 {
                    diags.push(IrDiagnostic {
                        code: "E003".into(),
                        severity: IrSeverity::Error,
                        element: elem.clone(),
                        source_hint: format!("{elem} (wcet={wcet} ms, period={period} ms)"),
                        message: format!(
                            "task `{elem}` has non-positive period ({period} ms) or negative wcet ({wcet} ms)"
                        ),
                        suggested_fix: format!("set positive `period_ms` and non-negative `wcet_ms` on `{elem}`"),
                    });
                } else if t.deadline_ms.is_none() && wcet > period {
                    diags.push(IrDiagnostic {
                        code: "E003".into(),
                        severity: IrSeverity::Error,
                        element: elem.clone(),
                        source_hint: format!("{elem} (wcet={wcet} ms, period={period} ms)"),
                        message: format!(
                            "task `{elem}` has wcet_ms ({wcet} ms) > period_ms ({period} ms)"
                        ),
                        suggested_fix: format!(
                            "reduce `wcet_ms` to <= {period} ms or increase `period_ms` on `{elem}`"
                        ),
                    });
                }
            }
        }
    }

    // Track allocations from partitions/tasks -> compute nodes
    let mut allocated_partitions: BTreeSet<&str> = BTreeSet::new();
    let mut node_utilization: BTreeMap<&str, f64> = BTreeMap::new();

    for alloc in &system.allocations {
        if let Some(node) = nodes.get(alloc.target.as_str()) {
            if let Some(part) = partitions.get(alloc.source.as_str()) {
                allocated_partitions.insert(part.name.as_str());
                let part_util: f64 = part
                    .tasks
                    .iter()
                    .filter_map(|t| match (t.wcet_ms, t.period_ms) {
                        (Some(w), Some(p)) if p > 0.0 => Some(w / p),
                        _ => None,
                    })
                    .sum();
                *node_utilization.entry(node.name.as_str()).or_insert(0.0) += part_util;
            } else if let Some((p_name, t_name)) = alloc.source.split_once('.') {
                if let Some(part) = partitions.get(p_name) {
                    allocated_partitions.insert(part.name.as_str());
                    if let Some(task) = part.tasks.iter().find(|t| t.name == t_name) {
                        if let (Some(w), Some(p)) = (task.wcet_ms, task.period_ms) {
                            if p > 0.0 {
                                *node_utilization.entry(node.name.as_str()).or_insert(0.0) += w / p;
                            }
                        }
                    }
                }
            }
        }
    }

    for (node_name, used_util) in &node_utilization {
        let node = nodes[node_name];
        if let Some(max_u) = node.max_utilization {
            let capacity = max_u * (node.cores.unwrap_or(1) as f64);
            if *used_util > capacity + 1e-9 {
                diags.push(IrDiagnostic {
                    code: "E003".into(),
                    severity: IrSeverity::Error,
                    element: (*node_name).to_string(),
                    source_hint: format!(
                        "{node_name} (used={:.3}, limit={:.3} across {} core(s))",
                        used_util,
                        capacity,
                        node.cores.unwrap_or(1)
                    ),
                    message: format!(
                        "compute node `{node_name}` utilization {:.1}% exceeds limit {:.1}% ({} core(s) x {:.0}%)",
                        used_util * 100.0,
                        capacity * 100.0,
                        node.cores.unwrap_or(1),
                        max_u * 100.0
                    ),
                    suggested_fix: format!(
                        "offload a task from `{node_name}`, reduce task `wcet_ms`, or add cores"
                    ),
                });
            }
        }
    }

    // -------------------------------------------------------------------------
    // W001: Safety-critical SwPartition has no ComputeNode allocation yet
    // (warning only, so shallow models pass cleanly)
    // -------------------------------------------------------------------------
    for p in &system.partitions {
        if p.asil.is_some_and(Asil::is_safety_critical)
            && !allocated_partitions.contains(p.name.as_str())
        {
            diags.push(IrDiagnostic {
                code: "W001".into(),
                severity: IrSeverity::Warning,
                element: p.name.clone(),
                source_hint: format!("{} ({})", p.name, p.asil.unwrap().as_str()),
                message: format!(
                    "safety-critical partition `{}` ({}) is not allocated to any ComputeNode yet",
                    p.name,
                    p.asil.unwrap().as_str()
                ),
                suggested_fix: format!(
                    "add `SwAllocation::new(\"alloc_{}\", \"{}\", \"<ComputeNode>\")` when hardware mapping is decided",
                    p.name, p.name
                ),
            });
        }
    }

    IrValidationReport::from_diagnostics(diags)
}

/// Fast in-memory structural validation for any generic [`sysml_ir::Model`] (`E001` broken
/// connection endpoints and `E002` dangling `satisfy` requirement references).
pub fn validate_model_ir(model: &Model) -> IrValidationReport {
    let mut diags = Vec::new();
    let mut req_names: BTreeSet<String> = BTreeSet::new();
    let mut part_defs: BTreeMap<String, &sysml_ir::PartDef> = BTreeMap::new();

    for pkg in &model.packages {
        for m in &pkg.members {
            match m {
                PackageMember::RequirementDef(r) => {
                    req_names.insert(r.name.clone());
                    if let Some(sn) = &r.short_name {
                        req_names.insert(sn.clone());
                    }
                    req_names.insert(format!("{}::{}", pkg.name, r.name));
                }
                PackageMember::PartDef(p) => {
                    part_defs.insert(p.name.clone(), p);
                }
                _ => {}
            }
        }
    }

    for pkg in &model.packages {
        for m in &pkg.members {
            if let PackageMember::PartDef(p) = m {
                // E002: dangling satisfy
                for sat in &p.satisfies {
                    let simple = sat.requirement.rsplit("::").next().unwrap_or(&sat.requirement);
                    if !req_names.contains(&sat.requirement) && !req_names.contains(simple) {
                        diags.push(IrDiagnostic {
                            code: "E002".into(),
                            severity: IrSeverity::Error,
                            element: format!("{}::{}", pkg.name, p.name),
                            source_hint: format!("satisfy {}", sat.requirement),
                            message: format!(
                                "part `{}` satisfies unknown requirement `{}`",
                                p.name, sat.requirement
                            ),
                            suggested_fix: format!(
                                "define requirement `{}` or fix the `satisfy` target on `{}`",
                                sat.requirement, p.name
                            ),
                        });
                    }
                }
                // E001: connection endpoints referencing unknown direct child parts
                let child_names: BTreeSet<&str> = p
                    .parts
                    .iter()
                    .filter_map(|c| c.name.as_deref())
                    .collect();
                let own_ports: BTreeSet<&str> = p.ports.iter().map(|pt| pt.name.as_str()).collect();
                for conn in &p.connections {
                    for (side, endpoint_opt) in [("from", &conn.from), ("to", &conn.to)] {
                        let Some(endpoint) = endpoint_opt.as_deref() else {
                            continue;
                        };
                        let root = endpoint.split('.').next().unwrap_or(endpoint);
                        if !child_names.contains(root) && !own_ports.contains(root) {
                            diags.push(IrDiagnostic {
                                code: "E001".into(),
                                severity: IrSeverity::Error,
                                element: format!(
                                    "{}::{}::{}",
                                    pkg.name,
                                    p.name,
                                    conn.name.as_deref().unwrap_or("<unnamed>")
                                ),
                                source_hint: format!("{side} = {endpoint}"),
                                message: format!(
                                    "connection `{}` on `{}` references unknown {side} endpoint `{endpoint}`",
                                    conn.name.as_deref().unwrap_or("<unnamed>"),
                                    p.name
                                ),
                                suggested_fix: format!(
                                    "add child part/port `{root}` to `{}` or fix `{side} = {endpoint}`",
                                    p.name
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    IrValidationReport::from_diagnostics(diags)
}
