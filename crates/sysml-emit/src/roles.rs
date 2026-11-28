//! C1 / D1: budget roles and dimensions — how `budget`, `trade-study` and `refute` find the
//! numbers they add up, without ever matching attribute *names* against a list.
//!
//! An attribute's role comes from its own `@SysmlStdBudget::BudgetRole` tag or, through
//! redefinition, from the attribute it redefines anywhere up the specialization chain (so every
//! `:>> ratedPower` of a sysml-std `PowerElement` is a power contributor by definition). Its
//! dimension comes from its unit via the closed SI table (`sysml_core::units`).

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use sysml_core::units::{parse_unit, Dim};
use sysml_ir::{
    AttributeUsage, BudgetRole, Model, Multiplicity, Package, PackageMember, PartDef, PartUsage,
    TbdKind,
};

use crate::emit_cli::is_sysml_std_package;

/// The physical dimension classes a budget domain is keyed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DimClass {
    Mass,
    Power,
    Time,
    Energy,
    Other,
}

/// Classify a unit symbol by dimension (`None` = no unit / unknown / custom unit).
pub fn dim_class(unit: Option<&str>) -> Option<DimClass> {
    let u = unit?.trim();
    if u.is_empty() {
        return None;
    }
    let d = parse_unit(u)?;
    let is = |reference: &str| parse_unit(reference).is_some_and(|r: Dim| r.eq(d));
    Some(if is("kg") {
        DimClass::Mass
    } else if is("W") {
        DimClass::Power
    } else if is("s") {
        DimClass::Time
    } else if is("J") {
        DimClass::Energy
    } else {
        DimClass::Other
    })
}

/// Factor converting a value in `unit` to the domain's base unit (`kg`, `W`, `s`, `J`).
/// `None` when the scale of a dimensionally correct unit is not known here (the value is then
/// reported, never summed with a wrong scale). A missing unit means "already in the base unit".
pub fn base_scale(unit: Option<&str>) -> Option<f64> {
    let Some(u) = unit.map(str::trim).filter(|u| !u.is_empty()) else {
        return Some(1.0);
    };
    Some(match u {
        "kg" | "W" | "s" | "J" | "J/s" | "N*m/s" => 1.0,
        "g" => 1e-3,
        "kW" => 1e3,
        "min" => 60.0,
        "h" => 3600.0,
        "d" => 86400.0,
        "kJ" => 1e3,
        "MJ" => 1e6,
        _ => return None,
    })
}

/// The value state of an effective attribute.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// A numeric literal (in the attribute's own unit).
    Number(f64),
    /// `tbd_value = "reason"`: the value is open in the source.
    Tbd,
    /// A non-literal expression (evaluated by C2, not here).
    Expr(String),
    /// Declared without a value anywhere in the chain.
    Absent,
}

/// One attribute of a part definition after resolving inheritance and redefinition.
#[derive(Debug, Clone)]
pub struct EffectiveAttr {
    /// Feature name (an anonymous `:>> ratedPower` redefinition is keyed `ratedPower`).
    pub key: String,
    pub role: Option<BudgetRole>,
    pub value: AttrValue,
    pub unit: Option<String>,
    pub is_evidence: bool,
    /// Name of the definition that supplied the value (or declared the feature).
    pub declared_in: String,
}

impl EffectiveAttr {
    pub fn number(&self) -> Option<f64> {
        match self.value {
            AttrValue::Number(v) => Some(v),
            _ => None,
        }
    }

    /// The value converted to the base unit of its dimension (`kg`, `W`, `s`, `J`).
    pub fn base_number(&self) -> Option<f64> {
        Some(self.number()? * base_scale(self.unit.as_deref())?)
    }

    pub fn dim(&self) -> Option<DimClass> {
        dim_class(self.unit.as_deref())
    }
}

fn simple(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name).trim()
}

