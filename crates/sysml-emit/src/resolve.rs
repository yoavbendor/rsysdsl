//! Model-level name resolution fix-ups applied before emission.
//!
//! `#[sysml::satisfy(req = other_pkg::Req, ...)]` and `#[sysml::verify(...)]` record only the
//! requirement's simple SysML name (`Req`). When the requirement lives in another package that
//! the satisfying package does not import, the emitted `satisfy Req;` would not resolve and the
//! strict OMG audit fails with `unresolved reference 'Req'`. rustc cannot see this (Rust paths
//! resolve fine), so the emitter qualifies such references (`satisfy OtherPkg::Req;`).

use std::collections::{BTreeMap, BTreeSet};
use sysml_ir::{
    AttributeUsage, Import, ImportWildcard, Model, Package, PackageMember, PartDef, PartUsage,
    PortionUsage, RequirementUsage, SatisfyStmt, TbdKind, VerifyStmt,
    CUSTOM_UNIT_METADATA_PACKAGE,
};

/// Qualify every unqualified `satisfy` / `verify` requirement reference that is not visible
/// from its owning package (not declared in that package or an enclosing one, and not
/// brought in by an import) but is declared as a requirement in exactly one other package.
///
/// Ambiguous or unknown names are left untouched so the validator still reports them.
pub fn qualify_requirement_refs(model: &Model) -> Model {
    // Qualified package path -> simple names of requirements declared directly in it.
    let mut pkg_reqs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for pkg in &model.packages {
        index_package(pkg, "", &mut pkg_reqs);
    }
    // Simple requirement name -> qualified names of every declaration.
    let mut by_name: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (pkg_path, names) in &pkg_reqs {
        for n in names {
            by_name
                .entry(n.clone())
                .or_default()
                .push(format!("{pkg_path}::{n}"));
        }
    }

    let mut out = model.clone();
    for pkg in &mut out.packages {
        rewrite_package(pkg, "", &[], &pkg_reqs, &by_name);
    }
    out
}

/// Auto-insert `private import ScalarValues::*;` on any non-library package that references
/// an unqualified OMG `ScalarValues` type (`Real`, `Integer`, `Boolean`, `String`, `Natural`,
/// `Positive`) without already importing `ScalarValues` (directly or from an enclosing package).
pub fn ensure_scalar_values_import(model: &mut Model) {
    for pkg in &mut model.packages {
        ensure_scalar_in_package(pkg, false);
    }
}

fn is_scalar_value_type(ty: &str) -> bool {
    matches!(
        ty.trim(),
        "Real" | "Integer" | "Boolean" | "String" | "Natural" | "Positive"
    )
}

fn imports_scalar_values(pkg: &Package) -> bool {
    pkg.imports.iter().any(|i| {
        let t = i.target.trim_start_matches("$::");
        t == "ScalarValues" || t.starts_with("ScalarValues::")
    })
}

fn attrs_use_scalar(attrs: &[AttributeUsage]) -> bool {
    attrs.iter().any(|a| {
        a.type_name.as_deref().is_some_and(is_scalar_value_type)
            || attrs_use_scalar(&a.nested_attributes)
    })
}

fn portions_use_scalar(portions: &[PortionUsage]) -> bool {
    portions.iter().any(|p| {
        attrs_use_scalar(&p.attributes)
            || part_usages_use_scalar(&p.parts)
            || portions_use_scalar(&p.portions)
    })
}

fn part_usages_use_scalar(parts: &[PartUsage]) -> bool {
    parts.iter().any(|p| {
        attrs_use_scalar(&p.attributes)
            || p.items.iter().any(|it| is_scalar_value_type(&it.type_name))
            || part_usages_use_scalar(&p.nested_parts)
            || req_usages_use_scalar(&p.requirements)
            || portions_use_scalar(&p.portions)
    })
}

fn req_usages_use_scalar(reqs: &[RequirementUsage]) -> bool {
    reqs.iter()
        .any(|r| req_usages_use_scalar(&r.nested_requirements))
}

