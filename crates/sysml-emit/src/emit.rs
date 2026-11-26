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

/// Format a SysML v2 unit bracket expression, e.g. `kg` -> `[kg]`, `m/s` -> `[m/s]`, `%` -> `['%']`.
fn format_unit(unit: &str) -> String {
    let u = unit.trim();
    if u.starts_with('\'') && u.ends_with('\'') {
        return format!("[{u}]");
    }
    // Simple SI expressions like `kg`, `m/s`, `1/h`, `W`, `s`, `m`
    let simple_expr = !u.is_empty()
        && u.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '/' || c == '*');
    let starts_valid = u
        .chars()
        .next()
        .map(|c| c.is_ascii_alphanumeric() || c == '_')
        .unwrap_or(false);
    // Keep prefix units like kN or special symbols quoted if they contain non-ASCII or punctuation,
    // while standard basic expressions like kg, m/s, 1/h, W, s, m, h, min stay unquoted.
    if simple_expr && starts_valid && !SYSML_KEYWORDS.contains(&u) {
        format!("[{u}]")
    } else {
        format!("[{}]", escape_sysml_name(u))
    }
}

fn sanitize_doc(doc: &str) -> String {
    doc.replace("*/", "* /")
}

/// Emit a complete `Model` (one or more packages) into deterministic OMG SysML v2 text.
pub fn emit_model(model: &Model) -> String {
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

fn emit_package_into(pkg: &Package, level: usize, out: &mut String) {
    let ind = indent(level);
    let lib_prefix = if pkg.is_library { "library " } else { "" };
    let _ = writeln!(
        out,
        "{ind}{lib_prefix}package {} {{",
        escape_sysml_name(&pkg.name)
    );
    emit_doc(&pkg.doc, level + 1, out);

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
        PackageMember::ActionDef(a) => emit_action_def(a, level, out),
        PackageMember::StateDef(s) => emit_state_def(s, level, out),
        PackageMember::RequirementDef(r) => emit_requirement_def(r, level, out),
        PackageMember::RequirementUsage(r) => emit_requirement_usage(r, level, out),
        PackageMember::VerificationDef(v) => emit_verification_def(v, level, out),
        PackageMember::CalcDef(c) => emit_calc_def(c, level, out),
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
    if a.doc.is_some() {
        let _ = writeln!(out, " {{");
        emit_doc(&a.doc, level + 1, out);
        let _ = writeln!(out, "{ind}}}");
    } else {
        let _ = writeln!(out, ";");
    }
}

fn emit_attribute_usage(a: &AttributeUsage, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = write!(out, "{ind}attribute");
    if let Some(name) = &a.name {
        let _ = write!(out, " {}", escape_sysml_name(name));
    }
    if let Some(mult) = &a.multiplicity {
        let _ = write!(out, " {}", mult.render());
    }
    if let Some(ty) = &a.type_name {
        let _ = write!(out, " : {}", format_qualified(ty));
    }
    if !a.subsets.is_empty() {
        let joined = a
            .subsets
            .iter()
            .map(|s| format_ref(s))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :> {joined}");
    }
    if !a.redefines.is_empty() {
        let joined = a
            .redefines
            .iter()
            .map(|s| format_ref(s))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " :>> {joined}");
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
    if a.doc.is_some() {
        let _ = writeln!(out, " {{");
        emit_doc(&a.doc, level + 1, out);
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
    let _ = writeln!(
        out,
        "{ind}{dir}item {}{mult} : {};",
        escape_sysml_name(&it.name),
        format_qualified(&it.type_name)
    );
}

fn emit_port_def(p: &PortDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = write!(out, "{ind}port def {}", escape_sysml_name(&p.name));
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
    let _ = write!(
        out,
        "{ind}port {}{mult} : {conj}{}",
        escape_sysml_name(&p.name),
        format_qualified(&p.type_name)
    );
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
    let _ = write!(out, "{ind}interface def {}", escape_sysml_name(&i.name));
    emit_specializes(&i.specializes, out);
    let _ = writeln!(out, " {{");
    emit_doc(&i.doc, level + 1, out);
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
        || !p.performs.is_empty()
        || !p.connections.is_empty();
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
        for perf in &p.performs {
            emit_perform_action(perf, level + 1, out);
        }
        for conn in &p.connections {
            emit_connection_usage(conn, level + 1, out);
        }
        let _ = writeln!(out, "{ind}}}");
    }
}

fn emit_part_def(p: &PartDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let abs = if p.is_abstract { "abstract " } else { "" };
    let ind_kw = if p.is_individual { "individual " } else { "" };
    let _ = write!(
        out,
        "{ind}{abs}{ind_kw}part def {}",
        escape_sysml_name(&p.name)
    );
    emit_specializes(&p.specializes, out);

    let has_body = p.doc.is_some()
        || !p.attributes.is_empty()
        || !p.parts.is_empty()
        || !p.ports.is_empty()
        || !p.items.is_empty()
        || !p.performs.is_empty()
        || !p.connections.is_empty()
        || !p.exhibit_states.is_empty()
        || !p.requirements.is_empty()
        || !p.satisfies.is_empty()
        || !p.verifies.is_empty();

    if !has_body {
        let _ = writeln!(out, ";");
        return;
    }

    let _ = writeln!(out, " {{");
    emit_doc(&p.doc, level + 1, out);
    for a in &p.attributes {
        emit_attribute_usage(a, level + 1, out);
    }
    for sub in &p.parts {
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

fn emit_calc_def(c: &CalcDef, level: usize, out: &mut String) {
    let ind = indent(level);
    let _ = writeln!(out, "{ind}calc def {} {{", escape_sysml_name(&c.name));
    emit_doc(&c.doc, level + 1, out);
    for p in &c.params {
        let _ = writeln!(
            out,
            "{}in {} : {};",
            indent(level + 1),
            escape_sysml_name(&p.name),
            format_qualified(&p.type_name)
        );
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
    let _ = writeln!(
        out,
        "{}return {} : {} = {};",
        indent(level + 1),
        escape_sysml_name(&c.return_name),
        format_qualified(&c.return_type),
        c.result_expr
    );
    let _ = writeln!(out, "{ind}}}");
}
