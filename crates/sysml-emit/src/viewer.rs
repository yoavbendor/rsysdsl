//! Standalone offline SysML v2 interactive HTML diagram viewer.
//!
//! Translates `sysml_ir::Model` (from Rust DSL models or lifted `.sysml` files)
//! into the `ModelSnapshot` JSON schema consumed by the vendored MIT-licensed
//! `daltskin/VSCode_SysML_Extension` browser diagram runtime (`runtime.js` +
//! `shell.js` + D3 / ELK / Cytoscape), and bundles everything into a single
//! self-contained `.html` file that works offline over `file://` with zero
//! Node.js, npm, Java, VS Code, or WASM dependencies.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use sysml_ir::{
    ActionDef, ActionUsage, AnalysisDef, AnalysisUsage, AttributeDef, AttributeUsage, CalcDef,
    ConnectionDef, ConnectionKind, ConnectionUsage, DirectedItem, Direction, EnumDef, InterfaceDef,
    ItemDef, MetadataDef, Model, Package, PackageMember, PartDef, PartUsage, PortDef, PortUsage,
    PortionKind, PortionUsage, RequirementDef, RequirementUsage, StateDef, StateUsage, Transition,
    VerificationDef, ViewUsage,
};

/// Diagram view modes supported by the embedded SysML v2 browser runtime.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewerView {
    /// General View (BDD / compound definitions & usages via ELK).
    #[default]
    General,
    /// Interconnection View (IBD — internal parts, ports, and connectors).
    Interconnection,
    /// State Transition View (states, initial pseudostates, guarded transitions).
    State,
    /// Action Flow View (activity nodes and control/object flows).
    Activity,
    /// Sequence View (lifelines and interaction messages).
    Sequence,
    /// Case View (use cases and actors).
    UseCase,
    /// Hierarchical Tree View.
    Tree,
    /// Package Dependency View.
    Package,
    /// Force-directed Graph View.
    Graph,
    /// Hierarchy Block View.
    Hierarchy,
}

impl ViewerView {
    /// Internal view identifier expected by `runtime.js` (`message.currentView`).
    pub const fn runtime_id(self) -> &'static str {
        match self {
            Self::General => "elk",
            Self::Interconnection => "ibd",
            Self::State => "state",
            Self::Activity => "activity",
            Self::Sequence => "sequence",
            Self::UseCase => "usecase",
            Self::Tree => "tree",
            Self::Package => "package",
            Self::Graph => "graph",
            Self::Hierarchy => "hierarchy",
        }
    }
}

/// Default color theme for the generated standalone viewer HTML.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ViewerTheme {
    #[default]
    Dark,
    Light,
}

impl ViewerTheme {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Dark => "dark",
            Self::Light => "light",
        }
    }
}

/// Configuration for generating a standalone offline HTML viewer.
#[derive(Debug, Clone)]
pub struct ViewerOptions {
    pub title: String,
    pub initial_view: ViewerView,
    pub theme: ViewerTheme,
    /// Optional serialized `sysml_sim::RunResult` JSON for interactive trace playback.
    pub trace_json: Option<String>,
}

impl Default for ViewerOptions {
    fn default() -> Self {
        Self {
            title: "SysML v2 Model Viewer".to_string(),
            initial_view: ViewerView::General,
            theme: ViewerTheme::Dark,
            trace_json: None,
        }
    }
}

/// Top-level `update` message payload consumed by `runtime.js`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerSnapshot {
    pub command: String,
    pub current_view: String,
    pub elements: Vec<ViewerElement>,
    pub relationships: Vec<ViewerRelationship>,
    pub sequence_diagrams: Vec<ViewerSequenceDiagram>,
    pub activity_diagrams: Vec<ViewerActivityDiagram>,
}

/// Element node matching `convertModelElements` output expected by `runtime.js`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerElement {
    pub id: String,
    pub name: String,
    pub declared_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub typing: Option<String>,
    pub typings: Vec<String>,
    pub attributes: BTreeMap<String, serde_json::Value>,
    pub properties: BTreeMap<String, serde_json::Value>,
    pub children: Vec<ViewerElement>,
    pub relationships: Vec<ViewerRelationship>,
}