fn package_uses_scalar_direct(pkg: &Package) -> bool {
    pkg.members.iter().any(|m| match m {
        PackageMember::Package(_) => false,
        PackageMember::PartDef(p) => {
            attrs_use_scalar(&p.attributes)
                || part_usages_use_scalar(&p.parts)
                || p.items.iter().any(|it| is_scalar_value_type(&it.type_name))
                || req_usages_use_scalar(&p.requirements)
        }
        PackageMember::PartUsage(p) => part_usages_use_scalar(std::slice::from_ref(p)),
        PackageMember::PortDef(p) => {
            attrs_use_scalar(&p.attributes)
                || p.items.iter().any(|it| is_scalar_value_type(&it.type_name))
        }
        PackageMember::InterfaceDef(i) => attrs_use_scalar(&i.attributes),
        PackageMember::ItemDef(i) => attrs_use_scalar(&i.attributes),
        PackageMember::AttributeDef(a) => attrs_use_scalar(&a.attributes),
        PackageMember::AttributeUsage(a) => attrs_use_scalar(std::slice::from_ref(a)),
        PackageMember::ActionDef(a) => a.params.iter().any(|p| is_scalar_value_type(&p.type_name)),
        PackageMember::RequirementDef(r) => attrs_use_scalar(&r.attributes),
        PackageMember::RequirementUsage(r) => req_usages_use_scalar(std::slice::from_ref(r)),
        PackageMember::CalcDef(c) | PackageMember::CalcUsage(c) => {
            is_scalar_value_type(&c.return_type)
                || c.params.iter().any(|p| is_scalar_value_type(&p.type_name))
        }
        PackageMember::AnalysisDef(a) => a
            .params
            .iter()
            .any(|p| p.type_name.as_deref().is_some_and(is_scalar_value_type)),
        PackageMember::AnalysisUsage(a) => a
            .params
            .iter()
            .any(|p| p.type_name.as_deref().is_some_and(is_scalar_value_type)),
        PackageMember::MetadataDef(m) => attrs_use_scalar(&m.attributes),
        _ => false,
    })
}

fn ensure_scalar_in_package(pkg: &mut Package, parent_has_scalar: bool) {
    let mut has_scalar = parent_has_scalar || imports_scalar_values(pkg);
    if !has_scalar && !pkg.is_library && package_uses_scalar_direct(pkg) {
        pkg.imports.insert(0, Import::private_star("ScalarValues"));
        has_scalar = true;
    }
    for m in &mut pkg.members {
        if let PackageMember::Package(sub) = m {
            ensure_scalar_in_package(sub, has_scalar);
        }
    }
}

/// Qualify `custom_unit` attribute units that are neither OMG SI symbols nor already defined
/// as attributes in any package of `model`, rewriting `[rpm]` -> `[CustomUnitMetadata::rpm]`
/// and returning the set of custom unit symbols that [`crate::emit::custom_unit_metadata_package`]
/// must declare.
pub fn resolve_custom_units(model: &mut Model) -> BTreeSet<String> {
    let mut defined_units = BTreeSet::new();
    for pkg in &model.packages {
        collect_defined_unit_names(pkg, &mut defined_units);
    }
    let mut needed = BTreeSet::new();
    for pkg in &mut model.packages {
        if pkg.name == CUSTOM_UNIT_METADATA_PACKAGE {
            continue;
        }
        rewrite_custom_units_in_pkg(pkg, &defined_units, &mut needed);
    }
    needed
}

fn collect_defined_unit_names(pkg: &Package, out: &mut BTreeSet<String>) {
    for m in &pkg.members {
        match m {
            PackageMember::Package(sub) => collect_defined_unit_names(sub, out),
            PackageMember::AttributeUsage(a) => {
                if let Some(n) = &a.name {
                    out.insert(n.trim_matches('\'').to_string());
                }
                if let Some(s) = &a.short_name {
                    out.insert(s.trim_matches('\'').to_string());
                }
            }
            PackageMember::AttributeDef(a) => {
                out.insert(a.name.trim_matches('\'').to_string());
            }
            _ => {}
        }
    }
}

