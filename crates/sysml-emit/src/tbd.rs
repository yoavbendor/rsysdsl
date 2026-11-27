//! S7 TBD ledger: every escape hatch (`tbd_unit`, `tbd_value`, `custom_unit`, `#[sysml::tbd]`,
//! `#[sysml::connect(tbd = ...)]`) of a model, with its reason, owner and `resolve_by` phase.
//!
//! The ledger is built from the IR, so it works both on a Rust-built `Model` and on a model
//! lifted from emitted `.sysml` (the `@TbdMetadata::TBD` metadata round-trips through `sysml-lift`).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use sysml_ir::{
    AttributeUsage, ConnectionUsage, Model, Package, PackageMember, PartUsage, StateUsage,
    TbdKind, TbdMark, TbdPhase, Transition,
};

/// One row of the TBD ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TbdEntry {
    /// Qualified element path, e.g. `Pkg::Motor::ratedPower` or `Pkg::Ctx::link`.
    pub element: String,
    pub kind: TbdKind,
    pub reason: String,
    #[serde(default)]
    pub owner: Option<String>,
    pub resolve_by: TbdPhase,
}

/// TBD totals per kind (every kind is listed, zeros included, for stable baselines).
pub fn count_by_kind(entries: &[TbdEntry]) -> BTreeMap<String, usize> {
    let mut out: BTreeMap<String, usize> = TbdKind::ALL
        .iter()
        .map(|k| (k.as_str().to_string(), 0))
        .collect();
    for e in entries {
        *out.entry(e.kind.as_str().to_string()).or_default() += 1;
    }
    out
}

/// `true` if any element of the model carries a TBD marker.
pub fn model_has_tbd(model: &Model) -> bool {
    !collect_tbd_ledger(model).is_empty()
}

/// Every TBD marker of the model (sysml-std library packages excluded), in model order.
pub fn collect_tbd_ledger(model: &Model) -> Vec<TbdEntry> {
    let mut out = Vec::new();
    for pkg in &model.packages {
        if crate::emit_cli::is_sysml_std_package(pkg) {
            continue;
        }
        walk_package(pkg, "", &mut out);
    }
    out
}

/// TBDs that `--maturity <phase>` rejects: those whose `resolve_by` is at or before `phase`.
/// The default maturity `concept` therefore only rejects TBDs due at `concept`; `release`
/// rejects every TBD.
pub fn overdue_at(entries: &[TbdEntry], maturity: TbdPhase) -> Vec<TbdEntry> {
    entries
        .iter()
        .filter(|e| e.resolve_by <= maturity)
        .cloned()
        .collect()
}

fn push(out: &mut Vec<TbdEntry>, element: &str, marks: &[TbdMark]) {
    for m in marks {
        out.push(TbdEntry {
            element: element.to_string(),
            kind: m.kind,
            reason: m.reason.clone(),
            owner: m.owner.clone(),
            resolve_by: m.resolve_by,
        });
    }
}

fn join(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}::{name}")
    }
}

fn attr_name(a: &AttributeUsage) -> String {
    a.name
        .clone()
        .or_else(|| a.redefines.first().cloned())
        .or_else(|| a.subsets.first().cloned())
        .unwrap_or_else(|| "<attribute>".to_string())
}

fn walk_attrs(attrs: &[AttributeUsage], owner: &str, out: &mut Vec<TbdEntry>) {
    for a in attrs {
        let path = format!("{owner}::{}", attr_name(a));
        push(out, &path, &a.tbd);
        walk_attrs(&a.nested_attributes, &path, out);
    }
}

fn conn_name(c: &ConnectionUsage) -> String {
    c.name.clone().unwrap_or_else(|| {
        format!(
            "{} -> {}",
            c.from.as_deref().unwrap_or("?"),
            c.to.as_deref().unwrap_or("?")
        )
    })
}

fn walk_connections(conns: &[ConnectionUsage], owner: &str, out: &mut Vec<TbdEntry>) {
    for c in conns {
        push(out, &format!("{owner}::{}", conn_name(c)), &c.tbd);
    }
}

fn walk_transitions(ts: &[Transition], owner: &str, out: &mut Vec<TbdEntry>) {
    for t in ts {
        let name = t
            .name
            .clone()
            .unwrap_or_else(|| format!("{} -> {}", t.source, t.target));
        push(out, &format!("{owner}::{name}"), &t.tbd);
    }
}

fn walk_states(states: &[StateUsage], owner: &str, out: &mut Vec<TbdEntry>) {
    for s in states {
        let path = format!("{owner}::{}", s.name);
        push(out, &path, &s.tbd);
        walk_states(&s.states, &path, out);
        walk_transitions(&s.transitions, &path, out);
    }
}

fn walk_part_usages(parts: &[PartUsage], owner: &str, out: &mut Vec<TbdEntry>) {
    for p in parts {
        let path = format!("{owner}::{}", p.name.as_deref().unwrap_or("<part>"));
        push(out, &path, &p.tbd);
        walk_attrs(&p.attributes, &path, out);
        walk_part_usages(&p.nested_parts, &path, out);
        for port in &p.ports {
            push(out, &format!("{path}::{}", port.name), &port.tbd);
        }
        walk_connections(&p.connections, &path, out);
    }
}