/// Directed edge matching `RelationshipDTO` expected by `runtime.js`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerRelationship {
    #[serde(rename = "type")]
    pub kind: String,
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerSequenceDiagram {
    pub name: String,
    pub participants: Vec<ViewerParticipant>,
    pub messages: Vec<ViewerMessage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerParticipant {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerMessage {
    pub name: String,
    pub from: String,
    pub to: String,
    pub payload: String,
    pub occurrence: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerActivityDiagram {
    pub name: String,
    pub actions: Vec<ViewerActivityAction>,
    pub decisions: Vec<ViewerActivityDecision>,
    pub flows: Vec<ViewerActivityFlow>,
    pub states: Vec<ViewerActivityState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerActivityAction {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub inputs: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub outputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_definition: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerActivityDecision {
    pub name: String,
    pub condition: String,
    pub branches: Vec<ViewerActivityBranch>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerActivityBranch {
    pub condition: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerActivityFlow {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guard: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewerActivityState {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
}

/// Build the `ModelSnapshot` JSON payload for `runtime.js` from any `sysml_ir::Model`.
pub fn build_viewer_snapshot(model: &Model, initial_view: ViewerView) -> ViewerSnapshot {
    let mut ctx = SnapshotBuilder::new(model);
    let mut elements = Vec::with_capacity(model.packages.len());
    for pkg in &model.packages {
        elements.push(ctx.convert_package(pkg));
    }

    ViewerSnapshot {
        command: "update".to_string(),
        current_view: initial_view.runtime_id().to_string(),
        elements,
        relationships: ctx.relationships,
        sequence_diagrams: ctx.sequence_diagrams,
        activity_diagrams: ctx.activity_diagrams,
    }
}

struct SnapshotBuilder {
    /// PortDef simple/qualified name -> inferred port direction ("in" | "out" | "inout").
    port_directions: HashMap<String, &'static str>,
    relationships: Vec<ViewerRelationship>,
    sequence_diagrams: Vec<ViewerSequenceDiagram>,
    activity_diagrams: Vec<ViewerActivityDiagram>,
}

impl SnapshotBuilder {
    fn new(model: &Model) -> Self {
        let mut port_directions = HashMap::new();
        for pkg in &model.packages {
            index_port_defs(pkg, &mut port_directions);
        }
        Self {
            port_directions,
            relationships: Vec::new(),
            sequence_diagrams: Vec::new(),
            activity_diagrams: Vec::new(),
        }
    }

    fn resolve_port_direction(&self, port: &PortUsage) -> &'static str {
        let simple = simple_name(&port.type_name);
        let base = self
            .port_directions
            .get(&port.type_name)
            .or_else(|| self.port_directions.get(simple))
            .copied()
            .unwrap_or("inout");
        if port.conjugated {
            match base {
                "in" => "out",
                "out" => "in",
                _ => "inout",
            }
        } else {
            base
        }
    }

    fn push_rel(&mut self, kind: &str, source: &str, target: &str, name: Option<String>) {
        if source.is_empty() || target.is_empty() {
            return;
        }
        self.relationships.push(ViewerRelationship {
            kind: kind.to_string(),
            source: source.to_string(),
            target: target.to_string(),
            name,
        });
    }

    fn convert_package(&mut self, pkg: &Package) -> ViewerElement {
        let mut el = make_element(&pkg.name, "package");
        set_doc(&mut el, pkg.doc.as_deref());

        for imp in &pkg.imports {
            let target_pkg = imp
                .target
                .split("::")
                .next()
                .unwrap_or(imp.target.as_str())
                .trim();
            if !is_stdlib_package(target_pkg) && target_pkg != pkg.name {
                let rel = ViewerRelationship {
                    kind: "dependency".to_string(),
                    source: pkg.name.clone(),
                    target: target_pkg.to_string(),
                    name: Some("import".to_string()),
                };
                el.relationships.push(rel.clone());
                self.relationships.push(rel);
            }
        }

        for member in &pkg.members {
            if let Some(child) = self.convert_member(member) {
                el.children.push(child);
            }
        }
        el
    }

    fn convert_member(&mut self, member: &PackageMember) -> Option<ViewerElement> {
        match member {
            PackageMember::Package(p) => Some(self.convert_package(p)),
            PackageMember::PartDef(p) => Some(self.convert_part_def(p)),
            PackageMember::PartUsage(p) => Some(self.convert_part_usage(p, None)),
            PackageMember::PortDef(p) => Some(self.convert_port_def(p)),
            PackageMember::InterfaceDef(i) => Some(self.convert_interface_def(i)),
            PackageMember::ConnectionDef(c) => Some(self.convert_connection_def(c)),
            PackageMember::ItemDef(i) => Some(self.convert_item_def(i)),
            PackageMember::EnumDef(e) => Some(self.convert_enum_def(e)),
            PackageMember::AttributeDef(a) => Some(self.convert_attribute_def(a)),
            PackageMember::ActionDef(a) => Some(self.convert_action_def(a)),
            PackageMember::StateDef(s) => Some(self.convert_state_def(s)),
            PackageMember::RequirementDef(r) => Some(self.convert_requirement_def(r)),
            PackageMember::RequirementUsage(r) => Some(self.convert_requirement_usage(r, None)),
            PackageMember::VerificationDef(v) => Some(self.convert_verification_def(v)),
            PackageMember::CalcDef(c) => Some(self.convert_calc_def(c, false)),
            PackageMember::CalcUsage(c) => Some(self.convert_calc_def(c, true)),
            PackageMember::AnalysisDef(a) => Some(self.convert_analysis_def(a)),
            PackageMember::AnalysisUsage(a) => Some(self.convert_analysis_usage(a)),
            PackageMember::AttributeUsage(a) => Some(self.convert_attribute_usage(a)),
            PackageMember::ViewUsage(v) => Some(self.convert_view_usage(v)),
            PackageMember::MetadataDef(m) => Some(self.convert_metadata_def(m)),
        }
    }

    fn convert_part_def(&mut self, p: &PartDef) -> ViewerElement {
        let mut el = make_element(&p.name, "part def");
        set_doc(&mut el, p.doc.as_deref());
        if p.is_abstract {
            el.attributes
                .insert("isAbstract".to_string(), serde_json::Value::Bool(true));
        }

        for sup in &p.specializes {
            let target = simple_name(sup);
            let rel = ViewerRelationship {
                kind: "specialization".to_string(),
                source: p.name.clone(),
                target: target.to_string(),
                name: None,
            };
            el.relationships.push(rel.clone());
            self.relationships.push(rel);
        }

        // Surface ISO 26262 FMEA & ASIL decomposition annotations cleanly in metadataAttributes & doc
        let mut annotations: Vec<String> = Vec::new();
        for fmea in &p.fmea {
            let asil_tag = fmea
                .asil
                .as_deref()
                .map(|a| format!(" [{a}]"))
                .unwrap_or_default();
            annotations.push(format!(
                "FMEA{}: {} -> {} (S={} O={} D={} RPN={})",
                asil_tag,
                fmea.failure_mode,
                fmea.effect,
                fmea.severity,
                fmea.occurrence,
                fmea.detection,
                fmea.rpn()
            ));
        }
        for dec in &p.asil_decompositions {
            annotations.push(format!(
                "ASIL Decomp ({}): {}({}) + {}({})",
                dec.goal, dec.left_part, dec.left_asil, dec.right_part, dec.right_asil
            ));
        }
        if !annotations.is_empty() {
            el.attributes.insert(
                "metadataAnnotations".to_string(),
                serde_json::Value::String(annotations.join("; ")),
            );
            append_doc(&mut el, &annotations.join("\n"));
        }

        for attr in &p.attributes {
            el.children.push(self.convert_attribute_usage(attr));
        }
        for port in &p.ports {
            el.children.push(self.convert_port_usage(port));
        }
        for item in &p.items {
            el.children.push(self.convert_directed_item(item));
        }
        for sub in &p.parts {
            el.children
                .push(self.convert_part_usage(sub, Some(&p.name)));
        }
        for v in &p.variants {
            let mut vel = make_element(&v.name, "part");
            let ty = simple_name(&v.type_name).to_string();
            vel.typing = Some(ty.clone());
            vel.typings.push(ty.clone());
            set_doc(&mut vel, v.doc.as_deref());
            el.children.push(vel);
            self.push_rel("typing", &v.name, &ty, Some("variant".to_string()));
        }
        for perf in &p.performs {
            let act_name = perf
                .name
                .clone()
                .or_else(|| perf.references.clone())
                .or_else(|| perf.action_type.clone())
                .unwrap_or_else(|| "perform".to_string());
            let mut act = make_element(&act_name, "action");
            if let Some(ty) = perf.action_type.as_deref().or(perf.references.as_deref()) {
                let target = simple_name(ty);
                act.typing = Some(target.to_string());
                act.typings.push(target.to_string());
                self.push_rel("allocation", &p.name, target, Some("perform".to_string()));
            }
            el.children.push(act);
        }
        for conn in &p.connections {
            self.record_connection_usage(conn);
        }
        for alloc in &p.allocations {
            self.push_rel(
                "allocation",
                &alloc.from,
                &alloc.to,
                alloc.name.clone().or_else(|| Some("allocate".to_string())),
            );
        }
        for st in &p.exhibit_states {
            el.children.push(self.convert_state_usage(st, Some(&p.name)));
        }
        for req in &p.requirements {
            el.children
                .push(self.convert_requirement_usage(req, Some(&p.name)));
        }
        for sat in &p.satisfies {
            let source = sat.by.as_deref().unwrap_or(&p.name);
            let target = simple_name(&sat.requirement);
            self.push_rel("satisfy", source, target, Some("satisfy".to_string()));
        }
        for ver in &p.verifies {
            let source = ver.by.as_deref().unwrap_or(&p.name);
            let target = simple_name(&ver.requirement);
            self.push_rel("verify", source, target, Some("verify".to_string()));
        }
        for r in &p.refinements {
            self.push_rel(
                "dependency",
                simple_name(&r.client),
                simple_name(&r.supplier),
                Some("refine".to_string()),
            );
        }
        el
    }

    fn convert_part_usage(&mut self, p: &PartUsage, owner: Option<&str>) -> ViewerElement {
        let name = p
            .name
            .clone()
            .or_else(|| p.redefines.first().cloned())
            .or_else(|| p.subsets.first().cloned())
            .or_else(|| p.type_names.first().cloned())
            .unwrap_or_else(|| "part".to_string());
        let mut el = make_element(&name, "part");
        set_doc(&mut el, p.doc.as_deref());

        let typings: Vec<String> = p
            .type_names
            .iter()
            .map(|t| simple_name(t).to_string())
            .collect();
        if let Some(first) = typings.first() {
            el.typing = Some(first.clone());
            el.attributes.insert(
                "partType".to_string(),
                serde_json::Value::String(first.clone()),
            );
            el.attributes.insert(
                "typedBy".to_string(),
                serde_json::Value::String(first.clone()),
            );
        }
        for ty in &typings {
            let rel = ViewerRelationship {
                kind: "typing".to_string(),
                source: name.clone(),
                target: ty.clone(),
                name: None,
            };
            el.relationships.push(rel.clone());
            self.relationships.push(rel);
        }
        el.typings = typings;

        if let Some(m) = &p.multiplicity {
            el.attributes.insert(
                "multiplicity".to_string(),
                serde_json::Value::String(m.render()),
            );
        }

        for attr in &p.attributes {
            el.children.push(self.convert_attribute_usage(attr));
        }
        for port in &p.ports {
            el.children.push(self.convert_port_usage(port));
        }
        for item in &p.items {
            el.children.push(self.convert_directed_item(item));
        }
        for sub in &p.nested_parts {
            el.children.push(self.convert_part_usage(sub, Some(&name)));
        }
        for perf in &p.performs {
            let act_name = perf
                .name
                .clone()
                .or_else(|| perf.references.clone())
                .or_else(|| perf.action_type.clone())
                .unwrap_or_else(|| "perform".to_string());
            let mut act = make_element(&act_name, "action");
            if let Some(ty) = perf.action_type.as_deref().or(perf.references.as_deref()) {
                let target = simple_name(ty);
                act.typing = Some(target.to_string());
                act.typings.push(target.to_string());
                self.push_rel("allocation", &name, target, Some("perform".to_string()));
            }
            el.children.push(act);
        }
        for conn in &p.connections {
            self.record_connection_usage(conn);
        }
        for req in &p.requirements {
            el.children
                .push(self.convert_requirement_usage(req, Some(&name)));
        }
        for portion in &p.portions {
            el.children
                .push(self.convert_portion_usage(portion, Some(&name)));
        }
        let _ = owner;
        el
    }

    fn convert_portion_usage(&mut self, p: &PortionUsage, owner: Option<&str>) -> ViewerElement {
        let kind = match p.kind {
            PortionKind::Timeslice => "part",
            PortionKind::Snapshot => "part",
        };
        let mut el = make_element(&p.name, kind);
        set_doc(&mut el, p.doc.as_deref());
        for ty in &p.type_names {
            let simple = simple_name(ty).to_string();
            if el.typing.is_none() {
                el.typing = Some(simple.clone());
            }
            el.typings.push(simple);
        }
        for attr in &p.attributes {
            el.children.push(self.convert_attribute_usage(attr));
        }
        for part in &p.parts {
            el.children
                .push(self.convert_part_usage(part, Some(&p.name)));
        }
        for sub in &p.portions {
            el.children
                .push(self.convert_portion_usage(sub, Some(&p.name)));
        }
        let _ = owner;
        el
    }

    fn convert_port_def(&mut self, p: &PortDef) -> ViewerElement {
        let mut el = make_element(&p.name, "port def");
        set_doc(&mut el, p.doc.as_deref());
        for sup in &p.specializes {
            self.push_rel("specialization", &p.name, simple_name(sup), None);
        }
        for item in &p.items {
            el.children.push(self.convert_directed_item(item));
        }
        for attr in &p.attributes {
            el.children.push(self.convert_attribute_usage(attr));
        }
        el
    }

    fn convert_port_usage(&self, p: &PortUsage) -> ViewerElement {
        let mut el = make_element(&p.name, "port");
        set_doc(&mut el, p.doc.as_deref());
        let ty = simple_name(&p.type_name).to_string();
        let display_ty = if p.conjugated {
            format!("~{ty}")
        } else {
            ty.clone()
        };
        el.typing = Some(ty.clone());
        el.typings.push(ty);
        el.attributes.insert(
            "portType".to_string(),
            serde_json::Value::String(display_ty),
        );
        el.attributes.insert(
            "direction".to_string(),
            serde_json::Value::String(self.resolve_port_direction(p).to_string()),
        );
        if let Some(m) = &p.multiplicity {
            el.attributes.insert(
                "multiplicity".to_string(),
                serde_json::Value::String(m.render()),
            );
        }
        el
    }

    fn convert_directed_item(&self, item: &DirectedItem) -> ViewerElement {
        let mut el = make_element(&item.name, "item");
        set_doc(&mut el, item.doc.as_deref());
        let ty = simple_name(&item.type_name).to_string();
        el.typing = Some(ty.clone());
        el.typings.push(ty.clone());
        el.attributes
            .insert("dataType".to_string(), serde_json::Value::String(ty));
        if let Some(dir) = item.direction {
            el.attributes.insert(
                "direction".to_string(),
                serde_json::Value::String(direction_str(dir).to_string()),
            );
        }
        if let Some(m) = &item.multiplicity {
            el.attributes.insert(
                "multiplicity".to_string(),
                serde_json::Value::String(m.render()),
            );
        }
        el
    }

    fn convert_interface_def(&mut self, i: &InterfaceDef) -> ViewerElement {
        let mut el = make_element(&i.name, "interface def");
        set_doc(&mut el, i.doc.as_deref());
        for sup in &i.specializes {
            self.push_rel("specialization", &i.name, simple_name(sup), None);
        }
        for attr in &i.attributes {
            el.children.push(self.convert_attribute_usage(attr));
        }
        for end in &i.ends {
            let mut end_el = make_element(&end.name, "end");
            let ty = simple_name(&end.type_name).to_string();
            end_el.typing = Some(ty.clone());
            end_el.typings.push(ty.clone());
            end_el.attributes.insert(
                "portType".to_string(),
                serde_json::Value::String(if end.conjugated {
                    format!("~{ty}")
                } else {
                    ty
                }),
            );
            el.children.push(end_el);
        }
        el
    }

    fn convert_connection_def(&mut self, c: &ConnectionDef) -> ViewerElement {
        let mut el = make_element(&c.name, "connection def");
        set_doc(&mut el, c.doc.as_deref());
        for sup in &c.specializes {
            self.push_rel("specialization", &c.name, simple_name(sup), None);
        }
        for end in &c.ends {
            let mut end_el = make_element(&end.name, "end");
            let ty = simple_name(&end.type_name).to_string();
            end_el.typing = Some(ty.clone());
            end_el.typings.push(ty);
            el.children.push(end_el);
        }
        el
    }

    fn record_connection_usage(&mut self, c: &ConnectionUsage) {
        let label = c
            .name
            .clone()
            .or_else(|| c.type_name.as_deref().map(|t| simple_name(t).to_string()))
            .unwrap_or_else(|| "connect".to_string());
        if let (Some(from), Some(to)) = (&c.from, &c.to) {
            let rel_kind = match c.kind {
                ConnectionKind::Interface => "interface",
                ConnectionKind::Connection => "connection",
            };
            self.push_rel(rel_kind, from, to, Some(label));
        }
    }

    fn convert_item_def(&mut self, i: &ItemDef) -> ViewerElement {
        let mut el = make_element(&i.name, "item def");
        set_doc(&mut el, i.doc.as_deref());
        for sup in &i.specializes {
            self.push_rel("specialization", &i.name, simple_name(sup), None);
        }
        for attr in &i.attributes {
            el.children.push(self.convert_attribute_usage(attr));
        }
        el
    }

    fn convert_enum_def(&self, e: &EnumDef) -> ViewerElement {
        let mut el = make_element(&e.name, "enum def");
        set_doc(&mut el, e.doc.as_deref());
        for v in &e.variants {
            el.children.push(make_element(v, "enum"));
        }
        el
    }

    fn convert_attribute_def(&mut self, a: &AttributeDef) -> ViewerElement {
        let mut el = make_element(&a.name, "attribute def");
        set_doc(&mut el, a.doc.as_deref());
        for sup in &a.specializes {
            self.push_rel("specialization", &a.name, simple_name(sup), None);
        }
        for attr in &a.attributes {
            el.children.push(self.convert_attribute_usage(attr));
        }
        el
    }

    fn convert_attribute_usage(&self, a: &AttributeUsage) -> ViewerElement {
        let name = a
            .name
            .clone()
            .or_else(|| a.redefines.first().cloned())
            .or_else(|| a.subsets.first().cloned())
            .unwrap_or_else(|| "attribute".to_string());
        let mut el = make_element(&name, "attribute");
        el.short_name = a.short_name.clone();
        set_doc(&mut el, a.doc.as_deref());
        if let Some(ty) = &a.type_name {
            let simple = simple_name(ty).to_string();
            el.typing = Some(simple.clone());
            el.typings.push(simple.clone());
            el.attributes
                .insert("dataType".to_string(), serde_json::Value::String(simple));
        }
        if let Some(val) = &a.value {
            let rendered = match &a.unit {
                Some(u) => format!("{val} [{u}]"),
                None => val.clone(),
            };
            el.attributes
                .insert("value".to_string(), serde_json::Value::String(rendered));
        }
        if let Some(m) = &a.multiplicity {
            el.attributes.insert(
                "multiplicity".to_string(),
                serde_json::Value::String(m.render()),
            );
        }
        for nested in &a.nested_attributes {
            el.children.push(self.convert_attribute_usage(nested));
        }
        el
    }

    fn convert_action_def(&mut self, a: &ActionDef) -> ViewerElement {
        let mut el = make_element(&a.name, "action def");
        set_doc(&mut el, a.doc.as_deref());
        for sup in &a.specializes {
            self.push_rel("specialization", &a.name, simple_name(sup), None);
        }
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for param in &a.params {
            let mut pel = make_element(&param.name, "attribute");
            let ty = simple_name(&param.type_name).to_string();
            pel.typing = Some(ty.clone());
            pel.typings.push(ty.clone());
            pel.attributes
                .insert("dataType".to_string(), serde_json::Value::String(ty));
            pel.attributes.insert(
                "direction".to_string(),
                serde_json::Value::String(direction_str(param.direction).to_string()),
            );
            el.children.push(pel);
            match param.direction {
                Direction::In => inputs.push(param.name.clone()),
                Direction::Out => outputs.push(param.name.clone()),
                Direction::InOut => {
                    inputs.push(param.name.clone());
                    outputs.push(param.name.clone());
                }
            }
        }
        for sub in &a.subactions {
            el.children.push(self.convert_action_usage(sub));
        }
        for step in &a.steps {
            if !a.subactions.iter().any(|s| s.name == step.name) {
                let mut sel = make_element(&step.name, "action");
                if let Some(ty) = &step.type_name {
                    let simple = simple_name(ty).to_string();
                    sel.typing = Some(simple.clone());
                    sel.typings.push(simple);
                }
                el.children.push(sel);
            }
        }
        for r in &a.refinements {
            self.push_rel(
                "dependency",
                simple_name(&r.client),
                simple_name(&r.supplier),
                Some("refine".to_string()),
            );
        }

        // Populate ActivityDiagramDTO so Action Flow View renders rich flow graphs
        if !a.subactions.is_empty() || !a.steps.is_empty() {
            let mut actions = Vec::new();
            let mut flows = Vec::new();
            let mut states = Vec::new();

            if a.has_initial_start {
                states.push(ViewerActivityState {
                    name: "start".to_string(),
                    kind: "initial".to_string(),
                });
            }
            if a.has_done {
                states.push(ViewerActivityState {
                    name: "done".to_string(),
                    kind: "final".to_string(),
                });
            }

            let mut seen_actions = HashMap::<String, usize>::new();
            for sub in &a.subactions {
                seen_actions.insert(sub.name.clone(), actions.len());
                actions.push(ViewerActivityAction {
                    name: sub.name.clone(),
                    kind: sub.type_name.clone().unwrap_or_else(|| "action".to_string()),
                    id: Some(sub.name.clone()),
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    parent: None,
                    is_definition: Some(false),
                });
            }
            let mut prev: Option<String> = if a.has_initial_start {
                Some("start".to_string())
            } else {
                None
            };
            for step in &a.steps {
                if !seen_actions.contains_key(&step.name) {
                    seen_actions.insert(step.name.clone(), actions.len());
                    actions.push(ViewerActivityAction {
                        name: step.name.clone(),
                        kind: step
                            .type_name
                            .clone()
                            .unwrap_or_else(|| "action".to_string()),
                        id: Some(step.name.clone()),
                        inputs: Vec::new(),
                        outputs: Vec::new(),
                        parent: None,
                        is_definition: Some(false),
                    });
                }
                if let Some(from) = &prev {
                    flows.push(ViewerActivityFlow {
                        from: from.clone(),
                        to: step.name.clone(),
                        condition: None,
                        guard: None,
                    });
                }
                prev = Some(step.name.clone());
            }
            if a.has_done {
                if let Some(from) = prev {
                    flows.push(ViewerActivityFlow {
                        from,
                        to: "done".to_string(),
                        condition: None,
                        guard: None,
                    });
                }
            }

            // If there were only subactions and no explicit `then` steps, show parent action box
            if actions.is_empty() {
                actions.push(ViewerActivityAction {
                    name: a.name.clone(),
                    kind: "action".to_string(),
                    id: Some(a.name.clone()),
                    inputs,
                    outputs,
                    parent: None,
                    is_definition: Some(true),
                });
            }

            self.activity_diagrams.push(ViewerActivityDiagram {
                name: a.name.clone(),
                actions,
                decisions: Vec::new(),
                flows,
                states,
            });
        }

        el
    }

    fn convert_action_usage(&self, a: &ActionUsage) -> ViewerElement {
        let mut el = make_element(&a.name, "action");
        set_doc(&mut el, a.doc.as_deref());
        if let Some(ty) = &a.type_name {
            let simple = simple_name(ty).to_string();
            el.typing = Some(simple.clone());
            el.typings.push(simple);
        }
        el
    }

    fn convert_state_def(&mut self, s: &StateDef) -> ViewerElement {
        let mut el = make_element(&s.name, "state def");
        set_doc(&mut el, s.doc.as_deref());
        for sup in &s.specializes {
            self.push_rel("specialization", &s.name, simple_name(sup), None);
        }
        if let Some(first) = s.states.first() {
            self.push_rel("transition", "__sysml_initial__", &first.name, None);
        }
        for child in &s.states {
            el.children
                .push(self.convert_state_usage(child, Some(&s.name)));
        }
        for tr in &s.transitions {
            self.push_transition(tr);
        }
        el
    }

    fn convert_state_usage(&mut self, s: &StateUsage, _owner: Option<&str>) -> ViewerElement {
        let kind = if s.is_exhibit { "exhibit" } else { "state" };
        let mut el = make_element(&s.name, kind);
        set_doc(&mut el, s.doc.as_deref());
        if let Some(ty) = &s.type_name {
            let simple = simple_name(ty).to_string();
            el.typing = Some(simple.clone());
            el.typings.push(simple);
        }
        if let Some(entry) = &s.entry_action {
            el.attributes.insert(
                "entryAction".to_string(),
                serde_json::Value::String(entry.name.clone()),
            );
        }
        if let Some(do_act) = &s.do_action {
            el.attributes.insert(
                "doActivity".to_string(),
                serde_json::Value::String(do_act.name.clone()),
            );
        }
        if let Some(exit) = &s.exit_action {
            el.attributes.insert(
                "exitAction".to_string(),
                serde_json::Value::String(exit.name.clone()),
            );
        }
        if let Some(first) = s.states.first() {
            self.push_rel("transition", "__sysml_initial__", &first.name, None);
        }
        for child in &s.states {
            el.children
                .push(self.convert_state_usage(child, Some(&s.name)));
        }
        for tr in &s.transitions {
            self.push_transition(tr);
        }
        el
    }

    fn push_transition(&mut self, tr: &Transition) {
        let mut parts = Vec::new();
        if let Some(trig) = &tr.trigger {
            parts.push(trig.clone());
        } else if let Some(name) = &tr.name {
            parts.push(name.clone());
        }
        if let Some(guard) = &tr.guard {
            parts.push(format!("[{guard}]"));
        }
        if let Some(eff) = &tr.effect {
            parts.push(format!("/ {eff}"));
        }
        let label = if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        };
        self.push_rel("transition", &tr.source, &tr.target, label);
    }

    fn convert_requirement_def(&mut self, r: &RequirementDef) -> ViewerElement {
        let mut el = make_element(&r.name, "requirement def");
        el.short_name = r.short_name.clone();
        if let Some(id) = &r.short_name {
            el.attributes
                .insert("shortName".to_string(), serde_json::Value::String(id.clone()));
        }

        let mut doc_lines = Vec::new();
        if let Some(d) = &r.doc {
            doc_lines.push(d.clone());
        }
        if let Some(rat) = &r.rationale {
            doc_lines.push(format!("Rationale: {rat}"));
        }
        if let Some(sg) = &r.safety_goal {
            let sg_str = format!(
                "SafetyGoal [{}]: safe_state='{}', FTTI={}ms",
                sg.asil, sg.safe_state, sg.ftti_ms
            );
            el.attributes.insert(
                "metadataAnnotations".to_string(),
                serde_json::Value::String(sg_str.clone()),
            );
            doc_lines.push(sg_str);
        }
        if !doc_lines.is_empty() {
            set_doc(&mut el, Some(&doc_lines.join("\n")));
        }

        if !r.require_constraints.is_empty() {
            el.attributes.insert(
                "constraint".to_string(),
                serde_json::Value::String(r.require_constraints.join(" && ")),
            );
        }
        if let Some(subj) = &r.subject {
            let mut sel = make_element(&subj.name, "subject");
            let ty = simple_name(&subj.type_name).to_string();
            sel.typing = Some(ty.clone());
            sel.typings.push(ty.clone());
            el.children.push(sel);
            self.push_rel("subject", &r.name, &ty, Some("subject".to_string()));
        }
        for attr in &r.attributes {
            el.children.push(self.convert_attribute_usage(attr));
        }
        for sup in &r.specializes {
            self.push_rel("specialization", &r.name, simple_name(sup), None);
        }
        for rf in &r.refinements {
            self.push_rel(
                "dependency",
                simple_name(&rf.client),
                simple_name(&rf.supplier),
                Some("refine".to_string()),
            );
        }
        el
    }

    fn convert_requirement_usage(&mut self, r: &RequirementUsage, owner: Option<&str>) -> ViewerElement {
        let mut el = make_element(&r.name, "requirement");
        el.short_name = r.short_name.clone();
        if let Some(id) = &r.short_name {
            el.attributes
                .insert("shortName".to_string(), serde_json::Value::String(id.clone()));
        }
        set_doc(&mut el, r.doc.as_deref());
        if let Some(ty) = &r.type_name {
            let simple = simple_name(ty).to_string();
            el.typing = Some(simple.clone());
            el.typings.push(simple);
        }
        for child in &r.nested_requirements {
            el.children
                .push(self.convert_requirement_usage(child, Some(&r.name)));
        }
        for sat in &r.satisfies {
            let source = sat.by.as_deref().or(owner).unwrap_or(&r.name);
            self.push_rel(
                "satisfy",
                source,
                simple_name(&sat.requirement),
                Some("satisfy".to_string()),
            );
        }
        for ver in &r.verifies {
            let source = ver.by.as_deref().or(owner).unwrap_or(&r.name);
            self.push_rel(
                "verify",
                source,
                simple_name(&ver.requirement),
                Some("verify".to_string()),
            );
        }
        el
    }

    fn convert_verification_def(&mut self, v: &VerificationDef) -> ViewerElement {
        let mut el = make_element(&v.name, "verification def");
        set_doc(&mut el, v.doc.as_deref());
        for ver in &v.verifies {
            let source = ver.by.as_deref().unwrap_or(&v.name);
            self.push_rel(
                "verify",
                source,
                simple_name(&ver.requirement),
                Some("verify".to_string()),
            );
        }
        el
    }

    fn convert_calc_def(&self, c: &CalcDef, is_usage: bool) -> ViewerElement {
        let mut el = make_element(&c.name, if is_usage { "function" } else { "function" });
        el.short_name = c.short_name.clone();
        set_doc(&mut el, c.doc.as_deref());
        if !c.result_expr.is_empty() {
            el.attributes.insert(
                "expression".to_string(),
                serde_json::Value::String(c.result_expr.clone()),
            );
        }
        for p in &c.params {
            let mut pel = make_element(&p.name, "attribute");
            let ty = simple_name(&p.type_name).to_string();
            pel.typing = Some(ty.clone());
            pel.typings.push(ty.clone());
            pel.attributes
                .insert("dataType".to_string(), serde_json::Value::String(ty));
            el.children.push(pel);
        }
        el
    }

    fn convert_analysis_def(&mut self, a: &AnalysisDef) -> ViewerElement {
        let mut el = make_element(&a.name, "action def");
        set_doc(&mut el, a.doc.as_deref());
        for sup in &a.specializes {
            self.push_rel("specialization", &a.name, simple_name(sup), None);
        }
        el
    }

    fn convert_analysis_usage(&self, a: &AnalysisUsage) -> ViewerElement {
        let mut el = make_element(&a.name, "action");
        set_doc(&mut el, a.doc.as_deref());
        if let Some(ty) = &a.type_name {
            let simple = simple_name(ty).to_string();
            el.typing = Some(simple.clone());
            el.typings.push(simple);
        }
        el
    }

    fn convert_view_usage(&self, v: &ViewUsage) -> ViewerElement {
        let mut el = make_element(&v.name, "view");
        set_doc(&mut el, v.doc.as_deref());
        if let Some(ty) = &v.type_name {
            let simple = simple_name(ty).to_string();
            el.typing = Some(simple.clone());
            el.typings.push(simple);
        }
        el
    }

    fn convert_metadata_def(&self, m: &MetadataDef) -> ViewerElement {
        let mut el = make_element(&m.name, "metadata def");
        set_doc(&mut el, m.doc.as_deref());
        for attr in &m.attributes {
            el.children.push(self.convert_attribute_usage(attr));
        }
        el
    }
}

fn make_element(name: &str, kind: &str) -> ViewerElement {
    ViewerElement {
        id: name.to_string(),
        name: name.to_string(),
        declared_name: name.to_string(),
        short_name: None,
        kind: kind.to_string(),
        typing: None,
        typings: Vec::new(),
        attributes: BTreeMap::new(),
        properties: BTreeMap::new(),
        children: Vec::new(),
        relationships: Vec::new(),
    }
}

fn set_doc(el: &mut ViewerElement, doc: Option<&str>) {
    if let Some(text) = doc {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            el.attributes.insert(
                "doc".to_string(),
                serde_json::Value::String(trimmed.to_string()),
            );
            el.properties.insert(
                "documentation".to_string(),
                serde_json::Value::String(trimmed.to_string()),
            );
        }
    }
}

fn append_doc(el: &mut ViewerElement, extra: &str) {
    let combined = match el.attributes.get("doc").and_then(|v| v.as_str()) {
        Some(existing) if !existing.is_empty() => format!("{existing}\n{extra}"),
        _ => extra.to_string(),
    };
    set_doc(el, Some(&combined));
}

fn direction_str(d: Direction) -> &'static str {
    match d {
        Direction::In => "in",
        Direction::Out => "out",
        Direction::InOut => "inout",
    }
}

fn simple_name(raw: &str) -> &str {
    let trimmed = raw.trim().trim_start_matches('~').trim();
    trimmed.rsplit("::").next().unwrap_or(trimmed).trim()
}

fn is_stdlib_package(pkg: &str) -> bool {
    matches!(
        pkg,
        "ScalarValues"
            | "SI"
            | "ISQ"
            | "ISQBase"
            | "ISQSpaceTime"
            | "ISQMechanics"
            | "ISQThermodynamics"
            | "ISQElectromagnetism"
            | "ModelingMetadata"
            | "Quantities"
            | "MeasurementReferences"
            | "Base"
            | "Links"
            | "Occurrences"
            | "Objects"
            | "Performances"
            | "ISO26262Metadata"
            | "TbdMetadata"
            | "CustomUnitMetadata"
    )
}

fn index_port_defs(pkg: &Package, map: &mut HashMap<String, &'static str>) {
    for m in &pkg.members {
        match m {
            PackageMember::Package(sub) => index_port_defs(sub, map),
            PackageMember::PortDef(p) => {
                let dir = if p.is_symmetric {
                    "inout"
                } else {
                    let has_in = p
                        .items
                        .iter()
                        .any(|i| matches!(i.direction, Some(Direction::In | Direction::InOut)));
                    let has_out = p
                        .items
                        .iter()
                        .any(|i| matches!(i.direction, Some(Direction::Out | Direction::InOut)));
                    match (has_in, has_out) {
                        (true, false) => "in",
                        (false, true) => "out",
                        _ => "inout",
                    }
                };
                map.insert(p.name.clone(), dir);
                map.insert(format!("{}::{}", pkg.name, p.name), dir);
            }
            _ => {}
        }
    }
}

#[cfg(feature = "viewer")]
const ASSET_SHELL_JS: &str = include_str!("../../../third_party/vscode-sysml-viewer/shell.js");
#[cfg(feature = "viewer")]
const ASSET_RUNTIME_JS: &str = include_str!("../../../third_party/vscode-sysml-viewer/runtime.js");
#[cfg(feature = "viewer")]
const ASSET_D3_JS: &str =
    include_str!("../../../third_party/vscode-sysml-viewer/vendor/d3.min.js");
#[cfg(feature = "viewer")]
const ASSET_ELK_JS: &str =
    include_str!("../../../third_party/vscode-sysml-viewer/vendor/elk.bundled.js");
#[cfg(feature = "viewer")]
const ASSET_CYTOSCAPE_JS: &str =
    include_str!("../../../third_party/vscode-sysml-viewer/vendor/cytoscape.min.js");
#[cfg(feature = "viewer")]
const ASSET_CYTOSCAPE_ELK_JS: &str =
    include_str!("../../../third_party/vscode-sysml-viewer/vendor/cytoscape-elk.js");
#[cfg(feature = "viewer")]
const ASSET_CYTOSCAPE_SVG_JS: &str =
    include_str!("../../../third_party/vscode-sysml-viewer/vendor/cytoscape-svg.js");

/// Standalone CSS definitions for all `--vscode-*` custom properties used by
/// `shell.js` and `runtime.js` (supporting both Dark and Light themes).
#[cfg(feature = "viewer")]
const STANDALONE_THEME_CSS: &str = r#"
:root, :root[data-theme="dark"] {
    color-scheme: dark;
    --vscode-font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    --vscode-editor-font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    --vscode-foreground: #d4d4d4;
    --vscode-descriptionForeground: #9da5b4;
    --vscode-errorForeground: #f48771;
    --vscode-editor-background: #1e1e1e;
    --vscode-editor-foreground: #d4d4d4;
    --vscode-editor-selectionBackground: #264f78;
    --vscode-editor-selectionHighlightBackground: rgba(173, 214, 255, 0.15);
    --vscode-editor-inactiveSelectionBackground: #3a3d41;
    --vscode-editorWidget-background: #252526;
    --vscode-panel-border: #3c3c3c;
    --vscode-focusBorder: #007fd4;
    --vscode-button-background: #0e639c;
    --vscode-button-foreground: #ffffff;
    --vscode-button-hoverBackground: #1177bb;
    --vscode-button-border: #3c3c3c;
    --vscode-button-secondaryBackground: #3a3d41;
    --vscode-button-secondaryForeground: #ffffff;
    --vscode-button-secondaryHoverBackground: #45494e;
    --vscode-input-background: #3c3c3c;
    --vscode-input-foreground: #cccccc;
    --vscode-input-border: #555555;
    --vscode-dropdown-background: #3c3c3c;
    --vscode-menu-background: #252526;
    --vscode-menu-foreground: #cccccc;
    --vscode-menu-border: #454545;
    --vscode-menu-separatorBackground: #454545;
    --vscode-list-hoverBackground: #2a2d2e;
    --vscode-list-activeSelectionBackground: #04395e;
    --vscode-list-activeSelectionForeground: #ffffff;
    --vscode-statusBar-background: #252526;
    --vscode-statusBar-foreground: #cccccc;
    --vscode-titleBar-activeBackground: #323233;
    --vscode-titleBar-activeForeground: #cccccc;
    --vscode-charts-blue: #4fc1ff;
    --vscode-charts-green: #89d185;
    --vscode-charts-red: #f14c4c;
    --vscode-charts-orange: #d18616;
    --vscode-charts-purple: #b180d7;
    --vscode-charts-yellow: #cca700;
    --vscode-charts-lines: #808080;
}

:root[data-theme="light"] {
    color-scheme: light;
    --vscode-font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    --vscode-editor-font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", monospace;
    --vscode-foreground: #1f2328;
    --vscode-descriptionForeground: #57606a;
    --vscode-errorForeground: #cf222e;
    --vscode-editor-background: #ffffff;
    --vscode-editor-foreground: #1f2328;
    --vscode-editor-selectionBackground: #add6ff;
    --vscode-editor-selectionHighlightBackground: rgba(173, 214, 255, 0.35);
    --vscode-editor-inactiveSelectionBackground: #e5ebf1;
    --vscode-editorWidget-background: #f3f3f3;
    --vscode-panel-border: #d0d7de;
    --vscode-focusBorder: #0969da;
    --vscode-button-background: #0969da;
    --vscode-button-foreground: #ffffff;
    --vscode-button-hoverBackground: #0550ae;
    --vscode-button-border: #d0d7de;
    --vscode-button-secondaryBackground: #eaeef2;
    --vscode-button-secondaryForeground: #1f2328;
    --vscode-button-secondaryHoverBackground: #dce2e8;
    --vscode-input-background: #ffffff;
    --vscode-input-foreground: #1f2328;
    --vscode-input-border: #d0d7de;
    --vscode-dropdown-background: #ffffff;
    --vscode-menu-background: #ffffff;
    --vscode-menu-foreground: #1f2328;
    --vscode-menu-border: #d0d7de;
    --vscode-menu-separatorBackground: #d0d7de;
    --vscode-list-hoverBackground: #f3f4f6;
    --vscode-list-activeSelectionBackground: #ddf4ff;
    --vscode-list-activeSelectionForeground: #1f2328;
    --vscode-statusBar-background: #f6f8fa;
    --vscode-statusBar-foreground: #24292f;
    --vscode-titleBar-activeBackground: #f6f8fa;
    --vscode-titleBar-activeForeground: #24292f;
    --vscode-charts-blue: #0969da;
    --vscode-charts-green: #1a7f37;
    --vscode-charts-red: #cf222e;
    --vscode-charts-orange: #bc4c00;
    --vscode-charts-purple: #8250df;
    --vscode-charts-yellow: #9a6700;
    --vscode-charts-lines: #6e7781;
}

#rsysdsl-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 8px;
    padding: 6px 12px;
    background: var(--vscode-titleBar-activeBackground);
    color: var(--vscode-titleBar-activeForeground);
    border: 1px solid var(--vscode-panel-border);
    border-radius: 4px;
    font-size: 12px;
}
#rsysdsl-topbar .rsysdsl-brand {
    font-weight: 600;
    letter-spacing: 0.01em;
}
#rsysdsl-topbar .rsysdsl-meta {
    color: var(--vscode-descriptionForeground);
    font-size: 11px;
}
#rsysdsl-theme-toggle {
    margin: 0;
    padding: 3px 10px;
    font-size: 11px;
    background: var(--vscode-button-secondaryBackground);
    color: var(--vscode-button-secondaryForeground);
    border: 1px solid var(--vscode-panel-border);
    border-radius: 4px;
    cursor: pointer;
}
#rsysdsl-theme-toggle:hover {
    background: var(--vscode-button-secondaryHoverBackground);
}
#dashboard-btn,
#workbench-btn,
#architecture-btn {
    display: none !important;
}
"#;

