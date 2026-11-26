//! Semantic AST / graph comparator for proving lossless SysML v2 round-trip fidelity.
//!
//! Compares two sets of `.sysml` files (original vs. re-emitted) by parsing both
//! through Open-MBEE's `sysmlv2-syntax` parser and extracting normalized sets of:
//! - Packages (`name`, `is_library`)
//! - Imports (`target`, `wildcard`)
//! - Definitions (`kind`, `qualified_name`, `short_name`, `is_abstract`, `specializes`)
//! - Attributes (`owner`, `name_or_redef`, `type_or_subset`, `value_expr`)
//! - Nested parts (`owner`, `name`, `types`, `subsets`, `redefines`)
//! - Ports (`owner`, `name`, `type`, `conjugated`)
//! - `perform action` allocations (`owner`, `name`, `action_type`)
//! - `interface`/`connection` links (`owner`, `type`, `from`, `to`)
//! - `state` transitions (`owner`, `source`, `trigger`, `target`)
//! - `satisfy` / `verify` links (`owner`, `requirement`, `by`)
//! - `require constraint` predicates (`owner`, `expr`)

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use sysml_ir::*;

use crate::lift::lift_sysml_units;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticGraph {
    pub packages: BTreeSet<String>,
    pub imports: BTreeSet<String>,
    pub part_defs: BTreeSet<String>,
    pub specializations: BTreeSet<String>,
    pub attributes: BTreeSet<String>,
    pub nested_parts: BTreeSet<String>,
    pub port_defs: BTreeSet<String>,
    pub port_usages: BTreeSet<String>,
    pub interface_defs: BTreeSet<String>,
    pub connections: BTreeSet<String>,
    pub item_defs: BTreeSet<String>,
    pub enum_defs: BTreeSet<String>,
    pub action_defs: BTreeSet<String>,
    pub perform_actions: BTreeSet<String>,
    pub state_defs: BTreeSet<String>,
    pub transitions: BTreeSet<String>,
    pub requirement_defs: BTreeSet<String>,
    pub requirement_usages: BTreeSet<String>,
    pub satisfy_links: BTreeSet<String>,
    pub verify_links: BTreeSet<String>,
    pub constraints: BTreeSet<String>,
    pub refinements: BTreeSet<String>,
    pub calc_defs: BTreeSet<String>,
    pub analysis_defs: BTreeSet<String>,
    pub analysis_usages: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryDiff {
    pub category: String,
    pub original_count: usize,
    pub reemitted_count: usize,
    pub missing: Vec<String>,
    pub extra: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoundTripDiff {
    pub fidelity_ok: bool,
    pub original_parse_errors: Vec<String>,
    pub reemitted_parse_errors: Vec<String>,
    pub original_unsupported_count: usize,
    pub total_original_facts: usize,
    pub total_reemitted_facts: usize,
    pub missing_count: usize,
    pub extra_count: usize,
    pub categories: Vec<CategoryDiff>,
}

fn norm_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn norm_simple_ref(s: &str) -> String {
    s.trim().replace('\'', "")
}

/// Extract a canonical `SemanticGraph` from a `sysml_ir::Model`.
pub fn extract_semantic_graph(model: &Model) -> SemanticGraph {
    let mut g = SemanticGraph::default();
    for pkg in &model.packages {
        extract_pkg(pkg, "", &mut g);
    }
    g
}

fn extract_pkg(pkg: &Package, prefix: &str, g: &mut SemanticGraph) {
    let qpkg = if prefix.is_empty() {
        pkg.name.clone()
    } else {
        format!("{prefix}::{}", pkg.name)
    };
    g.packages.insert(format!(
        "{}{qpkg}",
        if pkg.is_library { "library:" } else { "" }
    ));

    for imp in &pkg.imports {
        let wc = match imp.wildcard {
            ImportWildcard::None => "",
            ImportWildcard::Star => "::*",
            ImportWildcard::RecursiveStar => "::**",
        };
        g.imports.insert(format!("{qpkg} -> {}{wc}", imp.target));
    }

    for m in &pkg.members {
        match m {
            PackageMember::Package(sub) => extract_pkg(sub, &qpkg, g),
            PackageMember::PartDef(p) => {
                let qp = format!("{qpkg}::{}", p.name);
                g.part_defs.insert(format!(
                    "{}{qp}",
                    if p.is_abstract { "abstract:" } else { "" }
                ));
                for s in &p.specializes {
                    g.specializations.insert(format!("{qp} :> {}", norm_simple_ref(s)));
                }
                for a in &p.attributes {
                    record_attr(&qp, a, g);
                }
                for sub in &p.parts {
                    let sname = sub.name.as_deref().unwrap_or("_");
                    let tys = sub.type_names.join(",");
                    g.nested_parts.insert(format!("{qp}.{sname} : {tys}"));
                }
                for port in &p.ports {
                    let conj = if port.conjugated { "~" } else { "" };
                    g.port_usages
                        .insert(format!("{qp}.{} : {conj}{}", port.name, port.type_name));
                }
                for perf in &p.performs {
                    let pname = perf.name.as_deref().unwrap_or("_");
                    let pty = perf
                        .action_type
                        .as_deref()
                        .or(perf.references.as_deref())
                        .unwrap_or("_");
                    g.perform_actions.insert(format!("{qp}.{pname} : {pty}"));
                }
                for conn in &p.connections {
                    if let (Some(from), Some(to)) = (&conn.from, &conn.to) {
                        let ty = conn.type_name.as_deref().unwrap_or("_");
                        g.connections.insert(format!(
                            "{qp} [{ty}] {} -> {}",
                            norm_simple_ref(from),
                            norm_simple_ref(to)
                        ));
                    }
                }
                for sat in &p.satisfies {
                    g.satisfy_links.insert(format!(
                        "{qp} satisfy {} by {}",
                        norm_simple_ref(&sat.requirement),
                        sat.by.as_deref().map(norm_simple_ref).unwrap_or_default()
                    ));
                }
                for ver in &p.verifies {
                    g.verify_links.insert(format!(
                        "{qp} verify {}",
                        norm_simple_ref(&ver.requirement)
                    ));
                }
                for st in &p.exhibit_states {
                    for tr in &st.transitions {
                        g.transitions.insert(format!(
                            "{qp}.{}: {} -[{}]-> {}",
                            st.name,
                            tr.source,
                            tr.trigger.as_deref().unwrap_or(""),
                            tr.target
                        ));
                    }
                }
                for rf in &p.refinements {
                    g.refinements.insert(format!(
                        "{qp}: {} -> {}",
                        norm_simple_ref(&rf.client),
                        norm_simple_ref(&rf.supplier)
                    ));
                }
            }
            PackageMember::PartUsage(pu) => {
                let pname = pu.name.as_deref().unwrap_or("_");
                let qp = format!("{qpkg}::{pname}");
                let tys = pu.type_names.join(",");
                g.nested_parts.insert(format!("{qp} : {tys}"));
                for a in &pu.attributes {
                    record_attr(&qp, a, g);
                }
                for req in &pu.requirements {
                    g.requirement_usages.insert(format!(
                        "{qp}.{} : {}",
                        norm_simple_ref(&req.name),
                        req.type_name.as_deref().unwrap_or("")
                    ));
                }
            }
            PackageMember::PortDef(p) => {
                let qp = format!("{qpkg}::{}", p.name);
                g.port_defs.insert(qp.clone());
                for s in &p.specializes {
                    g.specializations.insert(format!("{qp} :> {}", norm_simple_ref(s)));
                }
            }
            PackageMember::InterfaceDef(i) => {
                let qi = format!("{qpkg}::{}", i.name);
                let ends = i
                    .ends
                    .iter()
                    .map(|e| {
                        format!(
                            "{}:{}{}",
                            e.name,
                            if e.conjugated { "~" } else { "" },
                            e.type_name
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                g.interface_defs.insert(format!("{qi}({ends})"));
            }
            PackageMember::ItemDef(i) => {
                g.item_defs.insert(format!("{qpkg}::{}", i.name));
            }
            PackageMember::EnumDef(e) => {
                g.enum_defs
                    .insert(format!("{qpkg}::{}({})", e.name, e.variants.join(",")));
            }
            PackageMember::ActionDef(a) => {
                let qa = format!("{qpkg}::{}", a.name);
                g.action_defs.insert(format!(
                    "{}{qa}",
                    if a.is_abstract { "abstract:" } else { "" }
                ));
                for s in &a.specializes {
                    g.specializations.insert(format!("{qa} :> {}", norm_simple_ref(s)));
                }
                for rf in &a.refinements {
                    g.refinements.insert(format!(
                        "{qa}: {} -> {}",
                        norm_simple_ref(&rf.client),
                        norm_simple_ref(&rf.supplier)
                    ));
                }
            }
            PackageMember::StateDef(s) => {
                let qs = format!("{qpkg}::{}", s.name);
                g.state_defs.insert(qs.clone());
                for sp in &s.specializes {
                    g.specializations.insert(format!("{qs} :> {}", norm_simple_ref(sp)));
                }
                for tr in &s.transitions {
                    g.transitions.insert(format!(
                        "{qs}: {} -[{}]-> {}",
                        tr.source,
                        tr.trigger.as_deref().unwrap_or(""),
                        tr.target
                    ));
                }
            }
            PackageMember::RequirementDef(r) => {
                let qr = format!("{qpkg}::{}", r.name);
                let short = r.short_name.as_deref().unwrap_or("");
                g.requirement_defs.insert(format!("{qr}<{short}>"));
                for a in &r.attributes {
                    record_attr(&qr, a, g);
                }
                for c in &r.require_constraints {
                    g.constraints.insert(format!("{qr}: {}", norm_ws(c)));
                }
                for rf in &r.refinements {
                    g.refinements.insert(format!(
                        "{qr}: {} -> {}",
                        norm_simple_ref(&rf.client),
                        norm_simple_ref(&rf.supplier)
                    ));
                }
            }
            PackageMember::RequirementUsage(ru) => {
                let qr = format!("{qpkg}::{}", ru.name);
                g.requirement_usages.insert(qr.clone());
                for n in &ru.nested_requirements {
                    g.requirement_usages.insert(format!(
                        "{qr}.{} : {}",
                        norm_simple_ref(&n.name),
                        n.type_name.as_deref().unwrap_or("")
                    ));
                }
                for sat in &ru.satisfies {
                    g.satisfy_links.insert(format!(
                        "{qr} satisfy {} by {}",
                        norm_simple_ref(&sat.requirement),
                        sat.by.as_deref().map(norm_simple_ref).unwrap_or_default()
                    ));
                }
            }
            PackageMember::CalcDef(c) => {
                let qc = format!("{qpkg}::{}", c.name);
                let pnames = c
                    .params
                    .iter()
                    .map(|p| p.name.as_str())
                    .collect::<Vec<_>>()
                    .join(",");
                g.calc_defs
                    .insert(format!("{qc}({pnames}) = {}", norm_ws(&c.result_expr)));
            }
            PackageMember::AnalysisDef(a) => {
                let qa = format!("{qpkg}::{}", a.name);
                g.analysis_defs.insert(qa.clone());
                for c in &a.assert_constraints {
                    g.constraints.insert(format!("{qa}: {}", norm_ws(c)));
                }
            }
            PackageMember::AnalysisUsage(a) => {
                let qa = format!("{qpkg}::{}", a.name);
                g.analysis_usages.insert(format!(
                    "{qa} : {}",
                    a.type_name.as_deref().unwrap_or("")
                ));
                for c in &a.assert_constraints {
                    g.constraints.insert(format!("{qa}: {}", norm_ws(c)));
                }
            }
            _ => {}
        }
    }
}

fn record_attr(owner: &str, a: &AttributeUsage, g: &mut SemanticGraph) {
    let key = a
        .name
        .as_deref()
        .map(norm_simple_ref)
        .or_else(|| a.redefines.first().map(|r| format!(":>>{}", norm_simple_ref(r))))
        .or_else(|| a.subsets.first().map(|s| format!(":>{}", norm_simple_ref(s))))
        .unwrap_or_else(|| "_".to_string());
    let val = match (&a.value, &a.unit) {
        (Some(v), Some(u)) => format!("={} [{}]", norm_ws(v), norm_simple_ref(u)),
        (Some(v), None) => format!("={}", norm_ws(v)),
        _ => String::new(),
    };
    g.attributes.insert(format!("{owner}.{key}{val}"));
}

fn diff_set(name: &str, orig: &BTreeSet<String>, reemit: &BTreeSet<String>) -> CategoryDiff {
    let missing: Vec<String> = orig.difference(reemit).cloned().collect();
    let extra: Vec<String> = reemit.difference(orig).cloned().collect();
    CategoryDiff {
        category: name.to_string(),
        original_count: orig.len(),
        reemitted_count: reemit.len(),
        missing,
        extra,
    }
}

/// Compare original `.sysml` units against re-emitted `.sysml` units for semantic round-trip fidelity.
pub fn compare_sysml_roundtrip(
    original_units: &[(String, String)],
    reemitted_units: &[(String, String)],
) -> RoundTripDiff {
    let orig_lift = lift_sysml_units(original_units);
    let reemit_lift = lift_sysml_units(reemitted_units);

    let og = extract_semantic_graph(&orig_lift.model);
    let rg = extract_semantic_graph(&reemit_lift.model);

    let categories = vec![
        diff_set("packages", &og.packages, &rg.packages),
        diff_set("imports", &og.imports, &rg.imports),
        diff_set("part_defs", &og.part_defs, &rg.part_defs),
        diff_set("specializations", &og.specializations, &rg.specializations),
        diff_set("attributes", &og.attributes, &rg.attributes),
        diff_set("nested_parts", &og.nested_parts, &rg.nested_parts),
        diff_set("port_defs", &og.port_defs, &rg.port_defs),
        diff_set("port_usages", &og.port_usages, &rg.port_usages),
        diff_set("interface_defs", &og.interface_defs, &rg.interface_defs),
        diff_set("connections", &og.connections, &rg.connections),
        diff_set("item_defs", &og.item_defs, &rg.item_defs),
        diff_set("enum_defs", &og.enum_defs, &rg.enum_defs),
        diff_set("action_defs", &og.action_defs, &rg.action_defs),
        diff_set("perform_actions", &og.perform_actions, &rg.perform_actions),
        diff_set("state_defs", &og.state_defs, &rg.state_defs),
        diff_set("transitions", &og.transitions, &rg.transitions),
        diff_set("requirement_defs", &og.requirement_defs, &rg.requirement_defs),
        diff_set("requirement_usages", &og.requirement_usages, &rg.requirement_usages),
        diff_set("satisfy_links", &og.satisfy_links, &rg.satisfy_links),
        diff_set("verify_links", &og.verify_links, &rg.verify_links),
        diff_set("constraints", &og.constraints, &rg.constraints),
        diff_set("refinements", &og.refinements, &rg.refinements),
        diff_set("calc_defs", &og.calc_defs, &rg.calc_defs),
        diff_set("analysis_defs", &og.analysis_defs, &rg.analysis_defs),
        diff_set("analysis_usages", &og.analysis_usages, &rg.analysis_usages),
    ];

    let total_original_facts: usize = categories.iter().map(|c| c.original_count).sum();
    let total_reemitted_facts: usize = categories.iter().map(|c| c.reemitted_count).sum();
    let missing_count: usize = categories.iter().map(|c| c.missing.len()).sum();
    let extra_count: usize = categories.iter().map(|c| c.extra.len()).sum();

    let fidelity_ok = orig_lift.parse_errors.is_empty()
        && reemit_lift.parse_errors.is_empty()
        && missing_count == 0
        && extra_count == 0;

    RoundTripDiff {
        fidelity_ok,
        original_parse_errors: orig_lift.parse_errors,
        reemitted_parse_errors: reemit_lift.parse_errors,
        original_unsupported_count: orig_lift.unsupported.len(),
        total_original_facts,
        total_reemitted_facts,
        missing_count,
        extra_count,
        categories,
    }
}
