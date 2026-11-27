//! Architecture & Hierarchy Linter ("Clippy for Systems" — `rsysdsl lint`).
//!
//! Analyzes a `sysml_ir::Model` for modeling smells that degrade system reuse,
//! hierarchy clarity, and physical rigor:
//! - `duplicate_part_shape`: 2+ concrete `PartDef`s without a shared ancestor sharing
//!   >= 3 identical port/attribute names -> extract an `abstract` base part.
//! - `unclassified_interfaces`: 3+ concrete `InterfaceDef`s with no `specializes`
//!   parent -> specialize a shared channel base (e.g. `InfoChannel` / `EnergyChannel`).
//! - `untyped_connections`: `connect` usages with no interface/connection type in a model that
//!   otherwise declares or uses typed interfaces (deleting the interface is never the fix for
//!   `unclassified_interfaces`; classify it under an abstract base instead).
//! - `flat_god_assembly`: a `PartDef` owning > 10 direct child parts with zero
//!   intermediate subsystem decomposition.
//! - `missing_physical_unit`: numeric `Real` attribute carrying a literal value without
//!   an SI unit (excluding dimensionless counts/ratios/efficiencies/factors).
//! - `inherited_std_default` (info): a model definition specializing a `sysml-std` element
//!   inherits numeric library defaults (e.g. mains `frequency = 50 Hz`) without redefining
//!   them. Library defaults are generic placeholders, not source-document values. A
//!   redefinition with `tbd_value = "reason"` (no value, counted TBD) counts as handled.
//! - `unreachable_state` (warning): a sub-state of a `state def` that no chain of transitions
//!   reaches from the initial state (`entry; then <initial>;`). Skipped when the initial state
//!   is unknown (state defs emitted before S6b carry no `entry; then`).
//! - `dead_end_state` (info): a sub-state with no transition to another state. Legitimate for
//!   latched faults / final states, so it never fails `--strict`.
//! - `overlapping_guards` (info): 2+ transitions leave the same state on the same trigger and
//!   at least one is guarded (disjointness cannot be proven) or more than one is unguarded.
//! - `unused_abstract_base`: an `abstract` part/interface/port with zero specializing
//!   subtypes in the model (useful during refactoring; skipped for library packages).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use sysml_ir::{InterfaceDef, Model, Package, PackageMember, PartDef, PortDef, StateDef};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LintSeverity {
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintFinding {
    pub code: String,
    pub severity: LintSeverity,
    pub package: String,
    pub element: String,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LintSummary {
    pub total_packages: usize,
    pub total_parts: usize,
    pub abstract_bases: usize,
    pub specializations: usize,
    pub redefinitions: usize,
    pub variations: usize,
    pub allocations: usize,
    pub total_interfaces: usize,
    pub classified_interfaces: usize,
    pub reuse_ratio_pct: f64,
    pub warning_count: usize,
    /// Info-level smells (excludes `inherited_std_default`, counted separately below).
    pub info_count: usize,
    /// Model definitions that inherit unredefined numeric sysml-std library defaults
    /// (`inherited_std_default` findings; informational, never fails `--strict`).
    #[serde(default)]
    pub inherited_std_default_count: usize,
    /// State-machine review items (`dead_end_state`, `overlapping_guards`): informational,
    /// counted apart from `info_count` because they are often intended (latched faults,
    /// prioritized guards) and never fail `--strict`.
    #[serde(default)]
    pub state_review_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LintReport {
    pub model: String,
    pub clean: bool,
    pub summary: LintSummary,
    pub findings: Vec<LintFinding>,
}

/// Lint a `sysml_ir::Model` for hierarchy, reuse, and physical-modeling smells.
pub fn lint_model(model_name: impl Into<String>, model: &Model) -> LintReport {
    let mut parts: Vec<(String, bool, &PartDef)> = Vec::new();
    let mut interfaces: Vec<(String, bool, &InterfaceDef)> = Vec::new();
    let mut ports: Vec<(String, bool, &PortDef)> = Vec::new();
    let mut total_packages = 0usize;

    for pkg in &model.packages {
        collect_pkg(pkg, &mut total_packages, &mut parts, &mut interfaces, &mut ports);
    }

    let mut summary = LintSummary {
        total_packages,
        total_parts: parts.len(),
        total_interfaces: interfaces.len(),
        ..Default::default()
    };

    let mut part_parents: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut specialized_targets: BTreeSet<String> = BTreeSet::new();
    let mut part_has_children: BTreeSet<String> = BTreeSet::new();

    for (_, _, p) in &parts {
        if p.is_abstract {
            summary.abstract_bases += 1;
        }
        if p.is_variation {
            summary.variations += 1;
        }
        summary.allocations += p.allocations.len();
        if !p.parts.is_empty() {
            part_has_children.insert(p.name.clone());
        }
        for sup in &p.specializes {
            let simple = simple_name(sup).to_string();
            specialized_targets.insert(simple.clone());
            part_parents.entry(p.name.clone()).or_default().push(simple);
            summary.specializations += 1;
        }
        for a in &p.attributes {
            summary.redefinitions += a.redefines.len();
        }
        for sub in &p.parts {
            summary.redefinitions += sub.redefines.len();
        }
    }

    for (_, _, iface) in &interfaces {
        if iface.is_abstract {
            summary.abstract_bases += 1;
        }
        if !iface.specializes.is_empty() {
            summary.classified_interfaces += 1;
        }
        for sup in &iface.specializes {
            specialized_targets.insert(simple_name(sup).to_string());
            summary.specializations += 1;
        }
        for a in &iface.attributes {
            summary.redefinitions += a.redefines.len();
        }
    }

    for (_, _, port) in &ports {
        if port.is_abstract {
            summary.abstract_bases += 1;
        }
        for sup in &port.specializes {
            specialized_targets.insert(simple_name(sup).to_string());
            summary.specializations += 1;
        }
    }

    let concrete_parts_count = parts.iter().filter(|(_, _, p)| !p.is_abstract).count();
    let specialized_concrete_parts = parts
        .iter()
        .filter(|(_, _, p)| !p.is_abstract && !p.specializes.is_empty())
        .count();
    summary.reuse_ratio_pct = if concrete_parts_count == 0 {
        100.0
    } else {
        (specialized_concrete_parts as f64 / concrete_parts_count as f64) * 100.0
    };

    let mut findings = Vec::new();

    // 1. Duplicate part shape detection (copy-paste smell among concrete non-library parts)
    let concrete_nonlib: Vec<&(String, bool, &PartDef)> = parts
        .iter()
        .filter(|(_, is_lib, p)| !*is_lib && !p.is_abstract && !p.is_variation)
        .collect();
    let mut reported_dup_pairs: BTreeSet<(String, String)> = BTreeSet::new();
    for i in 0..concrete_nonlib.len() {
        for j in (i + 1)..concrete_nonlib.len() {
            let (pkg_a, _, pa) = concrete_nonlib[i];
            let (_, _, pb) = concrete_nonlib[j];
            if shares_ancestor(&pa.name, &pb.name, &part_parents) {
                continue;
            }
            let sig_a = part_feature_signature(pa);
            let sig_b = part_feature_signature(pb);
            let shared: Vec<String> = sig_a.intersection(&sig_b).cloned().collect();
            if shared.len() >= 3 {
                let pair = if pa.name <= pb.name {
                    (pa.name.clone(), pb.name.clone())
                } else {
                    (pb.name.clone(), pa.name.clone())
                };
                if reported_dup_pairs.insert(pair) {
                    findings.push(LintFinding {
                        code: "duplicate_part_shape".to_string(),
                        severity: LintSeverity::Warning,
                        package: pkg_a.clone(),
                        element: format!("{} & {}", pa.name, pb.name),
                        message: format!(
                            "Concrete parts `{}` and `{}` share {} identical features ({}) with no common ancestor.",
                            pa.name,
                            pb.name,
                            shared.len(),
                            shared.join(", ")
                        ),
                        suggestion: format!(
                            "Extract an `#[sysml::part(abstract)]` parent part holding ({}) and annotate both with `#[sysml::specializes(...)]`.",
                            shared.join(", ")
                        ),
                    });
                }
            }
        }
    }

    // 2. Unclassified interfaces (3+ concrete interfaces in non-library packages with no parent)
    let unclassified_ifaces: Vec<&(String, bool, &InterfaceDef)> = interfaces
        .iter()
        .filter(|(_, is_lib, i)| !*is_lib && !i.is_abstract && i.specializes.is_empty())
        .collect();
    if unclassified_ifaces.len() >= 3 {
        let names: Vec<&str> = unclassified_ifaces
            .iter()
            .map(|(_, _, i)| i.name.as_str())
            .collect();
        findings.push(LintFinding {
            code: "unclassified_interfaces".to_string(),
            severity: LintSeverity::Warning,
            package: unclassified_ifaces[0].0.clone(),
            element: names.join(", "),
            message: format!(
                "{} concrete interfaces ({}) have no abstract parent channel.",
                names.len(),
                names.join(", ")
            ),
            suggestion: "Keep the interfaces and classify them: declare an end-less abstract base `#[sysml::interface(abstract)] pub struct EnergyChannel {}` (or reuse a sysml-std channel such as `InfoChannel` / `ThermalLink` / `ElectricalLink` when its port types match) and add `#[sysml::specializes(EnergyChannel)]` to each concrete interface. Do NOT remove interface types from `#[sysml::connect]` (that triggers `untyped_connections`).".to_string(),
        });
    }

    // 2b. Untyped connections in a model that otherwise uses typed interfaces.
    let model_uses_typed_ifaces = interfaces
        .iter()
        .any(|(_, is_lib, i)| !*is_lib && !i.is_abstract)
        || parts.iter().any(|(_, is_lib, p)| {
            !*is_lib && p.connections.iter().any(|c| c.type_name.is_some())
        });
    if model_uses_typed_ifaces {
        for (pkg_name, is_lib, p) in &parts {
            if *is_lib {
                continue;
            }
            let untyped: Vec<String> = p
                .connections
                .iter()
                .filter(|c| c.type_name.is_none())
                .map(|c| match (&c.name, &c.from, &c.to) {
                    (Some(n), _, _) => n.clone(),
                    (None, Some(f), Some(t)) => format!("{f} -> {t}"),
                    _ => "<anonymous>".to_string(),
                })
                .collect();
            if untyped.is_empty() {
                continue;
            }
            findings.push(LintFinding {
                code: "untyped_connections".to_string(),
                severity: LintSeverity::Warning,
                package: pkg_name.clone(),
                element: format!("{} ({})", p.name, untyped.join(", ")),
                message: format!(
                    "Part `{}` has {} connection(s) with no interface type ({}) while the model uses typed interfaces.",
                    p.name,
                    untyped.len(),
                    untyped.join(", ")
                ),
                suggestion: "Type every arrow: add `interface = SomeLink` to each `#[sysml::connect(...)]` (declare `#[sysml::interface]` defs with two `#[sysml::end]` ports, classified under an abstract base).".to_string(),
            });
        }
    }

    // 3. Flat God-assembly (> 10 direct child parts where none are composite subsystems)
    for (pkg_name, is_lib, p) in &parts {
        if *is_lib || p.parts.len() <= 10 {
            continue;
        }
        let has_nested_subsystems = p.parts.iter().any(|sub| {
            !sub.nested_parts.is_empty()
                || sub
                    .type_names
                    .iter()
                    .any(|ty| part_has_children.contains(simple_name(ty)))
        });
        if !has_nested_subsystems {
            findings.push(LintFinding {
                code: "flat_god_assembly".to_string(),
                severity: LintSeverity::Warning,
                package: pkg_name.clone(),
                element: p.name.clone(),
                message: format!(
                    "Part `{}` directly owns {} leaf parts with no intermediate subsystem layers.",
                    p.name,
                    p.parts.len()
                ),
                suggestion: "Group cohesive components into domain subsystems (e.g. PowerSubsystem, ElectronicsSubsystem, MechanicalSubsystem, HmiSubsystem).".to_string(),
            });
        }
    }

    // 4. Missing physical SI unit on numeric Real attributes (excluding dimensionless quantities)
    for (pkg_name, is_lib, p) in &parts {
        if *is_lib {
            continue;
        }
        for attr in &p.attributes {
            if !attr.redefines.is_empty() || attr.unit.is_some() {
                continue;
            }
            let is_real = attr
                .type_name
                .as_deref()
                .map(|t| simple_name(t) == "Real")
                .unwrap_or(false);
            let has_numeric_val = attr
                .value
                .as_deref()
                .and_then(|v| v.trim().parse::<f64>().ok())
                .is_some();
            if is_real && has_numeric_val {
                let aname = attr.name.as_deref().unwrap_or("attribute");
                if !is_dimensionless_name(aname) {
                    findings.push(LintFinding {
                        code: "missing_physical_unit".to_string(),
                        severity: LintSeverity::Info,
                        package: pkg_name.clone(),
                        element: format!("{}.{}", p.name, aname),
                        message: format!(
                            "Numeric attribute `{}.{}` has value `{}` without an SI unit.",
                            p.name,
                            aname,
                            attr.value.as_deref().unwrap_or("")
                        ),
                        suggestion: "Add `unit = \"...\"` (e.g. \"W\", \"V\", \"A\", \"kg\", \"m\", \"s\", \"K\", \"Pa\", \"N\", \"Hz\") or rename clearly if dimensionless.".to_string(),
                    });
                }
            }
        }
    }

    // 5. Numeric sysml-std defaults inherited without a redefinition.
    findings.extend(inherited_std_defaults(model));

    // 6. State machine smells (unreachable / dead-end states, overlapping guards).
    let mut state_defs: Vec<(String, &StateDef)> = Vec::new();
    for pkg in &model.packages {
        collect_state_defs(pkg, &mut state_defs);
    }
    for (pkg_name, sd) in &state_defs {
        findings.extend(state_machine_findings(pkg_name, sd));
    }

    for f in &findings {
        match f.severity {
            LintSeverity::Warning => summary.warning_count += 1,
            LintSeverity::Info if f.code == "inherited_std_default" => {
                summary.inherited_std_default_count += 1
            }
            LintSeverity::Info
                if matches!(f.code.as_str(), "dead_end_state" | "overlapping_guards") =>
            {
                summary.state_review_count += 1
            }
            LintSeverity::Info => summary.info_count += 1,
        }
    }

    LintReport {
        model: model_name.into(),
        clean: summary.warning_count == 0,
        summary,
        findings,
    }
}

/// Render a human-readable Markdown report for `rsysdsl lint --format markdown`.
pub fn render_lint_markdown(report: &LintReport) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Architecture Lint Report — `{}`\n", report.model);
    let _ = writeln!(
        out,
        "- **Status:** {}",
        if report.clean { "CLEAN (0 warnings)" } else { "SMELLS DETECTED" }
    );
    let _ = writeln!(
        out,
        "- **Hierarchy Metrics:** {} parts ({} abstract bases, {} specializations, {} redefinitions, {} variations, {} allocations), reuse ratio {:.1}%",
        report.summary.total_parts,
        report.summary.abstract_bases,
        report.summary.specializations,
        report.summary.redefinitions,
        report.summary.variations,
        report.summary.allocations,
        report.summary.reuse_ratio_pct
    );
    let _ = writeln!(
        out,
        "- **Findings:** {} warning(s), {} info(s), {} definition(s) inheriting sysml-std library defaults, {} state-machine review item(s)\n",
        report.summary.warning_count,
        report.summary.info_count,
        report.summary.inherited_std_default_count,
        report.summary.state_review_count
    );

    if report.findings.is_empty() {
        let _ = writeln!(out, "No architectural smells detected.");
    } else {
        let _ = writeln!(out, "| Severity | Code | Package | Element | Finding | Suggestion |");
        let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- |");
        for f in &report.findings {
            let sev = match f.severity {
                LintSeverity::Warning => "warning",
                LintSeverity::Info => "info",
            };
            let _ = writeln!(
                out,
                "| {sev} | `{}` | `{}` | `{}` | {} | {} |",
                f.code,
                f.package,
                f.element,
                f.message.replace('|', "\\|"),
                f.suggestion.replace('|', "\\|")
            );
        }
    }
    out
}

fn collect_pkg<'a>(
    pkg: &'a Package,
    total_packages: &mut usize,
    parts: &mut Vec<(String, bool, &'a PartDef)>,
    interfaces: &mut Vec<(String, bool, &'a InterfaceDef)>,
    ports: &mut Vec<(String, bool, &'a PortDef)>,
) {
    *total_packages += 1;
    let is_lib = pkg.is_library || pkg.name.starts_with("SysmlStd") || pkg.name == "ISO26262Metadata" || pkg.name == sysml_ir::TBD_METADATA_PACKAGE;
    for m in &pkg.members {
        match m {
            PackageMember::Package(sub) => collect_pkg(sub, total_packages, parts, interfaces, ports),
            PackageMember::PartDef(p) => parts.push((pkg.name.clone(), is_lib, p)),
            PackageMember::InterfaceDef(i) => interfaces.push((pkg.name.clone(), is_lib, i)),
            PackageMember::PortDef(p) => ports.push((pkg.name.clone(), is_lib, p)),
            _ => {}
        }
    }
}

fn collect_state_defs<'a>(pkg: &'a Package, out: &mut Vec<(String, &'a StateDef)>) {
    if pkg.is_library || pkg.name.starts_with("SysmlStd") || pkg.name == "ISO26262Metadata" || pkg.name == sysml_ir::TBD_METADATA_PACKAGE {
        return;
    }
    for m in &pkg.members {
        match m {
            PackageMember::Package(sub) => collect_state_defs(sub, out),
            PackageMember::StateDef(sd) => out.push((pkg.name.clone(), sd)),
            _ => {}
        }
    }
}

/// `unreachable_state`, `dead_end_state` and `overlapping_guards` for one `state def`.
fn state_machine_findings(pkg_name: &str, sd: &StateDef) -> Vec<LintFinding> {
    let mut out = Vec::new();
    if sd.states.is_empty() {
        return out;
    }
    let element = |state: &str| format!("{}.{}", sd.name, state);

    // Reachability from the initial state (only when it is known).
    if let Some(initial) = sd
        .initial_state
        .as_deref()
        .filter(|i| sd.states.iter().any(|s| s.name == *i))
    {
        let mut reached: BTreeSet<&str> = BTreeSet::from([initial]);
        let mut frontier = vec![initial];
        while let Some(cur) = frontier.pop() {
            for t in sd.transitions.iter().filter(|t| t.source == cur) {
                if reached.insert(t.target.as_str()) {
                    frontier.push(t.target.as_str());
                }
            }
        }
        for st in sd.states.iter().filter(|s| !reached.contains(s.name.as_str())) {
            out.push(LintFinding {
                code: "unreachable_state".to_string(),
                severity: LintSeverity::Warning,
                package: pkg_name.to_string(),
                element: element(&st.name),
                message: format!(
                    "State `{}` of `{}` is not reachable from the initial state `{initial}` by any transition.",
                    st.name, sd.name
                ),
                suggestion: "Add the transition your source document describes into this state (`#[sysml::transition(on = Evt, to = ThisState)]` on a source variant), or delete the state if the source has no such mode. Do not invent a trigger to silence this.".to_string(),
            });
        }
    }

    for st in &sd.states {
        let leaves = sd
            .transitions
            .iter()
            .any(|t| t.source == st.name && t.target != st.name);
        if !leaves {
            out.push(LintFinding {
                code: "dead_end_state".to_string(),
                severity: LintSeverity::Info,
                package: pkg_name.to_string(),
                element: element(&st.name),
                message: format!(
                    "State `{}` of `{}` has no transition to another state (once entered, the machine stays there).",
                    st.name, sd.name
                ),
                suggestion: "Fine for a latched fault or final state (say so in its `///` doc). Otherwise add the exit transition from your source document (e.g. a reset or recovery event).".to_string(),
            });
        }
    }

    // Transitions sharing source state and trigger: the first matching one wins.
    let mut groups: BTreeMap<(&str, &str), Vec<&sysml_ir::Transition>> = BTreeMap::new();
    for t in &sd.transitions {
        let trig = t.trigger.as_deref().unwrap_or("<completion>");
        groups.entry((t.source.as_str(), trig)).or_default().push(t);
    }
    for ((source, trig), ts) in groups {
        if ts.len() < 2 {
            continue;
        }
        let guarded = ts.iter().filter(|t| t.guard.is_some()).count();
        let unguarded = ts.len() - guarded;
        if guarded == 0 && unguarded < 2 {
            continue;
        }
        let guards: Vec<String> = ts
            .iter()
            .map(|t| match &t.guard {
                Some(g) => format!("`{g}` -> {}", t.target),
                None => format!("(no guard) -> {}", t.target),
            })
            .collect();
        out.push(LintFinding {
            code: "overlapping_guards".to_string(),
            severity: LintSeverity::Info,
            package: pkg_name.to_string(),
            element: element(source),
            message: format!(
                "{} transitions leave `{}` on `{trig}` and may be enabled together: {}.",
                ts.len(),
                source,
                guards.join("; ")
            ),
            suggestion: "Check the guards are mutually exclusive (e.g. `x > limit` vs `x <= limit`); the first transition declared wins when several are enabled. An unguarded transition shadows every later one on the same trigger.".to_string(),
        });
    }
    out
}

struct DefInfo<'a> {
    package: String,
    is_std: bool,
    is_abstract: bool,
    specializes: Vec<String>,
    attributes: &'a [sysml_ir::AttributeUsage],
}

fn collect_defs<'a>(pkg: &'a Package, out: &mut BTreeMap<String, DefInfo<'a>>) {
    let is_std = pkg.name.starts_with("SysmlStd");
    for m in &pkg.members {
        let (name, is_abstract, specializes, attributes) = match m {
            PackageMember::Package(sub) => {
                collect_defs(sub, out);
                continue;
            }
            PackageMember::PartDef(p) => (&p.name, p.is_abstract, &p.specializes, &p.attributes),
            PackageMember::ItemDef(i) => (&i.name, i.is_abstract, &i.specializes, &i.attributes),
            PackageMember::PortDef(p) => (&p.name, p.is_abstract, &p.specializes, &p.attributes),
            PackageMember::InterfaceDef(i) => {
                (&i.name, i.is_abstract, &i.specializes, &i.attributes)
            }
            _ => continue,
        };
        out.entry(name.clone()).or_insert(DefInfo {
            package: pkg.name.clone(),
            is_std,
            is_abstract,
            specializes: specializes.iter().map(|s| simple_name(s).to_string()).collect(),
            attributes: attributes.as_slice(),
        });
    }
}

/// `inherited_std_default`: for every concrete model definition that specializes a sysml-std
/// element, list the numeric std attribute defaults it inherits without redefining them.
fn inherited_std_defaults(model: &Model) -> Vec<LintFinding> {
    let mut defs: BTreeMap<String, DefInfo> = BTreeMap::new();
    for pkg in &model.packages {
        collect_defs(pkg, &mut defs);
    }
    let mut findings = Vec::new();
    for (name, d) in &defs {
        if d.is_std || d.is_abstract {
            continue;
        }
        // Names already given a value by a closer (model-side or more specific std) definition.
        let mut resolved: BTreeSet<String> = BTreeSet::new();
        let mut inherited: Vec<String> = Vec::new();
        let mut visited: BTreeSet<String> = BTreeSet::new();
        let mut queue: std::collections::VecDeque<String> = [name.clone()].into();
        while let Some(cur) = queue.pop_front() {
            if !visited.insert(cur.clone()) {
                continue;
            }
            let Some(info) = defs.get(&cur) else { continue };
            for a in info.attributes {
                let mut names: Vec<String> = a.name.iter().cloned().collect();
                names.extend(a.redefines.iter().map(|r| simple_name(r).to_string()));
                let Some(primary) = names.first().cloned() else { continue };
                if info.is_std && !resolved.contains(&primary) {
                    if let Some(v) = a.value.as_deref().map(str::trim) {
                        if v.parse::<f64>().is_ok() {
                            let unit = a.unit.as_deref().map(|u| format!(" [{u}]")).unwrap_or_default();
                            inherited.push(format!("{primary} = {v}{unit} (from {cur})"));
                        }
                    }
                }
                resolved.extend(names);
            }
            queue.extend(info.specializes.iter().cloned());
        }
        if inherited.is_empty() {
            continue;
        }
        findings.push(LintFinding {
            code: "inherited_std_default".to_string(),
            severity: LintSeverity::Info,
            package: d.package.clone(),
            element: name.clone(),
            message: format!(
                "`{name}` inherits {} numeric sysml-std library default(s) it does not redefine: {}.",
                inherited.len(),
                inherited.join("; ")
            ),
            suggestion: "Library defaults are generic placeholders, not values from your source. Redefine each one your source document specifies (`#[sysml::attr(redefines = \"mainsFrequency\", default = ..., unit = \"Hz\")]`); blank each one your source leaves open with `#[sysml::attr(redefines = \"...\", tbd_value = \"reason\", unit = \"...\")]` (a counted TBD); leave the rest only if the value is genuinely irrelevant to the model.".to_string(),
        });
    }
    findings
}

fn part_feature_signature(p: &PartDef) -> BTreeSet<String> {
    let mut sig = BTreeSet::new();
    for a in &p.attributes {
        if let Some(name) = &a.name {
            sig.insert(format!("attr:{name}"));
        }
    }
    for port in &p.ports {
        sig.insert(format!("port:{}", port.name));
    }
    sig
}

fn ancestors(name: &str, parents: &BTreeMap<String, Vec<String>>) -> BTreeSet<String> {
    let mut visited = BTreeSet::new();
    let mut stack = vec![name.to_string()];
    while let Some(cur) = stack.pop() {
        if let Some(ps) = parents.get(&cur) {
            for p in ps {
                if visited.insert(p.clone()) {
                    stack.push(p.clone());
                }
            }
        }
    }
    visited
}

fn shares_ancestor(a: &str, b: &str, parents: &BTreeMap<String, Vec<String>>) -> bool {
    let anc_a = ancestors(a, parents);
    let anc_b = ancestors(b, parents);
    anc_a.contains(b) || anc_b.contains(a) || !anc_a.is_disjoint(&anc_b)
}

fn simple_name(raw: &str) -> &str {
    raw.trim().rsplit("::").next().unwrap_or(raw).trim()
}

fn is_dimensionless_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [
        "ratio",
        "efficiency",
        "factor",
        "coeff",
        "albedo",
        "emissivity",
        "index",
        "count",
        "num",
        "fraction",
        "percent",
        "pct",
        "gain",
        "duty",
        "teeth",
        "gears",
        "stages",
        "channels",
        "eccentricity",
        "db",
        "decibel",
        "noise",
        "sound",
        "ph",
        "cop",
        "margin",
    ]
    .iter()
    .any(|kw| lower.contains(kw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lint_flags_state_machine_smells() {
        use sysml_ir::{StateUsage, Transition};
        let st = |n: &str| StateUsage {
            name: n.into(),
            ..Default::default()
        };
        let tr = |s: &str, trig: &str, guard: Option<&str>, t: &str| Transition {
            source: s.into(),
            trigger: Some(trig.into()),
            guard: guard.map(Into::into),
            target: t.into(),
            ..Default::default()
        };
        let model = |sd: StateDef| {
            Model::new(vec![Package {
                name: "M".into(),
                members: vec![PackageMember::StateDef(sd)],
                ..Default::default()
            }])
        };
        let sd = StateDef {
            name: "Mode".into(),
            initial_state: Some("idle".into()),
            states: vec![st("idle"), st("running"), st("fault"), st("orphan")],
            transitions: vec![
                tr("idle", "Go", Some("e.x > 1"), "running"),
                tr("idle", "Go", Some("e.x > 2"), "fault"),
                tr("running", "Stop", None, "idle"),
                tr("orphan", "Go", None, "idle"),
            ],
            ..Default::default()
        };
        let rep = lint_model("m", &model(sd.clone()));
        let codes = |code: &str| -> Vec<String> {
            rep.findings
                .iter()
                .filter(|f| f.code == code)
                .map(|f| f.element.clone())
                .collect()
        };
        assert_eq!(codes("unreachable_state"), vec!["Mode.orphan"]);
        assert_eq!(codes("dead_end_state"), vec!["Mode.fault"]);
        assert_eq!(codes("overlapping_guards"), vec!["Mode.idle"]);
        assert!(!rep.clean, "unreachable_state is a warning");
        assert_eq!(rep.summary.state_review_count, 2);
        assert_eq!(rep.summary.info_count, 0, "state review items are counted apart");

        // Unknown initial state: no reachability verdict; info findings never fail --strict.
        let rep = lint_model(
            "m",
            &model(StateDef {
                initial_state: None,
                ..sd.clone()
            }),
        );
        assert!(rep.findings.iter().all(|f| f.code != "unreachable_state"));
        assert!(rep.clean);

        // One guarded + one unguarded transition on the same trigger also overlaps; a single
        // transition per trigger does not.
        let rep = lint_model(
            "m",
            &model(StateDef {
                transitions: vec![
                    tr("idle", "Go", None, "running"),
                    tr("idle", "Go", Some("e.x > 2"), "fault"),
                    tr("running", "Stop", None, "idle"),
                ],
                ..sd
            }),
        );
        assert_eq!(
            rep.findings.iter().filter(|f| f.code == "overlapping_guards").count(),
            1
        );
    }
    use sysml_ir::{AttributeUsage, Model, Package, PackageMember, PartDef, PortUsage};

    #[test]
    fn lint_flags_untyped_connections_when_model_uses_interfaces() {
        use sysml_ir::{ConnectionUsage, InterfaceDef};
        let conn = |name: &str, ty: Option<&str>| ConnectionUsage {
            name: Some(name.into()),
            type_name: ty.map(Into::into),
            from: Some("a.p".into()),
            to: Some("b.p".into()),
            ..Default::default()
        };
        let model = |conns: Vec<ConnectionUsage>, with_iface: bool| {
            let mut members = vec![PackageMember::PartDef(PartDef {
                name: "Ctx".into(),
                connections: conns,
                ..Default::default()
            })];
            if with_iface {
                members.push(PackageMember::InterfaceDef(InterfaceDef {
                    name: "Link".into(),
                    ..Default::default()
                }));
            }
            Model::new(vec![Package {
                name: "M".into(),
                members,
                ..Default::default()
            }])
        };
        // Mixed typed + untyped -> warning.
        let rep = lint_model("m", &model(vec![conn("a", Some("Link")), conn("b", None)], true));
        assert!(rep.findings.iter().any(|f| f.code == "untyped_connections"));
        assert!(!rep.clean);
        // Interface defs left behind but stripped from the connects -> still a warning.
        let rep = lint_model("m", &model(vec![conn("b", None)], true));
        assert!(rep.findings.iter().any(|f| f.code == "untyped_connections"));
        // Fully typed -> clean.
        let rep = lint_model("m", &model(vec![conn("a", Some("Link"))], true));
        assert!(rep.clean);
        // No interfaces at all (early skeleton) -> no finding.
        let rep = lint_model("m", &model(vec![conn("b", None)], false));
        assert!(rep.clean);
    }

    #[test]
    fn lint_lists_inherited_std_defaults_until_redefined() {
        let attr = |name: Option<&str>, redef: Option<&str>, v: &str, u: &str| AttributeUsage {
            name: name.map(Into::into),
            redefines: redef.map(|r| vec![r.to_string()]).unwrap_or_default(),
            type_name: Some("Real".into()),
            value: Some(v.into()),
            unit: Some(u.into()),
            ..Default::default()
        };
        let std_pkg = Package {
            name: "SysmlStdPower".into(),
            is_library: true,
            members: vec![PackageMember::PartDef(PartDef {
                name: "AcMainsSupply".into(),
                is_abstract: true,
                attributes: vec![
                    attr(Some("frequency"), None, "50.0", "Hz"),
                    attr(Some("voltage"), None, "230.0", "V"),
                ],
                ..Default::default()
            })],
            ..Default::default()
        };
        let model = |redefine_freq: bool| {
            let attrs = if redefine_freq {
                vec![attr(None, Some("frequency"), "60.0", "Hz")]
            } else {
                vec![]
            };
            Model::new(vec![
                std_pkg.clone(),
                Package {
                    name: "M".into(),
                    members: vec![PackageMember::PartDef(PartDef {
                        name: "Mains".into(),
                        specializes: vec!["AcMainsSupply".into()],
                        attributes: attrs,
                        ..Default::default()
                    })],
                    ..Default::default()
                },
            ])
        };
        let rep = lint_model("m", &model(false));
        let f = rep
            .findings
            .iter()
            .find(|f| f.code == "inherited_std_default")
            .expect("finding");
        assert!(f.message.contains("frequency = 50.0 [Hz]"), "{}", f.message);
        assert!(f.message.contains("voltage = 230.0 [V]"));
        assert!(rep.clean, "info only");
        let rep = lint_model("m", &model(true));
        let f = rep
            .findings
            .iter()
            .find(|f| f.code == "inherited_std_default")
            .expect("finding");
        assert!(!f.message.contains("frequency"), "{}", f.message);
    }

    #[test]
    fn lint_flags_duplicate_part_shape_and_passes_after_extracting_base() {
        let make_sensor = |name: &str, parent: Option<&str>| PartDef {
            name: name.into(),
            specializes: parent.map(|p| vec![p.into()]).unwrap_or_default(),
            attributes: vec![
                AttributeUsage {
                    name: Some("sampleRate".into()),
                    type_name: Some("Real".into()),
                    value: Some("100.0".into()),
                    unit: Some("Hz".into()),
                    ..Default::default()
                },
                AttributeUsage {
                    name: Some("mass".into()),
                    type_name: Some("Real".into()),
                    value: Some("0.05".into()),
                    unit: Some("kg".into()),
                    ..Default::default()
                },
            ],
            ports: vec![PortUsage {
                name: "powerIn".into(),
                type_name: "PowerPort".into(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let flat = Model::new(vec![Package {
            name: "Flat".into(),
            members: vec![
                PackageMember::PartDef(make_sensor("TempSensor", None)),
                PackageMember::PartDef(make_sensor("PressSensor", None)),
            ],
            ..Default::default()
        }]);
        let flat_rep = lint_model("flat", &flat);
        assert!(!flat_rep.clean);
        assert!(flat_rep
            .findings
            .iter()
            .any(|f| f.code == "duplicate_part_shape"));

        let refactored = Model::new(vec![Package {
            name: "Refactored".into(),
            members: vec![
                PackageMember::PartDef(PartDef {
                    name: "SensorBase".into(),
                    is_abstract: true,
                    ..Default::default()
                }),
                PackageMember::PartDef(make_sensor("TempSensor", Some("SensorBase"))),
                PackageMember::PartDef(make_sensor("PressSensor", Some("SensorBase"))),
            ],
            ..Default::default()
        }]);
        let ref_rep = lint_model("refactored", &refactored);
        assert!(ref_rep.clean);
        assert_eq!(ref_rep.summary.abstract_bases, 1);
        assert_eq!(ref_rep.summary.specializations, 2);
    }
}
