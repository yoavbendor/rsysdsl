//! Minimal, shallow-first SW/HW execution modeling concepts that lower cleanly to
//! 100% standard OMG SysML v2 (`part def`, `part`, `attribute`, `port`, `connection`,
//! `allocation`, `requirement def`, `satisfy`, `state`).
//!
//! Design principle: plain Rust structs, enums, and builders — NO heavy proc-macros
//! or cross-file trait-bound gymnastics. All SW/HW timing, ASIL, and allocation fields
//! are `Option<...>` so teams can start with a shallow system model on Day 1 and add
//! execution details incrementally only where needed.

use serde::{Deserialize, Serialize};

use crate::ir::{
    AllocationUsage, AttributeUsage, ConnectionKind, ConnectionUsage, DirectedItem, Direction,
    Import, ItemDef, Model, Package, PackageMember, PartDef, PartUsage, PortDef, PortUsage,
    RequirementDef, SatisfyStmt, StateDef, StateUsage,
};

/// ISO 26262 Automotive Safety Integrity Level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Asil {
    QM,
    A,
    B,
    C,
    D,
}

impl Asil {
    pub fn as_str(self) -> &'static str {
        match self {
            Asil::QM => "QM",
            Asil::A => "ASIL_A",
            Asil::B => "ASIL_B",
            Asil::C => "ASIL_C",
            Asil::D => "ASIL_D",
        }
    }

    pub fn is_safety_critical(self) -> bool {
        !matches!(self, Asil::QM)
    }

    pub fn is_high_integrity(self) -> bool {
        matches!(self, Asil::C | Asil::D)
    }
}

/// Fault modes modeled on a communication bus/channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BusFaultMode {
    Drop,
    Delay,
    Corrupt,
}

impl BusFaultMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            BusFaultMode::Drop => "Drop",
            BusFaultMode::Delay => "Delay",
            BusFaultMode::Corrupt => "Corrupt",
        }
    }
}

/// Direction of a software data/signal port.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SwPortDirection {
    In,
    Out,
}

/// A typed data/signal port on a [`SwPartition`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwPort {
    pub name: String,
    pub direction: SwPortDirection,
    pub payload_type: String,
}

impl SwPort {
    pub fn input(name: impl Into<String>, payload_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            direction: SwPortDirection::In,
            payload_type: payload_type.into(),
        }
    }

    pub fn output(name: impl Into<String>, payload_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            direction: SwPortDirection::Out,
            payload_type: payload_type.into(),
        }
    }
}

/// 1. `Task` / `Thread` inside a [`SwPartition`], with optional real-time timing attributes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwTask {
    pub name: String,
    pub period_ms: Option<f64>,
    pub deadline_ms: Option<f64>,
    pub wcet_ms: Option<f64>,
}

impl SwTask {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            period_ms: None,
            deadline_ms: None,
            wcet_ms: None,
        }
    }

    pub fn period_ms(mut self, v: f64) -> Self {
        self.period_ms = Some(v);
        self
    }

    pub fn deadline_ms(mut self, v: f64) -> Self {
        self.deadline_ms = Some(v);
        self
    }

    pub fn wcet_ms(mut self, v: f64) -> Self {
        self.wcet_ms = Some(v);
        self
    }
}

/// 2. `SwPartition` / `Process` with optional ASIL, AI component marker, provenance,
///    safety guardrail / fallback state, tasks, and ports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwPartition {
    pub name: String,
    pub asil: Option<Asil>,
    pub is_ai_component: bool,
    /// Marks this partition as an explicit safety monitor / guardrail (e.g. RSS/plausibility checker)
    /// that can safely mediate inputs from a QM or AI component into high-ASIL consumers.
    pub is_safety_guardrail: bool,
    /// Optional fallback / safe state name (e.g. `"SafeStop"`).
    pub fallback_state: Option<String>,
    /// Optional provenance pointers (e.g. `"Safety Spec §4.2"`, `"src/brake_ctrl.cpp:142"`).
    pub provenance: Vec<String>,
    pub tasks: Vec<SwTask>,
    pub ports: Vec<SwPort>,
    /// Requirement IDs satisfied by this partition.
    pub satisfies: Vec<String>,
}

