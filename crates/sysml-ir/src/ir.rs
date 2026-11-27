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
pub struct Comment {
    pub name: Option<String>,
    pub body: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub is_library: bool,
    pub doc: Option<String>,
    #[serde(default)]
    pub comments: Vec<Comment>,
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
    CalcDef(CalcDef),
    CalcUsage(CalcDef),
    AnalysisDef(AnalysisDef),
    AnalysisUsage(AnalysisUsage),
    AttributeUsage(AttributeUsage),
    ViewUsage(ViewUsage),
    MetadataDef(MetadataDef),
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MetadataDef {
    pub name: String,
    pub doc: Option<String>,
    pub attributes: Vec<AttributeUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyGoalMeta {
    pub asil: String,
    pub safe_state: String,
    pub ftti_ms: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FmeaEntry {
    pub failure_mode: String,
    pub effect: String,
    pub severity: u8,
    pub occurrence: u8,
    pub detection: u8,
    pub asil: Option<String>,
}

impl FmeaEntry {
    pub const fn rpn(&self) -> u16 {
        (self.severity as u16) * (self.occurrence as u16) * (self.detection as u16)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsilDecompositionMeta {
    pub goal: String,
    pub left_part: String,
    pub left_asil: String,
    pub right_part: String,
    pub right_asil: String,
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
    #[serde(default)]
    pub attributes: Vec<AttributeUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AttributeUsage {
    #[serde(default)]
    pub short_name: Option<String>,
    pub name: Option<String>,
    #[serde(default)]
    pub is_private: bool,
    #[serde(default)]
    pub is_nonunique: bool,
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
    /// True for keyword-less SysML v2 feature bindings (e.g. `:> influenceLevel = ...;`).
    #[serde(default)]
    pub is_keywordless: bool,
    #[serde(default)]
    pub nested_attributes: Vec<AttributeUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct VariantPart {
    pub name: String,
    pub type_name: String,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AllocationUsage {
    pub name: Option<String>,
    pub from: String,
    pub to: String,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PartDef {
    pub name: String,
    pub is_abstract: bool,
    pub is_individual: bool,
    #[serde(default)]
    pub is_variation: bool,
    pub doc: Option<String>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    pub specializes: Vec<String>,
    pub attributes: Vec<AttributeUsage>,
    pub parts: Vec<PartUsage>,
    #[serde(default)]
    pub variants: Vec<VariantPart>,
    pub ports: Vec<PortUsage>,
    pub items: Vec<DirectedItem>,
    pub performs: Vec<PerformAction>,
    pub connections: Vec<ConnectionUsage>,
    #[serde(default)]
    pub allocations: Vec<AllocationUsage>,
    pub exhibit_states: Vec<StateUsage>,
    pub requirements: Vec<RequirementUsage>,
    pub satisfies: Vec<SatisfyStmt>,
    pub verifies: Vec<VerifyStmt>,
    #[serde(default)]
    pub refinements: Vec<RefinementDependency>,
    #[serde(default)]
    pub fmea: Vec<FmeaEntry>,
    #[serde(default)]
    pub asil_decompositions: Vec<AsilDecompositionMeta>,
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
    #[serde(default)]
    pub items: Vec<DirectedItem>,
    pub performs: Vec<PerformAction>,
    pub connections: Vec<ConnectionUsage>,
    #[serde(default)]
    pub requirements: Vec<RequirementUsage>,
    #[serde(default)]
    pub portions: Vec<PortionUsage>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortionKind {
    #[default]
    Timeslice,
    Snapshot,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PortionUsage {
    pub kind: PortionKind,
    pub is_then: bool,
    pub name: String,
    pub type_names: Vec<String>,
    pub subsets: Vec<String>,
    pub doc: Option<String>,
    pub assert_constraints: Vec<String>,
    pub attributes: Vec<AttributeUsage>,
    pub parts: Vec<PartUsage>,
    pub performs: Vec<PerformAction>,
    pub portions: Vec<PortionUsage>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ViewUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PortDef {
    pub name: String,
    #[serde(default)]
    pub is_abstract: bool,
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
    #[serde(default)]
    pub redefines: Vec<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DirectedItem {
    pub direction: Option<Direction>,
    pub name: String,
    pub type_name: String,
    pub multiplicity: Option<Multiplicity>,
    #[serde(default)]
    pub subsets: Vec<String>,
    /// Inherited item features this one redefines (`:>>`), e.g. when a port specializing a
    /// library port narrows the type of an inherited `out item`.
    #[serde(default)]
    pub redefines: Vec<String>,
    #[serde(default)]
    pub doc: Option<String>,
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
    #[serde(default)]
    pub is_abstract: bool,
    pub doc: Option<String>,
    pub specializes: Vec<String>,
    #[serde(default)]
    pub attributes: Vec<AttributeUsage>,
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
    #[serde(default)]
    pub subsets: Vec<String>,
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
    #[serde(default)]
    pub trigger_payload_name: Option<String>,
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
    #[serde(default)]
    pub subject_type: Option<String>,
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
    #[serde(default)]
    pub safety_goal: Option<SafetyGoalMeta>,
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

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CalcParam {
    pub name: String,
    pub type_name: String,
    #[serde(default)]
    pub subsets: Option<String>,
    #[serde(default)]
    pub multiplicity: Option<Multiplicity>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CalcLocal {
    pub name: String,
    pub expr: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CalcDef {
    #[serde(default)]
    pub short_name: Option<String>,
    pub name: String,
    pub doc: Option<String>,
    pub params: Vec<CalcParam>,
    pub locals: Vec<CalcLocal>,
    pub return_name: String,
    pub return_type: String,
    #[serde(default)]
    pub return_subsets: Option<String>,
    pub result_expr: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnalysisParam {
    pub direction: Option<Direction>,
    pub name: String,
    pub type_name: Option<String>,
    #[serde(default)]
    pub subsets: Vec<String>,
    #[serde(default)]
    pub multiplicity: Option<Multiplicity>,
    pub value: Option<String>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnalysisDef {
    pub name: String,
    pub doc: Option<String>,
    pub specializes: Vec<String>,
    pub subject: Option<SubjectDecl>,
    pub params: Vec<AnalysisParam>,
    pub assert_constraints: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AnalysisUsage {
    pub name: String,
    pub type_name: Option<String>,
    pub doc: Option<String>,
    pub subject: Option<SubjectDecl>,
    pub params: Vec<AnalysisParam>,
    pub assert_constraints: Vec<String>,
}