fn rewrite_custom_unit_attrs(
    attrs: &mut [AttributeUsage],
    defined_units: &BTreeSet<String>,
    needed: &mut BTreeSet<String>,
) {
    for a in attrs {
        if a.value.is_some() && a.tbd.iter().any(|m| m.kind == TbdKind::CustomUnit) {
            if let Some(u) = &a.unit {
                let raw = u.trim();
                let clean = raw
                    .strip_prefix('\'')
                    .and_then(|s| s.strip_suffix('\''))
                    .unwrap_or(raw)
                    .trim();
                if !clean.is_empty()
                    && !clean.contains("::")
                    && !matches!(
                        sysml_core::units::check_unit(clean),
                        sysml_core::units::UnitCheck::Ok(_)
                    )
                    && !defined_units.contains(clean)
                {
                    needed.insert(clean.to_string());
                    a.unit = Some(format!("{CUSTOM_UNIT_METADATA_PACKAGE}::{clean}"));
                }
            }
        }
        rewrite_custom_unit_attrs(&mut a.nested_attributes, defined_units, needed);
    }
}

fn rewrite_custom_unit_parts(
    parts: &mut [PartUsage],
    defined_units: &BTreeSet<String>,
    needed: &mut BTreeSet<String>,
) {
    for p in parts {
        rewrite_custom_unit_attrs(&mut p.attributes, defined_units, needed);
        rewrite_custom_unit_parts(&mut p.nested_parts, defined_units, needed);
        rewrite_custom_unit_portions(&mut p.portions, defined_units, needed);
    }
}

fn rewrite_custom_unit_portions(
    portions: &mut [PortionUsage],
    defined_units: &BTreeSet<String>,
    needed: &mut BTreeSet<String>,
) {
    for p in portions {
        rewrite_custom_unit_attrs(&mut p.attributes, defined_units, needed);
        rewrite_custom_unit_parts(&mut p.parts, defined_units, needed);
        rewrite_custom_unit_portions(&mut p.portions, defined_units, needed);
    }
}

fn rewrite_custom_units_in_pkg(
    pkg: &mut Package,
    defined_units: &BTreeSet<String>,
    needed: &mut BTreeSet<String>,
) {
    for m in &mut pkg.members {
        match m {
            PackageMember::Package(sub) => {
                rewrite_custom_units_in_pkg(sub, defined_units, needed);
            }
            PackageMember::PartDef(p) => {
                rewrite_custom_unit_attrs(&mut p.attributes, defined_units, needed);
                rewrite_custom_unit_parts(&mut p.parts, defined_units, needed);
            }
            PackageMember::PartUsage(p) => {
                rewrite_custom_unit_parts(std::slice::from_mut(p), defined_units, needed);
            }
            PackageMember::PortDef(p) => {
                rewrite_custom_unit_attrs(&mut p.attributes, defined_units, needed);
            }
            PackageMember::ItemDef(i) => {
                rewrite_custom_unit_attrs(&mut i.attributes, defined_units, needed);
            }
            PackageMember::InterfaceDef(i) => {
                rewrite_custom_unit_attrs(&mut i.attributes, defined_units, needed);
            }
            PackageMember::RequirementDef(r) => {
                rewrite_custom_unit_attrs(&mut r.attributes, defined_units, needed);
            }
            PackageMember::AttributeDef(a) => {
                rewrite_custom_unit_attrs(&mut a.attributes, defined_units, needed);
            }
            PackageMember::AttributeUsage(a) => {
                rewrite_custom_unit_attrs(std::slice::from_mut(a), defined_units, needed);
            }
            _ => {}
        }
    }
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}::{name}")
    }
}

