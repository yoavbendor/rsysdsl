//! `budget` — cross-domain static budget rollup, DAG critical-path latency (including LLM
//! "thinking" time), pipeline FPS bottleneck, and top-3 hot-spot attribution over `sysml_ir::Model`.
//!
//! Contributions come only from budget roles (C1 / D1, see [`crate::roles`]): never from
//! attribute names. Attributes whose dimension matches a domain but carry no role are reported
//! as `untracked`, and a limited domain with untracked candidates is `incomplete` (D2), never a
//! silent pass.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sysml_ir::{AttributeUsage, BudgetRole, Model, Package, PackageMember, PartDef, PartUsage};

use crate::emit_cli::is_sysml_std_package;

use crate::roles::{
    concrete_user_parts, instance_count, merge_part_usages, part_key, AttrValue, DimClass,
    EffectiveAttr, RoleIndex,
};

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

/// Verdict of one budget domain (D2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetStatus {
    /// Within the limit, every candidate attribute has a role, at least one contributor
    /// (also reported for a domain without a limit: nothing to check).
    Pass,
    /// The tracked total already exceeds the limit.
    Fail,
    /// A limit is set but the total cannot be trusted: untracked candidates, role-tagged
    /// values that cannot be summed (`unresolved`), or zero contributors.
    Incomplete,
}

impl BudgetStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            BudgetStatus::Pass => "pass",
            BudgetStatus::Fail => "fail",
            BudgetStatus::Incomplete => "incomplete",
        }
    }
}

/// An attribute the rollup could not use (see [`DomainRollup::untracked`] / `unresolved`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UntrackedAttr {
    pub element: String,
    pub attribute: String,
    pub unit: String,
    pub value: Option<f64>,
}

/// Additive rollup for one physical/economic domain (mass, power, waste heat, cost).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainRollup {
    pub domain: String,
    pub unit: String,
    pub total: f64,
    pub limit: Option<f64>,
    pub margin: Option<f64>,
    /// `status == "pass"` (kept for compatibility).
    pub pass: bool,
    pub status: BudgetStatus,
    /// Number of attribute values summed into `total`.
    pub contributors: usize,
    pub hot_spots: Vec<HotSpot>,
    /// Attributes whose dimension matches this domain but that carry no budget role
    /// (e.g. an untagged `ratedPowerW [W]`): not summed, and they make a limited domain
    /// `incomplete`. Tag them (`budget = "..."`) or opt them out (`budget = "none"`).
    pub untracked: Vec<UntrackedAttr>,
    /// Role-tagged attributes whose value cannot be summed: `tbd_value`, a non-literal
    /// expression, or a unit whose scale is unknown. They make a limited domain `incomplete`.
    pub unresolved: Vec<UntrackedAttr>,
    /// Concrete part definitions of the model with role values in this domain that are never
    /// instantiated from a top-level context part (C1.6): not summed, and they make a limited
    /// domain `incomplete`. `value` is in the domain unit (`null` when it cannot be used).
    pub uninstantiated: Vec<UntrackedAttr>,
    /// Why a limited domain is (or would be) `incomplete`, in words.
    pub reasons: Vec<String>,
}

/// How the additive domains were rolled up (C1.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollupMode {
    /// Over part usages from the top-level context part(s): one count per instance.
    UsageTree,
    /// No top-level context part was found: one count per definition (domains `incomplete`).
    PerDefinition,
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
    /// Verdict over `--max-latency-ms` / `--min-fps` (same rules as [`DomainRollup::status`]).
    pub status: BudgetStatus,
    /// Time-dimension attributes (`s`) of parts without a role.
    pub untracked: Vec<UntrackedAttr>,
    /// Latency-role attributes whose value cannot be used.
    pub unresolved: Vec<UntrackedAttr>,
}

/// Complete cross-domain budget report produced by [`analyze_budgets`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetReport {
    /// `false` unless every domain with a limit is `pass`.
    pub ok: bool,
    /// `usage_tree` (normal) or `per_definition` (fallback without a top-level context part).
    pub rollup: RollupMode,
    /// The top-level context parts the usage tree starts from (definition names, or names of
    /// package-level part usages).
    pub roots: Vec<String>,
    pub mass: DomainRollup,
    pub power: DomainRollup,
    pub waste_heat: DomainRollup,
    pub cost: DomainRollup,
    pub timing: TimingBudget,
}

#[derive(Default)]
struct DomainAcc {
    /// (element, summed value, instance count)
    items: Vec<(String, f64, usize)>,
    untracked: Vec<UntrackedAttr>,
    unresolved: Vec<UntrackedAttr>,
    uninstantiated: Vec<UntrackedAttr>,
    /// Model-level reasons that make a limited domain `incomplete`.
    reasons: Vec<String>,
}

impl DomainAcc {
    fn push_unique(list: &mut Vec<UntrackedAttr>, e: UntrackedAttr) {
        if !list.iter().any(|x| x.element == e.element && x.attribute == e.attribute) {
            list.push(e);
        }
    }
}

fn entry(element: &str, a: &EffectiveAttr) -> UntrackedAttr {
    UntrackedAttr {
        element: element.to_string(),
        attribute: a.key.clone(),
        unit: a.unit.clone().unwrap_or_default(),
        value: a.number(),
    }
}

/// The additive domains of the usage-tree rollup (timing is a critical path, not a sum).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dom {
    Mass = 0,
    Power = 1,
    Heat = 2,
    Cost = 3,
}

const DOMS: [Dom; 4] = [Dom::Mass, Dom::Power, Dom::Heat, Dom::Cost];

fn dom_unit(d: Dom) -> &'static str {
    match d {
        Dom::Mass => "kg",
        Dom::Power | Dom::Heat => "W",
        Dom::Cost => "USD",
    }
}

