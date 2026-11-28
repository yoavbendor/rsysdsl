//! Compile-time unit-symbol validation and dimensional analysis for
//! `#[sysml::attr(unit = ...)]`, `#[sysml::require_constraint(...)]` and
//! `#[sysml::transition(..., guard = ...)]`.
//!
//! The closed unit table and the dimension algebra live in `sysml_core::units` (shared with the
//! `const` assertions generated for transition guards, whose operand units are declared on other
//! `#[sysml::item]` / `#[sysml::part]` types and therefore are only known during const evaluation).

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use std::collections::BTreeMap;
use syn::spanned::Spanned;
use syn::{BinOp, Expr, Lit, Member, Path, UnOp};
use sysml_core::units::{self, compatible, DimVal, UnitCheck, UNITS};

use crate::util::{is_sysml_attr, KvArgs};

// ---------------------------------------------------------------------------------------------
// 1. Unit symbol validation
// ---------------------------------------------------------------------------------------------

/// `true` when the attribute opts out of the closed SI table with `custom_unit = "reason"`
/// (or the legacy bare `custom_unit` flag, still accepted on analysis parameters).
pub fn has_custom_unit(kv: &KvArgs) -> bool {
    kv.has_flag("custom_unit") || kv.get_str("custom_unit").is_some()
}

/// The unit to use for dimensional analysis: `"1"` for `dimensionless`, `""` (= unknown, checks
/// skipped) when the attribute has no unit, a `tbd_unit`, or a `custom_unit` (a unit defined by
/// the model or another library).
pub fn checked_unit(kv: &KvArgs) -> String {
    if has_custom_unit(kv) || kv.get_str("tbd_unit").is_some() {
        return String::new();
    }
    if kv.has_flag("dimensionless") {
        return "1".to_string();
    }
    kv.get_str("unit").unwrap_or_default()
}

/// Validate the `unit = "..."` entry of a parsed `#[sysml::attr(...)]` (if present).
/// `#[sysml::attr(unit = "...", custom_unit = "reason")]` opts out (model-defined / foreign
/// library unit).
pub fn validate_unit_kv(kv: &KvArgs) -> syn::Result<()> {
    if has_custom_unit(kv) {
        return Ok(());
    }
    let Some(unit) = kv.get_str("unit") else {
        return Ok(());
    };
    let span = kv.get_str_span("unit").unwrap_or_else(Span::call_site);
    validate_unit_str(&unit, span)
}

/// Rust types that make an attribute numeric (and therefore subject to the unit declaration rule).
fn is_numeric_rust_type(ty: &syn::Type) -> bool {
    const NUMERIC: &[&str] = &[
        "Quantity", "f64", "f32", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32",
        "u64", "u128", "usize",
    ];
    match ty {
        syn::Type::Path(tp) => tp
            .path
            .segments
            .last()
            .is_some_and(|seg| NUMERIC.iter().any(|n| seg.ident == n)),
        syn::Type::Paren(p) => is_numeric_rust_type(&p.elem),
        syn::Type::Group(g) => is_numeric_rust_type(&g.elem),
        _ => false,
    }
}

/// SysML scalar types whose explicit `ty = "..."` keeps the unit rule in force (any other
/// explicit type, e.g. `ty = "ISQ::MassValue"` or a model attribute def, carries its own unit).
fn is_scalar_sysml_type(ty: &str) -> bool {
    let simple = ty.rsplit("::").next().unwrap_or(ty);
    matches!(
        simple,
        "Real" | "Integer" | "Natural" | "Rational" | "Number" | "Positive" | "NumericalValue"
    )
}

/// A non-empty reason string for `key = "..."`, or an error naming the key.
pub fn non_empty_reason(kv: &KvArgs, key: &str, fallback: Span) -> syn::Result<Option<String>> {
    let Some(r) = kv.get_str(key) else {
        return Ok(None);
    };
    if r.trim().is_empty() {
        let span = kv.get_str_span(key).unwrap_or(fallback);
        return Err(syn::Error::new(
            span,
            format!(
                "`{key}` needs a non-empty reason\n\
                 help: say why it is open and where the answer will come from, e.g. \
                 `{key} = \"not stated in <source document>; ask <owner>\"`"
            ),
        ));
    }
    Ok(Some(r))
}

