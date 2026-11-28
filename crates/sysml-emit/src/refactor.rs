//! Phase C1 — Model Refactoring & Viewpoint Operators:
//! - [`elevate_model`]: computes structural intersections of sibling parts to propose
//!   `#[sysml::part(abstract)]` parents (or matching `sysml-std` bases) with ready-to-paste
//!   Rust DSL skeletons.
//! - [`slice_model`]: projects a unified multi-domain `Model` onto a single aspect
//!   (`Thermal`, `Power`, `Comms`, `Safety`, `Latency`, `Mechanics`, `Hmi`) for focused
//!   `rsysdsl view --aspect <domain>` diagrams and reviews.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sysml_ir::{AttributeUsage, BudgetRole, Model, Package, PackageMember, PartDef};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// Structural aspect classification of a model's own elements (C1.4). No English keywords:
/// an element belongs to an aspect through
/// - the sysml-std flow items its ports carry (`ThermalEnergy` -> thermal, `ElectricalEnergy`
///   -> power, `ControlCommand` / `TelemetrySignal` / `InformationFlow` -> comms,
///   `HumanInputSignal` / `HumanFeedbackSignal` / `AcousticEmission` -> hmi, `MechanicalEnergy`
///   / `FluidFlow` -> mechanics), for ports declared in the model (ports inherited from generic
///   sysml-std bases, such as every element's `heatOut`, do not count);
/// - interface ends (the port types they join);
/// - the sysml-std base types it specializes (`PowerElement` -> power, `ResistiveHeater` ->
///   thermal, `ElectricMotor` -> mechanics, `SafetyInterlockSwitch` -> safety, `HmiPanel` -> hmi,
///   `Microcontroller` -> comms, ...);
/// - the dimensions of its valued attributes (`K`, `W/K` -> thermal; `V`, `A`, `Ω` -> power;
///   `N`, `N*m`, `kg/s`, `Pa` -> mechanics; time or a latency role -> latency) and their budget
///   roles (power -> power, waste heat -> thermal);
/// - safety metadata (FMEA, ASIL decomposition, safety goals) -> safety;
/// - for requirements, additionally the aspects of the parts that satisfy them.
///
/// Elements matching nothing are listed in [`AspectClassification::unclassified`], not guessed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspectClassification {
    pub parts: BTreeMap<String, BTreeSet<ModelAspect>>,
    pub requirements: BTreeMap<String, BTreeSet<ModelAspect>>,
    pub interfaces: BTreeMap<String, BTreeSet<ModelAspect>>,
    /// Leaf part, requirement and interface definitions with no structural aspect.
    pub unclassified: Vec<String>,
}

fn std_item_aspect(name: &str) -> Option<ModelAspect> {
    use ModelAspect::*;
    Some(match name {
        "ElectricalEnergy" => Power,
        "ThermalEnergy" | "RadiativeEnergy" => Thermal,
        "MechanicalEnergy" | "FluidFlow" | "MaterialFlow" => Mechanics,
        "ControlCommand" | "TelemetrySignal" | "InformationFlow" => Comms,
        "HumanInputSignal" | "HumanFeedbackSignal" | "AcousticEmission" => Hmi,
        _ => return None,
    })
}

fn std_part_aspects(name: &str) -> &'static [ModelAspect] {
    use ModelAspect::*;
    match name {
        "PowerElement" => &[Power],
        "ResistiveHeater" | "RadiativeEmitterLoad" => &[Thermal],
        "ThermalEnclosure" | "AmbientEnvironment" | "DeepSpaceSink" => &[Thermal],
        "CoolingFan" => &[Thermal, Mechanics],
        "StructuralFrame" | "ElectricMotor" | "MechanicalTransmission" | "FluidPump"
        | "FluidValve" | "FluidDistributor" | "RotatingWheel" | "FrictionBrake" => &[Mechanics],
        "SafetyInterlockSwitch" => &[Safety],
        "Microcontroller" | "SensorBase" => &[Comms],
        "TemperatureSensor" => &[Thermal],
        "HmiPanel" | "HumanOperator" | "MechanicalHmiControl" => &[Hmi],
        _ => &[],
    }
}