/// Feature key of an attribute: its name, or the (simple) name of what it redefines.
pub fn attr_key(a: &AttributeUsage) -> Option<String> {
    a.name
        .as_deref()
        .or_else(|| a.redefines.first().map(String::as_str))
        .map(|n| simple(n).to_string())
}

fn parse_number(raw: &str) -> Option<f64> {
    let s = raw.trim();
    let token = s.split_whitespace().next()?;
    let v: f64 = token.parse().ok()?;
    // `1800.0 [W]` style or plain literal; anything else after the number is an expression.
    let rest = s[token.len()..].trim();
    (v.is_finite() && (rest.is_empty() || rest.starts_with('['))).then_some(v)
}

/// Index of every part definition in a model (user and library packages), by simple name.
pub struct RoleIndex<'a> {
    defs: BTreeMap<String, &'a PartDef>,
}

impl<'a> RoleIndex<'a> {
    pub fn new(model: &'a Model) -> Self {
        fn walk<'a>(pkg: &'a Package, defs: &mut BTreeMap<String, &'a PartDef>) {
            for m in &pkg.members {
                match m {
                    PackageMember::Package(sub) => walk(sub, defs),
                    PackageMember::PartDef(p) => {
                        defs.entry(p.name.clone()).or_insert(p);
                    }
                    _ => {}
                }
            }
        }
        let mut defs = BTreeMap::new();
        // User packages first so a user definition wins a simple-name clash with the library.
        for pkg in model.packages.iter().filter(|p| !is_sysml_std_package(p)) {
            walk(pkg, &mut defs);
        }
        for pkg in model.packages.iter().filter(|p| is_sysml_std_package(p)) {
            walk(pkg, &mut defs);
        }
        Self { defs }
    }

    pub fn get(&self, name: &str) -> Option<&'a PartDef> {
        self.defs.get(simple(name)).copied()
    }

    /// `def` followed by its ancestors, nearest first (breadth-first, cycle-safe).
    pub fn lineage(&self, def: &'a PartDef) -> Vec<&'a PartDef> {
        let mut out = vec![def];
        let mut seen: BTreeSet<&str> = BTreeSet::from([def.name.as_str()]);
        let mut queue: VecDeque<&'a PartDef> = VecDeque::from([def]);
        while let Some(cur) = queue.pop_front() {
            for parent in &cur.specializes {
                if let Some(p) = self.get(parent) {
                    if seen.insert(p.name.as_str()) {
                        out.push(p);
                        queue.push_back(p);
                    }
                }
            }
        }
        out
    }

    /// Every attribute feature of `def`, own and inherited, with role, value and unit resolved
    /// along the redefinition chain (most-derived declaration wins).
    pub fn effective_attributes(&self, def: &'a PartDef) -> Vec<EffectiveAttr> {
        let mut order: Vec<String> = Vec::new();
        let mut decls: BTreeMap<String, Vec<(&'a str, &'a AttributeUsage)>> = BTreeMap::new();
        for d in self.lineage(def) {
            for a in &d.attributes {
                let Some(key) = attr_key(a) else { continue };
                if !decls.contains_key(&key) {
                    order.push(key.clone());
                }
                decls.entry(key).or_default().push((d.name.as_str(), a));
            }
        }
        order
            .into_iter()
            .map(|key| {
                let chain = &decls[&key];
                let role = chain.iter().find_map(|(_, a)| a.budget_role);
                let is_evidence = chain.iter().any(|(_, a)| a.is_evidence);
                // The most-derived declaration that says anything about the value decides it.
                let valued = chain.iter().find(|(_, a)| {
                    a.value.is_some() || a.tbd.iter().any(|m| m.kind == TbdKind::Value)
                });
                let (value, declared_in, unit) = match valued {
                    Some((owner, a)) => {
                        let v = match a.value.as_deref() {
                            None => AttrValue::Tbd,
                            Some(raw) => match parse_number(raw) {
                                Some(n) => AttrValue::Number(n),
                                None => AttrValue::Expr(raw.trim().to_string()),
                            },
                        };
                        let unit = a
                            .unit
                            .clone()
                            .or_else(|| chain.iter().find_map(|(_, x)| x.unit.clone()));
                        (v, owner.to_string(), unit)
                    }
                    None => (
                        AttrValue::Absent,
                        chain[0].0.to_string(),
                        chain.iter().find_map(|(_, x)| x.unit.clone()),
                    ),
                };
                EffectiveAttr {
                    key,
                    role,
                    value,
                    unit,
                    is_evidence,
                    declared_in,
                }
            })
            .collect()
    }
}