/// What one instance of a definition contributes by itself (children excluded).
#[derive(Default)]
struct PartItems {
    items: Vec<(Dom, f64)>,
    unresolved: Vec<(Dom, UntrackedAttr)>,
    untracked: Vec<(Dom, UntrackedAttr)>,
    /// Latency-role sum in `s` (per definition; the critical path is not multiplied).
    latency: Option<f64>,
    time_unresolved: Vec<UntrackedAttr>,
    time_untracked: Vec<UntrackedAttr>,
}

/// Role contributions of one definition (or usage) from its effective attributes.
fn part_items(name: &str, attrs: &[EffectiveAttr]) -> PartItems {
    let mut out = PartItems::default();
    let mut part_power = 0.0;
    let mut part_heat: Option<f64> = None;
    let mut part_eff: Option<f64> = None;
    for a in attrs {
        if a.is_evidence {
            continue;
        }
        match a.role {
            Some(BudgetRole::None) => {}
            Some(BudgetRole::Efficiency) => {
                if let Some(v) = a.number() {
                    part_eff = Some(v);
                }
            }
            Some(role) => {
                let scaled = match role {
                    BudgetRole::Cost => a.number(),
                    _ => a.base_number(),
                };
                match (&a.value, scaled) {
                    (AttrValue::Absent, _) => {}
                    (_, Some(v)) => match role {
                        BudgetRole::Power => part_power += v,
                        BudgetRole::WasteHeat => *part_heat.get_or_insert(0.0) += v,
                        BudgetRole::Latency => *out.latency.get_or_insert(0.0) += v,
                        BudgetRole::Mass => out.items.push((Dom::Mass, v)),
                        _ => out.items.push((Dom::Cost, v)),
                    },
                    _ => match role {
                        BudgetRole::Mass => out.unresolved.push((Dom::Mass, entry(name, a))),
                        BudgetRole::Power => out.unresolved.push((Dom::Power, entry(name, a))),
                        BudgetRole::WasteHeat => out.unresolved.push((Dom::Heat, entry(name, a))),
                        BudgetRole::Latency => out.time_unresolved.push(entry(name, a)),
                        _ => out.unresolved.push((Dom::Cost, entry(name, a))),
                    },
                }
            }
            None => {
                if matches!(a.value, AttrValue::Tbd) {
                    continue;
                }
                match a.dim() {
                    Some(DimClass::Mass) => out.untracked.push((Dom::Mass, entry(name, a))),
                    Some(DimClass::Power) => {
                        out.untracked.push((Dom::Power, entry(name, a)));
                        out.untracked.push((Dom::Heat, entry(name, a)));
                    }
                    Some(DimClass::Time) => out.time_untracked.push(entry(name, a)),
                    _ => {}
                }
            }
        }
    }
    if part_power > 0.0 {
        out.items.push((Dom::Power, part_power));
    }
    match (part_heat, part_eff) {
        (Some(h), _) => out.items.push((Dom::Heat, h)),
        (None, Some(eta)) if part_power > 0.0 && (0.0..1.0).contains(&eta) => {
            out.items.push((Dom::Heat, part_power * (1.0 - eta)));
        }
        _ => {}
    }
    out
}

/// Contributions of one instance subtree, before it is scaled by its multiplicity.
#[derive(Default)]
struct SubTree {
    items: [Vec<(String, f64, usize)>; 4],
    unresolved: [Vec<UntrackedAttr>; 4],
    untracked: [Vec<UntrackedAttr>; 4],
}

impl SubTree {
    fn absorb(&mut self, child: SubTree, factor: u64) {
        for d in 0..4 {
            for (path, v, k) in child.items[d].iter().cloned() {
                if factor > 0 {
                    self.items[d].push((path, v * factor as f64, k * factor as usize));
                }
            }
            for e in child.unresolved[d].iter().cloned() {
                DomainAcc::push_unique(&mut self.unresolved[d], e);
            }
            for e in child.untracked[d].iter().cloned() {
                DomainAcc::push_unique(&mut self.untracked[d], e);
            }
        }
    }
}

/// Walks the part-usage tree from the top-level context part(s) (C1.6): a definition's role
/// values count once per usage instance, times the usage's fixed multiplicity.
struct UsageWalker<'i, 'a> {
    idx: &'i RoleIndex<'a>,
    reached: BTreeSet<String>,
    /// Structural reasons that make every limited domain `incomplete` (cycles).
    issues: Vec<String>,
}

