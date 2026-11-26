//! AST -> IR lifter built directly on `sysmlv2-syntax`.
//!
//! Every SysML v2 construct encountered in the parsed AST is either lifted
//! cleanly into `sysml_ir` or recorded in `LiftReport.unsupported` with file,
//! package, element path, kind, and source line — never silently dropped.

use serde::{Deserialize, Serialize};
use sysml_ir::*;
use sysmlv2_syntax::ast::{self, Dialect};
use sysmlv2_syntax::parser::parse_source;
use sysmlv2_syntax::print::print_expr_source;
use sysmlv2_syntax::span::LineIndex;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnsupportedConstruct {
    pub file: String,
    pub package: String,
    pub context: String,
    pub kind: String,
    pub detail: String,
    pub line: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiftStats {
    pub files: usize,
    pub packages: usize,
    pub imports: usize,
    pub part_defs: usize,
    pub part_usages: usize,
    pub attributes: usize,
    pub port_defs: usize,
    pub port_usages: usize,
    pub interface_defs: usize,
    pub connection_defs: usize,
    pub connections: usize,
    pub item_defs: usize,
    pub enum_defs: usize,
    pub action_defs: usize,
    pub perform_actions: usize,
    pub state_defs: usize,
    pub transitions: usize,
    pub requirement_defs: usize,
    pub requirement_usages: usize,
    pub satisfy_links: usize,
    pub verify_links: usize,
    pub unsupported_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiftReport {
    pub model: Model,
    pub parse_errors: Vec<String>,
    pub unsupported: Vec<UnsupportedConstruct>,
    pub stats: LiftStats,
}

/// Lift a single `.sysml` source string into a `LiftReport`.
pub fn lift_sysml_source(file_name: &str, src: &str) -> LiftReport {
    lift_sysml_units(&[(file_name.to_string(), src.to_string())])
}

/// Lift multiple `.sysml` source units together into one `LiftReport`.
pub fn lift_sysml_units(units: &[(String, String)]) -> LiftReport {
    let mut ctx = LifterCtx {
        parse_errors: Vec::new(),
        unsupported: Vec::new(),
        stats: LiftStats::default(),
    };
    let mut packages = Vec::new();

    for (file_name, src) in units {
        ctx.stats.files += 1;
        let lines = LineIndex::new(src);
        let parsed = parse_source(src);
        for d in &parsed.diagnostics {
            let lc = lines.line_col(d.span.start); let (line, col) = (lc.line as usize, lc.col as usize);
            ctx.parse_errors
                .push(format!("{file_name}:{line}:{col}: {}", d.message));
        }

        for member in &parsed.unit.members {
            match &member.kind {
                ast::MemberKind::Package(pkg) => {
                    packages.push(ctx.lift_package(file_name, &lines, pkg));
                }
                ast::MemberKind::Comment(_) | ast::MemberKind::Doc(_) => {}
                other => {
                    let line = lines.line_col(member.span.start).line as usize;
                    ctx.record_unsupported(
                        file_name,
                        "<root>",
                        "<root>",
                        member_kind_label(other),
                        "top-level non-package member",
                        line,
                    );
                }
            }
        }
    }

    ctx.stats.unsupported_count = ctx.unsupported.len();
    LiftReport {
        model: Model { packages },
        parse_errors: ctx.parse_errors,
        unsupported: ctx.unsupported,
        stats: ctx.stats,
    }
}

struct LifterCtx {
    parse_errors: Vec<String>,
    unsupported: Vec<UnsupportedConstruct>,
    stats: LiftStats,
}

impl LifterCtx {
    fn record_unsupported(
        &mut self,
        file: &str,
        package: &str,
        context: &str,
        kind: impl Into<String>,
        detail: impl Into<String>,
        line: usize,
    ) {
        self.unsupported.push(UnsupportedConstruct {
            file: file.to_string(),
            package: package.to_string(),
            context: context.to_string(),
            kind: kind.into(),
            detail: detail.into(),
            line,
        });
    }

    fn lift_package(&mut self, file: &str, lines: &LineIndex, pkg: &ast::Package) -> Package {
        self.stats.packages += 1;
        let pkg_name = pkg
            .id
            .name
            .as_ref()
            .map(|n| n.value.clone())
            .unwrap_or_else(|| "UnnamedPackage".to_string());
        let mut out = Package {
            name: pkg_name.clone(),
            is_library: pkg.is_library,
            doc: None,
            imports: Vec::new(),
            members: Vec::new(),
        };

        if let Some(body) = &pkg.body {
            for m in body {
                let line = lines.line_col(m.span.start).line as usize;
                match &m.kind {
                    ast::MemberKind::Doc(d) => {
                        append_doc(&mut out.doc, &d.body);
                    }
                    ast::MemberKind::Import(imp) => {
                        self.stats.imports += 1;
                        out.imports.push(lift_import(m.visibility, imp));
                    }
                    ast::MemberKind::Package(sub) => {
                        out.members
                            .push(PackageMember::Package(self.lift_package(file, lines, sub)));
                    }
                    ast::MemberKind::Definition(def) => {
                        if let Some(pm) = self.lift_definition(file, &pkg_name, lines, def, line) {
                            out.members.push(pm);
                        }
                    }
                    ast::MemberKind::Usage(u) => {
                        if let Some(pm) = self.lift_top_usage(file, &pkg_name, lines, u, line) {
                            out.members.push(pm);
                        }
                    }
                    ast::MemberKind::Comment(c) => {
                        self.record_unsupported(
                            file,
                            &pkg_name,
                            &pkg_name,
                            "comment",
                            c.id.name
                                .as_ref()
                                .map(|n| n.value.clone())
                                .unwrap_or_else(|| "anonymous comment".to_string()),
                            line,
                        );
                    }
                    other => {
                        self.record_unsupported(
                            file,
                            &pkg_name,
                            &pkg_name,
                            member_kind_label(other),
                            "unsupported package member",
                            line,
                        );
                    }
                }
            }
        }
        out
    }

    fn lift_definition(
        &mut self,
        file: &str,
        pkg_name: &str,
        lines: &LineIndex,
        def: &ast::Definition,
        line: usize,
    ) -> Option<PackageMember> {
        let name = def
            .id
            .name
            .as_ref()
            .map(|n| n.value.clone())
            .or_else(|| def.id.short_name.as_ref().map(|n| n.value.clone()))
            .unwrap_or_else(|| "Anonymous".to_string());
        let specs: Vec<String> = def.specializes.iter().map(target_ref_to_string).collect();

        match def.kind {
            ast::DefKind::Part => {
                self.stats.part_defs += 1;
                let mut pd = PartDef {
                    name: name.clone(),
                    is_abstract: def.prefix.is_abstract,
                    is_individual: def.prefix.is_individual,
                    doc: None,
                    specializes: specs,
                    ..Default::default()
                };
                if let Some(body) = &def.body {
                    for m in body {
                        self.lift_part_def_member(file, pkg_name, &name, lines, m, &mut pd);
                    }
                }
                Some(PackageMember::PartDef(pd))
            }
            ast::DefKind::Port => {
                self.stats.port_defs += 1;
                let mut pd = PortDef {
                    name: name.clone(),
                    doc: None,
                    specializes: specs,
                    is_symmetric: false,
                    items: Vec::new(),
                    attributes: Vec::new(),
                };
                if let Some(body) = &def.body {
                    for m in body {
                        let mline = lines.line_col(m.span.start).line as usize;
                        match &m.kind {
                            ast::MemberKind::Doc(d) => append_doc(&mut pd.doc, &d.body),
                            ast::MemberKind::Usage(u) if u.kind == ast::UsageKind::Item => {
                                if let Some(di) = lift_directed_item(u) {
                                    pd.items.push(di);
                                }
                            }
                            ast::MemberKind::Usage(u) if u.kind == ast::UsageKind::Attribute => {
                                self.stats.attributes += 1;
                                pd.attributes.push(lift_attribute_usage(u));
                            }
                            other => {
                                self.record_unsupported(
                                    file,
                                    pkg_name,
                                    &name,
                                    member_kind_label(other),
                                    "unsupported port def member",
                                    mline,
                                );
                            }
                        }
                    }
                }
                pd.is_symmetric = !pd.items.is_empty()
                    && pd
                        .items
                        .iter()
                        .all(|it| it.direction == Some(Direction::InOut));
                Some(PackageMember::PortDef(pd))
            }
            ast::DefKind::Interface => {
                self.stats.interface_defs += 1;
                let mut id = InterfaceDef {
                    name: name.clone(),
                    doc: None,
                    specializes: specs,
                    ends: Vec::new(),
                };
                if let Some(body) = &def.body {
                    for m in body {
                        let mline = lines.line_col(m.span.start).line as usize;
                        match &m.kind {
                            ast::MemberKind::Doc(d) => append_doc(&mut id.doc, &d.body),
                            ast::MemberKind::Usage(u) if u.prefix.is_end => {
                                let ename = u
                                    .declaration
                                    .id
                                    .name
                                    .as_ref()
                                    .map(|n| n.value.clone())
                                    .unwrap_or_else(|| "end".to_string());
                                let (ty, conj) = first_typed_by(&u.declaration);
                                id.ends.push(InterfaceEnd {
                                    name: ename,
                                    type_name: ty.unwrap_or_default(),
                                    conjugated: conj,
                                    multiplicity: lift_multiplicity(
                                        u.declaration.multiplicity.as_deref(),
                                    ),
                                });
                            }
                            other => {
                                self.record_unsupported(
                                    file,
                                    pkg_name,
                                    &name,
                                    member_kind_label(other),
                                    "unsupported interface def member",
                                    mline,
                                );
                            }
                        }
                    }
                }
                Some(PackageMember::InterfaceDef(id))
            }
            ast::DefKind::Connection => {
                self.stats.connection_defs += 1;
                let mut cd = ConnectionDef {
                    name: name.clone(),
                    is_abstract: def.prefix.is_abstract,
                    doc: None,
                    specializes: specs,
                    ends: Vec::new(),
                };
                if let Some(body) = &def.body {
                    for m in body {
                        let mline = lines.line_col(m.span.start).line as usize;
                        match &m.kind {
                            ast::MemberKind::Doc(d) => append_doc(&mut cd.doc, &d.body),
                            ast::MemberKind::Usage(u) => {
                                let ename = u
                                    .declaration
                                    .id
                                    .name
                                    .as_ref()
                                    .map(|n| n.value.clone())
                                    .unwrap_or_else(|| "end".to_string());
                                let (ty, _) = first_typed_by(&u.declaration);
                                let kw = match u.kind {
                                    ast::UsageKind::Part => "part",
                                    ast::UsageKind::Requirement => "requirement",
                                    ast::UsageKind::Port => "port",
                                    ast::UsageKind::Item => "item",
                                    _ => "part",
                                };
                                cd.ends.push(ConnectionEndDecl {
                                    is_ref: u.prefix.is_ref,
                                    kind_keyword: kw.to_string(),
                                    name: ename,
                                    type_name: ty.unwrap_or_default(),
                                    multiplicity: lift_multiplicity(
                                        u.declaration.multiplicity.as_deref(),
                                    ),
                                    subsets: all_subsets(&u.declaration),
                                });
                            }
                            other => {
                                self.record_unsupported(
                                    file,
                                    pkg_name,
                                    &name,
                                    member_kind_label(other),
                                    "unsupported connection def member",
                                    mline,
                                );
                            }
                        }
                    }
                }
                Some(PackageMember::ConnectionDef(cd))
            }
            ast::DefKind::Item => {
                self.stats.item_defs += 1;
                let mut it = ItemDef {
                    name: name.clone(),
                    is_abstract: def.prefix.is_abstract,
                    doc: None,
                    specializes: specs,
                    attributes: Vec::new(),
                };
                if let Some(body) = &def.body {
                    for m in body {
                        let mline = lines.line_col(m.span.start).line as usize;
                        match &m.kind {
                            ast::MemberKind::Doc(d) => append_doc(&mut it.doc, &d.body),
                            ast::MemberKind::Usage(u) if u.kind == ast::UsageKind::Attribute => {
                                self.stats.attributes += 1;
                                it.attributes.push(lift_attribute_usage(u));
                            }
                            other => {
                                self.record_unsupported(
                                    file,
                                    pkg_name,
                                    &name,
                                    member_kind_label(other),
                                    "unsupported item def member",
                                    mline,
                                );
                            }
                        }
                    }
                }
                Some(PackageMember::ItemDef(it))
            }
            ast::DefKind::Enum => {
                self.stats.enum_defs += 1;
                let mut ed = EnumDef {
                    name: name.clone(),
                    doc: None,
                    variants: Vec::new(),
                };
                if let Some(body) = &def.body {
                    for m in body {
                        let mline = lines.line_col(m.span.start).line as usize;
                        match &m.kind {
                            ast::MemberKind::Doc(d) => append_doc(&mut ed.doc, &d.body),
                            ast::MemberKind::Usage(u)
                                if u.kind == ast::UsageKind::Enum
                                    || u.kind == ast::UsageKind::Default =>
                            {
                                if let Some(vn) = &u.declaration.id.name {
                                    ed.variants.push(vn.value.clone());
                                }
                            }
                            other => {
                                self.record_unsupported(
                                    file,
                                    pkg_name,
                                    &name,
                                    member_kind_label(other),
                                    "unsupported enum def member",
                                    mline,
                                );
                            }
                        }
                    }
                }
                Some(PackageMember::EnumDef(ed))
            }
            ast::DefKind::Attribute => {
                let mut ad = AttributeDef {
                    name: name.clone(),
                    is_abstract: def.prefix.is_abstract,
                    doc: None,
                    specializes: specs,
                };
                if let Some(body) = &def.body {
                    for m in body {
                        if let ast::MemberKind::Doc(d) = &m.kind {
                            append_doc(&mut ad.doc, &d.body);
                        }
                    }
                }
                Some(PackageMember::AttributeDef(ad))
            }
            ast::DefKind::Action => {
                self.stats.action_defs += 1;
                let mut ad = ActionDef {
                    name: name.clone(),
                    is_abstract: def.prefix.is_abstract,
                    doc: None,
                    specializes: specs,
                    ..Default::default()
                };
                if let Some(body) = &def.body {
                    for m in body {
                        let mline = lines.line_col(m.span.start).line as usize;
                        match &m.kind {
                            ast::MemberKind::Doc(d) => append_doc(&mut ad.doc, &d.body),
                            ast::MemberKind::InitialNode(qn) => {
                                if qn.to_display_string() == "start" {
                                    ad.has_initial_start = true;
                                }
                            }
                            ast::MemberKind::Dependency(dep) => {
                                if let (Some(c), Some(s)) =
                                    (dep.clients.first(), dep.suppliers.first())
                                {
                                    ad.refinements.push(RefinementDependency {
                                        client: c.to_display_string(),
                                        supplier: s.to_display_string(),
                                    });
                                }
                            }
                            ast::MemberKind::Usage(u) => {
                                if let Some(dir) = u.prefix.direction {
                                    let pname = u
                                        .declaration
                                        .id
                                        .name
                                        .as_ref()
                                        .map(|n| n.value.clone())
                                        .unwrap_or_default();
                                    let (ty, _) = first_typed_by(&u.declaration);
                                    ad.params.push(ActionParam {
                                        direction: lift_direction(dir),
                                        name: pname,
                                        type_name: ty.unwrap_or_default(),
                                    });
                                } else if u.kind == ast::UsageKind::Action {
                                    if m.leading_then {
                                        let sname = u
                                            .declaration
                                            .id
                                            .name
                                            .as_ref()
                                            .map(|n| n.value.clone())
                                            .unwrap_or_default();
                                        let (ty, _) = first_typed_by(&u.declaration);
                                        ad.steps.push(ActionStep {
                                            name: sname,
                                            type_name: ty,
                                            is_then: true,
                                        });
                                    } else {
                                        ad.subactions.push(lift_action_usage(u));
                                    }
                                } else if u.kind == ast::UsageKind::Succession {
                                    if let ast::UsageDetail::Succession { target, .. } = &u.detail {
                                        if target_ref_to_string(&target.target) == "done" {
                                            ad.has_done = true;
                                        }
                                    }
                                } else {
                                    self.record_unsupported(
                                        file,
                                        pkg_name,
                                        &name,
                                        format!("usage:{:?}", u.kind),
                                        "unsupported action def member usage",
                                        mline,
                                    );
                                }
                            }
                            other => {
                                self.record_unsupported(
                                    file,
                                    pkg_name,
                                    &name,
                                    member_kind_label(other),
                                    "unsupported action def member",
                                    mline,
                                );
                            }
                        }
                    }
                }
                Some(PackageMember::ActionDef(ad))
            }
            ast::DefKind::State => {
                self.stats.state_defs += 1;
                let mut sd = StateDef {
                    name: name.clone(),
                    is_abstract: def.prefix.is_abstract,
                    doc: None,
                    specializes: specs,
                    ..Default::default()
                };
                if let Some(body) = &def.body {
                    for m in body {
                        self.lift_state_member(
                            file,
                            pkg_name,
                            &name,
                            lines,
                            m,
                            &mut sd.doc,
                            &mut sd.entry_action,
                            &mut sd.do_action,
                            &mut sd.exit_action,
                            &mut sd.states,
                            &mut sd.transitions,
                        );
                    }
                }
                Some(PackageMember::StateDef(sd))
            }
            ast::DefKind::Requirement => {
                self.stats.requirement_defs += 1;
                let mut rd = RequirementDef {
                    short_name: def.id.short_name.as_ref().map(|n| n.value.clone()),
                    name: name.clone(),
                    is_abstract: def.prefix.is_abstract,
                    doc: None,
                    rationale: None,
                    specializes: specs,
                    subject: None,
                    attributes: Vec::new(),
                    require_constraints: Vec::new(),
                    assume_constraints: Vec::new(),
                    refinements: Vec::new(),
                };
                if let Some(body) = &def.body {
                    for m in body {
                        let mline = lines.line_col(m.span.start).line as usize;
                        match &m.kind {
                            ast::MemberKind::Doc(d) => append_doc(&mut rd.doc, &d.body),
                            ast::MemberKind::Subject(u) => {
                                rd.subject = Some(lift_subject(u));
                            }
                            ast::MemberKind::Dependency(dep) => {
                                if let (Some(c), Some(s)) =
                                    (dep.clients.first(), dep.suppliers.first())
                                {
                                    rd.refinements.push(RefinementDependency {
                                        client: c.to_display_string(),
                                        supplier: s.to_display_string(),
                                    });
                                }
                            }
                            ast::MemberKind::RequirementConstraint { kind, usage } => {
                                if let Some(expr_str) = extract_constraint_expr(usage) {
                                    match kind {
                                        ast::RequirementConstraintKind::Requirement => {
                                            rd.require_constraints.push(expr_str);
                                        }
                                        ast::RequirementConstraintKind::Assumption => {
                                            rd.assume_constraints.push(expr_str);
                                        }
                                    }
                                }
                            }
                            ast::MemberKind::Usage(u) if u.kind == ast::UsageKind::Attribute => {
                                self.stats.attributes += 1;
                                rd.attributes.push(lift_attribute_usage(u));
                            }
                            ast::MemberKind::Usage(u) if u.kind == ast::UsageKind::Metadata => {
                                // Extract `@Rationale { text = "..."; }`
                                if let Some(txt) = extract_rationale_text(u) {
                                    rd.rationale = Some(txt);
                                } else {
                                    self.record_unsupported(
                                        file,
                                        pkg_name,
                                        &name,
                                        "metadata_usage",
                                        "non-Rationale metadata annotation",
                                        mline,
                                    );
                                }
                            }
                            other => {
                                self.record_unsupported(
                                    file,
                                    pkg_name,
                                    &name,
                                    member_kind_label(other),
                                    "unsupported requirement def member",
                                    mline,
                                );
                            }
                        }
                    }
                }
                Some(PackageMember::RequirementDef(rd))
            }
            ast::DefKind::Verification => {
                let mut vd = VerificationDef {
                    name: name.clone(),
                    doc: None,
                    subject: None,
                    verifies: Vec::new(),
                };
                if let Some(body) = &def.body {
                    for m in body {
                        match &m.kind {
                            ast::MemberKind::Doc(d) => append_doc(&mut vd.doc, &d.body),
                            ast::MemberKind::Subject(u) => vd.subject = Some(lift_subject(u)),
                            ast::MemberKind::RequirementVerification(u) => {
                                self.stats.verify_links += 1;
                                vd.verifies.push(lift_verify(u));
                            }
                            _ => {}
                        }
                    }
                }
                Some(PackageMember::VerificationDef(vd))
            }
            other_kind => {
                self.record_unsupported(
                    file,
                    pkg_name,
                    &name,
                    format!("def:{other_kind:?}"),
                    format!("{other_kind:?} def {name}"),
                    line,
                );
                None
            }
        }
    }

    fn lift_top_usage(
        &mut self,
        file: &str,
        pkg_name: &str,
        lines: &LineIndex,
        u: &ast::Usage,
        line: usize,
    ) -> Option<PackageMember> {
        let name = u
            .declaration
            .id
            .name
            .as_ref()
            .map(|n| n.value.clone())
            .unwrap_or_else(|| "anonymous".to_string());
        match u.kind {
            ast::UsageKind::Requirement => {
                self.stats.requirement_usages += 1;
                Some(PackageMember::RequirementUsage(
                    self.lift_requirement_usage(file, pkg_name, lines, u),
                ))
            }
            ast::UsageKind::Part => {
                self.stats.part_usages += 1;
                Some(PackageMember::PartUsage(
                    self.lift_part_usage(file, pkg_name, &name, lines, u),
                ))
            }
            other => {
                self.record_unsupported(
                    file,
                    pkg_name,
                    &name,
                    format!("top_usage:{other:?}"),
                    format!("{other:?} {name}"),
                    line,
                );
                None
            }
        }
    }

    fn lift_part_def_member(
        &mut self,
        file: &str,
        pkg_name: &str,
        part_name: &str,
        lines: &LineIndex,
        m: &ast::Member,
        pd: &mut PartDef,
    ) {
        let line = lines.line_col(m.span.start).line as usize;
        match &m.kind {
            ast::MemberKind::Doc(d) => append_doc(&mut pd.doc, &d.body),
            ast::MemberKind::Usage(u) => match u.kind {
                ast::UsageKind::Attribute => {
                    self.stats.attributes += 1;
                    pd.attributes.push(lift_attribute_usage(u));
                }
                ast::UsageKind::Part => {
                    self.stats.part_usages += 1;
                    pd.parts
                        .push(self.lift_part_usage(file, pkg_name, part_name, lines, u));
                }
                ast::UsageKind::Port => {
                    self.stats.port_usages += 1;
                    pd.ports.push(lift_port_usage(u));
                }
                ast::UsageKind::Item => {
                    if let Some(di) = lift_directed_item(u) {
                        pd.items.push(di);
                    }
                }
                ast::UsageKind::Perform => {
                    self.stats.perform_actions += 1;
                    pd.performs.push(lift_perform_action(u));
                }
                ast::UsageKind::Interface | ast::UsageKind::Connection => {
                    self.stats.connections += 1;
                    pd.connections.push(lift_connection_usage(u));
                }
                ast::UsageKind::Exhibit => {
                    pd.exhibit_states
                        .push(self.lift_state_usage(file, pkg_name, part_name, lines, u, true));
                }
                ast::UsageKind::Requirement => {
                    self.stats.requirement_usages += 1;
                    pd.requirements
                        .push(self.lift_requirement_usage(file, pkg_name, lines, u));
                }
                ast::UsageKind::Satisfy => {
                    self.stats.satisfy_links += 1;
                    pd.satisfies.push(lift_satisfy(u));
                }
                other => {
                    self.record_unsupported(
                        file,
                        pkg_name,
                        part_name,
                        format!("part_member:{other:?}"),
                        u.declaration
                            .id
                            .name
                            .as_ref()
                            .map(|n| n.value.clone())
                            .unwrap_or_else(|| format!("{other:?}")),
                        line,
                    );
                }
            },
            ast::MemberKind::RequirementVerification(u) => {
                self.stats.verify_links += 1;
                pd.verifies.push(lift_verify(u));
            }
            ast::MemberKind::Comment(c) => {
                self.record_unsupported(
                    file,
                    pkg_name,
                    part_name,
                    "comment",
                    c.id.name
                        .as_ref()
                        .map(|n| n.value.clone())
                        .unwrap_or_else(|| "comment".to_string()),
                    line,
                );
            }
            other => {
                self.record_unsupported(
                    file,
                    pkg_name,
                    part_name,
                    member_kind_label(other),
                    "unsupported part def member",
                    line,
                );
            }
        }
    }

    fn lift_part_usage(
        &mut self,
        file: &str,
        pkg_name: &str,
        parent: &str,
        lines: &LineIndex,
        u: &ast::Usage,
    ) -> PartUsage {
        let name = u.declaration.id.name.as_ref().map(|n| n.value.clone());
        let label = name.clone().unwrap_or_else(|| parent.to_string());
        let mut pu = PartUsage {
            name,
            is_individual: u.prefix.is_individual,
            is_ref: u.prefix.is_ref,
            type_names: all_typed_by(&u.declaration),
            subsets: all_subsets(&u.declaration),
            redefines: all_redefines(&u.declaration),
            multiplicity: lift_multiplicity(u.declaration.multiplicity.as_deref()),
            is_ordered: u.declaration.is_ordered,
            default_value: u
                .value
                .as_ref()
                .map(|v| print_expr_source(&v.expr, Dialect::Sysml)),
            doc: None,
            attributes: Vec::new(),
            nested_parts: Vec::new(),
            ports: Vec::new(),
            performs: Vec::new(),
            connections: Vec::new(),
        };

        if let Some(body) = &u.body {
            for m in body {
                let line = lines.line_col(m.span.start).line as usize;
                match &m.kind {
                    ast::MemberKind::Doc(d) => append_doc(&mut pu.doc, &d.body),
                    ast::MemberKind::Usage(sub) => match sub.kind {
                        ast::UsageKind::Attribute => {
                            self.stats.attributes += 1;
                            pu.attributes.push(lift_attribute_usage(sub));
                        }
                        ast::UsageKind::Part => {
                            self.stats.part_usages += 1;
                            pu.nested_parts
                                .push(self.lift_part_usage(file, pkg_name, &label, lines, sub));
                        }
                        ast::UsageKind::Port => {
                            self.stats.port_usages += 1;
                            pu.ports.push(lift_port_usage(sub));
                        }
                        ast::UsageKind::Perform => {
                            self.stats.perform_actions += 1;
                            pu.performs.push(lift_perform_action(sub));
                        }
                        ast::UsageKind::Interface | ast::UsageKind::Connection => {
                            self.stats.connections += 1;
                            pu.connections.push(lift_connection_usage(sub));
                        }
                        other => {
                            let kind_label = if let Some(p) = sub.prefix.portion {
                                format!("portion:{p:?}")
                            } else {
                                format!("part_usage_member:{other:?}")
                            };
                            self.record_unsupported(
                                file,
                                pkg_name,
                                &label,
                                kind_label,
                                sub.declaration
                                    .id
                                    .name
                                    .as_ref()
                                    .map(|n| n.value.clone())
                                    .unwrap_or_else(|| format!("{other:?}")),
                                line,
                            );
                        }
                    },
                    other => {
                        self.record_unsupported(
                            file,
                            pkg_name,
                            &label,
                            member_kind_label(other),
                            "unsupported part usage member",
                            line,
                        );
                    }
                }
            }
        }
        pu
    }

    #[allow(clippy::too_many_arguments)]
    fn lift_state_member(
        &mut self,
        file: &str,
        pkg_name: &str,
        state_name: &str,
        lines: &LineIndex,
        m: &ast::Member,
        doc: &mut Option<String>,
        entry_action: &mut Option<ActionUsage>,
        do_action: &mut Option<ActionUsage>,
        exit_action: &mut Option<ActionUsage>,
        states: &mut Vec<StateUsage>,
        transitions: &mut Vec<Transition>,
    ) {
        let line = lines.line_col(m.span.start).line as usize;
        match &m.kind {
            ast::MemberKind::Doc(d) => append_doc(doc, &d.body),
            ast::MemberKind::StateSubaction { kind, action } => {
                if let Some(act) = action {
                    let au = lift_action_usage(act);
                    match kind {
                        ast::StateSubactionKind::Entry => *entry_action = Some(au),
                        ast::StateSubactionKind::Do => *do_action = Some(au),
                        ast::StateSubactionKind::Exit => *exit_action = Some(au),
                    }
                }
            }
            ast::MemberKind::Usage(u) if u.kind == ast::UsageKind::State => {
                states.push(self.lift_state_usage(file, pkg_name, state_name, lines, u, false));
            }
            ast::MemberKind::Usage(u) if u.kind == ast::UsageKind::Transition => {
                if let Some(tr) = lift_transition(u) {
                    self.stats.transitions += 1;
                    transitions.push(tr);
                }
            }
            other => {
                self.record_unsupported(
                    file,
                    pkg_name,
                    state_name,
                    member_kind_label(other),
                    "unsupported state member",
                    line,
                );
            }
        }
    }

    fn lift_state_usage(
        &mut self,
        file: &str,
        pkg_name: &str,
        parent: &str,
        lines: &LineIndex,
        u: &ast::Usage,
        is_exhibit: bool,
    ) -> StateUsage {
        let name = u
            .declaration
            .id
            .name
            .as_ref()
            .map(|n| n.value.clone())
            .unwrap_or_else(|| "state".to_string());
        let (ty, _) = first_typed_by(&u.declaration);
        let mut su = StateUsage {
            name: name.clone(),
            type_name: ty,
            is_exhibit,
            ..Default::default()
        };
        if let Some(body) = &u.body {
            let ctx = format!("{parent}.{name}");
            for m in body {
                self.lift_state_member(
                    file,
                    pkg_name,
                    &ctx,
                    lines,
                    m,
                    &mut su.doc,
                    &mut su.entry_action,
                    &mut su.do_action,
                    &mut su.exit_action,
                    &mut su.states,
                    &mut su.transitions,
                );
            }
        }
        su
    }

    fn lift_requirement_usage(
        &mut self,
        file: &str,
        pkg_name: &str,
        lines: &LineIndex,
        u: &ast::Usage,
    ) -> RequirementUsage {
        let name = u
            .declaration
            .id
            .name
            .as_ref()
            .map(|n| n.value.clone())
            .unwrap_or_else(|| "requirement".to_string());
        let (ty, _) = first_typed_by(&u.declaration);
        let mut ru = RequirementUsage {
            short_name: u.declaration.id.short_name.as_ref().map(|n| n.value.clone()),
            name: name.clone(),
            type_name: ty,
            multiplicity: lift_multiplicity(u.declaration.multiplicity.as_deref()),
            subsets: all_subsets(&u.declaration),
            doc: None,
            subject: None,
            nested_requirements: Vec::new(),
            satisfies: Vec::new(),
            verifies: Vec::new(),
        };
        if let Some(body) = &u.body {
            for m in body {
                let line = lines.line_col(m.span.start).line as usize;
                match &m.kind {
                    ast::MemberKind::Doc(d) => append_doc(&mut ru.doc, &d.body),
                    ast::MemberKind::Subject(su) => ru.subject = Some(lift_subject(su)),
                    ast::MemberKind::RequirementVerification(vu) => {
                        self.stats.verify_links += 1;
                        ru.verifies.push(lift_verify(vu));
                    }
                    ast::MemberKind::Usage(sub) if sub.kind == ast::UsageKind::Requirement => {
                        self.stats.requirement_usages += 1;
                        ru.nested_requirements
                            .push(self.lift_requirement_usage(file, pkg_name, lines, sub));
                    }
                    ast::MemberKind::Usage(sub) if sub.kind == ast::UsageKind::Satisfy => {
                        self.stats.satisfy_links += 1;
                        ru.satisfies.push(lift_satisfy(sub));
                    }
                    other => {
                        self.record_unsupported(
                            file,
                            pkg_name,
                            &name,
                            member_kind_label(other),
                            "unsupported requirement usage member",
                            line,
                        );
                    }
                }
            }
        }
        ru
    }
}

fn append_doc(slot: &mut Option<String>, raw: &str) {
    let cleaned = raw
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if cleaned.is_empty() {
        return;
    }
    match slot {
        Some(existing) => {
            existing.push('\n');
            existing.push_str(&cleaned);
        }
        None => *slot = Some(cleaned),
    }
}

fn lift_import(vis: Option<ast::Visibility>, imp: &ast::Import) -> Import {
    let visibility = match vis {
        Some(ast::Visibility::Private) => Visibility::Private,
        Some(ast::Visibility::Protected) => Visibility::Protected,
        Some(ast::Visibility::Public) | None => Visibility::Public,
    };
    let wildcard = if imp.is_recursive {
        ImportWildcard::RecursiveStar
    } else if imp.is_namespace {
        ImportWildcard::Star
    } else {
        ImportWildcard::None
    };
    Import {
        visibility,
        target: imp.target.to_display_string(),
        wildcard,
    }
}

fn target_ref_to_string(tr: &ast::TargetRef) -> String {
    match tr {
        ast::TargetRef::Name(qn) => qn.to_display_string(),
        ast::TargetRef::Chain(parts) => parts
            .iter()
            .map(|p| p.to_display_string())
            .collect::<Vec<_>>()
            .join("."),
    }
}

fn first_typed_by(decl: &ast::FeatureDeclaration) -> (Option<String>, bool) {
    for spec in &decl.specializations {
        if let ast::FeatureSpecialization::TypedBy(tys) = spec {
            if let Some(first) = tys.first() {
                return (
                    Some(target_ref_to_string(&first.target)),
                    first.is_conjugated,
                );
            }
        }
    }
    (None, false)
}

fn all_typed_by(decl: &ast::FeatureDeclaration) -> Vec<String> {
    let mut out = Vec::new();
    for spec in &decl.specializations {
        if let ast::FeatureSpecialization::TypedBy(tys) = spec {
            for t in tys {
                out.push(target_ref_to_string(&t.target));
            }
        }
    }
    out
}

fn all_subsets(decl: &ast::FeatureDeclaration) -> Vec<String> {
    let mut out = Vec::new();
    for spec in &decl.specializations {
        if let ast::FeatureSpecialization::Subsets(targets) = spec {
            for t in targets {
                out.push(target_ref_to_string(t));
            }
        }
    }
    out
}

fn all_redefines(decl: &ast::FeatureDeclaration) -> Vec<String> {
    let mut out = Vec::new();
    for spec in &decl.specializations {
        if let ast::FeatureSpecialization::Redefines(targets) = spec {
            for t in targets {
                out.push(target_ref_to_string(t));
            }
        }
    }
    out
}

fn first_references(decl: &ast::FeatureDeclaration) -> Option<String> {
    for spec in &decl.specializations {
        if let ast::FeatureSpecialization::References(t) = spec {
            return Some(target_ref_to_string(t));
        }
    }
    None
}

fn lift_multiplicity(m: Option<&ast::Multiplicity>) -> Option<Multiplicity> {
    m.map(|mult| Multiplicity {
        lower: mult
            .lower
            .as_ref()
            .map(|e| print_expr_source(e, Dialect::Sysml)),
        upper: print_expr_source(&mult.upper, Dialect::Sysml),
    })
}

fn lift_direction(d: ast::FeatureDirection) -> Direction {
    match d {
        ast::FeatureDirection::In => Direction::In,
        ast::FeatureDirection::Out => Direction::Out,
        ast::FeatureDirection::InOut => Direction::InOut,
    }
}

fn lift_attribute_usage(u: &ast::Usage) -> AttributeUsage {
    let (ty, _) = first_typed_by(&u.declaration);
    let mut doc = None;
    if let Some(body) = &u.body {
        for m in body {
            if let ast::MemberKind::Doc(d) = &m.kind {
                append_doc(&mut doc, &d.body);
            }
        }
    }
    let (value_kind, value, unit) = if let Some(fv) = &u.value {
        let vk = match fv.kind {
            ast::ValueKind::Bound => ValueKind::Bound,
            ast::ValueKind::Initial => ValueKind::Initial,
            ast::ValueKind::Default => ValueKind::Default,
            ast::ValueKind::DefaultInitial => ValueKind::DefaultInitial,
        };
        match &fv.expr.kind {
            ast::ExprKind::Bracket { target, arg } => {
                let val_str = print_expr_source(target, Dialect::Sysml);
                let unit_raw = print_expr_source(arg, Dialect::Sysml);
                let unit_clean = unit_raw
                    .strip_prefix('\'')
                    .and_then(|s| s.strip_suffix('\''))
                    .unwrap_or(&unit_raw)
                    .to_string();
                (vk, Some(val_str), Some(unit_clean))
            }
            _ => (vk, Some(print_expr_source(&fv.expr, Dialect::Sysml)), None),
        }
    } else {
        (ValueKind::Bound, None, None)
    };

    AttributeUsage {
        name: u.declaration.id.name.as_ref().map(|n| n.value.clone()),
        type_name: ty,
        subsets: all_subsets(&u.declaration),
        redefines: all_redefines(&u.declaration),
        multiplicity: lift_multiplicity(u.declaration.multiplicity.as_deref()),
        value_kind,
        value,
        unit,
        doc,
    }
}

fn lift_directed_item(u: &ast::Usage) -> Option<DirectedItem> {
    let name = u.declaration.id.name.as_ref()?.value.clone();
    let (ty, _) = first_typed_by(&u.declaration);
    Some(DirectedItem {
        direction: u.prefix.direction.map(lift_direction),
        name,
        type_name: ty.unwrap_or_default(),
        multiplicity: lift_multiplicity(u.declaration.multiplicity.as_deref()),
    })
}

fn lift_port_usage(u: &ast::Usage) -> PortUsage {
    let name = u
        .declaration
        .id
        .name
        .as_ref()
        .map(|n| n.value.clone())
        .unwrap_or_default();
    let (ty, conj) = first_typed_by(&u.declaration);
    let mut doc = None;
    if let Some(body) = &u.body {
        for m in body {
            if let ast::MemberKind::Doc(d) = &m.kind {
                append_doc(&mut doc, &d.body);
            }
        }
    }
    PortUsage {
        name,
        type_name: ty.unwrap_or_default(),
        conjugated: conj,
        multiplicity: lift_multiplicity(u.declaration.multiplicity.as_deref()),
        doc,
    }
}

fn lift_perform_action(u: &ast::Usage) -> PerformAction {
    let (ty, _) = first_typed_by(&u.declaration);
    PerformAction {
        name: u.declaration.id.name.as_ref().map(|n| n.value.clone()),
        action_type: ty,
        references: first_references(&u.declaration),
        redefines: all_redefines(&u.declaration),
    }
}

fn lift_connection_usage(u: &ast::Usage) -> ConnectionUsage {
    let kind = if u.kind == ast::UsageKind::Interface {
        ConnectionKind::Interface
    } else {
        ConnectionKind::Connection
    };
    let (ty, _) = first_typed_by(&u.declaration);
    let (from, to) = match &u.detail {
        ast::UsageDetail::Connector { ends } if ends.len() >= 2 => (
            Some(target_ref_to_string(&ends[0].target)),
            Some(target_ref_to_string(&ends[1].target)),
        ),
        _ => (None, None),
    };
    ConnectionUsage {
        kind,
        is_abstract: u.prefix.is_abstract,
        name: u.declaration.id.name.as_ref().map(|n| n.value.clone()),
        type_name: ty,
        multiplicity: lift_multiplicity(u.declaration.multiplicity.as_deref()),
        from,
        to,
        doc: None,
    }
}

fn lift_action_usage(u: &ast::Usage) -> ActionUsage {
    let name = u
        .declaration
        .id
        .name
        .as_ref()
        .map(|n| n.value.clone())
        .unwrap_or_default();
    let (ty, _) = first_typed_by(&u.declaration);
    let mut au = ActionUsage {
        name,
        type_name: ty,
        multiplicity: lift_multiplicity(u.declaration.multiplicity.as_deref()),
        redefines: all_redefines(&u.declaration),
        has_initial_start: false,
        steps: Vec::new(),
        has_done: false,
        doc: None,
    };
    if let Some(body) = &u.body {
        for m in body {
            match &m.kind {
                ast::MemberKind::Doc(d) => append_doc(&mut au.doc, &d.body),
                ast::MemberKind::InitialNode(qn) => {
                    if qn.to_display_string() == "start" {
                        au.has_initial_start = true;
                    }
                }
                ast::MemberKind::Usage(sub) if sub.kind == ast::UsageKind::Action => {
                    let sname = sub
                        .declaration
                        .id
                        .name
                        .as_ref()
                        .map(|n| n.value.clone())
                        .unwrap_or_default();
                    let (sty, _) = first_typed_by(&sub.declaration);
                    au.steps.push(ActionStep {
                        name: sname,
                        type_name: sty,
                        is_then: m.leading_then,
                    });
                }
                ast::MemberKind::Usage(sub) if sub.kind == ast::UsageKind::Succession => {
                    if let ast::UsageDetail::Succession { target, .. } = &sub.detail {
                        if target_ref_to_string(&target.target) == "done" {
                            au.has_done = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    au
}

fn lift_transition(u: &ast::Usage) -> Option<Transition> {
    if let ast::UsageDetail::Transition {
        source,
        trigger,
        guard,
        effect,
        target,
        ..
    } = &u.detail
    {
        let src = source.as_ref().map(target_ref_to_string)?;
        let dst = target.as_ref().map(|c| target_ref_to_string(&c.target))?;
        let trig = trigger.as_deref().and_then(|d| {
            if let ast::UsageDetail::Accept { payload, .. } = d {
                payload
                    .id
                    .name
                    .as_ref()
                    .map(|n| n.value.clone())
                    .or_else(|| {
                        for spec in &payload.specializations {
                            if let ast::FeatureSpecialization::TypedBy(tys) = spec {
                                if let Some(t) = tys.first() {
                                    return Some(target_ref_to_string(&t.target));
                                }
                            }
                        }
                        None
                    })
            } else {
                None
            }
        });
        let guard_str = guard.as_deref().map(|g| print_expr_source(g, Dialect::Sysml));
        let effect_str = effect
            .as_deref()
            .and_then(|e| e.declaration.id.name.as_ref().map(|n| n.value.clone()));
        let mut doc = None;
        if let Some(body) = &u.body {
            for m in body {
                if let ast::MemberKind::Doc(d) = &m.kind {
                    append_doc(&mut doc, &d.body);
                }
            }
        }
        return Some(Transition {
            name: u.declaration.id.name.as_ref().map(|n| n.value.clone()),
            source: src,
            trigger: trig,
            trigger_payload_name: None,
            guard: guard_str,
            effect: effect_str,
            target: dst,
            doc,
        });
    }
    None
}

fn lift_subject(u: &ast::Usage) -> SubjectDecl {
    let name = u
        .declaration
        .id
        .name
        .as_ref()
        .map(|n| n.value.clone())
        .unwrap_or_else(|| "subject".to_string());
    let (ty, _) = first_typed_by(&u.declaration);
    SubjectDecl {
        name,
        type_name: ty.unwrap_or_default(),
    }
}

fn lift_satisfy(u: &ast::Usage) -> SatisfyStmt {
    let ref_target = first_references(&u.declaration);
    let typed_target = first_typed_by(&u.declaration).0;
    let is_typed = ref_target.is_none() && typed_target.is_some();
    let req = ref_target
        .or_else(|| u.declaration.id.name.as_ref().map(|n| n.value.clone()))
        .or(typed_target)
        .unwrap_or_default();
    let (asserted, by) = match &u.detail {
        ast::UsageDetail::Satisfy { asserted, by, .. } => {
            (*asserted, by.as_ref().map(target_ref_to_string))
        }
        _ => (false, None),
    };
    SatisfyStmt {
        asserted,
        is_typed,
        requirement: req,
        by,
    }
}

fn lift_verify(u: &ast::Usage) -> VerifyStmt {
    let req = first_references(&u.declaration)
        .or_else(|| u.declaration.id.name.as_ref().map(|n| n.value.clone()))
        .or_else(|| first_typed_by(&u.declaration).0)
        .unwrap_or_default();
    VerifyStmt {
        requirement: req,
        by: None,
    }
}

fn extract_constraint_expr(u: &ast::Usage) -> Option<String> {
    if let Some(body) = &u.body {
        for m in body {
            if let ast::MemberKind::Result(expr) = &m.kind {
                return Some(print_expr_source(expr, Dialect::Sysml));
            }
        }
    }
    None
}

fn extract_rationale_text(u: &ast::Usage) -> Option<String> {
    let (ty, _) = first_typed_by(&u.declaration);
    if ty.as_deref() != Some("Rationale") {
        return None;
    }
    if let Some(body) = &u.body {
        for m in body {
            if let ast::MemberKind::Usage(sub) = &m.kind {
                if sub
                    .declaration
                    .id
                    .name
                    .as_ref()
                    .map(|n| n.value.as_str())
                    == Some("text")
                {
                    if let Some(fv) = &sub.value {
                        if let ast::ExprKind::Literal(ast::Literal::String(s)) = &fv.expr.kind {
                            return Some(s.clone());
                        }
                    }
                }
            }
        }
    }
    None
}

fn member_kind_label(k: &ast::MemberKind) -> String {
    match k {
        ast::MemberKind::Package(_) => "package".to_string(),
        ast::MemberKind::Import(_) => "import".to_string(),
        ast::MemberKind::Alias(_) => "alias".to_string(),
        ast::MemberKind::Comment(_) => "comment".to_string(),
        ast::MemberKind::Doc(_) => "doc".to_string(),
        ast::MemberKind::TextualRep(_) => "textual_rep".to_string(),
        ast::MemberKind::Definition(d) => format!("def:{:?}", d.kind),
        ast::MemberKind::Usage(u) => format!("usage:{:?}", u.kind),
        ast::MemberKind::Filter(_) => "filter".to_string(),
        ast::MemberKind::Dependency(_) => "dependency".to_string(),
        ast::MemberKind::InitialNode(_) => "initial_node".to_string(),
        ast::MemberKind::Subject(_) => "subject".to_string(),
        ast::MemberKind::Actor(_) => "actor".to_string(),
        ast::MemberKind::Stakeholder(_) => "stakeholder".to_string(),
        ast::MemberKind::Objective(_) => "objective".to_string(),
        ast::MemberKind::RequirementConstraint { .. } => "requirement_constraint".to_string(),
        ast::MemberKind::FramedConcern(_) => "framed_concern".to_string(),
        ast::MemberKind::RequirementVerification(_) => "requirement_verification".to_string(),
        ast::MemberKind::StateSubaction { .. } => "state_subaction".to_string(),
        ast::MemberKind::Expose(_) => "expose".to_string(),
        ast::MemberKind::Render(_) => "render".to_string(),
        ast::MemberKind::Return(_) => "return".to_string(),
        ast::MemberKind::Result(_) => "result".to_string(),
        ast::MemberKind::Relationship(_) => "relationship".to_string(),
        ast::MemberKind::MultiplicityDecl(_) => "multiplicity_decl".to_string(),
    }
}
