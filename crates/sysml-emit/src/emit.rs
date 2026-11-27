//! 1:1 Deterministic IR -> OMG SysML v2 textual notation emitter.

use std::fmt::Write;
use sysml_ir::*;

/// SysML v2 reserved keywords that must be single-quoted if used as an identifier.
const SYSML_KEYWORDS: &[&str] = &[
    "about", "abstract", "accept", "action", "actor", "after", "alias", "all", "allocate",
    "allocation", "analysis", "and", "as", "assert", "assign", "assume", "at", "attribute",
    "bind", "binding", "by", "calc", "case", "comment", "concern", "connect", "connection",
    "constraint", "decide", "def", "default", "defined", "dependency", "derived", "do", "doc",
    "else", "end", "entry", "enum", "event", "exhibit", "exit", "expose", "false", "filter",
    "first", "flow", "for", "fork", "frame", "from", "hastype", "if", "implies", "import", "in",
    "include", "individual", "inout", "interface", "istype", "item", "join", "language",
    "library", "locale", "loop", "merge", "message", "meta", "metadata", "nonunique", "not",
    "null", "objective", "occurrence", "of", "or", "ordered", "out", "package", "parallel",
    "part", "perform", "port", "private", "protected", "public", "redefines", "ref", "references",
    "render", "rendering", "rep", "require", "requirement", "return", "satisfy", "send",
    "snapshot", "specializes", "stakeholder", "standard", "state", "subject", "subsets",
    "succession", "terminate", "then", "timeslice", "to", "transition", "true", "until", "use",
    "variant", "variation", "verification", "verify", "via", "view", "viewpoint", "when", "while",
    "xor",
];

/// Escape a single SysML v2 identifier using OMG KerML restricted-name rules
/// when it is not a plain identifier or collides with a reserved keyword.
pub fn escape_sysml_name(name: &str) -> String {
    if name.starts_with('\'') && name.ends_with('\'') && name.len() >= 2 {
        return name.to_string();
    }
    let is_basic = !name.is_empty()
        && name
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '_')
            .unwrap_or(false)
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !SYSML_KEYWORDS.contains(&name);
    if is_basic {
        name.to_string()
    } else {
        let mut out = String::with_capacity(name.len() + 2);
        out.push('\'');
        for c in name.chars() {
            match c {
                '\'' => out.push_str("\\'"),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                c => out.push(c),
            }
        }
        out.push('\'');
        out
    }
}

/// Format a qualified name or dot-separated feature chain, escaping individual segments as needed.
fn format_ref(s: &str) -> String {
    if s.contains('.') {
        s.split('.')
            .map(format_qualified)
            .collect::<Vec<_>>()
            .join(".")
    } else {
        format_qualified(s)
    }
}

fn format_qualified(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("$::") {
        format!("$::{}", format_qualified(rest))
    } else {
        trimmed
            .split("::")
            .map(|seg| escape_sysml_name(seg.trim()))
            .collect::<Vec<_>>()
            .join("::")
    }
}

/// Format a SysML v2 unit bracket expression, e.g. `kg` -> `[kg]`, `m/s` -> `[m/s]`,
/// `m^3/s` -> `[m^3/s]`, `J/(kg*K)` -> `[J/(kg*K)]`, `°C` -> `['°C']`, `W/°C` -> `[W/'°C']`.
///
/// Composite units are emitted as unquoted KerML operator expressions over the OMG `SI::*`
/// unit atoms (`*`, `/`, `^`, parentheses), so each atom resolves against the standard library;
/// only atoms that are not plain identifiers (e.g. `°C`, `Ω`) are quoted individually.
fn format_unit(unit: &str) -> String {
    let u = unit.trim();
    if u.starts_with('\'') && u.ends_with('\'') {
        return format!("[{u}]");
    }
    if sysml_core::units::parse_unit(u).is_none() {
        // Model-defined / foreign unit name (e.g. `pound-force per square inch (psi)`):
        // emit it as one (quoted if needed) name reference.
        return format!("[{}]", escape_sysml_name(u));
    }
    let is_delim = |c: char| matches!(c, '/' | '*' | '^' | '(' | ')');
    let mut out = String::with_capacity(u.len() + 4);
    out.push('[');
    let mut atom = String::new();
    let flush = |atom: &mut String, out: &mut String| {
        if atom.is_empty() {
            return;
        }
        if atom.chars().all(|c| c.is_ascii_digit() || c == '-') {
            out.push_str(atom);
        } else {
            out.push_str(&escape_sysml_name(atom));
        }
        atom.clear();
    };
    for c in u.chars() {
        if c == ' ' {
            continue;
        }
        if is_delim(c) {
            flush(&mut atom, &mut out);
            out.push(c);
        } else {
            atom.push(c);
        }
    }
    flush(&mut atom, &mut out);
    out.push(']');
    out
}

fn sanitize_doc(doc: &str) -> String {
    doc.replace("*/", "* /")
}

/// Emit a complete `Model` (one or more packages) into deterministic OMG SysML v2 text.
///
/// Requirement references in `satisfy` / `verify` that point into another package the owning
/// package does not import are emitted fully qualified (`satisfy OtherPkg::Req;`), so a
/// cross-package `#[sysml::satisfy]` resolves without a manual `#[sysml::import]`.
pub fn emit_model(model: &Model) -> String {
    let model = crate::resolve::qualify_requirement_refs(model);
    let mut out = String::new();
    for (i, pkg) in model.packages.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        emit_package_into(pkg, 0, &mut out);
    }
    out
}

/// Emit a single `Package` into deterministic OMG SysML v2 text.
pub fn emit_package(pkg: &Package) -> String {
    let mut out = String::new();
    emit_package_into(pkg, 0, &mut out);
    out
}

fn indent(level: usize) -> String {
    "    ".repeat(level)
}

fn emit_doc(doc: &Option<String>, level: usize, out: &mut String) {
    if let Some(d) = doc {
        let ind = indent(level);
        let clean = sanitize_doc(d.trim());
        if clean.contains('\n') {
            let _ = writeln!(out, "{ind}doc /*");
            for line in clean.lines() {
                let _ = writeln!(out, "{ind}    {}", line.trim());
            }
            let _ = writeln!(out, "{ind}*/");
        } else {
            let _ = writeln!(out, "{ind}doc /* {clean} */");
        }
    }
}

fn emit_comment(c: &Comment, level: usize, out: &mut String) {
    let ind = indent(level);
    let clean = sanitize_doc(c.body.trim());
    match &c.name {
        Some(name) => {
            let _ = writeln!(out, "{ind}comment {} /* {clean} */", escape_sysml_name(name));
        }
        None => {
            let _ = writeln!(out, "{ind}comment /* {clean} */");
        }
    }
}