impl<'i, 'a> UsageWalker<'i, 'a> {
    fn instance(
        &mut self,
        def: &'a PartDef,
        attr_overlays: &[&'a AttributeUsage],
        extra_parts: &[&'a PartUsage],
        path: &str,
        stack: &mut Vec<String>,
    ) -> SubTree {
        self.reached.insert(def.name.clone());
        let attrs = self.idx.overlay_attributes(
            self.idx.effective_attributes(def),
            attr_overlays,
            &def.name,
        );
        let own = part_items(&def.name, &attrs);
        let mut sub = SubTree::default();
        for (d, v) in own.items {
            sub.items[d as usize].push((path.to_string(), v, 1));
        }
        for (d, e) in own.unresolved {
            DomainAcc::push_unique(&mut sub.unresolved[d as usize], e);
        }
        for (d, e) in own.untracked {
            DomainAcc::push_unique(&mut sub.untracked[d as usize], e);
        }

        let mut parts = Vec::new();
        merge_part_usages(&mut parts, extra_parts.iter().copied());
        for p in self.idx.effective_parts(def) {
            match parts.iter_mut().find(|e| e.key == p.key) {
                Some(e) => {
                    if e.type_name.is_none() {
                        e.type_name = p.type_name.clone();
                    }
                    if e.multiplicity.is_none() {
                        e.multiplicity = p.multiplicity.clone();
                    }
                    e.attributes.extend(p.attributes.iter().copied());
                    e.nested_parts.extend(p.nested_parts.iter().copied());
                }
                None => parts.push(p),
            }
        }
        for p in parts {
            if p.is_ref {
                continue; // a reference is not an owned instance
            }
            let Some(child) = p.type_name.as_deref().and_then(|t| self.idx.get(t)) else {
                continue; // typed by a non-part definition (or untyped)
            };
            let child_path = format!("{path}.{}", p.key);
            if stack.iter().any(|s| s == &child.name) {
                let lower = match instance_count(p.multiplicity.as_ref()) {
                    Ok(n) | Err(n) => n,
                };
                if lower > 0 {
                    self.issues.push(format!(
                        "composition cycle at `{child_path}` (`{}` contains itself at least once): \
                         instance count is unbounded",
                        child.name
                    ));
                } else {
                    // Optional recursion (`part subcomponents[*] : Self`): zero instances are
                    // required; a recursive definition with role values of its own is open.
                    let own = part_items(&child.name, &self.idx.effective_attributes(child));
                    let mult = p.multiplicity.as_ref().map(|m| m.render()).unwrap_or_default();
                    for d in DOMS {
                        if own.items.iter().any(|(x, _)| *x == d) {
                            DomainAcc::push_unique(
                                &mut sub.unresolved[d as usize],
                                UntrackedAttr {
                                    element: child_path.clone(),
                                    attribute: format!("multiplicity {mult} (recursive)"),
                                    unit: dom_unit(d).to_string(),
                                    value: None,
                                },
                            );
                        }
                    }
                }
                continue;
            }
            let child_sub = self.usage(child, &p.attributes, &p.nested_parts, &child_path, stack);
            match instance_count(p.multiplicity.as_ref()) {
                Ok(n) => sub.absorb(child_sub, n),
                Err(lower) => {
                    let mult = p.multiplicity.as_ref().map(|m| m.render()).unwrap_or_default();
                    for d in DOMS {
                        if !child_sub.items[d as usize].is_empty() {
                            DomainAcc::push_unique(
                                &mut sub.unresolved[d as usize],
                                UntrackedAttr {
                                    element: child_path.clone(),
                                    attribute: format!("multiplicity {mult}"),
                                    unit: dom_unit(d).to_string(),
                                    value: None,
                                },
                            );
                        }
                    }
                    // The lower bound keeps `fail` sound; `unresolved` keeps `pass` impossible.
                    sub.absorb(child_sub, lower);
                }
            }
        }
        sub
    }

    /// One usage typed by `def`: a variation definition is a choice that `budget` does not
    /// bind (listed as `unresolved`), anything else is walked as an instance.
    fn usage(
        &mut self,
        def: &'a PartDef,
        attr_overlays: &[&'a AttributeUsage],
        extra_parts: &[&'a PartUsage],
        path: &str,
        stack: &mut Vec<String>,
    ) -> SubTree {
        if stack.iter().any(|s| s == &def.name) {
            self.issues.push(format!(
                "composition cycle at `{path}` (`{}` contains itself): instance count is unbounded",
                def.name
            ));
            return SubTree::default();
        }
        stack.push(def.name.clone());
        let sub = if def.is_variation {
            self.reached.insert(def.name.clone());
            let mut sub = SubTree::default();
            let mut contributes = [false; 4];
            for v in &def.variants {
                if let Some(alt) = self.idx.get(&v.type_name) {
                    let alt_sub = self.usage(alt, &[], &[], &format!("{path}.{}", v.name), stack);
                    for d in DOMS {
                        contributes[d as usize] |= !alt_sub.items[d as usize].is_empty();
                    }
                    for d in 0..4 {
                        for e in alt_sub.untracked[d].iter().cloned() {
                            DomainAcc::push_unique(&mut sub.untracked[d], e);
                        }
                    }
                }
            }
            for d in DOMS {
                if contributes[d as usize] {
                    DomainAcc::push_unique(
                        &mut sub.unresolved[d as usize],
                        UntrackedAttr {
                            element: path.to_string(),
                            attribute: format!("variation {} (variant not bound)", def.name),
                            unit: dom_unit(d).to_string(),
                            value: None,
                        },
                    );
                }
            }
            sub
        } else {
            self.instance(def, attr_overlays, extra_parts, path, stack)
        };
        stack.pop();
        sub
    }
}

/// Top-level context parts: concrete part definitions of the model's own packages that are
/// not the type of an owned part usage (nor a variant) anywhere in those packages, plus
/// package-level part usages. Returns (root definitions, package-level usages).
/// Every part definition of the model's own packages (abstract and variation ones included).
fn user_part_defs(model: &Model) -> Vec<&PartDef> {
    fn walk<'a>(pkg: &'a Package, out: &mut Vec<&'a PartDef>) {
        for m in &pkg.members {
            match m {
                PackageMember::Package(sub) => walk(sub, out),
                PackageMember::PartDef(p) => out.push(p),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for pkg in model.packages.iter().filter(|p| !is_sysml_std_package(p)) {
        walk(pkg, &mut out);
    }
    out
}

fn find_roots(model: &Model) -> (Vec<&PartDef>, Vec<&PartUsage>) {
    fn collect_usage_types(u: &PartUsage, used: &mut BTreeSet<String>) {
        if !u.is_ref {
            for t in &u.type_names {
                used.insert(t.rsplit("::").next().unwrap_or(t).to_string());
            }
        }
        for n in &u.nested_parts {
            collect_usage_types(n, used);
        }
    }
    fn walk<'a>(
        pkg: &'a Package,
        used: &mut BTreeSet<String>,
        usages: &mut Vec<&'a PartUsage>,
    ) {
        for m in &pkg.members {
            match m {
                PackageMember::Package(sub) => walk(sub, used, usages),
                PackageMember::PartDef(p) => {
                    for u in &p.parts {
                        collect_usage_types(u, used);
                    }
                    for v in &p.variants {
                        let t = &v.type_name;
                        used.insert(t.rsplit("::").next().unwrap_or(t).to_string());
                    }
                }
                PackageMember::PartUsage(u) if !u.is_ref => {
                    collect_usage_types(u, used);
                    usages.push(u);
                }
                _ => {}
            }
        }
    }
    let mut used = BTreeSet::new();
    let mut usages = Vec::new();
    for pkg in model.packages.iter().filter(|p| !is_sysml_std_package(p)) {
        walk(pkg, &mut used, &mut usages);
    }
    // A concrete definition that another user definition specializes is a base type, not a
    // context: its values reach the tree through its specializations.
    let specialized: BTreeSet<String> = user_part_defs(model)
        .iter()
        .flat_map(|p| p.specializes.iter())
        .map(|t| t.rsplit("::").next().unwrap_or(t).to_string())
        .collect();
    let candidates: Vec<&PartDef> = concrete_user_parts(model)
        .into_iter()
        .filter(|p| !used.contains(&p.name) && !specialized.contains(&p.name))
        .collect();
    // An `individual part def` names one specific instance (e.g. the flight article `'LM-5'`
    // of `ApolloLunarModule`); it is a root only when there is no other context part.
    let roots = if candidates.iter().any(|p| !p.is_individual) {
        candidates.into_iter().filter(|p| !p.is_individual).collect()
    } else {
        candidates
    };
    (roots, usages)
}

/// Analyze cross-domain budgets, DAG critical-path latency, pipeline FPS bottleneck, and top-3
/// hot spots on a [`Model`].
///
/// Contributions come only from budget roles (D1): an attribute that is, or redefines, a
/// role-tagged attribute (`#[sysml::attr(budget = "...")]` or a sysml-std role attribute such
/// as `PowerElement::ratedPower`). Attributes of requirement definitions, evidence attributes
/// and `tbd_value` attributes are never contributions nor `untracked` (role-tagged
/// `tbd_value` ones are listed as `unresolved`).
///
/// Mass, power, waste heat and cost roll up over the **usage tree** (C1.6): from the top-level
/// context part(s) (see [`BudgetReport::roots`]), every owned part usage is an instance, so a
/// definition used twice counts twice (times a fixed `[n]` multiplicity). Concrete definitions
/// never reached from a root are listed as `uninstantiated` (not summed). Without any root the
/// rollup falls back to one count per definition and every limited domain is `incomplete`.
pub fn analyze_budgets(model: &Model, limits: &BudgetLimits) -> BudgetReport {
    let idx = RoleIndex::new(model);
    let mut accs: [DomainAcc; 4] = Default::default();
    let mut time = DomainAcc::default();
    let mut stage_latency_ms: BTreeMap<String, f64> = BTreeMap::new();

    // Timing is a critical path over definitions (not multiplied by instances).
    let concrete = concrete_user_parts(model);
    for p in &concrete {
        let own = part_items(&p.name, &idx.effective_attributes(p));
        time.untracked.extend(own.time_untracked);
        time.unresolved.extend(own.time_unresolved);
        if let Some(lat_s) = own.latency {
            stage_latency_ms.insert(p.name.clone(), lat_s * 1000.0);
        }
    }

    // Mass / power / waste heat / cost: usage-tree rollup.
    let (root_defs, root_usages) = find_roots(model);
    let mut walker = UsageWalker {
        idx: &idx,
        reached: BTreeSet::new(),
        issues: Vec::new(),
    };
    let mut total = SubTree::default();
    let mut roots: Vec<String> = Vec::new();
    for def in &root_defs {
        roots.push(def.name.clone());
        let mut stack = Vec::new();
        let sub = walker.usage(def, &[], &[], &def.name, &mut stack);
        total.absorb(sub, 1);
    }
    for u in &root_usages {
        let Some(def) = u.type_names.first().and_then(|t| idx.get(t)) else {
            continue;
        };
        let name = part_key(u).unwrap_or_else(|| def.name.clone());
        roots.push(name.clone());
        let attrs: Vec<&AttributeUsage> = u.attributes.iter().collect();
        let nested: Vec<&PartUsage> = u.nested_parts.iter().collect();
        let mut stack = Vec::new();
        let sub = walker.usage(def, &attrs, &nested, &name, &mut stack);
        match instance_count(u.multiplicity.as_ref()) {
            Ok(n) => total.absorb(sub, n),
            Err(lower) => {
                for d in DOMS {
                    if !sub.items[d as usize].is_empty() {
                        DomainAcc::push_unique(
                            &mut total.unresolved[d as usize],
                            UntrackedAttr {
                                element: name.clone(),
                                attribute: format!(
                                    "multiplicity {}",
                                    u.multiplicity.as_ref().map(|m| m.render()).unwrap_or_default()
                                ),
                                unit: dom_unit(d).to_string(),
                                value: None,
                            },
                        );
                    }
                }
                total.absorb(sub, lower);
            }
        }
    }

    let rollup = if roots.is_empty() && !concrete.is_empty() {
        // No identifiable root: every definition counts once, and the result is not trusted.
        for p in &concrete {
            let own = part_items(&p.name, &idx.effective_attributes(p));
            for (d, v) in own.items {
                total.items[d as usize].push((p.name.clone(), v, 1));
            }
            for (d, e) in own.unresolved {
                DomainAcc::push_unique(&mut total.unresolved[d as usize], e);
            }
            for (d, e) in own.untracked {
                DomainAcc::push_unique(&mut total.untracked[d as usize], e);
            }
        }
        for acc in accs.iter_mut() {
            acc.reasons.push(
                "no top-level context part (every concrete part definition is used inside \
                 another): counted once per definition, instance counts unknown"
                    .to_string(),
            );
        }
        RollupMode::PerDefinition
    } else {
        // Ancestors of instantiated definitions are covered through their specializations.
        let covered: BTreeSet<String> = walker
            .reached
            .iter()
            .filter_map(|n| idx.get(n))
            .flat_map(|d| idx.lineage(d))
            .map(|d| d.name.clone())
            .collect();
        for p in &concrete {
            if covered.contains(&p.name) {
                continue;
            }
            // Never instantiated from a root: reported, never summed nor dropped.
            for a in idx.effective_attributes(p) {
                if a.is_evidence || matches!(a.value, AttrValue::Absent) {
                    continue;
                }
                let d = match a.role {
                    Some(BudgetRole::Mass) => Dom::Mass,
                    Some(BudgetRole::Power) => Dom::Power,
                    Some(BudgetRole::WasteHeat) => Dom::Heat,
                    Some(BudgetRole::Cost) => Dom::Cost,
                    Some(_) => continue,
                    None => {
                        if !matches!(a.value, AttrValue::Tbd) {
                            match a.dim() {
                                Some(DimClass::Mass) => DomainAcc::push_unique(
                                    &mut total.untracked[Dom::Mass as usize],
                                    entry(&p.name, &a),
                                ),
                                Some(DimClass::Power) => {
                                    DomainAcc::push_unique(
                                        &mut total.untracked[Dom::Power as usize],
                                        entry(&p.name, &a),
                                    );
                                    DomainAcc::push_unique(
                                        &mut total.untracked[Dom::Heat as usize],
                                        entry(&p.name, &a),
                                    );
                                }
                                _ => {}
                            }
                        }
                        continue;
                    }
                };
                let mut e = entry(&p.name, &a);
                if !matches!(a.role, Some(BudgetRole::Cost)) {
                    e.value = a.base_number();
                    e.unit = dom_unit(d).to_string();
                }
                accs[d as usize].uninstantiated.push(e);
            }
        }
        RollupMode::UsageTree
    };
    for issue in &walker.issues {
        for acc in accs.iter_mut() {
            if !acc.reasons.contains(issue) {
                acc.reasons.push(issue.clone());
            }
        }
    }
    for d in DOMS {
        let acc = &mut accs[d as usize];
        acc.items = std::mem::take(&mut total.items[d as usize]);
        acc.unresolved = std::mem::take(&mut total.unresolved[d as usize]);
        acc.untracked = std::mem::take(&mut total.untracked[d as usize]);
    }

    let part_defs: BTreeMap<String, &PartDef> = concrete_user_parts(model)
        .into_iter()
        .map(|p| (p.name.clone(), p))
        .collect();

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
    let timing_limited = limits.max_latency_ms.is_some() || limits.min_fps.is_some();
    let timing_status = if !timing_limited {
        BudgetStatus::Pass
    } else if (limits.max_latency_ms.is_some() && !latency_pass)
        || (limits.min_fps.is_some() && pipeline_fps.is_some() && !fps_pass)
    {
        BudgetStatus::Fail
    } else if !time.untracked.is_empty() || !time.unresolved.is_empty() || node_lat.is_empty() {
        BudgetStatus::Incomplete
    } else {
        BudgetStatus::Pass
    };

    let [mass, power, heat, cost] = accs;
    let mass = build_rollup("mass", "kg", mass, limits.max_mass_kg);
    let power = build_rollup("power", "W", power, limits.max_power_w);
    let waste_heat = build_rollup("waste_heat", "W", heat, None);
    let cost = build_rollup("cost", "USD", cost, limits.max_cost_usd);

    let ok = [&mass, &power, &waste_heat, &cost]
        .iter()
        .all(|d| d.limit.is_none() || d.status == BudgetStatus::Pass)
        && (!timing_limited || timing_status == BudgetStatus::Pass);

    BudgetReport {
        ok,
        rollup,
        roots,
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
            status: timing_status,
            untracked: time.untracked,
            unresolved: time.unresolved,
        },
    }
}

fn render_attr_list(label: &str, list: &[UntrackedAttr], out: &mut String) {
    if list.is_empty() {
        return;
    }
    let items = list
        .iter()
        .map(|u| {
            let v = u.value.map(|v| format!(" = {v}")).unwrap_or_default();
            let unit = if u.unit.is_empty() { String::new() } else { format!(" [{}]", u.unit) };
            format!("`{}.{}`{v}{unit}", u.element, u.attribute)
        })
        .collect::<Vec<_>>()
        .join(", ");
    out.push_str(&format!("  - {label}: {items}\n"));
}

/// Render a [`BudgetReport`] as clean Markdown.
pub fn render_budget_markdown(r: &BudgetReport) -> String {
    let mut out = String::new();
    out.push_str("# Cross-Domain System Budget & Critical-Path Report\n\n");
    out.push_str(&format!(
        "- **Overall Status:** {}\n",
        if r.ok { "PASS" } else { "NOT OK" }
    ));
    out.push_str(&format!(
        "- **Rollup:** {} from {}\n",
        match r.rollup {
            RollupMode::UsageTree => "per usage instance",
            RollupMode::PerDefinition => "per definition (no top-level context part)",
        },
        if r.roots.is_empty() {
            "—".to_string()
        } else {
            r.roots.iter().map(|x| format!("`{x}`")).collect::<Vec<_>>().join(", ")
        }
    ));
    out.push_str(&format!(
        "- **Critical Path Latency:** {:.2} ms ({}){} [{}]\n",
        r.timing.critical_path_ms,
        if r.timing.critical_path_nodes.is_empty() {
            "none".to_string()
        } else {
            r.timing.critical_path_nodes.join(" -> ")
        },
        r.timing
            .latency_margin_ms
            .map(|m| format!(" | margin: {m:+.2} ms"))
            .unwrap_or_default(),
        r.timing.status.as_str().to_uppercase()
    ));
    if let Some(fps) = r.timing.pipeline_fps {
        out.push_str(&format!(
            "- **Pipeline Throughput:** {:.2} FPS (bottleneck: `{}`)\n",
            fps,
            r.timing.fps_bottleneck_stage.as_deref().unwrap_or("—")
        ));
    }
    render_attr_list("untracked (no budget role)", &r.timing.untracked, &mut out);
    render_attr_list("unresolved (role, no usable value)", &r.timing.unresolved, &mut out);
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
            "- **{}:** {:.2} {}{} [{}] ({} contributors){}\n",
            d.domain,
            d.total,
            d.unit,
            lim,
            d.status.as_str().to_uppercase(),
            d.contributors,
            if top.is_empty() {
                String::new()
            } else {
                format!(" — hot spots: {top}")
            }
        ));
        render_attr_list("untracked (no budget role)", &d.untracked, &mut out);
        render_attr_list("unresolved (role, no usable value)", &d.unresolved, &mut out);
        render_attr_list("uninstantiated (not reached from a context part)", &d.uninstantiated, &mut out);
        if d.limit.is_some() && d.status == BudgetStatus::Incomplete {
            for reason in &d.reasons {
                out.push_str(&format!("  - incomplete: {reason}\n"));
            }
        }
    }
    out
}

fn build_rollup(domain: &str, unit: &str, mut acc: DomainAcc, limit: Option<f64>) -> DomainRollup {
    // One hot-spot row per element (usage path), instances merged.
    let mut merged: Vec<(String, f64, usize)> = Vec::new();
    for (el, v, k) in acc.items.drain(..) {
        match merged.iter_mut().find(|(e, _, _)| *e == el) {
            Some(m) => {
                m.1 += v;
                m.2 += k;
            }
            None => merged.push((el, v, k)),
        }
    }
    let total: f64 = merged.iter().map(|(_, v, _)| *v).sum();
    let contributors: usize = merged.iter().map(|(_, _, k)| *k).sum();
    let mut reasons = std::mem::take(&mut acc.reasons);
    if !acc.untracked.is_empty() {
        reasons.push(format!("{} attribute(s) of this dimension have no budget role (untracked)", acc.untracked.len()));
    }
    if !acc.unresolved.is_empty() {
        reasons.push(format!("{} role value(s) cannot be summed (unresolved)", acc.unresolved.len()));
    }
    if !acc.uninstantiated.is_empty() {
        reasons.push(format!(
            "{} role value(s) belong to definitions never instantiated from a top-level context part (uninstantiated)",
            acc.uninstantiated.len()
        ));
    }
    if merged.is_empty() {
        reasons.push("no contributors".to_string());
    }
    let status = match limit {
        None => BudgetStatus::Pass,
        Some(l) if total > l + 1e-9 => BudgetStatus::Fail,
        Some(_) if !reasons.is_empty() => BudgetStatus::Incomplete,
        Some(_) => BudgetStatus::Pass,
    };
    let items: Vec<(String, f64)> = merged.into_iter().map(|(e, v, _)| (e, v)).collect();
    DomainRollup {
        domain: domain.to_string(),
        unit: unit.to_string(),
        total: round2(total),
        limit,
        margin: limit.map(|l| round2(l - total)),
        pass: status == BudgetStatus::Pass,
        status,
        contributors,
        hot_spots: build_hot_spots(&items, total),
        untracked: acc.untracked,
        unresolved: acc.unresolved,
        uninstantiated: acc.uninstantiated,
        reasons,
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
    use sysml_ir::{
        AttributeUsage, ConnectionKind, ConnectionUsage, Package, PackageMember, PartUsage,
    };

    fn attr(name: &str, val: &str, unit: Option<&str>, role: Option<BudgetRole>) -> AttributeUsage {
        AttributeUsage {
            name: Some(name.into()),
            value: Some(val.into()),
            unit: unit.map(String::from),
            budget_role: role,
            ..Default::default()
        }
    }

    fn part(name: &str, attributes: Vec<AttributeUsage>) -> PackageMember {
        PackageMember::PartDef(PartDef {
            name: name.into(),
            attributes,
            ..Default::default()
        })
    }


    #[test]
    fn edge_ai_robot_critical_path_fps_and_hot_spots_from_roles() {
        let mut pkg = Package {
            name: "EdgeAiRobot".into(),
            ..Default::default()
        };
        pkg.members.push(part(
            "Camera",
            vec![
                attr("frameLatency", "0.016", Some("s"), Some(BudgetRole::Latency)),
                attr("ratedPower", "3.0", Some("W"), Some(BudgetRole::Power)),
                attr("mass", "0.05", Some("kg"), Some(BudgetRole::Mass)),
                attr("unitCost", "45.0", None, Some(BudgetRole::Cost)),
            ],
        ));
        pkg.members.push(part(
            "EdgeLlmAccelerator",
            vec![
                // LLM thinking time = time-to-first-token + decode time (40 tokens / 80 tok/s),
                // each a latency-role attribute: 120 ms + 500 ms = 620 ms.
                attr("ttft", "0.120", Some("s"), Some(BudgetRole::Latency)),
                attr("decodeTime", "0.5", Some("s"), Some(BudgetRole::Latency)),
                attr("ratedPower", "25.0", Some("W"), Some(BudgetRole::Power)),
                attr("eta", "0.75", None, Some(BudgetRole::Efficiency)),
                attr("mass", "80", Some("g"), Some(BudgetRole::Mass)),
                attr("unitCost", "149.0", None, Some(BudgetRole::Cost)),
            ],
        ));
        pkg.members.push(part(
            "MotorCtrl",
            vec![
                attr("loopLatency", "0.005", Some("s"), Some(BudgetRole::Latency)),
                attr("ratedPower", "12.0", Some("W"), Some(BudgetRole::Power)),
                attr("mass", "0.12", Some("kg"), Some(BudgetRole::Mass)),
                attr("unitCost", "35.0", None, Some(BudgetRole::Cost)),
            ],
        ));
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
            max_cost_usd: Some(250.0),   // 229 <= 250 -> PASS
        };

        let report = analyze_budgets(&model, &limits);
        assert!(!report.ok);
        assert!((report.timing.critical_path_ms - 641.0).abs() < 1e-6);
        assert_eq!(report.timing.critical_path_nodes, vec!["cam", "llm", "ctrl"]);
        assert!(!report.timing.latency_pass);
        assert_eq!(report.timing.status, BudgetStatus::Fail);
        assert_eq!(report.timing.fps_bottleneck_stage.as_deref(), Some("llm"));
        assert!(!report.timing.fps_pass);
        assert_eq!(report.timing.stage_hot_spots[0].element, "llm");
        assert_eq!(report.power.status, BudgetStatus::Pass);
        assert!(report.power.pass && (report.power.total - 40.0).abs() < 1e-6);
        assert!(report.mass.pass && (report.mass.total - 0.25).abs() < 1e-6);
        assert!(report.cost.pass && (report.cost.total - 229.0).abs() < 1e-6);
        assert!((report.waste_heat.total - 6.25).abs() < 1e-6);
    }

    /// P2 regression: a 2000 W element named `ratedPowerW` without a role used to be
    /// invisible (total 0 W, `pass: true`). It is now `untracked` and the limited power domain
    /// is `incomplete`, so `ok` is false.
    #[test]
    fn untagged_watts_are_untracked_and_never_a_silent_pass() {
        let pkg = Package {
            name: "WaterHeater".into(),
            members: vec![part(
                "HeatingElement",
                vec![
                    attr("ratedPowerW", "2000.0", Some("W"), None),
                    attr("supplyVoltage", "230.0", Some("V"), None),
                ],
            )],
            ..Default::default()
        };
        let model = Model::new(vec![pkg]);
        let r = analyze_budgets(
            &model,
            &BudgetLimits {
                max_power_w: Some(100.0),
                ..Default::default()
            },
        );
        assert!(!r.ok);
        assert_eq!(r.power.status, BudgetStatus::Incomplete);
        assert_eq!(r.power.contributors, 0);
        assert_eq!(r.power.untracked.len(), 1);
        assert_eq!(r.power.untracked[0].attribute, "ratedPowerW");
        assert_eq!(r.power.untracked[0].value, Some(2000.0));
        // No limit on mass: nothing to check.
        assert_eq!(r.mass.status, BudgetStatus::Pass);

        // Tagging it makes the budget decidable: 2000 W > 100 W -> fail.
        let mut tagged = model.clone();
        if let PackageMember::PartDef(p) = &mut tagged.packages[0].members[0] {
            p.attributes[0].budget_role = Some(BudgetRole::Power);
        }
        let r = analyze_budgets(
            &tagged,
            &BudgetLimits {
                max_power_w: Some(100.0),
                ..Default::default()
            },
        );
        assert_eq!(r.power.status, BudgetStatus::Fail);
        assert!((r.power.total - 2000.0).abs() < 1e-9);
        assert!(r.power.untracked.is_empty());
    }

    #[test]
    fn tbd_and_none_roles_and_inherited_std_roles() {
        let base = PartDef {
            name: "PowerElement".into(),
            is_abstract: true,
            attributes: vec![AttributeUsage {
                name: Some("ratedPower".into()),
                unit: Some("W".into()),
                budget_role: Some(BudgetRole::Power),
                ..Default::default()
            }],
            ..Default::default()
        };
        let fan = PartDef {
            name: "Fan".into(),
            specializes: vec!["PowerElement".into()],
            attributes: vec![
                AttributeUsage {
                    redefines: vec!["ratedPower".into()],
                    value: Some("170.0".into()),
                    unit: Some("W".into()),
                    ..Default::default()
                },
                attr("supplyCapacity", "500.0", Some("W"), Some(BudgetRole::None)),
            ],
            ..Default::default()
        };
        let pump = PartDef {
            name: "Pump".into(),
            specializes: vec!["PowerElement".into()],
            attributes: vec![AttributeUsage {
                redefines: vec!["ratedPower".into()],
                unit: Some("W".into()),
                tbd: vec![sysml_ir::TbdMark::new(sysml_ir::TbdKind::Value, "datasheet pending")],
                ..Default::default()
            }],
            ..Default::default()
        };
        let model = Model::new(vec![Package {
            name: "Vent".into(),
            members: vec![
                PackageMember::PartDef(base),
                PackageMember::PartDef(fan),
                PackageMember::PartDef(pump),
            ],
            ..Default::default()
        }]);
        let r = analyze_budgets(
            &model,
            &BudgetLimits {
                max_power_w: Some(1000.0),
                ..Default::default()
            },
        );
        assert!((r.power.total - 170.0).abs() < 1e-9);
        assert!(r.power.untracked.is_empty(), "{:?}", r.power.untracked);
        assert_eq!(r.power.unresolved.len(), 1);
        assert_eq!(r.power.unresolved[0].element, "Pump");
        assert_eq!(r.power.status, BudgetStatus::Incomplete);
        assert!(!r.ok);
    }

    fn usage(name: &str, ty: &str) -> PartUsage {
        PartUsage {
            name: Some(name.into()),
            type_names: vec![ty.into()],
            ..Default::default()
        }
    }

    fn fan_model(context_parts: Vec<PartUsage>, extra: Vec<PackageMember>) -> Model {
        let mut members = vec![
            part("Fan", vec![attr("ratedPower", "170.0", Some("W"), Some(BudgetRole::Power))]),
            PackageMember::PartDef(PartDef {
                name: "VentUnit".into(),
                parts: context_parts,
                ..Default::default()
            }),
        ];
        members.extend(extra);
        Model::new(vec![Package {
            name: "Vent".into(),
            members,
            ..Default::default()
        }])
    }

    fn power_limit(w: f64) -> BudgetLimits {
        BudgetLimits {
            max_power_w: Some(w),
            ..Default::default()
        }
    }

    /// C1.6: two usages of one fan definition total 2 x 170 W (used to be 170 W).
    #[test]
    fn two_usages_of_one_definition_count_twice() {
        let model = fan_model(vec![usage("supplyFan", "Fan"), usage("exhaustFan", "Fan")], vec![]);
        let r = analyze_budgets(&model, &power_limit(300.0));
        assert_eq!(r.rollup, RollupMode::UsageTree);
        assert_eq!(r.roots, vec!["VentUnit"]);
        assert!((r.power.total - 340.0).abs() < 1e-9, "{:?}", r.power);
        assert_eq!(r.power.contributors, 2);
        assert_eq!(r.power.status, BudgetStatus::Fail);
        let els: Vec<&str> = r.power.hot_spots.iter().map(|h| h.element.as_str()).collect();
        assert!(els.contains(&"VentUnit.supplyFan") && els.contains(&"VentUnit.exhaustFan"));
    }

    #[test]
    fn fixed_multiplicity_nested_and_inherited_usages_multiply() {
        // VentUnit { fans[3] : Fan; bank : FanBank }, FanBank :> AbstractBank { (inherited) pair[2] : Fan }
        let mut fans = usage("fans", "Fan");
        fans.multiplicity = Some(sysml_ir::Multiplicity::new(None::<String>, "3"));
        let mut pair = usage("pair", "Fan");
        pair.multiplicity = Some(sysml_ir::Multiplicity::new(Some("2"), "2"));
        let bank_base = PackageMember::PartDef(PartDef {
            name: "AbstractBank".into(),
            is_abstract: true,
            parts: vec![pair],
            ..Default::default()
        });
        let bank = PackageMember::PartDef(PartDef {
            name: "FanBank".into(),
            specializes: vec!["AbstractBank".into()],
            ..Default::default()
        });
        let model = fan_model(vec![fans, usage("bank", "FanBank")], vec![bank_base, bank]);
        let r = analyze_budgets(&model, &power_limit(1000.0));
        assert!((r.power.total - 5.0 * 170.0).abs() < 1e-9, "{:?}", r.power);
        assert_eq!(r.power.contributors, 5);
        assert_eq!(r.power.status, BudgetStatus::Pass);
        assert!(r.ok);
    }

    #[test]
    fn open_multiplicity_is_unresolved_lower_bound_never_a_pass() {
        let mut fans = usage("fans", "Fan");
        fans.multiplicity = Some(sysml_ir::Multiplicity::new(Some("1"), "*"));
        let model = fan_model(vec![fans], vec![]);
        let r = analyze_budgets(&model, &power_limit(1000.0));
        assert!((r.power.total - 170.0).abs() < 1e-9);
        assert_eq!(r.power.status, BudgetStatus::Incomplete);
        assert_eq!(r.power.unresolved[0].element, "VentUnit.fans");
        assert!(r.power.unresolved[0].attribute.contains("[1..*]"));
        // The lower bound alone already exceeds a small limit: `fail` is sound.
        let r = analyze_budgets(&model, &power_limit(100.0));
        assert_eq!(r.power.status, BudgetStatus::Fail);
    }

    #[test]
    fn uninstantiated_definitions_are_listed_and_make_limited_domains_incomplete() {
        let heater = part("Heater", vec![attr("ratedPower", "2000.0", Some("W"), Some(BudgetRole::Power))]);
        // `Spare` is only used inside an abstract definition nobody specializes or uses.
        let orphan_holder = PackageMember::PartDef(PartDef {
            name: "UnusedAssembly".into(),
            is_abstract: true,
            parts: vec![usage("h", "Heater")],
            ..Default::default()
        });
        let model = fan_model(vec![usage("fan", "Fan")], vec![heater, orphan_holder]);
        let r = analyze_budgets(&model, &power_limit(1000.0));
        assert!((r.power.total - 170.0).abs() < 1e-9);
        assert_eq!(r.power.uninstantiated.len(), 1, "{:?}", r.power.uninstantiated);
        assert_eq!(r.power.uninstantiated[0].element, "Heater");
        assert_eq!(r.power.uninstantiated[0].value, Some(2000.0));
        assert_eq!(r.power.status, BudgetStatus::Incomplete);
        assert!(r.power.reasons.iter().any(|x| x.contains("uninstantiated")));
        assert!(!r.ok);
        // No limit on power: nothing to check.
        let r = analyze_budgets(&model, &BudgetLimits::default());
        assert_eq!(r.power.status, BudgetStatus::Pass);
    }

    #[test]
    fn ref_usages_variations_and_usage_level_redefinitions() {
        let mut r#ref = usage("observed", "Fan");
        r#ref.is_ref = true;
        let mut boosted = usage("boosted", "Fan");
        boosted.attributes.push(AttributeUsage {
            redefines: vec!["ratedPower".into()],
            value: Some("200.0".into()),
            unit: Some("W".into()),
            ..Default::default()
        });
        let variation = PackageMember::PartDef(PartDef {
            name: "FanChoice".into(),
            is_variation: true,
            variants: vec![sysml_ir::VariantPart {
                name: "std".into(),
                type_name: "Fan".into(),
                doc: None,
            }],
            ..Default::default()
        });
        let model = fan_model(
            vec![r#ref, boosted, usage("choice", "FanChoice")],
            vec![variation],
        );
        let r = analyze_budgets(&model, &power_limit(1000.0));
        // ref: not an owned instance; boosted: usage-level value; variation: unbound choice.
        assert!((r.power.total - 200.0).abs() < 1e-9, "{:?}", r.power);
        assert_eq!(r.power.unresolved.len(), 1);
        assert!(r.power.unresolved[0].attribute.contains("FanChoice"));
        assert_eq!(r.power.status, BudgetStatus::Incomplete);
    }

    #[test]
    fn no_root_falls_back_per_definition_and_is_incomplete() {
        // A contains B and B contains A: no top-level context part.
        let model = Model::new(vec![Package {
            name: "Loop".into(),
            members: vec![
                PackageMember::PartDef(PartDef {
                    name: "A".into(),
                    attributes: vec![attr("ratedPower", "10.0", Some("W"), Some(BudgetRole::Power))],
                    parts: vec![usage("b", "B")],
                    ..Default::default()
                }),
                PackageMember::PartDef(PartDef {
                    name: "B".into(),
                    attributes: vec![attr("ratedPower", "5.0", Some("W"), Some(BudgetRole::Power))],
                    parts: vec![usage("a", "A")],
                    ..Default::default()
                }),
            ],
            ..Default::default()
        }]);
        let r = analyze_budgets(&model, &power_limit(1000.0));
        assert_eq!(r.rollup, RollupMode::PerDefinition);
        assert!(r.roots.is_empty());
        assert!((r.power.total - 15.0).abs() < 1e-9);
        assert_eq!(r.power.status, BudgetStatus::Incomplete);
        assert!(r.power.reasons.iter().any(|x| x.contains("no top-level context part")));
    }
}