impl SwPartition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            asil: None,
            is_ai_component: false,
            is_safety_guardrail: false,
            fallback_state: None,
            provenance: Vec::new(),
            tasks: Vec::new(),
            ports: Vec::new(),
            satisfies: Vec::new(),
        }
    }

    pub fn asil(mut self, asil: Asil) -> Self {
        self.asil = Some(asil);
        self
    }

    pub fn ai_component(mut self, is_ai: bool) -> Self {
        self.is_ai_component = is_ai;
        self
    }

    pub fn safety_guardrail(mut self, is_guardrail: bool) -> Self {
        self.is_safety_guardrail = is_guardrail;
        self
    }

    pub fn fallback_state(mut self, state: impl Into<String>) -> Self {
        self.fallback_state = Some(state.into());
        self
    }

    pub fn provenance(mut self, source: impl Into<String>) -> Self {
        self.provenance.push(source.into());
        self
    }

    pub fn task(mut self, task: SwTask) -> Self {
        self.tasks.push(task);
        self
    }

    pub fn port(mut self, port: SwPort) -> Self {
        self.ports.push(port);
        self
    }

    pub fn satisfies(mut self, req_id: impl Into<String>) -> Self {
        self.satisfies.push(req_id.into());
        self
    }
}

/// 3. `ComputeNode` (ECU / SoC / core) with optional cores, scheduling policy, and utilization cap.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComputeNode {
    pub name: String,
    pub cores: Option<u32>,
    pub scheduling_policy: Option<String>,
    pub max_utilization: Option<f64>,
}

impl ComputeNode {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            cores: None,
            scheduling_policy: None,
            max_utilization: None,
        }
    }

    pub fn cores(mut self, cores: u32) -> Self {
        self.cores = Some(cores);
        self
    }

    pub fn scheduling_policy(mut self, policy: impl Into<String>) -> Self {
        self.scheduling_policy = Some(policy.into());
        self
    }

    pub fn max_utilization(mut self, u: f64) -> Self {
        self.max_utilization = Some(u);
        self
    }
}

/// 4. `Bus` / `Channel` (e.g., CAN-FD, Automotive Ethernet) with optional latency and fault modes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusChannel {
    pub name: String,
    pub latency_ms: Option<f64>,
    pub fault_modes: Vec<BusFaultMode>,
}

impl BusChannel {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            latency_ms: None,
            fault_modes: Vec::new(),
        }
    }

    pub fn latency_ms(mut self, ms: f64) -> Self {
        self.latency_ms = Some(ms);
        self
    }

    pub fn fault_mode(mut self, mode: BusFaultMode) -> Self {
        if !self.fault_modes.contains(&mode) {
            self.fault_modes.push(mode);
        }
        self
    }
}

/// A directed data/signal connection between two [`SwPartition`] ports, optionally bound to a [`BusChannel`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwConnection {
    pub name: String,
    pub from_partition: String,
    pub from_port: String,
    pub to_partition: String,
    pub to_port: String,
    pub bus: Option<String>,
}

impl SwConnection {
    pub fn new(
        name: impl Into<String>,
        from_partition: impl Into<String>,
        from_port: impl Into<String>,
        to_partition: impl Into<String>,
        to_port: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            from_partition: from_partition.into(),
            from_port: from_port.into(),
            to_partition: to_partition.into(),
            to_port: to_port.into(),
            bus: None,
        }
    }

    pub fn over_bus(mut self, bus: impl Into<String>) -> Self {
        self.bus = Some(bus.into());
        self
    }
}