fn emit_package_into(pkg: &Package, level: usize, out: &mut String) {
    let ind = indent(level);
    let lib_prefix = if pkg.is_library { "library " } else { "" };
    let _ = writeln!(
        out,
        "{ind}{lib_prefix}package {} {{",
        escape_sysml_name(&pkg.name)
    );
    emit_doc(&pkg.doc, level + 1, out);
    for c in &pkg.comments {
        emit_comment(c, level + 1, out);
    }

    if !pkg.imports.is_empty() {
        for imp in &pkg.imports {
            let vis = match imp.visibility {
                Visibility::Public => "public ",
                Visibility::Private => "private ",
                Visibility::Protected => "protected ",
            };
            let suffix = match imp.wildcard {
                ImportWildcard::None => "",
                ImportWildcard::Star => "::*",
                ImportWildcard::RecursiveStar => "::**",
            };
            let _ = writeln!(
                out,
                "{}{vis}import {}{suffix};",
                indent(level + 1),
                format_qualified(&imp.target)
            );
        }
        if !pkg.members.is_empty() {
            out.push('\n');
        }
    }

    for (i, member) in pkg.members.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        emit_member(member, level + 1, out);
    }

    let _ = writeln!(out, "{ind}}}");
}

fn emit_member(member: &PackageMember, level: usize, out: &mut String) {
    match member {
        PackageMember::Package(p) => emit_package_into(p, level, out),
        PackageMember::PartDef(p) => emit_part_def(p, level, out),
        PackageMember::PartUsage(p) => emit_part_usage(p, level, out),
        PackageMember::PortDef(p) => emit_port_def(p, level, out),
        PackageMember::InterfaceDef(i) => emit_interface_def(i, level, out),
        PackageMember::ConnectionDef(c) => emit_connection_def(c, level, out),
        PackageMember::ItemDef(i) => emit_item_def(i, level, out),
        PackageMember::EnumDef(e) => emit_enum_def(e, level, out),
        PackageMember::AttributeDef(a) => emit_attribute_def(a, level, out),
        PackageMember::AttributeUsage(a) => emit_attribute_usage(a, level, out),
        PackageMember::ActionDef(a) => emit_action_def(a, level, out),
        PackageMember::StateDef(s) => emit_state_def(s, level, out),
        PackageMember::RequirementDef(r) => emit_requirement_def(r, level, out),
        PackageMember::RequirementUsage(r) => emit_requirement_usage(r, level, out),
        PackageMember::VerificationDef(v) => emit_verification_def(v, level, out),
        PackageMember::CalcDef(c) => emit_calc(c, true, level, out),
        PackageMember::CalcUsage(c) => emit_calc(c, false, level, out),
        PackageMember::AnalysisDef(a) => emit_analysis_def(a, level, out),
        PackageMember::AnalysisUsage(a) => emit_analysis_usage(a, level, out),
        PackageMember::ViewUsage(v) => emit_view_usage(v, level, out),
        PackageMember::MetadataDef(m) => emit_metadata_def(m, level, out),
    }
}

fn emit_specializes(specs: &[String], out: &mut String) {
    if !specs.is_empty() {
        let joined = specs
            .iter()
            .map(|s| format_qualified(s))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :> {joined}");
    }
}

fn emit_attribute_def(a: &AttributeDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if a.is_abstract { "abstract " } else { "" };
    let _ = write!(out, "{ind}{abs}attribute def {}", escape_sysml_name(&a.name));
    emit_specializes(&a.specializes, out);
    if a.doc.is_some() || !a.attributes.is_empty() {
        let _ = writeln!(out, " {{");
        emit_doc(&a.doc, level + 1, out);
        for sub in &a.attributes {
            emit_attribute_usage(sub, level + 1, out);
        }
        let _ = writeln!(out, "{ind}}}");
    } else {
        let _ = writeln!(out, ";");
    }
}

fn emit_attribute_usage(a: &AttributeUsage, level: usize, out: &mut String) {
    let ind = indent(level);
    let priv_kw = if a.is_private { "private " } else { "" };
    let mut need_space = false;
    if a.is_keywordless {
        let _ = write!(out, "{ind}{priv_kw}");
    } else {
        let _ = write!(out, "{ind}{priv_kw}attribute");
        need_space = true;
    }
    if let Some(short) = &a.short_name {
        if need_space {
            out.push(' ');
        }
        let _ = write!(out, "<{}>", escape_sysml_name(short));
        need_space = true;
    }
    if let Some(name) = &a.name {
        if need_space {
            out.push(' ');
        }
        let _ = write!(out, "{}", escape_sysml_name(name));
        need_space = true;
    }
    if let Some(ty) = &a.type_name {
        if need_space {
            out.push(' ');
        }
        let _ = write!(out, ": {}", format_qualified(ty));
        need_space = true;
    }
    if let Some(mult) = &a.multiplicity {
        if need_space && a.type_name.is_none() {
            out.push(' ');
        }
        let _ = write!(out, "{}", mult.render());
        need_space = true;
    }
    if a.is_nonunique {
        if need_space {
            out.push(' ');
        }
        let _ = write!(out, "nonunique");
        need_space = true;
    }
    if !a.subsets.is_empty() {
        let joined = a
            .subsets
            .iter()
            .map(|s| format_ref(s))
            .collect::<Vec<_>>()
            .join(", ");
        if need_space {
            out.push(' ');
        }
        let _ = write!(out, ":> {joined}");
        need_space = true;
    }
    if !a.redefines.is_empty() {
        let joined = a
            .redefines
            .iter()
            .map(|s| format_ref(s))
            .collect::<Vec<_>>()
            .join(", ");
        if need_space {
            out.push(' ');
        }
        let _ = write!(out, ":>> {joined}");
    }
    if let Some(val) = &a.value {
        let op = match a.value_kind {
            ValueKind::Bound => " = ",
            ValueKind::Initial => " := ",
            ValueKind::Default => " default ",
            ValueKind::DefaultInitial => " default := ",
        };
        let _ = write!(out, "{op}{val}");
        if let Some(unit) = &a.unit {
            let _ = write!(out, " {}", format_unit(unit));
        }
    }
    let doc = if a.is_evidence {
        let base = "measured value supplied by a verification case; any model value is a placeholder, not evidence";
        Some(match &a.doc {
            Some(d) => format!("{} {}", sysml_ir::EVIDENCE_DOC_TAG, d.trim()),
            None => format!("{} {base}", sysml_ir::EVIDENCE_DOC_TAG),
        })
    } else {
        a.doc.clone()
    };
    if doc.is_some() || !a.nested_attributes.is_empty() {
        let _ = writeln!(out, " {{");
        emit_doc(&doc, level + 1, out);
        for sub in &a.nested_attributes {
            emit_attribute_usage(sub, level + 1, out);
        }
        let _ = writeln!(out, "{ind}}}");
    } else {
        let _ = writeln!(out, ";");
    }
}