/// One part feature of a definition (own or inherited) after resolving redefinition: the
/// most-derived usage decides, missing type/multiplicity are filled from the redefined one.
#[derive(Debug, Clone)]
pub struct EffectivePart<'a> {
    /// Usage name (an anonymous `part :>> battery` redefinition is keyed `battery`).
    pub key: String,
    pub type_name: Option<String>,
    pub multiplicity: Option<Multiplicity>,
    pub is_ref: bool,
    /// Usage-level attribute redefinitions, most-derived first.
    pub attributes: Vec<&'a AttributeUsage>,
    /// Usage-level nested parts, most-derived first.
    pub nested_parts: Vec<&'a PartUsage>,
}

/// Feature key of a part usage: its name, or the (simple) name of what it redefines.
pub fn part_key(p: &PartUsage) -> Option<String> {
    p.name
        .as_deref()
        .or_else(|| p.redefines.first().map(String::as_str))
        .map(|n| simple(n).to_string())
}

/// Merge `usages` (less derived than what `out` already holds) into `out` by feature key.
pub fn merge_part_usages<'a>(out: &mut Vec<EffectivePart<'a>>, usages: impl IntoIterator<Item = &'a PartUsage>) {
    for u in usages {
        let Some(key) = part_key(u) else { continue };
        match out.iter_mut().find(|e| e.key == key) {
            Some(e) => {
                if e.type_name.is_none() {
                    e.type_name = u.type_names.first().cloned();
                }
                if e.multiplicity.is_none() {
                    e.multiplicity = u.multiplicity.clone();
                }
                e.attributes.extend(u.attributes.iter());
                e.nested_parts.extend(u.nested_parts.iter());
            }
            None => out.push(EffectivePart {
                key,
                type_name: u.type_names.first().cloned(),
                multiplicity: u.multiplicity.clone(),
                is_ref: u.is_ref,
                attributes: u.attributes.iter().collect(),
                nested_parts: u.nested_parts.iter().collect(),
            }),
        }
    }
}

