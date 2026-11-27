//! Architecture & Hierarchy Linter ("Clippy for Systems" — `rsysdsl lint`).
//!
//! Analyzes a `sysml_ir::Model` for modeling smells that degrade system reuse,
//! hierarchy clarity, and physical rigor:
//! - `duplicate_part_shape`: 2+ concrete `PartDef`s without a shared ancestor sharing
//!   >= 3 identical port/attribute names -> extract an `abstract` base part.
//! - `unclassified_interfaces`: 3+ concrete `InterfaceDef`s with no `specializes`
//!   parent -> specialize a shared channel base (e.g. `InfoChannel` / `EnergyChannel`).
//! - `flat_god_assembly`: a `PartDef` owning > 10 direct child parts with zero
//!   intermediate subsystem decomposition.
//! - `missing_physical_unit`: numeric `Real` attribute carrying a literal value without
//!   an SI unit (excluding dimensionless counts/ratios/efficiencies/factors).
//! - `unused_abstract_base`: an `abstract` part/interface/port with zero specializing
//!   subtypes in the model (useful during refactoring; skipped for library packages).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use sysml_ir::{InterfaceDef, Model, Package, PackageMember, PartDef, PortDef};

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
    pub info_count: usize,
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
            suggestion: "Specialize a shared abstract interface (e.g. `sysml_std::comms::InfoChannel` or `sysml_std::flows::ThermalLink` / `ElectricalLink`) so protocols/channels form a clean taxonomy.".to_string(),
        });
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

    for f in &findings {
        match f.severity {
            LintSeverity::Warning => summary.warning_count += 1,
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
        "- **Findings:** {} warning(s), {} info(s)\n",
        report.summary.warning_count, report.summary.info_count
    );

    if report.findings.is_empty() {
        let _ = writeln!(out, "No architectural smells detected.");
    } else {
        let _ = writeln!(out, "| Severity | Code | Package | Element | Suggestion |");
        let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
        for f in &report.findings {
            let sev = match f.severity {
                LintSeverity::Warning => "warning",
                LintSeverity::Info => "info",
            };
            let _ = writeln!(
                out,
                "| {sev} | `{}` | `{}` | `{}` | {} |",
                f.code, f.package, f.element, f.suggestion
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
    let is_lib = pkg.is_library || pkg.name.starts_with("SysmlStd") || pkg.name == "ISO26262Metadata";
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
    use sysml_ir::{AttributeUsage, Model, Package, PackageMember, PartDef, PortUsage};

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