fn emit_item_def(item: &ItemDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if item.is_abstract { "abstract " } else { "" };
    let _ = write!(out, "{ind}{abs}item def {}", escape_sysml_name(&item.name));
    emit_specializes(&item.specializes, out);
    if item.doc.is_none() && item.attributes.is_empty() {
        let _ = writeln!(out, ";");
    } else {
        let _ = writeln!(out, " {{");
        emit_doc(&item.doc, level + 1, out);
        for a in &item.attributes {
            emit_attribute_usage(a, level + 1, out);
        }
        let _ = writeln!(out, "{ind}}}");
    }
}

fn emit_enum_def(e: &EnumDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = writeln!(out, "{ind}enum def {} {{", escape_sysml_name(&e.name));
    emit_doc(&e.doc, level + 1, out);
    for v in &e.variants {
        let _ = writeln!(out, "{}enum {};", indent(level + 1), escape_sysml_name(v));
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_directed_item(it: &DirectedItem, level: usize, out: &mut String) {
    let ind = indent(level);
    let dir = match it.direction {
        Some(Direction::In) => "in ",
        Some(Direction::Out) => "out ",
        Some(Direction::InOut) => "inout ",
        None => "",
    };
    let mult = it
        .multiplicity
        .as_ref()
        .map(|m| m.render())
        .unwrap_or_default();
    let same_name_redef = it.redefines.len() == 1 && it.redefines[0] == it.name;
    if same_name_redef {
        // `out item :>> reading : NarrowerReading;` (redefines the inherited `reading`).
        let _ = write!(
            out,
            "{ind}{dir}item :>> {}{mult}",
            escape_sysml_name(&it.name),
        );
    } else {
        let _ = write!(
            out,
            "{ind}{dir}item {}{mult}",
            escape_sysml_name(&it.name),
        );
    }
    if !it.type_name.is_empty() {
        let _ = write!(out, " : {}", format_qualified(&it.type_name));
    }
    if !same_name_redef && !it.redefines.is_empty() {
        let joined = it
            .redefines
            .iter()
            .map(|r| format_ref(r))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :>> {joined}");
    }
    if !it.subsets.is_empty() {
        let joined = it
            .subsets
            .iter()
            .map(|s| format_ref(s))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :> {joined}");
    }
    if it.doc.is_some() {
        let _ = writeln!(out, " {{");
        emit_doc(&it.doc, level + 1, out);
        let _ = writeln!(out, "{ind}}}");
    } else {
        let _ = writeln!(out, ";");
    }
}

fn emit_port_def(p: &PortDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if p.is_abstract { "abstract " } else { "" };
    let _ = write!(out, "{ind}{abs}port def {}", escape_sysml_name(&p.name));
    emit_specializes(&p.specializes, out);
    if p.doc.is_none() && p.items.is_empty() && p.attributes.is_empty() {
        let _ = writeln!(out, ";");
    } else {
        let _ = writeln!(out, " {{");
        emit_doc(&p.doc, level + 1, out);
        for it in &p.items {
            emit_directed_item(it, level + 1, out);
        }
        for a in &p.attributes {
            emit_attribute_usage(a, level + 1, out);
        }
        let _ = writeln!(out, "{ind}}}");
    }
}

fn emit_port_usage(p: &PortUsage, level: usize, out: &mut String) {
    let ind = indent(level);
    let conj = if p.conjugated { "~" } else { "" };
    let mult = p
        .multiplicity
        .as_ref()
        .map(|m| m.render())
        .unwrap_or_default();
    if !p.redefines.is_empty() {
        let joined = p
            .redefines
            .iter()
            .map(|r| format_ref(r))
            .collect::<Vec<_>>()
            .join(", ");
        if p.redefines.len() == 1 && p.redefines[0] == p.name {
            let _ = write!(out, "{ind}port{mult} :>> {joined}");
        } else {
            let _ = write!(
                out,
                "{ind}port {}{mult} : {conj}{} :>> {joined}",
                escape_sysml_name(&p.name),
                format_qualified(&p.type_name)
            );
        }
    } else {
        let _ = write!(
            out,
            "{ind}port {}{mult} : {conj}{}",
            escape_sysml_name(&p.name),
            format_qualified(&p.type_name)
        );
    }
    if p.doc.is_some() {
        let _ = writeln!(out, " {{");
        emit_doc(&p.doc, level + 1, out);
        let _ = writeln!(out, "{ind}}}");
    } else {
        let _ = writeln!(out, ";");
    }
}

fn emit_interface_def(i: &InterfaceDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if i.is_abstract { "abstract " } else { "" };
    let _ = write!(out, "{ind}{abs}interface def {}", escape_sysml_name(&i.name));
    emit_specializes(&i.specializes, out);
    if i.doc.is_none() && i.attributes.is_empty() && i.ends.is_empty() {
        let _ = writeln!(out, ";");
        return;
    }
    let _ = writeln!(out, " {{");
    emit_doc(&i.doc, level + 1, out);
    for a in &i.attributes {
        emit_attribute_usage(a, level + 1, out);
    }
    for end in &i.ends {
        let conj = if end.conjugated { "~" } else { "" };
        let mult = end
            .multiplicity
            .as_ref()
            .map(|m| m.render())
            .unwrap_or_default();
        let _ = writeln!(
            out,
            "{}end {}{mult} : {conj}{};",
            indent(level + 1),
            escape_sysml_name(&end.name),
            format_qualified(&end.type_name)
        );
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_connection_def(c: &ConnectionDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if c.is_abstract { "abstract " } else { "" };
    let _ = write!(out, "{ind}{abs}connection def {}", escape_sysml_name(&c.name));
    emit_specializes(&c.specializes, out);
    if c.doc.is_none() && c.ends.is_empty() {
        let _ = writeln!(out, ";");
    } else {
        let _ = writeln!(out, " {{");
        emit_doc(&c.doc, level + 1, out);
        for end in &c.ends {
            let ref_kw = if end.is_ref { "ref " } else { "" };
            let mult = end
                .multiplicity
                .as_ref()
                .map(|m| m.render())
                .unwrap_or_default();
            let _ = write!(
                out,
                "{}{ref_kw}{} {}{mult} : {}",
                indent(level + 1),
                end.kind_keyword,
                escape_sysml_name(&end.name),
                format_qualified(&end.type_name)
            );
            emit_specializes(&end.subsets, out);
            let _ = writeln!(out, ";");
        }
        let _ = writeln!(out, "{ind}}}");
    }
}

fn emit_connection_usage(c: &ConnectionUsage, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if c.is_abstract { "abstract " } else { "" };
    let kw = match c.kind {
        ConnectionKind::Connection => "connection",
        ConnectionKind::Interface => "interface",
    };
    let _ = write!(out, "{ind}{abs}{kw}");
    if let Some(name) = &c.name {
        let _ = write!(out, " {}", escape_sysml_name(name));
    }
    if let Some(mult) = &c.multiplicity {
        let _ = write!(out, "{}", mult.render());
    }
    if let Some(ty) = &c.type_name {
        let _ = write!(out, " : {}", format_qualified(ty));
    }
    if !c.subsets.is_empty() {
        let joined = c
            .subsets
            .iter()
            .map(|s| format_ref(s))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :> {joined}");
    }
    if let (Some(from), Some(to)) = (&c.from, &c.to) {
        let _ = write!(out, " connect {} to {}", format_ref(from), format_ref(to));
    }
    if c.doc.is_some() {
        let _ = writeln!(out, " {{");
        emit_doc(&c.doc, level + 1, out);
        let _ = writeln!(out, "{ind}}}");
    } else {
        let _ = writeln!(out, ";");
    }
}

fn emit_perform_action(p: &PerformAction, level: usize, out: &mut String) {
    let ind = indent(level);
    if let Some(ref_target) = &p.references {
        let _ = write!(out, "{ind}perform {}", format_ref(ref_target));
        if !p.redefines.is_empty() {
            let joined = p
                .redefines
                .iter()
                .map(|r| format_ref(r))
                .collect::<Vec<_>>()
                .join(", ");
            let _ = write!(out, " :>> {joined}");
        }
        let _ = writeln!(out, ";");
    } else {
        let _ = write!(out, "{ind}perform action");
        if let Some(name) = &p.name {
            let _ = write!(out, " {}", escape_sysml_name(name));
        }
        if let Some(ty) = &p.action_type {
            let _ = write!(out, " : {}", format_qualified(ty));
        }
        let _ = writeln!(out, ";");
    }
}

fn emit_part_usage(p: &PartUsage, level: usize, out: &mut String) {
    let ind = indent(level);
    let ind_kw = if p.is_individual { "individual " } else { "" };
    let ref_kw = if p.is_ref { "ref " } else { "" };
    let _ = write!(out, "{ind}{ind_kw}{ref_kw}part");
    if let Some(name) = &p.name {
        let _ = write!(out, " {}", escape_sysml_name(name));
    }
    if let Some(mult) = &p.multiplicity {
        let _ = write!(out, "{}", mult.render());
    }
    if p.is_ordered {
        let _ = write!(out, " ordered");
    }
    if !p.type_names.is_empty() {
        let joined = p
            .type_names
            .iter()
            .map(|t| format_qualified(t))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " : {joined}");
    }
    if !p.subsets.is_empty() {
        let joined = p
            .subsets
            .iter()
            .map(|s| format_ref(s))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :> {joined}");
    }
    if !p.redefines.is_empty() {
        let joined = p
            .redefines
            .iter()
            .map(|r| format_ref(r))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :>> {joined}");
    }
    if let Some(def) = &p.default_value {
        let _ = write!(out, " default {def}");
    }
    let has_body = p.doc.is_some()
        || !p.attributes.is_empty()
        || !p.nested_parts.is_empty()
        || !p.ports.is_empty()
        || !p.items.is_empty()
        || !p.performs.is_empty()
        || !p.connections.is_empty()
        || !p.requirements.is_empty()
        || !p.portions.is_empty();
    if !has_body {
        let _ = writeln!(out, ";");
    } else {
        let _ = writeln!(out, " {{");
        emit_doc(&p.doc, level + 1, out);
        for a in &p.attributes {
            emit_attribute_usage(a, level + 1, out);
        }
        for sub in &p.nested_parts {
            emit_part_usage(sub, level + 1, out);
        }
        for port in &p.ports {
            emit_port_usage(port, level + 1, out);
        }
        for it in &p.items {
            emit_directed_item(it, level + 1, out);
        }
        for perf in &p.performs {
            emit_perform_action(perf, level + 1, out);
        }
        for conn in &p.connections {
            emit_connection_usage(conn, level + 1, out);
        }
        for req in &p.requirements {
            emit_requirement_usage(req, level + 1, out);
        }
        for portion in &p.portions {
            emit_portion_usage(portion, level + 1, out);
        }
        let _ = writeln!(out, "{ind}}}");
    }
}

fn emit_portion_usage(p: &PortionUsage, level: usize, out: &mut String) {
    let ind = indent(level);
    let then_kw = if p.is_then { "then " } else { "" };
    let kind_kw = match p.kind {
        PortionKind::Timeslice => "timeslice",
        PortionKind::Snapshot => "snapshot",
    };
    let _ = write!(out, "{ind}{then_kw}{kind_kw} {}", escape_sysml_name(&p.name));
    if !p.subsets.is_empty() {
        let joined = p
            .subsets
            .iter()
            .map(|s| format_ref(s))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :> {joined}");
    }
    if !p.type_names.is_empty() {
        let joined = p
            .type_names
            .iter()
            .map(|t| format_qualified(t))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " : {joined}");
    }
    let has_body = p.doc.is_some()
        || !p.assert_constraints.is_empty()
        || !p.attributes.is_empty()
        || !p.parts.is_empty()
        || !p.performs.is_empty()
        || !p.portions.is_empty();
    if !has_body {
        let _ = writeln!(out, ";");
    } else {
        let _ = writeln!(out, " {{");
        emit_doc(&p.doc, level + 1, out);
        for c in &p.assert_constraints {
            let _ = writeln!(out, "{}assert constraint {{ {c} }}", indent(level + 1));
        }
        for a in &p.attributes {
            emit_attribute_usage(a, level + 1, out);
        }
        for sub in &p.parts {
            emit_part_usage(sub, level + 1, out);
        }
        for perf in &p.performs {
            emit_perform_action(perf, level + 1, out);
        }
        for child in &p.portions {
            emit_portion_usage(child, level + 1, out);
        }
        let _ = writeln!(out, "{ind}}}");
    }
}

fn emit_part_def(p: &PartDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if p.is_abstract { "abstract " } else { "" };
    let var = if p.is_variation { "variation " } else { "" };
    let ind_kw = if p.is_individual { "individual " } else { "" };
    let _ = write!(
        out,
        "{ind}{abs}{var}{ind_kw}part def {}",
        escape_sysml_name(&p.name)
    );
    emit_specializes(&p.specializes, out);

    let has_body = p.doc.is_some()
        || !p.comments.is_empty()
        || !p.attributes.is_empty()
        || !p.parts.is_empty()
        || !p.variants.is_empty()
        || !p.ports.is_empty()
        || !p.items.is_empty()
        || !p.performs.is_empty()
        || !p.connections.is_empty()
        || !p.allocations.is_empty()
        || !p.exhibit_states.is_empty()
        || !p.requirements.is_empty()
        || !p.satisfies.is_empty()
        || !p.verifies.is_empty()
        || !p.refinements.is_empty()
        || !p.fmea.is_empty()
        || !p.asil_decompositions.is_empty();

    if !has_body {
        let _ = writeln!(out, ";");
        return;
    }

    let _ = writeln!(out, " {{");
    emit_doc(&p.doc, level + 1, out);
    for c in &p.comments {
        emit_comment(c, level + 1, out);
    }
    for f in &p.fmea {
        let fm = f.failure_mode.replace('\\', "\\\\").replace('"', "\\\"");
        let eff = f.effect.replace('\\', "\\\\").replace('"', "\\\"");
        let _ = writeln!(out, "{}@FMEA {{", indent(level + 1));
        let _ = writeln!(out, "{}failureMode = \"{fm}\";", indent(level + 2));
        let _ = writeln!(out, "{}effect = \"{eff}\";", indent(level + 2));
        let _ = writeln!(out, "{}severity = {};", indent(level + 2), f.severity);
        let _ = writeln!(out, "{}'occurrence' = {};", indent(level + 2), f.occurrence);
        let _ = writeln!(out, "{}detection = {};", indent(level + 2), f.detection);
        let _ = writeln!(out, "{}rpn = {};", indent(level + 2), f.rpn());
        if let Some(asil) = &f.asil {
            let _ = writeln!(out, "{}asil = \"{asil}\";", indent(level + 2));
        }
        let _ = writeln!(out, "{}}}", indent(level + 1));
    }
    for d in &p.asil_decompositions {
        let _ = writeln!(out, "{}@AsilDecomposition {{", indent(level + 1));
        let _ = writeln!(out, "{}goal = \"{}\";", indent(level + 2), d.goal);
        let _ = writeln!(out, "{}leftPart = \"{}\";", indent(level + 2), d.left_part);
        let _ = writeln!(out, "{}leftAsil = \"{}\";", indent(level + 2), d.left_asil);
        let _ = writeln!(out, "{}rightPart = \"{}\";", indent(level + 2), d.right_part);
        let _ = writeln!(out, "{}rightAsil = \"{}\";", indent(level + 2), d.right_asil);
        let _ = writeln!(out, "{}}}", indent(level + 1));
    }
    for a in &p.attributes {
        emit_attribute_usage(a, level + 1, out);
    }
    for sub in &p.parts {
        emit_part_usage(sub, level + 1, out);
    }
    for v in &p.variants {
        let _ = write!(
            out,
            "{}variant part {} : {}",
            indent(level + 1),
            escape_sysml_name(&v.name),
            format_qualified(&v.type_name)
        );
        if v.doc.is_some() {
            let _ = writeln!(out, " {{");
            emit_doc(&v.doc, level + 2, out);
            let _ = writeln!(out, "{}}}", indent(level + 1));
        } else {
            let _ = writeln!(out, ";");
        }
    }
    for port in &p.ports {
        emit_port_usage(port, level + 1, out);
    }
    for it in &p.items {
        emit_directed_item(it, level + 1, out);
    }
    for perf in &p.performs {
        emit_perform_action(perf, level + 1, out);
    }
    for conn in &p.connections {
        emit_connection_usage(conn, level + 1, out);
    }
    for alloc in &p.allocations {
        let from_s = format_ref(&alloc.from);
        let to_s = format_ref(&alloc.to);
        if let Some(name) = &alloc.name {
            let _ = writeln!(
                out,
                "{}allocation {} allocate {from_s} to {to_s};",
                indent(level + 1),
                escape_sysml_name(name)
            );
        } else {
            let _ = writeln!(out, "{}allocate {from_s} to {to_s};", indent(level + 1));
        }
    }
    for st in &p.exhibit_states {
        emit_state_usage(st, level + 1, out);
    }
    for req in &p.requirements {
        emit_requirement_usage(req, level + 1, out);
    }
    for sat in &p.satisfies {
        emit_satisfy(sat, level + 1, out);
    }
    for ver in &p.verifies {
        emit_verify(ver, level + 1, out);
    }
    for rf in &p.refinements {
        let _ = writeln!(
            out,
            "{}#refinement dependency {} to {};",
            indent(level + 1),
            format_qualified(&rf.client),
            format_qualified(&rf.supplier)
        );
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_action_usage(a: &ActionUsage, prefix_kw: &str, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = write!(out, "{ind}{prefix_kw}action {}", escape_sysml_name(&a.name));
    if let Some(mult) = &a.multiplicity {
        let _ = write!(out, "{}", mult.render());
    }
    if let Some(ty) = &a.type_name {
        let _ = write!(out, " : {}", format_qualified(ty));
    }
    if !a.redefines.is_empty() {
        let joined = a
            .redefines
            .iter()
            .map(|r| format_ref(r))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :>> {joined}");
    }
    let has_body = a.doc.is_some() || a.has_initial_start || !a.steps.is_empty() || a.has_done;
    if !has_body {
        let _ = writeln!(out, ";");
    } else {
        let _ = writeln!(out, " {{");
        emit_doc(&a.doc, level + 1, out);
        if a.has_initial_start {
            let _ = writeln!(out, "{}first start;", indent(level + 1));
        }
        for step in &a.steps {
            let then_kw = if step.is_then { "then " } else { "" };
            let _ = write!(
                out,
                "{}{then_kw}action {}",
                indent(level + 1),
                escape_sysml_name(&step.name)
            );
            if let Some(ty) = &step.type_name {
                let _ = write!(out, " : {}", format_qualified(ty));
            }
            let _ = writeln!(out, ";");
        }
        if a.has_done {
            let _ = writeln!(out, "{}then done;", indent(level + 1));
        }
        let _ = writeln!(out, "{ind}}}");
    }
}

fn emit_action_def(a: &ActionDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if a.is_abstract { "abstract " } else { "" };
    let _ = write!(out, "{ind}{abs}action def {}", escape_sysml_name(&a.name));
    emit_specializes(&a.specializes, out);
    let has_body = a.doc.is_some()
        || !a.params.is_empty()
        || !a.subactions.is_empty()
        || a.has_initial_start
        || !a.steps.is_empty()
        || a.has_done
        || !a.refinements.is_empty();
    if !has_body {
        let _ = writeln!(out, ";");
        return;
    }
    let _ = writeln!(out, " {{");
    for p in &a.params {
        let dir = match p.direction {
            Direction::In => "in",
            Direction::Out => "out",
            Direction::InOut => "inout",
        };
        let _ = writeln!(
            out,
            "{}{dir} {} : {};",
            indent(level + 1),
            escape_sysml_name(&p.name),
            format_qualified(&p.type_name)
        );
    }
    emit_doc(&a.doc, level + 1, out);
    for sub in &a.subactions {
        emit_action_usage(sub, "", level + 1, out);
    }
    if a.has_initial_start {
        let _ = writeln!(out, "{}first start;", indent(level + 1));
    }
    for step in &a.steps {
        let then_kw = if step.is_then { "then " } else { "" };
        let _ = write!(
            out,
            "{}{then_kw}action {}",
            indent(level + 1),
            escape_sysml_name(&step.name)
        );
        if let Some(ty) = &step.type_name {
            let _ = write!(out, " : {}", format_qualified(ty));
        }
        let _ = writeln!(out, ";");
    }
    if a.has_done {
        let _ = writeln!(out, "{}then done;", indent(level + 1));
    }
    for r in &a.refinements {
        let _ = writeln!(
            out,
            "{}#refinement dependency {} to {};",
            indent(level + 1),
            format_qualified(&r.client),
            format_qualified(&r.supplier)
        );
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_transition(t: &Transition, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = write!(out, "{ind}transition");
    if let Some(name) = &t.name {
        let _ = write!(out, " {}", escape_sysml_name(name));
    }
    let _ = write!(out, " first {}", format_ref(&t.source));
    if let Some(trig) = &t.trigger {
        if let Some(pname) = &t.trigger_payload_name {
            let _ = write!(
                out,
                " accept {} : {}",
                escape_sysml_name(pname),
                format_qualified(trig)
            );
        } else {
            let _ = write!(out, " accept {}", format_qualified(trig));
        }
    }
    if let Some(guard) = &t.guard {
        let _ = write!(out, " if {guard}");
    }
    if let Some(effect) = &t.effect {
        let _ = write!(out, " do {effect}");
    }
    let _ = write!(out, " then {}", format_ref(&t.target));
    if t.doc.is_some() {
        let _ = writeln!(out, " {{");
        emit_doc(&t.doc, level + 1, out);
        let _ = writeln!(out, "{ind}}}");
    } else {
        let _ = writeln!(out, ";");
    }
}

fn emit_state_usage(s: &StateUsage, level: usize, out: &mut String) {
    let ind = indent(level);
    let ex = if s.is_exhibit { "exhibit " } else { "" };
    let _ = write!(out, "{ind}{ex}state {}", escape_sysml_name(&s.name));
    if let Some(ty) = &s.type_name {
        let _ = write!(out, " : {}", format_qualified(ty));
    }
    let has_body = s.doc.is_some()
        || s.entry_action.is_some()
        || s.do_action.is_some()
        || s.exit_action.is_some()
        || !s.states.is_empty()
        || !s.transitions.is_empty();
    if !has_body {
        let _ = writeln!(out, ";");
        return;
    }
    let _ = writeln!(out, " {{");
    emit_doc(&s.doc, level + 1, out);
    if let Some(e) = &s.entry_action {
        emit_action_usage(e, "entry ", level + 1, out);
    }
    if let Some(d) = &s.do_action {
        emit_action_usage(d, "do ", level + 1, out);
    }
    if let Some(x) = &s.exit_action {
        emit_action_usage(x, "exit ", level + 1, out);
    }
    for sub in &s.states {
        emit_state_usage(sub, level + 1, out);
    }
    for tr in &s.transitions {
        emit_transition(tr, level + 1, out);
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_state_def(s: &StateDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if s.is_abstract { "abstract " } else { "" };
    let _ = write!(out, "{ind}{abs}state def {}", escape_sysml_name(&s.name));
    emit_specializes(&s.specializes, out);
    let has_body = s.doc.is_some()
        || s.subject_type.is_some()
        || s.entry_action.is_some()
        || s.do_action.is_some()
        || s.exit_action.is_some()
        || !s.states.is_empty()
        || !s.transitions.is_empty();
    if !has_body {
        let _ = writeln!(out, ";");
        return;
    }
    let _ = writeln!(out, " {{");
    emit_doc(&s.doc, level + 1, out);
    if let Some(subj_ty) = &s.subject_type {
        let _ = writeln!(
            out,
            "{}ref part target : {};",
            indent(level + 1),
            format_qualified(subj_ty)
        );
    }
    if let Some(e) = &s.entry_action {
        emit_action_usage(e, "entry ", level + 1, out);
    }
    if let Some(d) = &s.do_action {
        emit_action_usage(d, "do ", level + 1, out);
    }
    if let Some(x) = &s.exit_action {
        emit_action_usage(x, "exit ", level + 1, out);
    }
    for sub in &s.states {
        emit_state_usage(sub, level + 1, out);
    }
    for tr in &s.transitions {
        emit_transition(tr, level + 1, out);
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_requirement_def(r: &RequirementDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if r.is_abstract { "abstract " } else { "" };
    let _ = write!(out, "{ind}{abs}requirement def");
    if let Some(short) = &r.short_name {
        let _ = write!(out, " <{}>", escape_sysml_name(short));
    }
    let _ = write!(out, " {}", escape_sysml_name(&r.name));
    emit_specializes(&r.specializes, out);
    let has_body = r.doc.is_some()
        || r.rationale.is_some()
        || r.safety_goal.is_some()
        || r.subject.is_some()
        || !r.attributes.is_empty()
        || !r.require_constraints.is_empty()
        || !r.assume_constraints.is_empty()
        || !r.refinements.is_empty();
    if !has_body {
        let _ = writeln!(out, ";");
        return;
    }
    let _ = writeln!(out, " {{");
    emit_doc(&r.doc, level + 1, out);
    if let Some(rat) = &r.rationale {
        let escaped = rat.replace('\\', "\\\\").replace('"', "\\\"");
        let _ = writeln!(out, "{}@Rationale {{", indent(level + 1));
        let _ = writeln!(out, "{}text = \"{escaped}\";", indent(level + 2));
        let _ = writeln!(out, "{}}}", indent(level + 1));
    }
    if let Some(sg) = &r.safety_goal {
        let ss = sg.safe_state.replace('\\', "\\\\").replace('"', "\\\"");
        let _ = writeln!(out, "{}@SafetyGoal {{", indent(level + 1));
        let _ = writeln!(out, "{}asil = \"{}\";", indent(level + 2), sg.asil);
        let _ = writeln!(out, "{}safeState = \"{ss}\";", indent(level + 2));
        let _ = writeln!(out, "{}fttiMs = {};", indent(level + 2), sg.ftti_ms);
        let _ = writeln!(out, "{}}}", indent(level + 1));
    }
    if let Some(subj) = &r.subject {
        let _ = writeln!(
            out,
            "{}subject {} : {};",
            indent(level + 1),
            escape_sysml_name(&subj.name),
            format_qualified(&subj.type_name)
        );
    }
    for a in &r.attributes {
        emit_attribute_usage(a, level + 1, out);
    }
    for c in &r.assume_constraints {
        let _ = writeln!(out, "{}assume constraint {{ {c} }}", indent(level + 1));
    }
    for c in &r.require_constraints {
        let _ = writeln!(out, "{}require constraint {{ {c} }}", indent(level + 1));
    }
    for rf in &r.refinements {
        let _ = writeln!(
            out,
            "{}#refinement dependency {} to {};",
            indent(level + 1),
            format_qualified(&rf.client),
            format_qualified(&rf.supplier)
        );
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_requirement_usage(r: &RequirementUsage, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = write!(out, "{ind}requirement");
    if let Some(short) = &r.short_name {
        let _ = write!(out, " <{}>", escape_sysml_name(short));
    }
    let _ = write!(out, " {}", escape_sysml_name(&r.name));
    if let Some(mult) = &r.multiplicity {
        let _ = write!(out, "{}", mult.render());
    }
    if let Some(ty) = &r.type_name {
        let _ = write!(out, " : {}", format_qualified(ty));
    }
    emit_specializes(&r.subsets, out);
    let has_body = r.doc.is_some()
        || r.subject.is_some()
        || !r.nested_requirements.is_empty()
        || !r.satisfies.is_empty()
        || !r.verifies.is_empty();
    if !has_body {
        let _ = writeln!(out, ";");
        return;
    }
    let _ = writeln!(out, " {{");
    emit_doc(&r.doc, level + 1, out);
    if let Some(subj) = &r.subject {
        let _ = writeln!(
            out,
            "{}subject {} : {};",
            indent(level + 1),
            escape_sysml_name(&subj.name),
            format_qualified(&subj.type_name)
        );
    }
    for n in &r.nested_requirements {
        emit_requirement_usage(n, level + 1, out);
    }
    for sat in &r.satisfies {
        emit_satisfy(sat, level + 1, out);
    }
    for ver in &r.verifies {
        emit_verify(ver, level + 1, out);
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_satisfy(s: &SatisfyStmt, level: usize, out: &mut String) {
    let ind = indent(level);
    let assert_kw = if s.asserted { "assert " } else { "" };
    if s.is_typed {
        let _ = write!(
            out,
            "{ind}{assert_kw}satisfy requirement : {}",
            format_qualified(&s.requirement)
        );
    } else {
        let _ = write!(
            out,
            "{ind}{assert_kw}satisfy {}",
            format_qualified(&s.requirement)
        );
    }
    if let Some(by) = &s.by {
        let _ = write!(out, " by {}", format_ref(by));
    }
    let _ = writeln!(out, ";");
}

fn emit_verify(v: &VerifyStmt, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = write!(out, "{ind}verify {}", format_qualified(&v.requirement));
    if let Some(by) = &v.by {
        let _ = write!(out, " by {}", format_ref(by));
    }
    let _ = writeln!(out, ";");
}

fn emit_verification_def(v: &VerificationDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = writeln!(
        out,
        "{ind}verification def {} {{",
        escape_sysml_name(&v.name)
    );
    emit_doc(&v.doc, level + 1, out);
    if let Some(subj) = &v.subject {
        let _ = writeln!(
            out,
            "{}subject {} : {};",
            indent(level + 1),
            escape_sysml_name(&subj.name),
            format_qualified(&subj.type_name)
        );
    }
    for ver in &v.verifies {
        emit_verify(ver, level + 1, out);
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_calc(c: &CalcDef, is_def: bool, level: usize, out: &mut String) {
    let ind = indent(level);
    let def_kw = if is_def { " def" } else { "" };
    let _ = write!(out, "{ind}calc{def_kw}");
    if let Some(short) = &c.short_name {
        let _ = write!(out, " <{}>", escape_sysml_name(short));
    }
    let _ = writeln!(out, " {} {{", escape_sysml_name(&c.name));
    emit_doc(&c.doc, level + 1, out);
    for p in &c.params {
        let mult = p
            .multiplicity
            .as_ref()
            .map(|m| m.render())
            .unwrap_or_default();
        let _ = write!(out, "{}in {}", indent(level + 1), escape_sysml_name(&p.name));
        if !p.type_name.is_empty() {
            let _ = write!(out, " : {}{mult}", format_qualified(&p.type_name));
        } else if let Some(sub) = &p.subsets {
            let _ = write!(out, " :> {}{mult}", format_ref(sub));
        } else if !mult.is_empty() {
            let _ = write!(out, "{mult}");
        }
        let _ = writeln!(out, ";");
    }
    for local in &c.locals {
        let _ = writeln!(
            out,
            "{}attribute {} = {};",
            indent(level + 1),
            escape_sysml_name(&local.name),
            local.expr
        );
    }
    let _ = write!(out, "{}return", indent(level + 1));
    if !c.return_name.is_empty() {
        let _ = write!(out, " {}", escape_sysml_name(&c.return_name));
    }
    if !c.return_type.is_empty() {
        let _ = write!(out, " : {}", format_qualified(&c.return_type));
    } else if let Some(sub) = &c.return_subsets {
        let _ = write!(out, " :> {}", format_ref(sub));
    }
    if !c.result_expr.is_empty() {
        let _ = write!(out, " = {}", c.result_expr);
    }
    let _ = writeln!(out, ";");
    let _ = writeln!(out, "{ind}}}");
}

fn emit_view_usage(v: &ViewUsage, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = write!(out, "{ind}view {}", escape_sysml_name(&v.name));
    if let Some(ty) = &v.type_name {
        let _ = write!(out, " : {}", format_qualified(ty));
    }
    if v.doc.is_some() {
        let _ = writeln!(out, " {{");
        emit_doc(&v.doc, level + 1, out);
        let _ = writeln!(out, "{ind}}}");
    } else {
        let _ = writeln!(out, ";");
    }
}

fn emit_analysis_param(p: &AnalysisParam, level: usize, out: &mut String) {
    let ind = indent(level);
    let dir = match p.direction {
        Some(Direction::In) => "in ",
        Some(Direction::Out) => "out ",
        Some(Direction::InOut) => "inout ",
        None => "",
    };
    let mult = p
        .multiplicity
        .as_ref()
        .map(|m| m.render())
        .unwrap_or_default();
    let _ = write!(out, "{ind}{dir}{}", escape_sysml_name(&p.name));
    if let Some(ty) = &p.type_name {
        let _ = write!(out, " : {}{mult}", format_qualified(ty));
    } else if !p.subsets.is_empty() {
        let joined = p
            .subsets
            .iter()
            .map(|s| format_ref(s))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :> {joined}{mult}");
    } else if !mult.is_empty() {
        let _ = write!(out, "{mult}");
    }
    if let Some(val) = &p.value {
        let _ = write!(out, " = {val}");
        if let Some(unit) = &p.unit {
            let _ = write!(out, " {}", format_unit(unit));
        }
    }
    let _ = writeln!(out, ";");
}

fn emit_analysis_def(a: &AnalysisDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = write!(out, "{ind}analysis def {}", escape_sysml_name(&a.name));
    emit_specializes(&a.specializes, out);
    let has_body = a.doc.is_some()
        || a.subject.is_some()
        || !a.params.is_empty()
        || !a.assert_constraints.is_empty();
    if !has_body {
        let _ = writeln!(out, ";");
        return;
    }
    let _ = writeln!(out, " {{");
    emit_doc(&a.doc, level + 1, out);
    if let Some(subj) = &a.subject {
        let _ = writeln!(
            out,
            "{}subject {} : {};",
            indent(level + 1),
            escape_sysml_name(&subj.name),
            format_qualified(&subj.type_name)
        );
    }
    for p in &a.params {
        emit_analysis_param(p, level + 1, out);
    }
    for c in &a.assert_constraints {
        let _ = writeln!(out, "{}assert constraint {{ {c} }}", indent(level + 1));
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_analysis_usage(a: &AnalysisUsage, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = write!(out, "{ind}analysis {}", escape_sysml_name(&a.name));
    if let Some(ty) = &a.type_name {
        let _ = write!(out, " : {}", format_qualified(ty));
    }
    let has_body = a.doc.is_some()
        || a.subject.is_some()
        || !a.params.is_empty()
        || !a.assert_constraints.is_empty();
    if !has_body {
        let _ = writeln!(out, ";");
        return;
    }
    let _ = writeln!(out, " {{");
    emit_doc(&a.doc, level + 1, out);
    if let Some(subj) = &a.subject {
        let _ = writeln!(
            out,
            "{}subject {} : {};",
            indent(level + 1),
            escape_sysml_name(&subj.name),
            format_qualified(&subj.type_name)
        );
    }
    for p in &a.params {
        emit_analysis_param(p, level + 1, out);
    }
    for c in &a.assert_constraints {
        let _ = writeln!(out, "{}assert constraint {{ {c} }}", indent(level + 1));
    }
    let _ = writeln!(out, "{ind}}}");
}

fn emit_metadata_def(m: &MetadataDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = write!(out, "{ind}metadata def {}", escape_sysml_name(&m.name));
    if m.doc.is_none() && m.attributes.is_empty() {
        let _ = writeln!(out, ";");
    } else {
        let _ = writeln!(out, " {{");
        emit_doc(&m.doc, level + 1, out);
        for a in &m.attributes {
            emit_attribute_usage(a, level + 1, out);
        }
        let _ = writeln!(out, "{ind}}}");
    }
}

/// Normative SysML v2 `library package ISO26262Metadata` defining `@SafetyGoal`,
/// `@FMEA`, and `@AsilDecomposition` metadata definitions so models using ISO 26262
/// annotations validate with 0 errors and 0 warnings against Open-MBEE + OMG stdlib.
pub fn iso26262_metadata_package() -> Package {
    let attr = |name: &str, ty: &str| AttributeUsage {
        name: Some(name.to_string()),
        type_name: Some(ty.to_string()),
        ..Default::default()
    };
    Package {
        name: "ISO26262Metadata".to_string(),
        is_library: true,
        doc: Some("ISO 26262 functional safety metadata definitions (@SafetyGoal, @FMEA, @AsilDecomposition).".to_string()),
        comments: Vec::new(),
        imports: vec![Import::private_star("ScalarValues")],
        members: vec![
            PackageMember::MetadataDef(MetadataDef {
                name: "SafetyGoal".to_string(),
                doc: Some("ISO 26262-3 top-level safety goal metadata.".to_string()),
                attributes: vec![
                    attr("asil", "String"),
                    attr("safeState", "String"),
                    attr("fttiMs", "Integer"),
                ],
            }),
            PackageMember::MetadataDef(MetadataDef {
                name: "FMEA".to_string(),
                doc: Some("ISO 26262 / ARP4761 Failure Modes and Effects Analysis entry.".to_string()),
                attributes: vec![
                    attr("failureMode", "String"),
                    attr("effect", "String"),
                    attr("severity", "Integer"),
                    attr("occurrence", "Integer"),
                    attr("detection", "Integer"),
                    attr("rpn", "Integer"),
                    attr("asil", "String"),
                ],
            }),
            PackageMember::MetadataDef(MetadataDef {
                name: "AsilDecomposition".to_string(),
                doc: Some("ISO 26262-9 Clause 5 verified ASIL decomposition witness.".to_string()),
                attributes: vec![
                    attr("goal", "String"),
                    attr("leftPart", "String"),
                    attr("leftAsil", "String"),
                    attr("rightPart", "String"),
                    attr("rightAsil", "String"),
                ],
            }),
        ],
    }
}

#[cfg(test)]
mod unit_format_tests {
    use super::format_unit;

    #[test]
    fn formats_atoms_and_composites() {
        assert_eq!(format_unit("kg"), "[kg]");
        assert_eq!(format_unit("m/s"), "[m/s]");
        assert_eq!(format_unit("1/h"), "[1/h]");
        assert_eq!(format_unit("N*m"), "[N*m]");
        assert_eq!(format_unit("m^3/s"), "[m^3/s]");
        assert_eq!(format_unit("m/s^-2"), "[m/s^-2]");
        assert_eq!(format_unit("J/(kg*K)"), "[J/(kg*K)]");
        assert_eq!(format_unit("°C"), "['°C']");
        assert_eq!(format_unit("W/°C"), "[W/'°C']");
        assert_eq!(format_unit("'km/h'"), "['km/h']");
    }
}