/// S7.1: every numeric attribute declares exactly one of `unit = "..."`, `dimensionless`,
/// `tbd_unit = "reason"` or `unit = "...", custom_unit = "reason"`. Omitting all of them used to
/// skip the dimension checks silently.
pub fn require_unit_decl(
    kv: &KvArgs,
    field_name: &str,
    field_ty: &syn::Type,
    span: Span,
) -> syn::Result<()> {
    if kv.has_flag("custom_unit") {
        return Err(syn::Error::new(
            span,
            format!(
                "`custom_unit` on attribute `{field_name}` needs a reason\n\
                 help: write `custom_unit = \"<why this unit is outside the SI table>\"` (it is emitted as @TBD and counted in the TBD ledger)"
            ),
        ));
    }
    let tbd_unit = non_empty_reason(kv, "tbd_unit", span)?;
    let custom = non_empty_reason(kv, "custom_unit", span)?;
    let has_unit = kv.get_str("unit").is_some();
    let dimensionless = kv.has_flag("dimensionless");
    if custom.is_some() && !has_unit {
        return Err(syn::Error::new(
            span,
            format!(
                "`custom_unit` on attribute `{field_name}` needs the unit symbol itself\n\
                 help: write `unit = \"<symbol>\", custom_unit = \"<reason>\"`"
            ),
        ));
    }
    let declared: Vec<&str> = [
        (has_unit && custom.is_none(), "unit"),
        (dimensionless, "dimensionless"),
        (tbd_unit.is_some(), "tbd_unit"),
        (custom.is_some(), "custom_unit"),
    ]
    .into_iter()
    .filter(|(on, _)| *on)
    .map(|(_, n)| n)
    .collect();
    if declared.len() > 1 {
        return Err(syn::Error::new(
            span,
            format!(
                "attribute `{field_name}` declares more than one unit option ({})\n\
                 help: keep exactly one of `unit = \"...\"`, `dimensionless`, `tbd_unit = \"reason\"`, `custom_unit = \"reason\"`",
                declared.join(", ")
            ),
        ));
    }
    if !declared.is_empty() || !is_numeric_rust_type(field_ty) {
        return Ok(());
    }
    // The dimension comes from SysML itself: an explicit quantity / attribute-def type
    // (`ty = "ISQ::MassValue"`), a subsetted ISQ quantity (`subsets = "ISQ::mass"`), or a
    // derived SysML expression (`expr = "dryMass + propellantMass"`).
    if let Some(t) = kv.get_str("ty").or_else(|| kv.get_str("type")) {
        if !t.is_empty() && !is_scalar_sysml_type(&t) {
            return Ok(());
        }
    }
    if kv.get_all_str("subsets").iter().any(|s| s.starts_with("ISQ::"))
        || kv.get_str("expr").is_some()
    {
        return Ok(());
    }
    Err(syn::Error::new(
        span,
        format!(
            "numeric attribute `{field_name}` declares no unit\n\
             help: add exactly one of:\n\
             \x20 unit = \"<SI symbol>\"        e.g. unit = \"W\", unit = \"m/s\"\n\
             \x20 dimensionless               counts, ratios, indices, flags\n\
             \x20 tbd_unit = \"<reason>\"       unit not in the source yet (emitted as @TBD, counted in the TBD ledger)\n\
             \x20 custom_unit = \"<reason>\"    with unit = \"...\": a unit defined in your model or another library"
        ),
    ))
}

