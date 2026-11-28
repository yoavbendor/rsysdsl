//! Phase C1 — Model Refactoring & Viewpoint Operators:
//! - [`elevate_model`]: computes structural intersections of sibling parts to propose
//!   `#[sysml::part(abstract)]` parents (or matching `sysml-std` bases) with ready-to-paste
//!   Rust DSL skeletons.
//! - [`slice_model`]: projects a unified multi-domain `Model` onto a single aspect
//!   (`Thermal`, `Power`, `Comms`, `Safety`, `Latency`, `Mechanics`, `Hmi`) for focused
//!   `rsysdsl view --aspect <domain>` diagrams and reviews.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sysml_ir::{AttributeUsage, Model, Package, PackageMember, PartDef};

use crate::emit_cli::is_sysml_std_package;

/// One candidate generalization proposed by [`elevate_model`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElevateProposal {
    pub suggested_base: String,
    pub matching_std_base: Option<String>,
    pub member_parts: Vec<String>,
    pub shared_attributes: Vec<String>,
    pub shared_ports: Vec<String>,
    pub suggested_rust_dsl: String,
}

/// Complete report returned by [`elevate_model`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElevateReport {
    pub parts_analyzed: usize,
    pub proposal_count: usize,
    pub proposals: Vec<ElevateProposal>,
}

/// Domain aspect for [`slice_model`] viewpoint projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelAspect {
    Thermal,
    Power,
    Comms,
    Safety,
    Latency,
    Mechanics,
    Hmi,
}

impl ModelAspect {
    pub fn as_str(self) -> &'static str {
        match self {
            ModelAspect::Thermal => "thermal",
            ModelAspect::Power => "power",
            ModelAspect::Comms => "comms",
            ModelAspect::Safety => "safety",
            ModelAspect::Latency => "latency",
            ModelAspect::Mechanics => "mechanics",
            ModelAspect::Hmi => "hmi",
        }
    }

    fn keywords(self) -> &'static [&'static str] {
        match self {
            ModelAspect::Thermal => &[
                "heat", "temp", "thermal", "cool", "fan", "conductance", "radiat", "heater",
            ],
            ModelAspect::Power => &[
                "power", "volt", "current", "batt", "psu", "mains", "energy", "relay", "inverter",
                "charger", "watt", "hv",
            ],
            ModelAspect::Comms => &[
                "can", "uart", "eth", "mavlink", "dds", "uorb", "spi", "i2c", "bus", "comm",
                "telemetry", "cmd", "command", "signal", "link",
            ],
            ModelAspect::Safety => &[
                "safety", "safe", "interlock", "asil", "fmea", "fault", "cutoff", "guard",
                "failsafe", "flood", "abort", "disarm", "monitor", "brake",
            ],
            ModelAspect::Latency => &[
                "latency", "wcet", "period", "deadline", "ttft", "fps", "cutoff", "delay", "rate",
                "hz",
            ],
            ModelAspect::Mechanics => &[
                "mech", "motor", "pump", "gear", "torque", "mass", "force", "valve", "tub",
                "blade", "chuck", "anvil", "spindle", "fluid", "water",
            ],
            ModelAspect::Hmi => &[
                "hmi", "panel", "button", "display", "operator", "user", "door", "trigger", "chime",
                "led", "noise", "acoustic",
            ],
        }
    }
}

