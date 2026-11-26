//! Structured SysML v2 Intermediate Representation (IR).
//!
//! Designed for 1:1 deterministic emission (`sysml-emit`) and lossless semantic
//! lifting (`sysml-lift`) over the CoSMA + safety-critical SysML v2 subset.

use serde::{Deserialize, Serialize};

/// A collection of SysML v2 packages forming a model unit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub packages: Vec<Package>,
}

impl Model {
    pub fn new(packages: Vec<Package>) -> Self {
        Self { packages }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    #[default]
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportWildcard {
    #[default]
    None,
    /// `::*`
    Star,
    /// `::**`
    RecursiveStar,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Import {
    pub visibility: Visibility,
    pub target: String,
    pub wildcard: ImportWildcard,
}

impl Import {
    pub fn private_star(target: impl Into<String>) -> Self {
        Self {
            visibility: Visibility::Private,
            target: target.into(),
            wildcard: ImportWildcard::Star,
        }
    }

    pub fn private_recursive(target: impl Into<String>) -> Self {
        Self {
            visibility: Visibility::Private,
            target: target.into(),
            wildcard: ImportWildcard::RecursiveStar,
        }
    }

    pub fn private_item(target: impl Into<String>) -> Self {
        Self {
            visibility: Visibility::Private,
            target: target.into(),
            wildcard: ImportWildcard::None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub is_library: bool,
    pub doc: Option<String>,
    pub imports: Vec<Import>,
    pub members: Vec<PackageMember>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PackageMember {
    Package(Package),
    PartDef(PartDef),
    PartUsage(PartUsage),
    PortDef(PortDef),
    InterfaceDef(InterfaceDef),
    ConnectionDef(ConnectionDef),
    ItemDef(ItemDef),
    EnumDef(EnumDef),
    AttributeDef(AttributeDef),
    ActionDef(ActionDef),
    StateDef(StateDef),
    RequirementDef(RequirementDef),
    RequirementUsage(RequirementUsage),
    VerificationDef(VerificationDef),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    In,
    Out,
    InOut,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueKind {
    #[default]
    Bound,
    Initial,
    Default,
    DefaultInitial,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Multiplicity {
    pub lower: Option<String>,
    pub upper: String,
}

impl Multiplicity {
    pub fn new(lower: Option<impl Into<String>>, upper: impl Into<String>) -> Self {
        Self {
            lower: lower.map(Into::into),
            upper: upper.into(),
        }
    }

    pub fn render(&self) -> String {
        match &self.lower {
            Some(l) => format!("[{}..{}]", l, self.upper),
            None => format!("[{}]", self.upper),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AttributeDef {
    pub name: String,
    pub is_abstract: bool,
    pub doc: Option<String>,
    pub specializes: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AttributeUsage {
    pub name: Option<String>,
    pub type_name: Option<String>,
    pub subsets: Vec<String>,
    pub redefines: Vec<String>,
    pub multiplicity: Option<Multiplicity>,
    pub value_kind: ValueKind,
    /// Raw SysML v2 expression or scalar literal (without trailing unit brackets if `unit` is set).
    pub value: Option<String>,
    /// Optional physical unit symbol (e.g. `"kg"`, `"m/s"`, `"kN"`, `"%"`).
    pub unit: Option<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PartDef {
    pub name: String,
    pub is_abstract: bool,
    pub is_individual: bool,
    pub doc: Option<String>,
    pub specializes: Vec<String>,
    pub attributes: Vec<AttributeUsage>,
    pub parts: Vec<PartUsage>,
    pub ports: Vec<PortUsage>,
    pub items: Vec<DirectedItem>,
    pub performs: Vec<PerformAction>,
    pub connections: Vec<ConnectionUsage>,
    pub exhibit_states: Vec<StateUsage>,
    pub requirements: Vec<RequirementUsage>,
    pub satisfies: Vec<SatisfyStmt>,
    pub verifies: Vec<VerifyStmt>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PartUsage {
    pub name: Option<String>,
    pub is_individual: bool,
    pub is_ref: bool,
    pub type_names: Vec<String>,
    pub subsets: Vec<String>,
    pub redefines: Vec<String>,
    pub multiplicity: Option<Multiplicity>,
    pub is_ordered: bool,
    pub default_value: Option<String>,
    pub doc: Option<String>,
    pub attributes: Vec<AttributeUsage>,
    pub nested_parts: Vec<PartUsage>,
    pub ports: Vec<PortUsage>,
    pub performs: Vec<PerformAction>,
    pub connections: Vec<ConnectionUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PortDef {
    pub name: String,
    pub doc: Option<String>,
    pub specializes: Vec<String>,
    pub is_symmetric: bool,
    pub items: Vec<DirectedItem>,
    pub attributes: Vec<AttributeUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PortUsage {
    pub name: String,
    pub type_name: String,
    pub conjugated: bool,
    pub multiplicity: Option<Multiplicity>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DirectedItem {
    pub direction: Option<Direction>,
    pub name: String,
    pub type_name: String,
    pub multiplicity: Option<Multiplicity>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ItemDef {
    pub name: String,
    pub is_abstract: bool,
    pub doc: Option<String>,
    pub specializes: Vec<String>,
    pub attributes: Vec<AttributeUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EnumDef {
    pub name: String,
    pub doc: Option<String>,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InterfaceEnd {
    pub name: String,
    pub type_name: String,
    pub conjugated: bool,
    pub multiplicity: Option<Multiplicity>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InterfaceDef {
    pub name: String,
    pub doc: Option<String>,
    pub specializes: Vec<String>,
    pub ends: Vec<InterfaceEnd>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConnectionEndDecl {
    pub is_ref: bool,
    pub kind_keyword: String,
    pub name: String,
    pub type_name: String,
    pub multiplicity: Option<Multiplicity>,
    pub subsets: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConnectionDef {
    pub name: String,
    pub is_abstract: bool,
    pub doc: Option<String>,
    pub specializes: Vec<String>,
    pub ends: Vec<ConnectionEndDecl>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    #[default]
    Connection,
    Interface,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ConnectionUsage {
    pub kind: ConnectionKind,
    pub is_abstract: bool,
    pub name: Option<String>,
    pub type_name: Option<String>,
    pub multiplicity: Option<Multiplicity>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RefinementDependency {
    pub client: String,
    pub supplier: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionParam {
    pub direction: Direction,
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionStep {
    pub name: String,
    pub type_name: Option<String>,
    pub is_then: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub multiplicity: Option<Multiplicity>,
    pub redefines: Vec<String>,
    pub has_initial_start: bool,
    pub steps: Vec<ActionStep>,
    pub has_done: bool,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActionDef {
    pub name: String,
    pub is_abstract: bool,
    pub doc: Option<String>,
    pub specializes: Vec<String>,
    pub params: Vec<ActionParam>,
    pub subactions: Vec<ActionUsage>,
    pub has_initial_start: bool,
    pub steps: Vec<ActionStep>,
    pub has_done: bool,
    pub refinements: Vec<RefinementDependency>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PerformAction {
    pub name: Option<String>,
    pub action_type: Option<String>,
    pub references: Option<String>,
    pub redefines: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    pub name: Option<String>,
    pub source: String,
    pub trigger: Option<String>,
    pub guard: Option<String>,
    pub effect: Option<String>,
    pub target: String,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StateUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub is_exhibit: bool,
    pub doc: Option<String>,
    pub entry_action: Option<ActionUsage>,
    pub do_action: Option<ActionUsage>,
    pub exit_action: Option<ActionUsage>,
    pub states: Vec<StateUsage>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StateDef {
    pub name: String,
    pub is_abstract: bool,
    pub doc: Option<String>,
    pub specializes: Vec<String>,
    pub entry_action: Option<ActionUsage>,
    pub do_action: Option<ActionUsage>,
    pub exit_action: Option<ActionUsage>,
    pub states: Vec<StateUsage>,
    pub transitions: Vec<Transition>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SubjectDecl {
    pub name: String,
    pub type_name: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RequirementDef {
    pub short_name: Option<String>,
    pub name: String,
    pub is_abstract: bool,
    pub doc: Option<String>,
    pub rationale: Option<String>,
    pub specializes: Vec<String>,
    pub subject: Option<SubjectDecl>,
    pub attributes: Vec<AttributeUsage>,
    pub require_constraints: Vec<String>,
    pub assume_constraints: Vec<String>,
    pub refinements: Vec<RefinementDependency>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RequirementUsage {
    pub short_name: Option<String>,
    pub name: String,
    pub type_name: Option<String>,
    pub multiplicity: Option<Multiplicity>,
    pub subsets: Vec<String>,
    pub doc: Option<String>,
    pub subject: Option<SubjectDecl>,
    pub nested_requirements: Vec<RequirementUsage>,
    pub satisfies: Vec<SatisfyStmt>,
    pub verifies: Vec<VerifyStmt>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SatisfyStmt {
    pub asserted: bool,
    #[serde(default)]
    pub is_typed: bool,
    pub requirement: String,
    pub by: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VerifyStmt {
    pub requirement: String,
    pub by: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VerificationDef {
    pub name: String,
    pub doc: Option<String>,
    pub subject: Option<SubjectDecl>,
    pub verifies: Vec<VerifyStmt>,
}