/// C1.1 / D1: `#[sysml::attr(..., budget = "<role>")]`. Returns the role variant name
/// (`"Power"`, ...) after checking that the declared unit's dimension matches the role:
/// `mass` => M (`kg`, `g`), `power` / `waste_heat` => W, `latency` => T (`s`, `min`, `h`),
/// `efficiency` => `dimensionless`, `cost` => `dimensionless` (an amount in the model's single
/// currency: the SI table has no currency unit) or `unit = "<currency>", custom_unit = "reason"`,
/// `none` => any unit (explicitly not a budget contributor).
pub fn budget_role(kv: &KvArgs, field_name: &str, span: Span) -> syn::Result<Option<&'static str>> {
    let Some(role) = kv.get_str("budget") else {
        return Ok(None);
    };
    let at = kv.get_str_span("budget").unwrap_or(span);
    let (variant, expect): (&'static str, Option<(&str, &str)>) = match role.trim() {
        "mass" => ("Mass", Some(("kg", "mass (e.g. unit = \"kg\")"))),
        "power" => ("Power", Some(("W", "power (e.g. unit = \"W\")"))),
        "waste_heat" => ("WasteHeat", Some(("W", "power (e.g. unit = \"W\")"))),
        "latency" => ("Latency", Some(("s", "time (e.g. unit = \"s\")"))),
        "efficiency" => ("Efficiency", None),
        "cost" => ("Cost", None),
        "none" => ("None", None),
        other => {
            return Err(syn::Error::new(
                at,
                format!(
                    "unknown budget role `{other}` on attribute `{field_name}`\n\
                     help: use one of \"mass\", \"power\", \"waste_heat\", \"cost\", \"latency\", \"efficiency\", \"none\""
                ),
            ))
        }
    };
    let dimensionless = kv.has_flag("dimensionless");
    let custom = has_custom_unit(kv);
    let unit = kv.get_str("unit");
    let declared = if dimensionless {
        "dimensionless".to_string()
    } else if let Some(r) = kv.get_str("tbd_unit") {
        format!("tbd_unit = \"{r}\"")
    } else if custom {
        format!("custom unit `{}`", unit.clone().unwrap_or_default())
    } else if let Some(u) = &unit {
        format!("unit `{u}`")
    } else {
        "no unit".to_string()
    };
    let ok = match (variant, expect) {
        (_, Some((reference, _))) => match (&unit, dimensionless || custom) {
            (Some(u), false) => match (units::parse_unit(u), units::parse_unit(reference)) {
                (Some(d), Some(r)) => d.eq(r),
                _ => false,
            },
            _ => false,
        },
        ("Efficiency", None) => dimensionless,
        ("Cost", None) => dimensionless || (custom && unit.is_some()),
        _ => true,
    };
    if ok {
        return Ok(Some(variant));
    }
    let need = match (variant, expect) {
        (_, Some((_, dim))) => format!("a unit of dimension {dim}"),
        ("Efficiency", None) => "`dimensionless` (a 0..1 fraction)".to_string(),
        _ => "`dimensionless` (an amount in the model's currency) or `unit = \"<currency>\", custom_unit = \"reason\"`".to_string(),
    };
    Err(syn::Error::new(
        at,
        format!(
            "budget role `{role}` on attribute `{field_name}` needs {need}, but it declares {declared}\n\
             help: fix the unit, or pick the role that matches its dimension (`budget = \"none\"` opts the attribute out of budgets)"
        ),
    ))
}