/// Analyze `model` for sibling `PartDef`s sharing >= 2 attributes/ports that do not yet
/// share a common non-library parent, and propose abstract bases (`#[sysml::part(abstract)]`).
pub fn elevate_model(model: &Model) -> ElevateReport {
    let mut std_bases: Vec<(String, BTreeSet<String>)> = Vec::new();
    let mut parts: Vec<&PartDef> = Vec::new();

    for pkg in &model.packages {
        let is_std = is_sysml_std_package(pkg);
        for m in &pkg.members {
            if let PackageMember::PartDef(p) = m {
                if is_std {
                    if p.is_abstract {
                        std_bases.push((p.name.clone(), part_feature_set(p)));
                    }
                } else if !p.is_abstract && !p.is_variation && p.parts.is_empty() {
                    parts.push(p);
                }
            }
        }
    }

    let mut used: BTreeSet<String> = BTreeSet::new();
    let mut proposals: Vec<ElevateProposal> = Vec::new();

    for i in 0..parts.len() {
        let p_i = parts[i];
        if used.contains(&p_i.name) {
            continue;
        }
        let f_i = part_feature_set(p_i);
        if f_i.len() < 2 {
            continue;
        }

        let mut group: Vec<&PartDef> = vec![p_i];
        let mut common = f_i.clone();

        for p_j in parts.iter().skip(i + 1) {
            if used.contains(&p_j.name) {
                continue;
            }
            // Skip if they already explicitly specialize the exact same non-empty parent
            if !p_i.specializes.is_empty() && p_i.specializes == p_j.specializes {
                continue;
            }
            let f_j = part_feature_set(p_j);
            let inter: BTreeSet<String> = common.intersection(&f_j).cloned().collect();
            if inter.len() >= 2 {
                group.push(p_j);
                common = inter;
            }
        }

        if group.len() >= 2 {
            let member_names: Vec<String> = group.iter().map(|p| p.name.clone()).collect();
            for name in &member_names {
                used.insert(name.clone());
            }

            let mut shared_attrs = Vec::new();
            let mut shared_ports = Vec::new();
            for feat in &common {
                if let Some(a) = feat.strip_prefix("attr:") {
                    shared_attrs.push(a.to_string());
                } else if let Some(pt) = feat.strip_prefix("port:") {
                    shared_ports.push(pt.to_string());
                }
            }

            // Check if a loaded sysml-std abstract base covers >= 50% of the shared features
            let mut best_std: Option<(usize, String)> = None;
            for (std_name, std_feats) in &std_bases {
                let overlap = common.intersection(std_feats).count();
                if overlap > 0 && overlap * 2 >= common.len() {
                    if best_std.as_ref().is_none_or(|(b, _)| overlap > *b) {
                        best_std = Some((overlap, std_name.clone()));
                    }
                }
            }

            let suggested_base = suggest_base_name(&member_names);
            let matching_std_base = best_std.map(|(_, n)| n);
            let suggested_rust_dsl =
                render_elevate_snippet(&suggested_base, matching_std_base.as_deref(), &member_names, &shared_attrs, &shared_ports);

            proposals.push(ElevateProposal {
                suggested_base,
                matching_std_base,
                member_parts: member_names,
                shared_attributes: shared_attrs,
                shared_ports,
                suggested_rust_dsl,
            });
        }
    }

    ElevateReport {
        parts_analyzed: parts.len(),
        proposal_count: proposals.len(),
        proposals,
    }
}