/// 5. `Allocation`: binds a `SwPartition` or `Task` to a `ComputeNode` (or a `Connection` to a `Bus`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwAllocation {
    pub name: String,
    pub source: String,
    pub target: String,
}

impl SwAllocation {
    pub fn new(
        name: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            target: target.into(),
        }
    }
}

/// A lightweight requirement declaration inside a [`SwHwSystem`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwRequirement {
    pub id: String,
    pub name: String,
    pub text: String,
    pub verified_by: Vec<String>,
}

impl SwRequirement {
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            text: text.into(),
            verified_by: Vec::new(),
        }
    }

    pub fn verified_by(mut self, target: impl Into<String>) -> Self {
        self.verified_by.push(target.into());
        self
    }
}

/// Top-level shallow-first SW/HW execution architecture container.
///
/// Lowers via [`SwHwSystem::to_sysml_model`] to 100% standard OMG SysML v2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwHwSystem {
    pub package_name: String,
    pub system_name: String,
    pub partitions: Vec<SwPartition>,
    pub compute_nodes: Vec<ComputeNode>,
    pub buses: Vec<BusChannel>,
    pub connections: Vec<SwConnection>,
    pub allocations: Vec<SwAllocation>,
    pub requirements: Vec<SwRequirement>,
}

impl SwHwSystem {
    pub fn new(package_name: impl Into<String>, system_name: impl Into<String>) -> Self {
        Self {
            package_name: package_name.into(),
            system_name: system_name.into(),
            partitions: Vec::new(),
            compute_nodes: Vec::new(),
            buses: Vec::new(),
            connections: Vec::new(),
            allocations: Vec::new(),
            requirements: Vec::new(),
        }
    }

    pub fn partition(mut self, p: SwPartition) -> Self {
        self.partitions.push(p);
        self
    }

    pub fn compute_node(mut self, n: ComputeNode) -> Self {
        self.compute_nodes.push(n);
        self
    }

    pub fn bus(mut self, b: BusChannel) -> Self {
        self.buses.push(b);
        self
    }

    pub fn connection(mut self, c: SwConnection) -> Self {
        self.connections.push(c);
        self
    }

    pub fn allocation(mut self, a: SwAllocation) -> Self {
        self.allocations.push(a);
        self
    }

    pub fn requirement(mut self, r: SwRequirement) -> Self {
        self.requirements.push(r);
        self
    }