pub fn validate_unit_str(unit: &str, span: Span) -> syn::Result<()> {
    match units::check_unit(unit) {
        UnitCheck::Ok(_) => return Ok(()),
        UnitCheck::BareOne => {
            return Err(syn::Error::new(
                span,
                format!(
                    "unit `{unit}` is not a SysML v2 unit: dimensionless attributes carry no unit\n\
                     help: remove `unit = \"{unit}\"` (and document the scale in a `///` doc comment)"
                ),
            ))
        }
        UnitCheck::NotInOmg(def) => {
            let within = if def.symbol == unit {
                String::new()
            } else {
                format!(" (in `{unit}`)")
            };
            return Err(syn::Error::new(
                span,
                format!(
                    "unit symbol `{sym}` ({name}){within} is not defined in the OMG SysML v2 `SI` library; \
                     the strict audit would report an unresolved reference\n\
                     help: use `{inst}` instead and convert the value accordingly",
                    sym = def.symbol,
                    name = def.name,
                    inst = def.use_instead,
                ),
            ))
        }
        UnitCheck::Unknown => {}
    }
    let suggestions = suggest(unit);
    let hint = if suggestions.is_empty() {
        String::new()
    } else {
        let list: Vec<String> = suggestions.iter().map(|s| format!("`{s}`")).collect();
        format!("; did you mean {}?", list.join(" or "))
    };
    Err(syn::Error::new(
        span,
        format!(
            "unknown unit symbol `{unit}` in #[sysml::attr(unit = ...)]{hint}\n\
             help: use an SI symbol from the closed table in `sysml_core::units::UNITS` \
             (e.g. \"m\", \"kg\", \"s\", \"A\", \"K\", \"N\", \"Pa\", \"J\", \"W\", \"V\", \"Hz\"), \
             optionally combined with `*`, `/`, `^` (e.g. \"m/s^2\", \"N*m\", \"m^3/s\"); for a unit defined in your own model or another library add `custom_unit = \"<reason>\"`"
        ),
    ))
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// Closest known symbols (atoms + common composites), at most 3, within a small edit distance.
fn suggest(unit: &str) -> Vec<String> {
    let candidates = UNITS
        .iter()
        .filter(|u| u.is_omg() && u.symbol != "1")
        .map(|u| u.symbol)
        .chain(units::COMMON_COMPOSITES.iter().copied());
    // Case-insensitive exact match wins outright (e.g. "hz" -> "Hz", "KG" -> "kg").
    let ci: Vec<String> = candidates
        .clone()
        .filter(|c| c.eq_ignore_ascii_case(unit))
        .map(str::to_string)
        .collect();
    if !ci.is_empty() {
        return ci;
    }
    let max = (unit.chars().count() / 2).clamp(1, 2);
    let mut scored: Vec<(usize, &str)> = candidates
        .map(|c| (levenshtein(unit, c), c))
        .filter(|(d, c)| *d <= max && *d < c.chars().count().max(unit.chars().count()))
        .collect();
    let Some(best) = scored.iter().map(|(d, _)| *d).min() else {
        return Vec::new();
    };
    scored.retain(|(d, _)| *d == best);
    scored.dedup_by(|a, b| a.1 == b.1);
    scored
        .into_iter()
        .take(3)
        .map(|(_, c)| c.to_string())
        .collect()
}

/// `unit = "..."` of each named field's `#[sysml::attr(...)]` (`""` when absent).
pub fn field_units(fields: &syn::Fields) -> syn::Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    if let syn::Fields::Named(named) = fields {
        for f in &named.named {
            let mut unit = String::new();
            for a in &f.attrs {
                if is_sysml_attr(a, "attr") {
                    if let syn::Meta::List(_) = &a.meta {
                        let kv: KvArgs = a.parse_args()?;
                        unit = checked_unit(&kv);
                    }
                }
            }
            out.push((f.ident.as_ref().unwrap().to_string(), unit));
        }
    }
    Ok(out)
}

/// Hidden `const fn __sysml_field_unit(name) -> &'static str` on `#[sysml::item]` / `#[sysml::part]`
/// types, consumed by the `const` dimension assertions of transition guards.
pub fn field_unit_fn(ident: &syn::Ident, fields: &syn::Fields) -> syn::Result<TokenStream> {
    let arms = field_units(fields)?
        .into_iter()
        .filter(|(_, u)| !u.is_empty() && units::is_known_unit(u))
        .map(|(f, u)| {
            quote! {
                if ::sysml_dsl::core_contracts::const_str_eq(name, #f) { return #u; }
            }
        });
    Ok(quote! {
        impl #ident {
            #[doc(hidden)]
            #[allow(dead_code)]
            pub const fn __sysml_field_unit(name: &str) -> &'static str {
                #(#arms)*
                let _ = name;
                ""
            }
        }
    })
}

// ---------------------------------------------------------------------------------------------
// 2. Dimensional analysis of closed expressions
// ---------------------------------------------------------------------------------------------