/// Render [`ElevateReport`] as clean Markdown.
pub fn render_elevate_markdown(r: &ElevateReport) -> String {
    let mut out = String::new();
    out.push_str("# Model Generalization & Elevation Report (`rsysdsl elevate`)\n\n");
    out.push_str(&format!(
        "- **Leaf Parts Analyzed:** {} | **Elevation Proposals:** {}\n\n",
        r.parts_analyzed, r.proposal_count
    ));
    if r.proposals.is_empty() {
        out.push_str("No un-generalized duplicate part shapes found (hierarchy is clean).\n");
        return out;
    }
    for (idx, p) in r.proposals.iter().enumerate() {
        out.push_str(&format!(
            "## {}. Extract `{}` from {}\n\n",
            idx + 1,
            p.suggested_base,
            p.member_parts
                .iter()
                .map(|m| format!("`{m}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        if let Some(std_b) = &p.matching_std_base {
            out.push_str(&format!("- **Matching `sysml-std` Base:** `{std_b}`\n"));
        }
        if !p.shared_attributes.is_empty() {
            out.push_str(&format!(
                "- **Shared Attributes:** {}\n",
                p.shared_attributes.join(", ")
            ));
        }
        if !p.shared_ports.is_empty() {
            out.push_str(&format!(
                "- **Shared Ports:** {}\n",
                p.shared_ports.join(", ")
            ));
        }
        out.push_str("\n```rust\n");
        out.push_str(&p.suggested_rust_dsl);
        out.push_str("```\n\n");
    }
    out
}

/// Project a unified multi-domain [`Model`] onto a single [`ModelAspect`] viewpoint.
///
/// Keeps library packages intact, retains only domain-relevant `PartDef`s, `RequirementDef`s,
/// `InterfaceDef`s, `StateDef`s, and trims assembly `PartDef`s to only matching child parts
/// and connections.
pub fn slice_model(model: &Model, aspect: ModelAspect) -> Model {
    let kws = aspect.keywords();
    let mut kept_part_types: BTreeSet<String> = BTreeSet::new();

    // Pass 1: identify leaf/domain PartDefs matching the aspect
    for pkg in &model.packages {
        if is_sysml_std_package(pkg) {
            continue;
        }
        for m in &pkg.members {
            if let PackageMember::PartDef(p) = m {
                if part_matches_aspect(p, kws) {
                    kept_part_types.insert(p.name.clone());
                }
            }
        }
    }

    // Pass 2: propagate upward to any parent assembly composing a kept part (fixed point)
    let mut changed = true;
    while changed {
        changed = false;
        for pkg in &model.packages {
            if is_sysml_std_package(pkg) {
                continue;
            }
            for m in &pkg.members {
                if let PackageMember::PartDef(p) = m {
                    if !kept_part_types.contains(&p.name)
                        && p.parts.iter().any(|c| {
                            c.type_names
                                .iter()
                                .any(|t| kept_part_types.contains(t))
                        })
                    {
                        kept_part_types.insert(p.name.clone());
                        changed = true;
                    }
                }
            }
        }
    }

    // Pass 3: build sliced Model
    let mut new_pkgs: Vec<Package> = Vec::new();
    for pkg in &model.packages {
        if is_sysml_std_package(pkg) {
            new_pkgs.push(pkg.clone());
            continue;
        }
        let mut sliced_pkg = pkg.clone();
        sliced_pkg.members.clear();

        for m in &pkg.members {
            match m {
                PackageMember::PartDef(p) => {
                    if !kept_part_types.contains(&p.name) {
                        continue;
                    }
                    let mut sp = p.clone();
                    if !sp.parts.is_empty() {
                        sp.parts.retain(|c| {
                            c.type_names
                                .iter()
                                .any(|t| kept_part_types.contains(t))
                        });
                        let kept_fields: BTreeSet<&str> = sp
                            .parts
                            .iter()
                            .filter_map(|c| c.name.as_deref())
                            .collect();
                        let own_ports: BTreeSet<&str> =
                            sp.ports.iter().map(|pt| pt.name.as_str()).collect();
                        sp.connections.retain(|conn| {
                            let ok_side = |ep: Option<&str>| -> bool {
                                let Some(e) = ep else { return false };
                                let root = e.split('.').next().unwrap_or(e);
                                kept_fields.contains(root) || own_ports.contains(root)
                            };
                            ok_side(conn.from.as_deref()) && ok_side(conn.to.as_deref())
                        });
                    }
                    sliced_pkg.members.push(PackageMember::PartDef(sp));
                }
                PackageMember::RequirementDef(r) => {
                    let hay = format!(
                        "{} {} {} {}",
                        r.name,
                        r.short_name.as_deref().unwrap_or(""),
                        r.doc.as_deref().unwrap_or(""),
                        r.rationale.as_deref().unwrap_or("")
                    )
                    .to_lowercase();
                    if kws.iter().any(|k| hay.contains(k)) || (aspect == ModelAspect::Safety && r.safety_goal.is_some()) {
                        sliced_pkg.members.push(PackageMember::RequirementDef(r.clone()));
                    }
                }
                PackageMember::InterfaceDef(i) => {
                    let hay = format!("{} {}", i.name, i.doc.as_deref().unwrap_or("")).to_lowercase();
                    if kws.iter().any(|k| hay.contains(k)) || i.is_abstract {
                        sliced_pkg.members.push(PackageMember::InterfaceDef(i.clone()));
                    }
                }
                // Keep ItemDefs, PortDefs, EnumDefs, StateDefs, Calcs so references remain valid
                other => sliced_pkg.members.push(other.clone()),
            }
        }
        new_pkgs.push(sliced_pkg);
    }

    Model::new(new_pkgs)
}

fn part_matches_aspect(p: &PartDef, kws: &[&str]) -> bool {
    if !p.fmea.is_empty() || !p.asil_decompositions.is_empty() {
        if kws.contains(&"safety") {
            return true;
        }
    }
    let mut hay = String::new();
    hay.push_str(&p.name);
    hay.push(' ');
    if let Some(d) = &p.doc {
        hay.push_str(d);
        hay.push(' ');
    }
    for s in &p.specializes {
        hay.push_str(s);
        hay.push(' ');
    }
    for a in &p.attributes {
        if let Some(n) = attr_effective_name(a) {
            hay.push_str(n);
            hay.push(' ');
        }
    }
    for pt in &p.ports {
        hay.push_str(&pt.name);
        hay.push(' ');
        hay.push_str(&pt.type_name);
        hay.push(' ');
    }
    let lower = hay.to_lowercase();
    kws.iter().any(|k| lower.contains(k))
}

fn part_feature_set(p: &PartDef) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    for a in &p.attributes {
        if let Some(n) = attr_effective_name(a) {
            set.insert(format!("attr:{n}"));
        }
    }
    for pt in &p.ports {
        set.insert(format!("port:{}:{}", pt.name, pt.type_name));
    }
    set
}

fn attr_effective_name(a: &AttributeUsage) -> Option<&str> {
    a.name
        .as_deref()
        .or_else(|| a.redefines.first().map(String::as_str))
}

fn suggest_base_name(members: &[String]) -> String {
    // Find longest common suffix among PascalCase names (e.g. PrimaryDoorInterlock + MonitorDoorInterlock -> DoorInterlockBase)
    let first = &members[0];
    let mut common_suffix = first.as_str();
    for m in &members[1..] {
        let mut rev_len = 0;
        for (a, b) in common_suffix.chars().rev().zip(m.chars().rev()) {
            if a == b {
                rev_len += a.len_utf8();
            } else {
                break;
            }
        }
        common_suffix = &common_suffix[common_suffix.len() - rev_len..];
    }
    let trimmed = common_suffix.trim_start_matches(|c: char| !c.is_uppercase());
    if trimmed.len() >= 4 {
        format!("{trimmed}Base")
    } else {
        format!("{}Base", first)
    }
}

fn render_elevate_snippet(
    base: &str,
    std_base: Option<&str>,
    members: &[String],
    attrs: &[String],
    ports: &[String],
) -> String {
    let mut s = String::new();
    s.push_str("#[sysml::part(abstract)]\n");
    if let Some(sb) = std_base {
        s.push_str(&format!("#[sysml::specializes({sb})]\n"));
    }
    s.push_str(&format!("pub struct {base} {{\n"));
    for a in attrs {
        s.push_str(&format!("    // shared attribute: {a}\n"));
    }
    for pt in ports {
        s.push_str(&format!("    // shared port: {pt}\n"));
    }
    s.push_str("}\n");
    for m in members {
        s.push_str(&format!(
            "\n#[sysml::part]\n#[sysml::specializes({base})]\npub struct {m} {{ /* :>> redefines */ }}\n"
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysml_ir::{ConnectionKind, ConnectionUsage, PartUsage, PortUsage};

    fn attr(name: &str) -> AttributeUsage {
        AttributeUsage {
            name: Some(name.into()),
            value: Some("1.0".into()),
            ..Default::default()
        }
    }

    #[test]
    fn elevate_detects_shared_shape_and_proposes_abstract_base() {
        let mut pkg = Package {
            name: "ElevateTest".into(),
            ..Default::default()
        };
        for name in ["PrimaryDoorInterlock", "MonitorDoorInterlock"] {
            pkg.members.push(PackageMember::PartDef(PartDef {
                name: name.into(),
                attributes: vec![attr("cutoffLatency"), attr("contactResistance")],
                ports: vec![PortUsage {
                    name: "tripOut".into(),
                    type_name: "SignalPort".into(),
                    ..Default::default()
                }],
                ..Default::default()
            }));
        }
        let model = Model::new(vec![pkg]);
        let rep = elevate_model(&model);
        assert_eq!(rep.proposal_count, 1);
        assert_eq!(rep.proposals[0].suggested_base, "DoorInterlockBase");
        assert_eq!(rep.proposals[0].member_parts.len(), 2);
        assert_eq!(rep.proposals[0].shared_attributes.len(), 2);
        assert_eq!(rep.proposals[0].shared_ports.len(), 1);
    }

    #[test]
    fn slice_projects_multi_domain_model_to_thermal_aspect() {
        let mut pkg = Package {
            name: "SliceTest".into(),
            ..Default::default()
        };
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "WashHeater".into(),
            attributes: vec![attr("heatOutput")],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "CanBusTransceiver".into(),
            attributes: vec![attr("baudRate")],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "SystemTop".into(),
            parts: vec![
                PartUsage { name: Some("heater".into()), type_names: vec!["WashHeater".into()], ..Default::default() },
                PartUsage { name: Some("canTx".into()), type_names: vec!["CanBusTransceiver".into()], ..Default::default() },
            ],
            connections: vec![ConnectionUsage {
                kind: ConnectionKind::Connection,
                from: Some("canTx.tx".into()),
                to: Some("heater.ctrl".into()),
                ..Default::default()
            }],
            ..Default::default()
        }));
        let model = Model::new(vec![pkg]);
        let sliced = slice_model(&model, ModelAspect::Thermal);
        let kept_names: Vec<String> = sliced.packages[0]
            .members
            .iter()
            .filter_map(|m| match m {
                PackageMember::PartDef(p) => Some(p.name.clone()),
                _ => None,
            })
            .collect();
        assert!(kept_names.contains(&"WashHeater".to_string()));
        assert!(kept_names.contains(&"SystemTop".to_string()));
        assert!(!kept_names.contains(&"CanBusTransceiver".to_string()));
    }
}