fn index_package(pkg: &Package, prefix: &str, out: &mut BTreeMap<String, BTreeSet<String>>) {
    let path = join(prefix, &pkg.name);
    let entry = out.entry(path.clone()).or_default();
    let mut nested = Vec::new();
    for m in &pkg.members {
        match m {
            PackageMember::RequirementDef(r) => {
                entry.insert(r.name.clone());
            }
            PackageMember::RequirementUsage(r) => {
                entry.insert(r.name.clone());
            }
            PackageMember::Package(p) => nested.push(p),
            _ => {}
        }
    }
    for p in nested {
        index_package(p, &path, out);
    }
}

/// Names visible inside `path` through its own and enclosing scopes plus their imports.
fn visible_names(
    path: &str,
    scope_imports: &[(String, ImportWildcard)],
    pkg_reqs: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut vis = BTreeSet::new();
    // Own + enclosing packages.
    let mut cur = Some(path.to_string());
    while let Some(p) = cur {
        if let Some(names) = pkg_reqs.get(&p) {
            vis.extend(names.iter().cloned());
        }
        cur = p.rsplit_once("::").map(|(head, _)| head.to_string());
    }
    for (target, wildcard) in scope_imports {
        let target = target.trim_start_matches("$::");
        // Resolve the import target as absolute, or relative to an enclosing package.
        let mut candidates = vec![target.to_string()];
        let mut cur = Some(path.to_string());
        while let Some(p) = cur {
            candidates.push(format!("{p}::{target}"));
            cur = p.rsplit_once("::").map(|(head, _)| head.to_string());
        }
        match wildcard {
            ImportWildcard::None => {
                // `import Pkg::Req;` makes `Req` visible.
                if let Some((_, last)) = target.rsplit_once("::") {
                    vis.insert(last.to_string());
                } else {
                    vis.insert(target.to_string());
                }
            }
            ImportWildcard::Star => {
                for c in &candidates {
                    if let Some(names) = pkg_reqs.get(c) {
                        vis.extend(names.iter().cloned());
                    }
                }
            }
            ImportWildcard::RecursiveStar => {
                for c in &candidates {
                    let nested_prefix = format!("{c}::");
                    for (p, names) in pkg_reqs {
                        if p == c || p.starts_with(&nested_prefix) {
                            vis.extend(names.iter().cloned());
                        }
                    }
                }
            }
        }
    }
    vis
}

fn rewrite_package(
    pkg: &mut Package,
    prefix: &str,
    outer_imports: &[(String, ImportWildcard)],
    pkg_reqs: &BTreeMap<String, BTreeSet<String>>,
    by_name: &BTreeMap<String, Vec<String>>,
) {
    let path = join(prefix, &pkg.name);
    let mut imports: Vec<(String, ImportWildcard)> = outer_imports.to_vec();
    imports.extend(pkg.imports.iter().map(|i| (i.target.clone(), i.wildcard)));
    let visible = visible_names(&path, &imports, pkg_reqs);
    let fix = |name: &mut String| {
        if name.contains("::") || visible.contains(name.as_str()) {
            return;
        }
        if let Some(decls) = by_name.get(name.as_str()) {
            if decls.len() == 1 {
                *name = decls[0].clone();
            }
        }
    };
    for m in &mut pkg.members {
        match m {
            PackageMember::Package(p) => rewrite_package(p, &path, &imports, pkg_reqs, by_name),
            PackageMember::PartDef(p) => rewrite_part_def(p, &fix),
            PackageMember::RequirementUsage(r) => rewrite_req_usage(r, &fix),
            PackageMember::VerificationDef(v) => {
                for s in &mut v.verifies {
                    rewrite_verify(s, &fix)
                }
            }
            _ => {}
        }
    }
}

fn rewrite_satisfy(s: &mut SatisfyStmt, fix: &impl Fn(&mut String)) {
    fix(&mut s.requirement);
}

fn rewrite_verify(v: &mut VerifyStmt, fix: &impl Fn(&mut String)) {
    fix(&mut v.requirement);
}

fn rewrite_part_def(p: &mut PartDef, fix: &impl Fn(&mut String)) {
    for s in &mut p.satisfies {
        rewrite_satisfy(s, fix);
    }
    for v in &mut p.verifies {
        rewrite_verify(v, fix);
    }
    for r in &mut p.requirements {
        rewrite_req_usage(r, fix);
    }
}