fn walk_package(pkg: &Package, prefix: &str, out: &mut Vec<TbdEntry>) {
    let pkg_path = join(prefix, &pkg.name);
    for m in &pkg.members {
        match m {
            PackageMember::Package(p) => walk_package(p, &pkg_path, out),
            PackageMember::PartDef(p) => {
                let path = join(&pkg_path, &p.name);
                push(out, &path, &p.tbd);
                walk_attrs(&p.attributes, &path, out);
                walk_part_usages(&p.parts, &path, out);
                for port in &p.ports {
                    push(out, &format!("{path}::{}", port.name), &port.tbd);
                }
                walk_connections(&p.connections, &path, out);
                walk_states(&p.exhibit_states, &path, out);
            }
            PackageMember::PartUsage(p) => walk_part_usages(std::slice::from_ref(p), &pkg_path, out),
            PackageMember::PortDef(p) => {
                let path = join(&pkg_path, &p.name);
                push(out, &path, &p.tbd);
                walk_attrs(&p.attributes, &path, out);
            }
            PackageMember::ItemDef(i) => {
                let path = join(&pkg_path, &i.name);
                push(out, &path, &i.tbd);
                walk_attrs(&i.attributes, &path, out);
            }
            PackageMember::InterfaceDef(i) => {
                let path = join(&pkg_path, &i.name);
                push(out, &path, &i.tbd);
                walk_attrs(&i.attributes, &path, out);
            }
            PackageMember::RequirementDef(r) => {
                let path = join(&pkg_path, &r.name);
                push(out, &path, &r.tbd);
                walk_attrs(&r.attributes, &path, out);
            }
            PackageMember::StateDef(s) => {
                let path = join(&pkg_path, &s.name);
                push(out, &path, &s.tbd);
                walk_states(&s.states, &path, out);
                walk_transitions(&s.transitions, &path, out);
            }
            PackageMember::AttributeDef(a) => {
                walk_attrs(&a.attributes, &join(&pkg_path, &a.name), out);
            }
            PackageMember::AttributeUsage(a) => walk_attrs(std::slice::from_ref(a), &pkg_path, out),
            _ => {}
        }
    }
}

/// `rsysdsl tbd` output: the ledger plus totals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TbdLedger {
    pub model: String,
    pub tbd_count: usize,
    /// Count per kind (`unit`, `value`, `element`, `channel`, `custom_unit`), zeros included.
    pub by_kind: BTreeMap<String, usize>,
    pub entries: Vec<TbdEntry>,
}

/// Build the TBD ledger report for `model`.
pub fn tbd_ledger_report(model_name: impl Into<String>, model: &Model) -> TbdLedger {
    let entries = collect_tbd_ledger(model);
    TbdLedger {
        model: model_name.into(),
        tbd_count: entries.len(),
        by_kind: count_by_kind(&entries),
        entries,
    }
}

/// Markdown rendering of a [`TbdLedger`].
pub fn render_tbd_markdown(l: &TbdLedger) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let _ = writeln!(out, "# TBD ledger — `{}`\n", l.model);
    let kinds: Vec<String> = l
        .by_kind
        .iter()
        .filter(|(_, n)| **n > 0)
        .map(|(k, n)| format!("{n} {k}"))
        .collect();
    let _ = writeln!(
        out,
        "- **TBDs:** {} open{}\n",
        l.tbd_count,
        if kinds.is_empty() {
            String::new()
        } else {
            format!(" ({})", kinds.join(", "))
        }
    );
    if l.entries.is_empty() {
        let _ = writeln!(out, "No TBDs.");
        return out;
    }
    let _ = writeln!(out, "| Element | Kind | Resolve by | Owner | Reason |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for e in &l.entries {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} |",
            e.element,
            e.kind.as_str(),
            e.resolve_by.as_str(),
            e.owner.as_deref().unwrap_or("—"),
            e.reason.replace('|', "\\|")
        );
    }
    out
}

/// Result of `rsysdsl tbd --baseline <file> --check` (the TBD ratchet).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TbdRatchet {
    /// `false` when the total or any per-kind count rose above the baseline.
    pub ok: bool,
    pub baseline_count: usize,
    pub current_count: usize,
    /// Kinds (and `total`) whose count rose: `kind: baseline -> current`.
    pub rose: Vec<String>,
    /// Current entries absent from the baseline (matched on element, kind and reason).
    pub new_entries: Vec<TbdEntry>,
    /// Baseline entries no longer present (resolved TBDs; lower the baseline to lock them in).
    pub resolved_entries: Vec<TbdEntry>,
}

/// Compare a current ledger with a committed baseline. Counts may fall freely; any rise (in
/// total or per kind) fails, so adding a TBD requires updating the baseline deliberately.
pub fn check_tbd_ratchet(baseline: &TbdLedger, current: &TbdLedger) -> TbdRatchet {
    let key = |e: &TbdEntry| (e.element.clone(), e.kind, e.reason.clone());
    let base_keys: std::collections::BTreeSet<_> = baseline.entries.iter().map(key).collect();
    let cur_keys: std::collections::BTreeSet<_> = current.entries.iter().map(key).collect();
    let mut rose = Vec::new();
    if current.tbd_count > baseline.tbd_count {
        rose.push(format!("total: {} -> {}", baseline.tbd_count, current.tbd_count));
    }
    for k in TbdKind::ALL {
        let b = baseline.by_kind.get(k.as_str()).copied().unwrap_or(0);
        let c = current.by_kind.get(k.as_str()).copied().unwrap_or(0);
        if c > b {
            rose.push(format!("{}: {b} -> {c}", k.as_str()));
        }
    }
    TbdRatchet {
        ok: rose.is_empty(),
        baseline_count: baseline.tbd_count,
        current_count: current.tbd_count,
        rose,
        new_entries: current
            .entries
            .iter()
            .filter(|e| !base_keys.contains(&key(e)))
            .cloned()
            .collect(),
        resolved_entries: baseline
            .entries
            .iter()
            .filter(|e| !cur_keys.contains(&key(e)))
            .cloned()
            .collect(),
    }
}