/// Escape closing `</script>` sequences so arbitrary JS/JSON embeds safely in `<script>` tags.
fn escape_script_tag(s: &str) -> String {
    s.replace("</script", "<\\/script")
        .replace("</SCRIPT", "<\\/SCRIPT")
}

fn escape_html_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Render a completely self-contained, single-file offline HTML interactive SysML v2 viewer.
///
/// The generated HTML contains all vendor JS libraries (D3, ELK, Cytoscape),
/// the MIT-licensed `VSCode_SysML_Extension` diagram runtime, light/dark themes,
/// working SVG/PNG/JSON download export, and the model snapshot JSON. It opens
/// directly in any modern browser over `file://` or `http://` with zero network access.
#[cfg(feature = "viewer")]
pub fn render_standalone_html(model: &Model, options: &ViewerOptions) -> String {
    let snapshot = build_viewer_snapshot(model, options.initial_view);
    let snapshot_json =
        serde_json::to_string(&snapshot).expect("ViewerSnapshot serialization cannot fail");
    let safe_snapshot_json = escape_script_tag(&snapshot_json);
    let safe_title_html = escape_html_text(&options.title);
    let safe_title_js = escape_script_tag(
        &serde_json::to_string(&options.title).unwrap_or_else(|_| "\"sysml\"".into()),
    );
    let theme_attr = options.theme.as_str();

    let safe_shell = escape_script_tag(ASSET_SHELL_JS);
    let safe_runtime = escape_script_tag(ASSET_RUNTIME_JS);
    let safe_d3 = escape_script_tag(ASSET_D3_JS);
    let safe_elk = escape_script_tag(ASSET_ELK_JS);
    let safe_cy = escape_script_tag(ASSET_CYTOSCAPE_JS);
    let safe_cy_elk = escape_script_tag(ASSET_CYTOSCAPE_ELK_JS);
    let safe_cy_svg = escape_script_tag(ASSET_CYTOSCAPE_SVG_JS);

    let mut html = String::with_capacity(
        safe_shell.len()
            + safe_runtime.len()
            + safe_d3.len()
            + safe_elk.len()
            + safe_cy.len()
            + safe_cy_elk.len()
            + safe_cy_svg.len()
            + safe_snapshot_json.len()
            + 8192,
    );

    html.push_str("<!DOCTYPE html>\n<html lang=\"en\" data-theme=\"");
    html.push_str(theme_attr);
    html.push_str("\">\n<head>\n<meta charset=\"UTF-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n<title>");
    html.push_str(&safe_title_html);
    html.push_str("</title>\n<style id=\"rsysdsl-theme-style\">");
    html.push_str(STANDALONE_THEME_CSS);
    html.push_str("</style>\n<script>");
    html.push_str(&safe_shell);
    html.push_str("</script>\n<script>");
    html.push_str(&safe_runtime);
    html.push_str("</script>\n<script>");
    html.push_str(&safe_d3);
    html.push_str("</script>\n<script>");
    html.push_str(&safe_elk);
    html.push_str("</script>\n<script>");
    html.push_str(&safe_cy);
    html.push_str("</script>\n<script>");
    html.push_str(&safe_cy_elk);
    html.push_str("</script>\n<script>");
    html.push_str(&safe_cy_svg);
    html.push_str("</script>\n</head>\n<body>\n<script>\n(function() {\n");
    html.push_str("  const MODEL_TITLE = ");
    html.push_str(&safe_title_js);
    html.push_str(";\n  const SNAPSHOT = ");
    html.push_str(&safe_snapshot_json);
    html.push_str(";\n  window.__RSYSDSL_SNAPSHOT__ = SNAPSHOT;\n");
    html.push_str(
        r#"
  if (globalThis.SysMLDiagramShell) {
    const shellStyle = document.createElement('style');
    shellStyle.id = 'sysml-shell-style';
    shellStyle.textContent = globalThis.SysMLDiagramShell.css;
    document.head.appendChild(shellStyle);
    document.body.innerHTML =
      '<div id="rsysdsl-topbar">' +
      '  <div><span class="rsysdsl-brand">' + MODEL_TITLE.replace(/&/g,'&amp;').replace(/</g,'&lt;') + '</span>' +
      '  <span class="rsysdsl-meta"> &middot; rsysdsl offline SysML v2 viewer</span></div>' +
      '  <button id="rsysdsl-theme-toggle" type="button" title="Toggle Dark / Light theme">Theme: ' +
      document.documentElement.getAttribute('data-theme') + '</button>' +
      '</div>' +
      globalThis.SysMLDiagramShell.body;
  }

  let savedState = {};
  window.acquireVsCodeApi = function() {
    return {
      getState() { return savedState; },
      setState(s) { savedState = s || {}; },
      postMessage(msg) {
        if (!msg || typeof msg !== 'object') return;
        if (msg.command === 'export' && msg.data) {
          const ext = msg.format || 'svg';
          const slug = MODEL_TITLE.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'sysml-diagram';
          const a = document.createElement('a');
          a.href = msg.data;
          a.download = slug + '.' + ext;
          document.body.appendChild(a);
          a.click();
          a.remove();
        }
      }
    };
  };

  // Pass null for elkWorkerUrl so ELK uses main-thread layout (works over file:// without Worker CORS errors).
  SysMLDiagramRuntime.mount(window, document, d3, window.acquireVsCodeApi, null);

  const toggleBtn = document.getElementById('rsysdsl-theme-toggle');
  if (toggleBtn) {
    toggleBtn.addEventListener('click', function() {
      const next = document.documentElement.getAttribute('data-theme') === 'light' ? 'dark' : 'light';
      document.documentElement.setAttribute('data-theme', next);
      toggleBtn.textContent = 'Theme: ' + next;
      window.dispatchEvent(new MessageEvent('message', { data: SNAPSHOT }));
    });
  }

  window.dispatchEvent(new MessageEvent('message', { data: SNAPSHOT }));
"#,
    );

    if let Some(raw_trace) = &options.trace_json {
        let safe_trace_json = escape_script_tag(raw_trace);
        html.push_str("\n  const SIM_TRACE = ");
        html.push_str(&safe_trace_json);
        html.push_str(";\n  window.__RSYSDSL_TRACE__ = SIM_TRACE;\n");
        html.push_str(SIM_TRACE_PLAYBACK_JS);
    }

    html.push_str("})();\n</script>\n</body>\n</html>\n");
    html
}