fn rewrite_req_usage(r: &mut RequirementUsage, fix: &impl Fn(&mut String)) {
    for s in &mut r.satisfies {
        rewrite_satisfy(s, fix);
    }
    for v in &mut r.verifies {
        rewrite_verify(v, fix);
    }
    for n in &mut r.nested_requirements {
        rewrite_req_usage(n, fix);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysml_ir::{Import, RequirementDef, TbdMark, TbdPhase, ValueKind};

    fn req_pkg() -> Package {
        Package {
            name: "Reqs".into(),
            members: vec![PackageMember::RequirementDef(RequirementDef {
                name: "MaxPower".into(),
                ..Default::default()
            })],
            ..Default::default()
        }
    }

    fn arch_pkg(imports: Vec<Import>) -> Package {
        Package {
            name: "Arch".into(),
            imports,
            members: vec![PackageMember::PartDef(PartDef {
                name: "Psu".into(),
                satisfies: vec![SatisfyStmt {
                    requirement: "MaxPower".into(),
                    ..Default::default()
                }],
                ..Default::default()
            })],
            ..Default::default()
        }
    }

    fn satisfy_target(m: &Model) -> String {
        match &m.packages[1].members[0] {
            PackageMember::PartDef(p) => p.satisfies[0].requirement.clone(),
            _ => unreachable!(),
        }
    }

    #[test]
    fn qualifies_unimported_cross_package_satisfy() {
        let m = Model::new(vec![req_pkg(), arch_pkg(vec![])]);
        assert_eq!(satisfy_target(&qualify_requirement_refs(&m)), "Reqs::MaxPower");
    }

    #[test]
    fn leaves_imported_satisfy_alone() {
        let m = Model::new(vec![req_pkg(), arch_pkg(vec![Import::private_star("Reqs")])]);
        assert_eq!(satisfy_target(&qualify_requirement_refs(&m)), "MaxPower");
    }

    #[test]
    fn auto_imports_scalar_values_only_when_used() {
        let mut m = Model::new(vec![
            Package {
                name: "UsesScalar".into(),
                members: vec![PackageMember::PartDef(PartDef {
                    name: "Pump".into(),
                    attributes: vec![AttributeUsage {
                        name: Some("speed".into()),
                        type_name: Some("Real".into()),
                        ..Default::default()
                    }],
                    ..Default::default()
                })],
                ..Default::default()
            },
            Package {
                name: "NoScalar".into(),
                members: vec![PackageMember::PartDef(PartDef {
                    name: "Box".into(),
                    ..Default::default()
                })],
                ..Default::default()
            },
        ]);
        ensure_scalar_values_import(&mut m);
        assert_eq!(m.packages[0].imports, vec![Import::private_star("ScalarValues")]);
        assert!(m.packages[1].imports.is_empty());
    }

    #[test]
    fn resolves_undefined_custom_units_to_metadata_package() {
        let mut m = Model::new(vec![Package {
            name: "Demo".into(),
            members: vec![PackageMember::PartDef(PartDef {
                name: "Fan".into(),
                attributes: vec![AttributeUsage {
                    name: Some("speed".into()),
                    type_name: Some("Real".into()),
                    value_kind: ValueKind::Default,
                    value: Some("2400.0".into()),
                    unit: Some("rpm".into()),
                    tbd: vec![TbdMark {
                        kind: TbdKind::CustomUnit,
                        reason: "datasheet gives rpm".into(),
                        owner: None,
                        resolve_by: TbdPhase::Detailed,
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            })],
            ..Default::default()
        }]);
        let needed = resolve_custom_units(&mut m);
        assert!(needed.contains("rpm"));
        if let PackageMember::PartDef(p) = &m.packages[0].members[0] {
            assert_eq!(
                p.attributes[0].unit.as_deref(),
                Some("CustomUnitMetadata::rpm")
            );
        } else {
            unreachable!();
        }
    }
}
