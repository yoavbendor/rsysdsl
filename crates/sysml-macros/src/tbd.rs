//! S7 TBD markers: controlled escape hatches for gradual design.
//!
//! Every hatch costs a written reason, is emitted as `@TbdMetadata::TBD { ... }` metadata on the
//! element, and is listed in the TBD ledger (`rsysdsl tbd`). Sources:
//! - `#[sysml::attr(tbd_unit = "reason")]`, `#[sysml::attr(tbd_value = "reason")]`,
//!   `#[sysml::attr(unit = "...", custom_unit = "reason")]` (with optional `tbd_owner = "..."`,
//!   `tbd_resolve_by = "<phase>"`);
//! - `#[sysml::tbd(reason = "...", owner = "...", resolve_by = "<phase>")]` on definitions,
//!   requirements, state variants and part fields;
//! - `tbd = "reason"` on `#[sysml::connect(...)]` and `#[sysml::transition(...)]`.

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::Attribute;

use crate::units_check::non_empty_reason;
use crate::util::{is_sysml_attr, KvArgs};

const PHASES: &[&str] = &["concept", "preliminary", "detailed", "release"];

fn phase_tokens(phase: Option<String>, span: Span) -> syn::Result<TokenStream> {
    let p = phase.unwrap_or_else(|| "detailed".to_string());
    let variant = match p.trim() {
        "concept" => quote!(Concept),
        "preliminary" => quote!(Preliminary),
        "detailed" => quote!(Detailed),
        "release" => quote!(Release),
        other => {
            return Err(syn::Error::new(
                span,
                format!(
                    "unknown TBD phase `{other}`\nhelp: `resolve_by` must be one of {} (default: detailed)",
                    PHASES.join(", ")
                ),
            ))
        }
    };
    Ok(quote!(::sysml_dsl::ir::TbdPhase::#variant))
}

/// One `::sysml_dsl::ir::TbdMark { ... }` literal.
pub fn mark_tokens(
    kind: &str,
    reason: &str,
    owner: Option<String>,
    resolve_by: Option<String>,
    span: Span,
) -> syn::Result<TokenStream> {
    let kind_ident = syn::Ident::new(kind, Span::call_site());
    let phase = phase_tokens(resolve_by, span)?;
    let owner_tok = match owner {
        Some(o) => quote!(::std::option::Option::Some(::std::string::String::from(#o))),
        None => quote!(::std::option::Option::None),
    };
    Ok(quote! {
        ::sysml_dsl::ir::TbdMark {
            kind: ::sysml_dsl::ir::TbdKind::#kind_ident,
            reason: ::std::string::String::from(#reason),
            owner: #owner_tok,
            resolve_by: #phase,
        }
    })
}

/// `::std::vec![marks...]`.
pub fn quote_marks(marks: &[TokenStream]) -> TokenStream {
    quote!(::std::vec![#(#marks),*])
}

/// TBD marks carried by an attribute's `#[sysml::attr(...)]` keys (`tbd_unit`, `tbd_value`,
/// `custom_unit`), sharing `tbd_owner` / `tbd_resolve_by`.
pub fn attr_marks(kv: &KvArgs, span: Span) -> syn::Result<Vec<TokenStream>> {
    let owner = kv.get_str("tbd_owner");
    let resolve_by = kv.get_str("tbd_resolve_by");
    let rb_span = kv.get_str_span("tbd_resolve_by").unwrap_or(span);
    let mut out = Vec::new();
    for (key, kind) in [("tbd_unit", "Unit"), ("tbd_value", "Value"), ("custom_unit", "CustomUnit")] {
        if let Some(reason) = non_empty_reason(kv, key, span)? {
            out.push(mark_tokens(kind, &reason, owner.clone(), resolve_by.clone(), rb_span)?);
        }
    }
    if out.is_empty() && (owner.is_some() || resolve_by.is_some()) {
        return Err(syn::Error::new(
            span,
            "`tbd_owner` / `tbd_resolve_by` need a TBD on the same attribute\n\
             help: add `tbd_unit = \"reason\"`, `tbd_value = \"reason\"` or `custom_unit = \"reason\"`",
        ));
    }
    Ok(out)
}

/// Parse one `#[sysml::tbd(reason = "...", owner = "...", resolve_by = "...")]`.
pub fn parse_tbd_attr(attr: &Attribute) -> syn::Result<TokenStream> {
    let kv: KvArgs = attr.parse_args()?;
    let span = attr.span();
    for (k, _) in &kv.strings {
        if !matches!(k.as_str(), "reason" | "owner" | "resolve_by") {
            return Err(syn::Error::new(
                kv.get_str_span(k).unwrap_or(span),
                format!("unknown key `{k}` in #[sysml::tbd(...)]\nhelp: expected `reason = \"...\"`, optional `owner = \"...\"`, `resolve_by = \"concept|preliminary|detailed|release\"`"),
            ));
        }
    }
    let Some(reason) = non_empty_reason(&kv, "reason", span)? else {
        return Err(syn::Error::new(
            span,
            "#[sysml::tbd(...)] needs `reason = \"...\"`\n\
             help: e.g. #[sysml::tbd(reason = \"supplier not selected yet\", owner = \"systems\", resolve_by = \"preliminary\")]",
        ));
    };
    let rb_span = kv.get_str_span("resolve_by").unwrap_or(span);
    mark_tokens("Element", &reason, kv.get_str("owner"), kv.get_str("resolve_by"), rb_span)
}

/// All `#[sysml::tbd(...)]` markers among `attrs`.
pub fn extract_tbd_marks(attrs: &[Attribute]) -> syn::Result<Vec<TokenStream>> {
    attrs
        .iter()
        .filter(|a| is_sysml_attr(a, "tbd"))
        .map(parse_tbd_attr)
        .collect()
}

/// Same as [`extract_tbd_marks`], quoted as a `Vec<TbdMark>` expression.
pub fn tbd_vec_tokens(attrs: &[Attribute]) -> syn::Result<TokenStream> {
    Ok(quote_marks(&extract_tbd_marks(attrs)?))
}

/// `#[sysml::tbd]` on an element kind that cannot carry it is an error (never silently dropped).
pub fn reject_tbd(attrs: &[Attribute], kind: &str) -> syn::Result<()> {
    if let Some(a) = attrs.iter().find(|a| is_sysml_attr(a, "tbd")) {
        return Err(syn::Error::new(
            a.span(),
            format!(
                "#[sysml::tbd] is not supported on #[sysml::{kind}]\n\
                 help: put it on a part, port, item, interface, requirement or state (or a part / port field), \
                 or use `tbd = \"reason\"` on #[sysml::connect] / #[sysml::transition]"
            ),
        ));
    }
    Ok(())
}