/// Pretty source text of a (sub)expression for diagnostics.
fn render(e: &Expr) -> String {
    quote!(#e)
        .to_string()
        .replace(" . ", ".")
        .replace("( ", "(")
        .replace(" )", ")")
}

trait DimEval {
    type V: Clone;
    fn leaf(&mut self, e: &Expr) -> Self::V;
    fn wildcard(&self) -> Self::V;
    fn unknown(&self) -> Self::V;
    fn mul(&self, a: Self::V, b: Self::V) -> Self::V;
    fn div(&self, a: Self::V, b: Self::V) -> Self::V;
    fn add(&self, a: Self::V, b: Self::V) -> Self::V;
    /// Both sides must have the same dimension.
    fn check(
        &mut self,
        op: &str,
        at: &Expr,
        l: &Expr,
        lv: &Self::V,
        r: &Expr,
        rv: &Self::V,
    ) -> syn::Result<()>;
}

fn walk<E: DimEval>(e: &Expr, ev: &mut E) -> syn::Result<E::V> {
    Ok(match e {
        Expr::Paren(p) => walk(&p.expr, ev)?,
        Expr::Group(g) => walk(&g.expr, ev)?,
        Expr::Lit(l) => match &l.lit {
            Lit::Int(_) | Lit::Float(_) => ev.wildcard(),
            _ => ev.unknown(),
        },
        Expr::Path(_) | Expr::Field(_) => ev.leaf(e),
        Expr::Unary(u) => {
            let v = walk(&u.expr, ev)?;
            match u.op {
                UnOp::Neg(_) => v,
                _ => ev.unknown(),
            }
        }
        Expr::Binary(b) => {
            let lv = walk(&b.left, ev)?;
            let rv = walk(&b.right, ev)?;
            let sym = match b.op {
                BinOp::Add(_) => "+",
                BinOp::Sub(_) => "-",
                BinOp::Lt(_) => "<",
                BinOp::Le(_) => "<=",
                BinOp::Gt(_) => ">",
                BinOp::Ge(_) => ">=",
                BinOp::Eq(_) => "==",
                BinOp::Ne(_) => "!=",
                BinOp::Mul(_) => return Ok(ev.mul(lv, rv)),
                BinOp::Div(_) => return Ok(ev.div(lv, rv)),
                _ => return Ok(ev.unknown()),
            };
            ev.check(sym, e, &b.left, &lv, &b.right, &rv)?;
            if matches!(b.op, BinOp::Add(_) | BinOp::Sub(_)) {
                ev.add(lv, rv)
            } else {
                ev.unknown()
            }
        }
        Expr::If(eif) => {
            walk(&eif.cond, ev)?;
            let then_e = single_expr(&eif.then_branch);
            let else_e = match eif.else_branch.as_ref().map(|(_, b)| &**b) {
                Some(Expr::Block(b)) => single_expr(&b.block),
                Some(other) => Some(other),
                None => None,
            };
            match (then_e, else_e) {
                (Some(t), Some(f)) => {
                    let tv = walk(t, ev)?;
                    let fv = walk(f, ev)?;
                    ev.check("if/else", e, t, &tv, f, &fv)?;
                    ev.add(tv, fv)
                }
                _ => ev.unknown(),
            }
        }
        _ => ev.unknown(),
    })
}

fn single_expr(block: &syn::Block) -> Option<&Expr> {
    match block.stmts.as_slice() {
        [syn::Stmt::Expr(e, None)] => Some(e),
        _ => None,
    }
}

// --- 2a. Requirement constraints: units known at expansion time --------------------------------

struct ReqEval<'a> {
    units: &'a BTreeMap<String, String>,
}

impl DimEval for ReqEval<'_> {
    type V = DimVal;

    fn leaf(&mut self, e: &Expr) -> DimVal {
        let name = match e {
            Expr::Path(p) => p.path.get_ident().map(|i| i.to_string()),
            Expr::Field(f) => match (&*f.base, &f.member) {
                (Expr::Path(root), Member::Named(m)) if root.path.is_ident("self") => {
                    Some(m.to_string())
                }
                _ => None,
            },
            _ => None,
        };
        match name.and_then(|n| self.units.get(&n)) {
            Some(u) if !u.is_empty() => DimVal::from_unit(Box::leak(u.clone().into_boxed_str())),
            _ => DimVal::UNKNOWN,
        }
    }
    fn wildcard(&self) -> DimVal {
        DimVal::WILDCARD
    }
    fn unknown(&self) -> DimVal {
        DimVal::UNKNOWN
    }
    fn mul(&self, a: DimVal, b: DimVal) -> DimVal {
        a.mul(b)
    }
    fn div(&self, a: DimVal, b: DimVal) -> DimVal {
        a.div(b)
    }
    fn add(&self, a: DimVal, b: DimVal) -> DimVal {
        a.add(b)
    }
    fn check(
        &mut self,
        op: &str,
        at: &Expr,
        l: &Expr,
        lv: &DimVal,
        r: &Expr,
        rv: &DimVal,
    ) -> syn::Result<()> {
        if compatible(*lv, *rv) {
            return Ok(());
        }
        let msg = units::mismatch_message(op, &render(l), *lv, &render(r), *rv);
        Err(syn::Error::new_spanned(at, msg.as_str()))
    }
}