    /// Lower this SW/HW architecture into a standard OMG SysML v2 [`Model`].
    ///
    /// Uses only standard SysML v2 elements (`item def`, `port def`, `part def`,
    /// `attribute`, `part`, `connection`, `allocation`, `requirement def`, `satisfy`,
    /// `state def`).
    pub fn to_sysml_model(&self) -> Model {
        let mut pkg = Package {
            name: self.package_name.clone(),
            ..Default::default()
        };
        pkg.imports.push(Import::private_star("ScalarValues"));

        // 1. Collect unique payload item defs + port defs
        let mut payload_types: Vec<String> = Vec::new();
        for p in &self.partitions {
            for port in &p.ports {
                if !payload_types.contains(&port.payload_type) {
                    payload_types.push(port.payload_type.clone());
                }
            }
        }
        for pt in &payload_types {
            pkg.members.push(PackageMember::ItemDef(ItemDef {
                name: pt.clone(),
                ..Default::default()
            }));
            let mut port_def = PortDef {
                name: format!("{pt}Port"),
                ..Default::default()
            };
            port_def.items.push(DirectedItem {
                direction: Some(Direction::Out),
                name: "payload".to_string(),
                type_name: pt.clone(),
                ..Default::default()
            });
            pkg.members.push(PackageMember::PortDef(port_def));
        }

        // 2. Requirements
        let mut req_id_to_name = std::collections::BTreeMap::new();
        for r in &self.requirements {
            req_id_to_name.insert(r.id.clone(), r.name.clone());
            pkg.members
                .push(PackageMember::RequirementDef(RequirementDef {
                    name: r.name.clone(),
                    short_name: Some(r.id.clone()),
                    doc: Some(r.text.clone()),
                    ..Default::default()
                }));
        }

        // 3. Fallback state defs (only when declared)
        for p in &self.partitions {
            if let Some(fb) = &p.fallback_state {
                let mut sdef = StateDef {
                    name: format!("{}Mode", p.name),
                    initial_state: Some("Nominal".to_string()),
                    ..Default::default()
                };
                sdef.states.push(StateUsage {
                    name: "Nominal".to_string(),
                    ..Default::default()
                });
                sdef.states.push(StateUsage {
                    name: fb.clone(),
                    ..Default::default()
                });
                pkg.members.push(PackageMember::StateDef(sdef));
            }
        }

        // 4. Task & Partition PartDefs
        for p in &self.partitions {
            for t in &p.tasks {
                let mut tdef = PartDef {
                    name: format!("{}_{}", p.name, t.name),
                    ..Default::default()
                };
                if let Some(v) = t.period_ms {
                    tdef.attributes.push(real_attr("periodMs", v));
                }
                if let Some(v) = t.deadline_ms {
                    tdef.attributes.push(real_attr("deadlineMs", v));
                }
                if let Some(v) = t.wcet_ms {
                    tdef.attributes.push(real_attr("wcetMs", v));
                }
                pkg.members.push(PackageMember::PartDef(tdef));
            }

            let mut pdef = PartDef {
                name: p.name.clone(),
                ..Default::default()
            };
            if !p.provenance.is_empty() {
                pdef.doc = Some(format!("Provenance: {}", p.provenance.join("; ")));
            }
            if let Some(asil) = p.asil {
                pdef.attributes.push(string_attr("asil", asil.as_str()));
            }
            if p.is_ai_component {
                pdef.attributes.push(bool_attr("isAiComponent", true));
            }
            if p.is_safety_guardrail {
                pdef.attributes.push(bool_attr("isSafetyGuardrail", true));
            }
            if let Some(fb) = &p.fallback_state {
                pdef.attributes.push(string_attr("fallbackState", fb));
            }
            for port in &p.ports {
                pdef.ports.push(PortUsage {
                    name: port.name.clone(),
                    type_name: format!("{}Port", port.payload_type),
                    conjugated: port.direction == SwPortDirection::In,
                    ..Default::default()
                });
            }
            for t in &p.tasks {
                pdef.parts.push(PartUsage {
                    name: Some(to_lower_camel(&t.name)),
                    type_names: vec![format!("{}_{}", p.name, t.name)],
                    ..Default::default()
                });
            }
            for req_id in &p.satisfies {
                if let Some(req_name) = req_id_to_name.get(req_id) {
                    pdef.satisfies.push(SatisfyStmt {
                        is_typed: true,
                        requirement: req_name.clone(),
                        ..Default::default()
                    });
                }
            }
            pkg.members.push(PackageMember::PartDef(pdef));
        }

        // 5. ComputeNode PartDefs
        for n in &self.compute_nodes {
            let mut ndef = PartDef {
                name: n.name.clone(),
                ..Default::default()
            };
            if let Some(cores) = n.cores {
                ndef.attributes.push(int_attr("cores", cores as i64));
            }
            if let Some(pol) = &n.scheduling_policy {
                ndef.attributes.push(string_attr("schedulingPolicy", pol));
            }
            if let Some(u) = n.max_utilization {
                ndef.attributes.push(real_attr("maxUtilization", u));
            }
            pkg.members.push(PackageMember::PartDef(ndef));
        }

        // 6. BusChannel PartDefs
        for b in &self.buses {
            let mut bdef = PartDef {
                name: b.name.clone(),
                ..Default::default()
            };
            if let Some(lat) = b.latency_ms {
                bdef.attributes.push(real_attr("latencyMs", lat));
            }
            if !b.fault_modes.is_empty() {
                let modes = b
                    .fault_modes
                    .iter()
                    .map(|m| m.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                bdef.attributes.push(string_attr("faultModes", &modes));
            }
            pkg.members.push(PackageMember::PartDef(bdef));
        }

        // 7. Top-level System Assembly PartDef wiring parts, connections, and allocations
        let mut sys = PartDef {
            name: self.system_name.clone(),
            ..Default::default()
        };
        let mut part_field_names = std::collections::BTreeMap::new();
        for p in &self.partitions {
            let field = to_lower_camel(&p.name);
            part_field_names.insert(p.name.clone(), field.clone());
            sys.parts.push(PartUsage {
                name: Some(field),
                type_names: vec![p.name.clone()],
                ..Default::default()
            });
        }
        for n in &self.compute_nodes {
            let field = to_lower_camel(&n.name);
            part_field_names.insert(n.name.clone(), field.clone());
            sys.parts.push(PartUsage {
                name: Some(field),
                type_names: vec![n.name.clone()],
                ..Default::default()
            });
        }
        for b in &self.buses {
            let field = to_lower_camel(&b.name);
            part_field_names.insert(b.name.clone(), field.clone());
            sys.parts.push(PartUsage {
                name: Some(field),
                type_names: vec![b.name.clone()],
                ..Default::default()
            });
        }

        for c in &self.connections {
            if let (Some(src_f), Some(dst_f)) = (
                part_field_names.get(&c.from_partition),
                part_field_names.get(&c.to_partition),
            ) {
                sys.connections.push(ConnectionUsage {
                    name: Some(c.name.clone()),
                    kind: ConnectionKind::Connection,
                    from: Some(format!("{src_f}.{}", c.from_port)),
                    to: Some(format!("{dst_f}.{}", c.to_port)),
                    ..Default::default()
                });
            }
        }

        for a in &self.allocations {
            if let (Some(src_path), Some(dst_path)) = (
                resolve_alloc_path(&a.source, &part_field_names),
                resolve_alloc_path(&a.target, &part_field_names),
            ) {
                sys.allocations.push(AllocationUsage {
                    name: Some(a.name.clone()),
                    from: src_path,
                    to: dst_path,
                    doc: None,
                });
            }
        }

        pkg.members.push(PackageMember::PartDef(sys));
        Model::new(vec![pkg])
    }
}

fn resolve_alloc_path(
    name: &str,
    part_fields: &std::collections::BTreeMap<String, String>,
) -> Option<String> {
    if let Some(f) = part_fields.get(name) {
        return Some(f.clone());
    }
    if let Some((part, task)) = name.split_once('.') {
        if let Some(f) = part_fields.get(part) {
            return Some(format!("{f}.{}", to_lower_camel(task)));
        }
    }
    None
}

fn to_lower_camel(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

fn real_attr(name: &str, v: f64) -> AttributeUsage {
    AttributeUsage {
        name: Some(name.to_string()),
        type_name: Some("Real".to_string()),
        value: Some(format!("{v}")),
        ..Default::default()
    }
}

fn int_attr(name: &str, v: i64) -> AttributeUsage {
    AttributeUsage {
        name: Some(name.to_string()),
        type_name: Some("Integer".to_string()),
        value: Some(format!("{v}")),
        ..Default::default()
    }
}

fn bool_attr(name: &str, v: bool) -> AttributeUsage {
    AttributeUsage {
        name: Some(name.to_string()),
        type_name: Some("Boolean".to_string()),
        value: Some(if v { "true" } else { "false" }.to_string()),
        ..Default::default()
    }
}

fn string_attr(name: &str, v: &str) -> AttributeUsage {
    AttributeUsage {
        name: Some(name.to_string()),
        type_name: Some("String".to_string()),
        value: Some(format!("\"{}\"", v.replace('"', "\\\""))),
        ..Default::default()
    }
}