#[cfg(feature = "viewer")]
const SIM_TRACE_PLAYBACK_JS: &str = r##"
  (function mountTraceDrawer(run) {
    if (!run || !run.trace || !Array.isArray(run.trace.time_s) || run.trace.time_s.length === 0) return;
    const style = document.createElement('style');
    style.textContent = `
      #rsysdsl-trace-drawer { position: fixed; left: 12px; right: 12px; bottom: 12px; z-index: 9999;
        background: rgba(18, 22, 30, 0.96); color: #e6edf3; border: 1px solid #30363d; border-radius: 10px;
        font-family: ui-sans-serif, system-ui, -apple-system, sans-serif; font-size: 12px; box-shadow: 0 10px 30px rgba(0,0,0,0.45); }
      [data-theme="light"] #rsysdsl-trace-drawer { background: rgba(255,255,255,0.97); color: #1f2328; border-color: #d0d7de; }
      #rsysdsl-trace-header { display: flex; align-items: center; gap: 10px; padding: 8px 12px; cursor: pointer; user-select: none; border-bottom: 1px solid rgba(128,128,128,0.25); flex-wrap: wrap; }
      .rst-badge { padding: 2px 7px; border-radius: 999px; font-weight: 600; font-size: 11px; }
      .rst-pass { background: #1f6f3d; color: #fff; }
      .rst-fail { background: #b62324; color: #fff; }
      .rst-mode { background: #1f6feb; color: #fff; }
      #rsysdsl-trace-body { padding: 10px 12px; max-height: 360px; overflow-y: auto; }
      #rsysdsl-trace-drawer.collapsed #rsysdsl-trace-body { display: none; }
      #rsysdsl-trace-controls { display: flex; align-items: center; gap: 10px; margin-bottom: 10px; }
      #rsysdsl-trace-controls button { padding: 4px 10px; border-radius: 6px; border: 1px solid #484f58; background: #21262d; color: #e6edf3; cursor: pointer; }
      [data-theme="light"] #rsysdsl-trace-controls button { background: #f6f8fa; color: #1f2328; border-color: #d0d7de; }
      #rsysdsl-trace-scrubber { flex: 1; }
      #rsysdsl-trace-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 10px; }
      .rst-card { border: 1px solid rgba(128,128,128,0.25); border-radius: 8px; padding: 6px 8px; background: rgba(128,128,128,0.06); }
      .rst-card-hdr { display: flex; justify-content: space-between; font-size: 11px; margin-bottom: 4px; font-family: ui-monospace, monospace; }
      .rst-verdicts { display: flex; gap: 6px; flex-wrap: wrap; margin-left: auto; }
    `;
    document.head.appendChild(style);

    const times = run.trace.time_s;
    const signals = run.trace.signals || [];
    const events = run.trace.events || [];
    const n = times.length;
    const tMax = times[n - 1] || 1;
    let idx = 0;
    let timer = null;

    const drawer = document.createElement('div');
    drawer.id = 'rsysdsl-trace-drawer';
    const verdictsHtml = (run.verdicts || []).map(v => {
      const cls = v.pass ? 'rst-pass' : 'rst-fail';
      const title = (v.message || '') + (v.min_margin != null ? ' | min_margin=' + Number(v.min_margin).toFixed(3) : '');
      return '<span class="rst-badge ' + cls + '" title="' + title.replace(/"/g, '&quot;') + '">' + v.requirement_id + (v.pass ? ' PASS' : ' FAIL') + '</span>';
    }).join('');

    drawer.innerHTML =
      '<div id="rsysdsl-trace-header">' +
      '  <strong>Sim Trace: ' + (run.scenario || 'scenario') + '</strong>' +
      '  <span class="rst-badge ' + (run.pass ? 'rst-pass' : 'rst-fail') + '">' + (run.pass ? 'PASS' : 'FAIL') + '</span>' +
      '  <span id="rst-mode-pill" class="rst-badge rst-mode">mode</span>' +
      '  <div class="rst-verdicts">' + verdictsHtml + '</div>' +
      '  <span id="rst-toggle-lbl" style="opacity:0.75;margin-left:8px;">[hide]</span>' +
      '</div>' +
      '<div id="rsysdsl-trace-body">' +
      '  <div id="rsysdsl-trace-controls">' +
      '    <button id="rst-play" type="button">Play</button>' +
      '    <input id="rsysdsl-trace-scrubber" type="range" min="0" max="' + (n - 1) + '" value="0" />' +
      '    <span id="rst-time-lbl" style="min-width:95px;text-align:right;font-family:ui-monospace,monospace;">t = 0.00 s</span>' +
      '  </div>' +
      '  <div id="rsysdsl-trace-grid"></div>' +
      '</div>';
    document.body.appendChild(drawer);

    const header = drawer.querySelector('#rsysdsl-trace-header');
    const toggleLbl = drawer.querySelector('#rst-toggle-lbl');
    header.addEventListener('click', function() {
      drawer.classList.toggle('collapsed');
      toggleLbl.textContent = drawer.classList.contains('collapsed') ? '[show]' : '[hide]';
    });

    const grid = drawer.querySelector('#rsysdsl-trace-grid');
    const W = 250, H = 64, PAD = 4;
    const stride = Math.max(1, Math.floor(n / 400));
    const cards = signals.map(sig => {
      const vals = sig.values || [];
      let minV = Infinity, maxV = -Infinity;
      for (let i = 0; i < vals.length; i++) {
        if (vals[i] < minV) minV = vals[i];
        if (vals[i] > maxV) maxV = vals[i];
      }
      if (!isFinite(minV)) { minV = 0; maxV = 1; }
      const span = (maxV - minV) || 1;
      const pts = [];
      for (let i = 0; i < vals.length; i += stride) {
        const x = PAD + (times[i] / (tMax || 1)) * (W - 2 * PAD);
        const y = H - PAD - ((vals[i] - minV) / span) * (H - 2 * PAD);
        pts.push(x.toFixed(1) + ',' + y.toFixed(1));
      }
      const evLines = events.map(e => {
        const x = PAD + (e.t_s / (tMax || 1)) * (W - 2 * PAD);
        const color = e.kind === 'fault' ? '#f0883e' : '#58a6ff';
        return '<line x1="' + x.toFixed(1) + '" y1="0" x2="' + x.toFixed(1) + '" y2="' + H + '" stroke="' + color + '" stroke-dasharray="2,2" stroke-opacity="0.7"><title>' + e.kind + ': ' + e.name + ' @ ' + e.t_s.toFixed(3) + 's</title></line>';
      }).join('');
      const card = document.createElement('div');
      card.className = 'rst-card';
      card.innerHTML =
        '<div class="rst-card-hdr"><span>' + sig.name + '</span><span class="rst-val">0</span></div>' +
        '<svg viewBox="0 0 ' + W + ' ' + H + '" width="100%" height="' + H + '">' +
        evLines +
        '<polyline fill="none" stroke="#3fb950" stroke-width="1.6" points="' + pts.join(' ') + '" />' +
        '<line class="rst-cursor" x1="0" y1="0" x2="0" y2="' + H + '" stroke="#ff7b72" stroke-width="1.5" />' +
        '</svg>';
      grid.appendChild(card);
      return { sig, valEl: card.querySelector('.rst-val'), cursorEl: card.querySelector('.rst-cursor') };
    });

    const scrubber = drawer.querySelector('#rsysdsl-trace-scrubber');
    const timeLbl = drawer.querySelector('#rst-time-lbl');
    const modePill = drawer.querySelector('#rst-mode-pill');
    const playBtn = drawer.querySelector('#rst-play');

    function renderAt(i) {
      idx = Math.max(0, Math.min(n - 1, i));
      const t = times[idx];
      scrubber.value = String(idx);
      timeLbl.textContent = 't = ' + t.toFixed(2) + ' s';
      let curMode = '—';
      for (let j = 0; j < events.length; j++) {
        if (events[j].kind === 'mode' && events[j].t_s <= t + 1e-9) curMode = events[j].name;
      }
      modePill.textContent = curMode;
      const x = PAD + (t / (tMax || 1)) * (W - 2 * PAD);
      for (const c of cards) {
        const v = c.sig.values[idx];
        c.valEl.textContent = v != null ? Number(v).toFixed(2) : '—';
        c.cursorEl.setAttribute('x1', x.toFixed(1));
        c.cursorEl.setAttribute('x2', x.toFixed(1));
      }
    }

    scrubber.addEventListener('input', () => renderAt(Number(scrubber.value)));
    playBtn.addEventListener('click', () => {
      if (timer) { clearInterval(timer); timer = null; playBtn.textContent = 'Play'; return; }
      playBtn.textContent = 'Pause';
      const step = Math.max(1, Math.floor(n / 120));
      timer = setInterval(() => {
        if (idx + step >= n) { renderAt(n - 1); clearInterval(timer); timer = null; playBtn.textContent = 'Play'; }
        else renderAt(idx + step);
      }, 40);
    });
    renderAt(0);
  })(SIM_TRACE);
"##;

#[cfg(test)]
mod tests {
    use super::*;
    use sysml_ir::{
        ConnectionKind, ConnectionUsage, DirectedItem, Direction, FmeaEntry, Model, Package,
        PackageMember, PartDef, PartUsage, PortDef, PortUsage, RequirementDef, SafetyGoalMeta,
        SatisfyStmt, StateDef, StateUsage, Transition,
    };

    fn sample_model() -> Model {
        let pkg = Package {
            name: "DemoSystem".into(),
            members: vec![
                PackageMember::PortDef(PortDef {
                    name: "TempPort".into(),
                    items: vec![DirectedItem {
                        direction: Some(Direction::Out),
                        name: "reading".into(),
                        type_name: "Real".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                PackageMember::PartDef(PartDef {
                    name: "Sensor".into(),
                    ports: vec![PortUsage {
                        name: "temp_out".into(),
                        type_name: "TempPort".into(),
                        conjugated: false,
                        ..Default::default()
                    }],
                    fmea: vec![FmeaEntry {
                        failure_mode: "StuckHigh".into(),
                        effect: "FalseAlarm".into(),
                        severity: 7,
                        occurrence: 2,
                        detection: 3,
                        asil: Some("ASIL_B".into()),
                    }],
                    ..Default::default()
                }),
                PackageMember::PartDef(PartDef {
                    name: "Controller".into(),
                    ports: vec![PortUsage {
                        name: "temp_in".into(),
                        type_name: "TempPort".into(),
                        conjugated: true,
                        ..Default::default()
                    }],
                    satisfies: vec![SatisfyStmt {
                        requirement: "MaxTempReq".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                PackageMember::PartDef(PartDef {
                    name: "Monitor".into(),
                    parts: vec![
                        PartUsage {
                            name: Some("sensor".into()),
                            type_names: vec!["Sensor".into()],
                            ..Default::default()
                        },
                        PartUsage {
                            name: Some("ctrl".into()),
                            type_names: vec!["Controller".into()],
                            ..Default::default()
                        },
                    ],
                    connections: vec![ConnectionUsage {
                        kind: ConnectionKind::Interface,
                        name: Some("temp_link".into()),
                        from: Some("sensor.temp_out".into()),
                        to: Some("ctrl.temp_in".into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                PackageMember::StateDef(StateDef {
                    name: "Mode".into(),
                    states: vec![
                        StateUsage {
                            name: "Normal".into(),
                            ..Default::default()
                        },
                        StateUsage {
                            name: "Alarm".into(),
                            ..Default::default()
                        },
                    ],
                    transitions: vec![Transition {
                        source: "Normal".into(),
                        trigger: Some("OverTemp".into()),
                        guard: Some("temp > 85.0".into()),
                        target: "Alarm".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                }),
                PackageMember::RequirementDef(RequirementDef {
                    short_name: Some("REQ-001".into()),
                    name: "MaxTempReq".into(),
                    doc: Some("System shall alarm above 85 C".into()),
                    rationale: Some("Source: Spec §2.1".into()),
                    safety_goal: Some(SafetyGoalMeta {
                        asil: "ASIL_B".into(),
                        safe_state: "Alarm".into(),
                        ftti_ms: 100,
                    }),
                    require_constraints: vec!["temp <= 85.0".into()],
                    ..Default::default()
                }),
            ],
            ..Default::default()
        };
        Model::new(vec![pkg])
    }

    #[test]
    fn snapshot_maps_ports_connections_states_and_requirements() {
        let snap = build_viewer_snapshot(&sample_model(), ViewerView::Interconnection);
        assert_eq!(snap.command, "update");
        assert_eq!(snap.current_view, "ibd");
        assert_eq!(snap.elements.len(), 1);
        let pkg = &snap.elements[0];
        assert_eq!(pkg.name, "DemoSystem");

        // Verify port direction resolution (normal Out vs conjugated In)
        let sensor = pkg.children.iter().find(|c| c.name == "Sensor").unwrap();
        assert_eq!(
            sensor.children[0].attributes.get("direction").and_then(|v| v.as_str()),
            Some("out")
        );
        let ctrl = pkg.children.iter().find(|c| c.name == "Controller").unwrap();
        assert_eq!(
            ctrl.children[0].attributes.get("direction").and_then(|v| v.as_str()),
            Some("in")
        );

        // Verify connection, satisfy, and transition relationships
        assert!(snap
            .relationships
            .iter()
            .any(|r| r.kind == "interface" && r.source == "sensor.temp_out" && r.target == "ctrl.temp_in"));
        assert!(snap
            .relationships
            .iter()
            .any(|r| r.kind == "satisfy" && r.source == "Controller" && r.target == "MaxTempReq"));
        assert!(snap
            .relationships
            .iter()
            .any(|r| r.kind == "transition" && r.source == "__sysml_initial__" && r.target == "Normal"));
        assert!(snap.relationships.iter().any(|r| {
            r.kind == "transition"
                && r.source == "Normal"
                && r.target == "Alarm"
                && r.name.as_deref() == Some("OverTemp [temp > 85.0]")
        }));
    }

    #[cfg(feature = "viewer")]
    #[test]
    fn standalone_html_is_self_contained_and_escapes_script_tags() {
        let html = render_standalone_html(
            &sample_model(),
            &ViewerOptions {
                title: "Demo </script> Model".into(),
                initial_view: ViewerView::General,
                theme: ViewerTheme::Light,
                trace_json: None,
            },
        );
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("data-theme=\"light\""));
        assert!(html.contains("SysMLDiagramRuntime.mount"));
        assert!(html.contains("DemoSystem"));
        assert!(html.contains("sensor.temp_out"));
        assert!(!html.contains("Demo </script> Model"));
    }
}