fn unit_aspect(unit: &str) -> Option<ModelAspect> {
    use sysml_core::units::parse_unit;
    let d = parse_unit(unit.trim())?;
    let is = |r: &str| parse_unit(r).is_some_and(|x| x.eq(d));
    if ["K", "W/K", "J/K", "W/(m*K)", "W/m^2", "W/(m^2*K)", "J/(kg*K)"].iter().any(|r| is(r)) {
        Some(ModelAspect::Thermal)
    } else if ["V", "A", "Ω", "C", "F", "H", "S"].iter().any(|r| is(r)) {
        Some(ModelAspect::Power)
    } else if ["N", "N*m", "kg/s", "m^3/s", "Pa", "m/s", "m/s^2"].iter().any(|r| is(r)) {
        Some(ModelAspect::Mechanics)
    } else if is("s") {
        Some(ModelAspect::Latency)
    } else {
        None
    }
}

struct TypeIndex<'a> {
    parts: crate::roles::RoleIndex<'a>,
    ports: BTreeMap<String, (bool, &'a sysml_ir::PortDef)>,
    items: BTreeMap<String, (bool, &'a sysml_ir::ItemDef)>,
    interfaces: BTreeMap<String, (bool, &'a sysml_ir::InterfaceDef)>,
    std_part_names: BTreeSet<String>,
}

fn simple(n: &str) -> &str {
    n.trim().trim_start_matches('~').rsplit("::").next().unwrap_or(n).trim()
}

impl<'a> TypeIndex<'a> {
    fn new(model: &'a Model) -> Self {
        fn walk<'a>(pkg: &'a Package, std: bool, ix: &mut TypeIndex<'a>) {
            for m in &pkg.members {
                match m {
                    PackageMember::Package(sub) => walk(sub, std, ix),
                    PackageMember::PortDef(p) => {
                        ix.ports.entry(p.name.clone()).or_insert((std, p));
                    }
                    PackageMember::ItemDef(i) => {
                        ix.items.entry(i.name.clone()).or_insert((std, i));
                    }
                    PackageMember::InterfaceDef(i) => {
                        ix.interfaces.entry(i.name.clone()).or_insert((std, i));
                    }
                    _ => {}
                }
            }
        }
        let mut ix = TypeIndex {
            parts: crate::roles::RoleIndex::new(model),
            ports: BTreeMap::new(),
            items: BTreeMap::new(),
            interfaces: BTreeMap::new(),
            std_part_names: BTreeSet::new(),
        };
        for std in [false, true] {
            for pkg in model.packages.iter().filter(|p| is_sysml_std_package(p) == std) {
                walk(pkg, std, &mut ix);
            }
        }
        ix
    }

    fn item_aspects(&self, ty: &str, depth: usize, out: &mut BTreeSet<ModelAspect>) {
        let name = simple(ty);
        match self.items.get(name) {
            Some((true, _)) | None => {
                if let Some(a) = std_item_aspect(name) {
                    out.insert(a);
                    return;
                }
            }
            _ => {}
        }
        if depth > 16 {
            return;
        }
        if let Some((_, def)) = self.items.get(name) {
            for parent in &def.specializes {
                self.item_aspects(parent, depth + 1, out);
            }
        }
    }

    fn port_aspects(&self, ty: &str, depth: usize, out: &mut BTreeSet<ModelAspect>) {
        let Some((_, def)) = self.ports.get(simple(ty)) else {
            return;
        };
        for it in &def.items {
            self.item_aspects(&it.type_name, 0, out);
        }
        if depth < 16 {
            for parent in &def.specializes {
                self.port_aspects(parent, depth + 1, out);
            }
        }
    }

    fn interface_aspects(&self, def: &sysml_ir::InterfaceDef, depth: usize) -> BTreeSet<ModelAspect> {
        let mut out = BTreeSet::new();
        for end in &def.ends {
            self.port_aspects(&end.type_name, 0, &mut out);
        }
        if depth < 16 {
            for parent in &def.specializes {
                if let Some((_, p)) = self.interfaces.get(simple(parent)) {
                    out.extend(self.interface_aspects(p, depth + 1));
                }
            }
        }
        out
    }

    fn part_aspects(&self, p: &'a PartDef) -> BTreeSet<ModelAspect> {
        let mut out = BTreeSet::new();
        if !p.fmea.is_empty() || !p.asil_decompositions.is_empty() {
            out.insert(ModelAspect::Safety);
        }
        for d in self.parts.lineage(p) {
            let from_std = self.is_std_part(&d.name) && !std::ptr::eq(d, p);
            if from_std {
                out.extend(std_part_aspects(&d.name).iter().copied());
                continue;
            }
            for pt in &d.ports {
                self.port_aspects(&pt.type_name, 0, &mut out);
            }
        }
        for a in self.parts.effective_attributes(p) {
            if matches!(a.value, crate::roles::AttrValue::Absent) {
                continue;
            }
            match a.role {
                Some(BudgetRole::Power) => {
                    out.insert(ModelAspect::Power);
                }
                Some(BudgetRole::WasteHeat) => {
                    out.insert(ModelAspect::Thermal);
                }
                Some(BudgetRole::Latency) => {
                    out.insert(ModelAspect::Latency);
                }
                _ => {}
            }
            if let Some(asp) = a.unit.as_deref().and_then(unit_aspect) {
                out.insert(asp);
            }
        }
        out
    }

    fn is_std_part(&self, name: &str) -> bool {
        self.std_part_names.contains(name)
    }
}

/// Classify every user part, requirement and interface definition structurally (see
/// [`AspectClassification`]).
pub fn classify_model(model: &Model) -> AspectClassification {
    let mut ix = TypeIndex::new(model);
    ix.std_part_names = model
        .packages
        .iter()
        .filter(|p| is_sysml_std_package(p))
        .flat_map(|p| p.members.iter())
        .filter_map(|m| match m {
            PackageMember::PartDef(d) => Some(d.name.clone()),
            _ => None,
        })
        .collect();
    let mut out = AspectClassification::default();
    let user_pkgs = || model.packages.iter().filter(|p| !is_sysml_std_package(p));

    let mut safety_reqs: BTreeSet<String> = BTreeSet::new();
    for pkg in user_pkgs() {
        for m in &pkg.members {
            if let PackageMember::RequirementDef(r) = m {
                if r.safety_goal.is_some() {
                    safety_reqs.insert(r.name.clone());
                }
            }
        }
    }

    let mut satisfied_by: BTreeMap<String, BTreeSet<ModelAspect>> = BTreeMap::new();
    for pkg in user_pkgs() {
        for m in &pkg.members {
            match m {
                PackageMember::PartDef(p) => {
                    let mut asp = ix.part_aspects(p);
                    if p.satisfies.iter().any(|s| safety_reqs.contains(simple(&s.requirement))) {
                        asp.insert(ModelAspect::Safety);
                    }
                    for s in &p.satisfies {
                        satisfied_by
                            .entry(simple(&s.requirement).to_string())
                            .or_default()
                            .extend(asp.iter().copied());
                    }
                    if asp.is_empty() && p.parts.is_empty() && !p.is_variation {
                        out.unclassified.push(format!("{}::{}", pkg.name, p.name));
                    }
                    out.parts.insert(p.name.clone(), asp);
                }
                PackageMember::InterfaceDef(i) => {
                    let asp = ix.interface_aspects(i, 0);
                    if asp.is_empty() && !i.is_abstract {
                        out.unclassified.push(format!("{}::{}", pkg.name, i.name));
                    }
                    out.interfaces.insert(i.name.clone(), asp);
                }
                _ => {}
            }
        }
    }
    for pkg in user_pkgs() {
        for m in &pkg.members {
            if let PackageMember::RequirementDef(r) = m {
                let mut asp: BTreeSet<ModelAspect> = r
                    .attributes
                    .iter()
                    .filter_map(|a| a.unit.as_deref().and_then(unit_aspect))
                    .collect();
                if r.safety_goal.is_some() {
                    asp.insert(ModelAspect::Safety);
                }
                if let Some(by) = satisfied_by.get(&r.name) {
                    asp.extend(by.iter().copied());
                }
                if asp.is_empty() {
                    out.unclassified.push(format!("{}::{}", pkg.name, r.name));
                }
                out.requirements.insert(r.name.clone(), asp);
            }
        }
    }
    out
}

/// Project a unified multi-domain [`Model`] onto a single [`ModelAspect`] viewpoint.
///
/// Keeps library packages intact, retains only the part, requirement and interface definitions
/// structurally classified into `aspect` (see [`classify_model`]), and trims assembly
/// `PartDef`s to matching child parts and connections.
pub fn slice_model(model: &Model, aspect: ModelAspect) -> Model {
    slice_model_report(model, aspect).0
}

/// [`slice_model`] plus the elements that could not be classified into any aspect.
pub fn slice_model_report(model: &Model, aspect: ModelAspect) -> (Model, Vec<String>) {
    let cls = classify_model(model);
    let has = |m: &BTreeMap<String, BTreeSet<ModelAspect>>, n: &str| {
        m.get(n).is_some_and(|s| s.contains(&aspect))
    };
    let mut kept_part_types: BTreeSet<String> = cls
        .parts
        .iter()
        .filter(|(_, a)| a.contains(&aspect))
        .map(|(n, _)| n.clone())
        .collect();

    // Propagate upward to any parent assembly composing a kept part (fixed point)
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
                                .any(|t| kept_part_types.contains(simple(t)))
                        })
                    {
                        kept_part_types.insert(p.name.clone());
                        changed = true;
                    }
                }
            }
        }
    }

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
                                .any(|t| kept_part_types.contains(simple(t)))
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
                    if has(&cls.requirements, &r.name) {
                        sliced_pkg.members.push(PackageMember::RequirementDef(r.clone()));
                    }
                }
                PackageMember::InterfaceDef(i) => {
                    if i.is_abstract || has(&cls.interfaces, &i.name) {
                        sliced_pkg.members.push(PackageMember::InterfaceDef(i.clone()));
                    }
                }
                // Keep ItemDefs, PortDefs, EnumDefs, StateDefs, Calcs so references remain valid
                other => sliced_pkg.members.push(other.clone()),
            }
        }
        new_pkgs.push(sliced_pkg);
    }

    (Model::new(new_pkgs), cls.unclassified)
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
        // Names carry no meaning: the heater is thermal because of its temperature attribute,
        // the transceiver is unclassifiable (a frequency says nothing about the aspect).
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "WashHeater".into(),
            attributes: vec![AttributeUsage { unit: Some("K".into()), ..attr("maxElementTemp") }],
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "CanBusTransceiver".into(),
            attributes: vec![AttributeUsage { unit: Some("Hz".into()), ..attr("baudRate") }],
            ..Default::default()
        }));
        // A part named like a heater but with nothing thermal about it is not kept.
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "HeatShieldLabel".into(),
            attributes: vec![AttributeUsage { unit: Some("m".into()), ..attr("width") }],
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
        assert!(!kept_names.contains(&"HeatShieldLabel".to_string()));
        let (_, unclassified) = slice_model_report(&model, ModelAspect::Thermal);
        assert!(unclassified.contains(&"SliceTest::CanBusTransceiver".to_string()), "{unclassified:?}");
        assert!(unclassified.contains(&"SliceTest::HeatShieldLabel".to_string()));
    }
}
