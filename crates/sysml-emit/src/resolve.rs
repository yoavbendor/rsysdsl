//! Model-level name resolution fix-ups applied before emission.
//!
//! `#[sysml::satisfy(req = other_pkg::Req, ...)]` and `#[sysml::verify(...)]` record only the
//! requirement's simple SysML name (`Req`). When the requirement lives in another package that
//! the satisfying package does not import, the emitted `satisfy Req;` would not resolve and the
//! strict OMG audit fails with `unresolved reference 'Req'`. rustc cannot see this (Rust paths
//! resolve fine), so the emitter qualifies such references (`satisfy OtherPkg::Req;`).

use std::collections::{BTreeMap, BTreeSet};
use sysml_ir::{
    ImportWildcard, Model, Package, PackageMember, PartDef, RequirementUsage, SatisfyStmt,
    VerifyStmt,
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
    use sysml_ir::{Import, RequirementDef};

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
}