/// Check a `#[sysml::require_constraint]` / `#[sysml::assume_constraint]` expression.
/// `field_units` maps Rust field name -> declared unit (`""` = none, check skipped).
pub fn check_requirement_dims(
    expr: &Expr,
    field_units: &BTreeMap<String, String>,
) -> syn::Result<()> {
    let mut ev = ReqEval { units: field_units };
    walk(expr, &mut ev).map(|_| ())
}

// --- 2b. Transition guards: units live on other types -> const assertions ----------------------

struct GuardEval<'a> {
    event_ty: Option<&'a Path>,
    subject_ty: Option<&'a Path>,
    asserts: Vec<TokenStream>,
}

impl DimEval for GuardEval<'_> {
    type V = TokenStream;

    fn leaf(&mut self, e: &Expr) -> TokenStream {
        let unknown = self.unknown();
        let Expr::Field(f) = e else { return unknown };
        let (Expr::Path(root), Member::Named(m)) = (&*f.base, &f.member) else {
            return unknown;
        };
        let ty = if root.path.is_ident("event") {
            self.event_ty
        } else if root.path.is_ident("subject") {
            self.subject_ty
        } else {
            None
        };
        let Some(ty) = ty else { return unknown };
        let fname = m.to_string();
        quote!(::sysml_dsl::core_contracts::units::DimVal::from_unit(<#ty>::__sysml_field_unit(#fname)))
    }
    fn wildcard(&self) -> TokenStream {
        quote!(::sysml_dsl::core_contracts::units::DimVal::WILDCARD)
    }
    fn unknown(&self) -> TokenStream {
        quote!(::sysml_dsl::core_contracts::units::DimVal::UNKNOWN)
    }
    fn mul(&self, a: TokenStream, b: TokenStream) -> TokenStream {
        quote!((#a).mul(#b))
    }
    fn div(&self, a: TokenStream, b: TokenStream) -> TokenStream {
        quote!((#a).div(#b))
    }
    fn add(&self, a: TokenStream, b: TokenStream) -> TokenStream {
        quote!((#a).add(#b))
    }
    fn check(
        &mut self,
        op: &str,
        at: &Expr,
        l: &Expr,
        lv: &TokenStream,
        r: &Expr,
        rv: &TokenStream,
    ) -> syn::Result<()> {
        let (ls, rs) = (render(l), render(r));
        let span = at.span();
        self.asserts.push(quote_spanned! {span=>
            const _: () = ::sysml_dsl::core_contracts::units::assert_same_dim(#op, #ls, #lv, #rs, #rv);
        });
        Ok(())
    }
}

/// Build module-level `const` assertions checking the dimensional consistency of a guard.
pub fn guard_dim_asserts(
    expr: &Expr,
    event_ty: Option<&Path>,
    subject_ty: Option<&Path>,
) -> syn::Result<TokenStream> {
    let mut ev = GuardEval {
        event_ty,
        subject_ty,
        asserts: Vec::new(),
    };
    walk(expr, &mut ev)?;
    let asserts = ev.asserts;
    Ok(quote!(#(#asserts)*))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions() {
        assert_eq!(suggest("hz"), vec!["Hz"]);
        assert!(suggest("kgs").contains(&"kg".to_string()));
        assert!(suggest("furlongs").is_empty());
    }

    #[test]
    fn requirement_dims() {
        let mut m = BTreeMap::new();
        m.insert("temp_k".to_string(), "K".to_string());
        m.insert("power_w".to_string(), "W".to_string());
        m.insert("limit_k".to_string(), "K".to_string());
        m.insert("raw".to_string(), String::new());
        let ok: Expr = syn::parse_quote!(temp_k <= limit_k + 5.0 && self.raw > power_w);
        assert!(check_requirement_dims(&ok, &m).is_ok());
        let bad: Expr = syn::parse_quote!(temp_k <= power_w);
        let err = check_requirement_dims(&bad, &m).unwrap_err().to_string();
        assert!(
            err.contains("`temp_k` [K]") && err.contains("`power_w` [W]"),
            "{err}"
        );
        let ratio: Expr = syn::parse_quote!(power_w / temp_k * limit_k <= power_w);
        assert!(check_requirement_dims(&ratio, &m).is_ok());
    }
}