impl<'a> RoleIndex<'a> {
    /// Every part feature of `def`, own and inherited (most-derived usage wins).
    pub fn effective_parts(&self, def: &'a PartDef) -> Vec<EffectivePart<'a>> {
        let mut out = Vec::new();
        for d in self.lineage(def) {
            merge_part_usages(&mut out, d.parts.iter());
        }
        out
    }

    /// Apply usage-level attribute redefinitions (`part fan : Fan { attribute :>> ratedPower =
    /// 200 [W]; }`, most-derived first) on top of a definition's effective attributes.
    pub fn overlay_attributes(
        &self,
        mut base: Vec<EffectiveAttr>,
        overlays: &[&AttributeUsage],
        owner: &str,
    ) -> Vec<EffectiveAttr> {
        // Least-derived first so the most-derived overlay is applied last and wins.
        for a in overlays.iter().rev() {
            let Some(key) = attr_key(a) else { continue };
            let has_value = a.value.is_some() || a.tbd.iter().any(|m| m.kind == TbdKind::Value);
            let value = match a.value.as_deref() {
                None => AttrValue::Tbd,
                Some(raw) => match parse_number(raw) {
                    Some(n) => AttrValue::Number(n),
                    None => AttrValue::Expr(raw.trim().to_string()),
                },
            };
            match base.iter_mut().find(|e| e.key == key) {
                Some(e) => {
                    if a.budget_role.is_some() {
                        e.role = a.budget_role;
                    }
                    if a.unit.is_some() {
                        e.unit = a.unit.clone();
                    }
                    e.is_evidence |= a.is_evidence;
                    if has_value {
                        e.value = value;
                        e.declared_in = owner.to_string();
                    }
                }
                None => base.push(EffectiveAttr {
                    key,
                    role: a.budget_role,
                    value: if has_value { value } else { AttrValue::Absent },
                    unit: a.unit.clone(),
                    is_evidence: a.is_evidence,
                    declared_in: owner.to_string(),
                }),
            }
        }
        base
    }
}

/// Fixed instance count of a multiplicity: `Ok(n)` for `[n]` / `[n..n]` (no multiplicity = 1),
/// `Err(lower)` when the count is a range or unbounded (`lower` = its lower bound, `*` = 0).
pub fn instance_count(m: Option<&Multiplicity>) -> Result<u64, u64> {
    let Some(m) = m else { return Ok(1) };
    let num = |s: &str| s.trim().parse::<u64>().ok();
    match (m.lower.as_deref(), num(&m.upper)) {
        (None, Some(n)) => Ok(n),
        (Some(l), Some(u)) if num(l) == Some(u) => Ok(u),
        (Some(l), _) => Err(num(l).unwrap_or(0)),
        (None, None) => Err(0),
    }
}

/// Non-abstract, non-variation part definitions of the model's own (non-library) packages.
pub fn concrete_user_parts(model: &Model) -> Vec<&PartDef> {
    fn walk<'a>(pkg: &'a Package, out: &mut Vec<&'a PartDef>) {
        for m in &pkg.members {
            match m {
                PackageMember::Package(sub) => walk(sub, out),
                PackageMember::PartDef(p) if !p.is_abstract && !p.is_variation => out.push(p),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_classes_and_scales() {
        assert_eq!(dim_class(Some("kg")), Some(DimClass::Mass));
        assert_eq!(dim_class(Some("g")), Some(DimClass::Mass));
        assert_eq!(dim_class(Some("kW")), Some(DimClass::Power));
        assert_eq!(dim_class(Some("J/s")), Some(DimClass::Power));
        assert_eq!(dim_class(Some("min")), Some(DimClass::Time));
        assert_eq!(dim_class(Some("J")), Some(DimClass::Energy));
        assert_eq!(dim_class(Some("V")), Some(DimClass::Other));
        assert_eq!(dim_class(None), None);
        assert_eq!(base_scale(Some("kW")), Some(1000.0));
        assert_eq!(base_scale(Some("g")), Some(0.001));
        assert_eq!(base_scale(Some("W/m^2*m^2")), None);
    }

    #[test]
    fn role_is_inherited_through_redefinition_not_by_name() {
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
        let heater = PartDef {
            name: "Heater".into(),
            specializes: vec!["PowerElement".into()],
            attributes: vec![
                AttributeUsage {
                    redefines: vec!["ratedPower".into()],
                    value: Some("1800.0".into()),
                    unit: Some("W".into()),
                    ..Default::default()
                },
                // Same dimension, no role: never matched by its name.
                AttributeUsage {
                    name: Some("ratedPowerW".into()),
                    value: Some("2000.0".into()),
                    unit: Some("W".into()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        let model = Model::new(vec![Package {
            name: "P".into(),
            members: vec![PackageMember::PartDef(base), PackageMember::PartDef(heater)],
            ..Default::default()
        }]);
        let idx = RoleIndex::new(&model);
        let heater = idx.get("Heater").unwrap();
        let eff = idx.effective_attributes(heater);
        let rp = eff.iter().find(|e| e.key == "ratedPower").unwrap();
        assert_eq!(rp.role, Some(BudgetRole::Power));
        assert_eq!(rp.number(), Some(1800.0));
        let w = eff.iter().find(|e| e.key == "ratedPowerW").unwrap();
        assert_eq!(w.role, None);
        assert_eq!(w.dim(), Some(DimClass::Power));
    }
}
