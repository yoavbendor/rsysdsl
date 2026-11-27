//! Shared parsing, naming, and attribute helpers for `sysml-macros`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Expr, ExprLit, GenericArgument, Ident, Lit, Meta, Path, PathArguments, Token, Type};

/// Extract concatenated Rust `///` doc comments from attributes, if any.
pub fn extract_doc(attrs: &[Attribute]) -> Option<String> {
    let mut lines = Vec::new();
    for attr in attrs {
        if attr.path().is_ident("doc") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    let val = s.value();
                    lines.push(val.trim().to_string());
                }
            }
        }
    }
    if lines.is_empty() {
        None
    } else {
        Some(lines.join("\n"))
    }
}

/// Check whether an attribute path ends with `name` (matches both `#[sysml::part]`
/// and `#[part]` or `#[sysml_macros::part]`).
pub fn is_sysml_attr(attr: &Attribute, name: &str) -> bool {
    let segs: Vec<String> = attr
        .path()
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    match segs.as_slice() {
        [last] => last == name,
        [first, last] => (first == "sysml" || first == "sysml_macros" || first == "sysml_dsl") && last == name,
        [.., prev, last] => (prev == "sysml" || prev == "sysml_macros" || prev == "sysml_dsl") && last == name,
        _ => false,
    }
}

/// Convert camelCase SysML field identifier to snake_case Rust field identifier
/// for compile-time parent field redefinition checks.
pub fn camel_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            for lc in ch.to_lowercase() {
                out.push(lc);
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Convert a Rust `snake_case` field name into SysML `camelCase`.
pub fn snake_to_camel(s: &str) -> String {
    let s = s.trim_start_matches("r#");
    let mut out = String::with_capacity(s.len());
    let mut upper_next = false;
    for ch in s.chars() {
        if ch == '_' {
            if !out.is_empty() {
                upper_next = true;
            }
        } else if upper_next {
            out.push(ch.to_ascii_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Convert a Rust `snake_case` module name into SysML `PascalCase`.
pub fn snake_to_pascal(s: &str) -> String {
    let s = s.trim_start_matches("r#");
    let mut out = String::with_capacity(s.len());
    let mut upper_next = true;
    for ch in s.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.push(ch.to_ascii_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Extract an explicit `#[sysml::name = "..."]` override if present.
pub fn extract_sysml_name_override(attrs: &[Attribute]) -> syn::Result<Option<String>> {
    for attr in attrs {
        if is_sysml_attr(attr, "name") {
            if let Meta::NameValue(nv) = &attr.meta {
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
                {
                    return Ok(Some(s.value()));
                }
            }
            if let Meta::List(list) = &attr.meta {
                let s: syn::LitStr = list.parse_args()?;
                return Ok(Some(s.value()));
            }
        }
    }
    Ok(None)
}

/// Convert a `syn::Path` to a SysML qualified name string (`A::B`).
pub fn path_to_sysml_string(path: &Path) -> String {
    path.segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect::<Vec<_>>()
        .join("::")
}

/// Inspect a Rust field type to extract `(sysml_type_name, is_conjugated, implicit_multiplicity)`.
pub fn analyze_rust_type(ty: &Type) -> (String, bool, Option<(Option<String>, String)>) {
    if let Type::Path(tp) = ty {
        if let Some(last) = tp.path.segments.last() {
            let ident = last.ident.to_string();
            if ident == "Conjugated" {
                if let PathArguments::AngleBracketed(args) = &last.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        let (inner_name, _, mult) = analyze_rust_type(inner);
                        return (inner_name, true, mult);
                    }
                }
            }
            if ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &last.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        let (inner_name, conj, _) = analyze_rust_type(inner);
                        return (inner_name, conj, Some((None, "*".to_string())));
                    }
                }
            }
            if ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &last.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        let (inner_name, conj, _) = analyze_rust_type(inner);
                        return (inner_name, conj, Some((Some("0".to_string()), "1".to_string())));
                    }
                }
            }
            if ident == "Quantity" {
                return ("Real".to_string(), false, None);
            }
            let mapped = match ident.as_str() {
                "f64" | "f32" => "Real".to_string(),
                "i64" | "i32" | "isize" | "u64" | "u32" | "usize" => "Integer".to_string(),
                "bool" => "Boolean".to_string(),
                "String" => "String".to_string(),
                _ => path_to_sysml_string(&tp.path),
            };
            return (mapped, false, None);
        }
    }
    if let Type::Array(arr) = ty {
        let (inner_name, conj, _) = analyze_rust_type(&arr.elem);
        let len_str = quote!(#arr.len).to_string();
        let _ = len_str;
        let len = &arr.len;
        let upper = quote!(#len).to_string();
        return (inner_name, conj, Some((None, upper)));
    }
    (quote!(#ty).to_string(), false, None)
}

/// Quote an `Option<String>`.
pub fn quote_opt_str(opt: &Option<String>) -> TokenStream {
    match opt {
        Some(s) => quote!(::std::option::Option::Some(::std::string::String::from(#s))),
        None => quote!(::std::option::Option::None),
    }
}

/// Quote a `Vec<String>`.
pub fn quote_vec_str(v: &[String]) -> TokenStream {
    quote!(::std::vec![#(::std::string::String::from(#v)),*])
}

/// Parse a comma-separated list of Rust type paths, e.g. `HardwareComponent, PowerConsumer`.
pub struct PathList {
    pub paths: Punctuated<Path, Token![,]>,
}

impl Parse for PathList {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            paths: Punctuated::parse_terminated(input)?,
        })
    }
}

/// Parse key-value or flag arguments inside `#[sysml::foo(...)]`.
#[derive(Default)]
pub struct KvArgs {
    pub flags: Vec<String>,
    pub strings: Vec<(String, String)>,
    pub paths: Vec<(String, Path)>,
    /// Source spans of string-literal values (for precise diagnostics, e.g. `unit = "..."`).
    pub str_spans: Vec<(String, proc_macro2::Span)>,
}

impl Parse for KvArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut out = KvArgs::default();
        while !input.is_empty() {
            let key: Ident = if input.peek(Token![abstract]) {
                let tok: Token![abstract] = input.parse()?;
                Ident::new("abstract", tok.span)
            } else if input.peek(Token![ref]) {
                let tok: Token![ref] = input.parse()?;
                Ident::new("ref", tok.span)
            } else if input.peek(Token![type]) {
                let tok: Token![type] = input.parse()?;
                Ident::new("type", tok.span)
            } else {
                input.parse()?
            };
            let key_str = key.to_string();
            if input.peek(Token![=]) {
                let _eq: Token![=] = input.parse()?;
                if input.peek(syn::LitStr) {
                    let s: syn::LitStr = input.parse()?;
                    out.str_spans.push((key_str.clone(), s.span()));
                    out.strings.push((key_str, s.value()));
                } else if input.peek(syn::LitBool) {
                    let b: syn::LitBool = input.parse()?;
                    out.strings.push((key_str, b.value.to_string()));
                } else if input.peek(syn::LitInt) {
                    let n: syn::LitInt = input.parse()?;
                    out.strings.push((key_str, n.to_string()));
                } else if input.peek(syn::LitFloat) {
                    let n: syn::LitFloat = input.parse()?;
                    out.strings.push((key_str, n.to_string()));
                } else {
                    let p: Path = input.parse()?;
                    out.paths.push((key_str, p));
                }
            } else {
                out.flags.push(key_str);
            }
            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
            }
        }
        Ok(out)
    }
}

impl KvArgs {
    pub fn has_flag(&self, name: &str) -> bool {
        self.flags.iter().any(|f| f == name)
    }

    pub fn get_str(&self, name: &str) -> Option<String> {
        self.strings
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    }

    /// Span of the string literal bound to `name`, if any.
    pub fn get_str_span(&self, name: &str) -> Option<proc_macro2::Span> {
        self.str_spans
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, sp)| *sp)
    }

    pub fn get_all_str(&self, name: &str) -> Vec<String> {
        self.strings
            .iter()
            .filter(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
            .collect()
    }

    pub fn get_path(&self, name: &str) -> Option<Path> {
        self.paths
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.clone())
    }
}

/// Parse a multiplicity string such as `"*"`, `"1..*"`, `"0..2"`, `"3"`.
pub fn parse_multiplicity_str(s: &str) -> (Option<String>, String) {
    let trimmed = s.trim().trim_start_matches('[').trim_end_matches(']');
    if let Some((l, u)) = trimmed.split_once("..") {
        (Some(l.trim().to_string()), u.trim().to_string())
    } else {
        (None, trimmed.to_string())
    }
}

/// Quote an `Option<(Option<String>, String)>` as `Option<sysml_ir::Multiplicity>`.
pub fn quote_opt_mult(opt: &Option<(Option<String>, String)>) -> TokenStream {
    match opt {
        Some((lower, upper)) => {
            let l = quote_opt_str(lower);
            quote!(::std::option::Option::Some(::sysml_dsl::ir::Multiplicity {
                lower: #l,
                upper: ::std::string::String::from(#upper),
            }))
        }
        None => quote!(::std::option::Option::None),
    }
}

/// Strip all `sysml::*` helper attributes from an attribute slice so `rustc`
/// never sees unresolved custom helper attributes.
pub fn strip_sysml_attrs(attrs: Vec<Attribute>) -> Vec<Attribute> {
    const HELPER_NAMES: &[&str] = &[
        "name",
        "specializes",
        "perform",
        "satisfy",
        "verify",
        "connect",
        "allocate",
        "attr",
        "part",
        "port",
        "item",
        "in",
        "out",
        "inout",
        "in_item",
        "out_item",
        "inout_item",
        "in_param",
        "out_param",
        "inout_param",
        "end",
        "rationale",
        "require_constraint",
        "assume_constraint",
        "subject",
        "refines",
        "do_action",
        "entry_action",
        "exit_action",
        "initial",
        "transition",
        "import",
        "exhibit",
        "step",
        "subaction",
        "requirement_ref",
        "comment",
        "assert_constraint",
        "local",
        "return_expr",
        "safety_goal",
        "fmea",
        "independent",
        "asil_decompose",
        "no_std_reflect",
        "tbd",
    ];
    attrs
        .into_iter()
        .filter(|a| !HELPER_NAMES.iter().any(|h| is_sysml_attr(a, h)))
        .collect()
}

/// Returns `true` when an item or its enclosing `#[sysml::package(no_std)]` opted into
/// `#![no_std]` Ferrocene-compatible compilation with `#[cfg(feature = "reflect")]` IR gating.
pub fn has_no_std_mode(kv: &KvArgs, attrs: &[Attribute]) -> bool {
    kv.has_flag("no_std") || attrs.iter().any(|a| is_sysml_attr(a, "no_std_reflect"))
}

/// Returns `#[cfg(feature = "reflect")]` in `no_std` mode, or empty tokens on standard host builds.
pub fn reflect_cfg(is_no_std: bool) -> TokenStream {
    if is_no_std {
        quote!(#[cfg(feature = "reflect")])
    } else {
        quote!()
    }
}

/// Parse `#[sysml::comment("...")]` or `#[sysml::comment(name = "...", body = "...")]`.
pub fn extract_comments(attrs: &[Attribute]) -> syn::Result<Vec<(Option<String>, String)>> {
    let mut out = Vec::new();
    for attr in attrs {
        if is_sysml_attr(attr, "comment") {
            if let Ok(lit) = attr.parse_args::<syn::LitStr>() {
                out.push((None, lit.value()));
            } else {
                let kv: KvArgs = attr.parse_args()?;
                let name = kv.get_str("name");
                let body = kv.get_str("body").ok_or_else(|| {
                    syn::Error::new_spanned(
                        attr,
                        "expected `body = \"...\"` in #[sysml::comment(...)]",
                    )
                })?;
                out.push((name, body));
            }
        }
    }
    Ok(out)
}

/// Parse `#refinement dependency` helper attributes:
/// `#[sysml::refines(supplier = "Pkg::Target")]` or `#[sysml::refines(client = "...", supplier = "...")]`
pub fn extract_refinements(attrs: &[Attribute], default_client: &str) -> syn::Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    for attr in attrs {
        if is_sysml_attr(attr, "refines") {
            let args: KvArgs = attr.parse_args()?;
            let client = args
                .get_str("client")
                .unwrap_or_else(|| default_client.to_string());
            let supplier = args.get_str("supplier").ok_or_else(|| {
                syn::Error::new_spanned(attr, "expected `supplier = \"...\"` in #[sysml::refines(...)]")
            })?;
            out.push((client, supplier));
        }
    }
    Ok(out)
}

/// Parse `#[sysml::attr(...)]` on a struct field into an IR builder token stream.
pub fn build_attr_usage_tokens(
    field_name_rust: &str,
    field_ty: &Type,
    attrs: &[Attribute],
) -> syn::Result<TokenStream> {
    let doc = extract_doc(attrs);
    let doc_tok = quote_opt_str(&doc);
    let mut kv = KvArgs::default();
    let mut attr_span = syn::spanned::Spanned::span(field_ty);
    for attr in attrs {
        if is_sysml_attr(attr, "attr") {
            attr_span = syn::spanned::Spanned::span(attr);
            if let Meta::List(_) = &attr.meta {
                kv = attr.parse_args()?;
            }
        }
    }
    crate::units_check::require_unit_decl(&kv, field_name_rust, field_ty, attr_span)?;
    let tbd_marks = crate::tbd::attr_marks(&kv, attr_span)?;
    let tbd_tok = crate::tbd::quote_marks(&tbd_marks);
    let name_override = extract_sysml_name_override(attrs)?.or_else(|| kv.get_str("name"));
    let mut redefines = kv.get_all_str("redefines");
    if redefines.is_empty() && kv.has_flag("redefines") {
        redefines.push(snake_to_camel(field_name_rust));
    }
    let subsets = kv.get_all_str("subsets");
    let is_keywordless = kv.has_flag("keywordless");
    let is_evidence = kv.has_flag("evidence");
    let is_anonymous_redef = kv.has_flag("anonymous")
        || (!redefines.is_empty() && name_override.is_none() && kv.get_str("keep_name").is_none())
        || (is_keywordless && !subsets.is_empty() && name_override.is_none() && kv.get_str("keep_name").is_none());

    let attr_name: Option<String> = if is_anonymous_redef {
        None
    } else {
        Some(name_override.unwrap_or_else(|| snake_to_camel(field_name_rust)))
    };

    let (rust_ty_name, _, rust_mult) = analyze_rust_type(field_ty);
    let explicit_ty = kv.get_str("ty").or_else(|| kv.get_str("type"));
    let type_name: Option<String> = if let Some(t) = explicit_ty {
        if t.is_empty() { None } else { Some(t) }
    } else if !subsets.is_empty() || !redefines.is_empty() || is_keywordless {
        // In SysML v2, `attribute mass :> ISQ::mass`, `attribute :>> dryMass = 2100 [kg]`,
        // or `:> influenceLevel = ...` does not also emit `: Real`.
        None
    } else {
        Some(rust_ty_name)
    };

    let mult = kv
        .get_str("mult")
        .map(|m| parse_multiplicity_str(&m))
        .or(rust_mult);

    let value_kind = match kv.get_str("value_kind").as_deref() {
        Some("initial") => quote!(::sysml_dsl::ir::ValueKind::Initial),
        Some("default") => quote!(::sysml_dsl::ir::ValueKind::Default),
        Some("default_initial") => quote!(::sysml_dsl::ir::ValueKind::DefaultInitial),
        _ => {
            if kv.get_str("default").is_some() {
                quote!(::sysml_dsl::ir::ValueKind::Default)
            } else {
                quote!(::sysml_dsl::ir::ValueKind::Bound)
            }
        }
    };

    let value = kv
        .get_str("value")
        .or_else(|| kv.get_str("default"))
        .or_else(|| kv.get_str("expr"));
    let unit = kv.get_str("unit");
    crate::units_check::validate_unit_kv(&kv)?;

    let name_tok = quote_opt_str(&attr_name);
    let ty_tok = quote_opt_str(&type_name);
    let subsets_tok = quote_vec_str(&subsets);
    let redefines_tok = quote_vec_str(&redefines);
    let mult_tok = quote_opt_mult(&mult);
    let val_tok = quote_opt_str(&value);
    let unit_tok = quote_opt_str(&unit);

    Ok(quote! {
        ::sysml_dsl::ir::AttributeUsage {
            short_name: ::std::option::Option::None,
            name: #name_tok,
            is_private: false,
            is_nonunique: false,
            type_name: #ty_tok,
            subsets: #subsets_tok,
            redefines: #redefines_tok,
            multiplicity: #mult_tok,
            value_kind: #value_kind,
            value: #val_tok,
            unit: #unit_tok,
            doc: #doc_tok,
            is_keywordless: #is_keywordless,
            nested_attributes: ::std::vec![],
            is_evidence: #is_evidence,
            tbd: #tbd_tok,
        }
    })
}

/// Returns `true` if `#[derive(..., Default, ...)]` is already present on `attrs`.
pub fn has_derive_default(attrs: &[Attribute]) -> bool {
    for attr in attrs {
        if attr.path().is_ident("derive") {
            let mut found = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("Default") {
                    found = true;
                }
                Ok(())
            });
            if found {
                return true;
            }
        }
    }
    false
}

/// Auto-generates `impl Default for #ident` from `#[sysml::attr(value = "...", default = "...", unit = "...")]`
/// metadata whenever `#[derive(Default)]` is NOT explicitly present on a named-field struct.
///
/// Works in both `std` and `#![no_std]` crates.
pub fn build_auto_default_impl(item: &syn::ItemStruct) -> syn::Result<TokenStream> {
    if has_derive_default(&item.attrs) {
        return Ok(quote!());
    }
    let syn::Fields::Named(fields) = &item.fields else {
        return Ok(quote!());
    };

    let mut has_any_literal_default = false;
    let mut field_inits = Vec::new();
    for field in &fields.named {
        let fname = field.ident.as_ref().unwrap();
        let mut kv = KvArgs::default();
        for attr in &field.attrs {
            if is_sysml_attr(attr, "attr") {
                if let Meta::List(_) = &attr.meta {
                    kv = attr.parse_args()?;
                }
            }
        }
        let raw_val = kv.get_str("value").or_else(|| kv.get_str("default"));
        let unit = kv.get_str("unit").unwrap_or_default();
        let is_float = is_quantity_type(&field.ty)
            || is_prim_ident(&field.ty, "f64")
            || is_prim_ident(&field.ty, "f32");

        let init_expr = if raw_val.is_none() && kv.has_flag("evidence") && is_float {
            // Unmeasured evidence is NaN: every comparison is false, so `evaluate()` cannot
            // pass until a verification case supplies the measured value.
            has_any_literal_default = true;
            if is_quantity_type(&field.ty) {
                quote!(::sysml_dsl::core_contracts::Quantity::new(f64::NAN, #unit))
            } else {
                quote!(f64::NAN as _)
            }
        } else if let Some(val_str) = raw_val {
            let trimmed = val_str.trim();
            if is_quantity_type(&field.ty) {
                if let Ok(num) = trimmed.parse::<f64>() {
                    has_any_literal_default = true;
                    quote!(::sysml_dsl::core_contracts::Quantity::new(#num, #unit))
                } else {
                    quote!(::core::default::Default::default())
                }
            } else if is_prim_ident(&field.ty, "f64") || is_prim_ident(&field.ty, "f32") {
                if let Ok(num) = trimmed.parse::<f64>() {
                    has_any_literal_default = true;
                    quote!(#num as _)
                } else {
                    quote!(::core::default::Default::default())
                }
            } else if is_prim_ident(&field.ty, "i64")
                || is_prim_ident(&field.ty, "i32")
                || is_prim_ident(&field.ty, "u32")
                || is_prim_ident(&field.ty, "u64")
                || is_prim_ident(&field.ty, "usize")
            {
                if let Ok(num) = trimmed.parse::<i64>() {
                    has_any_literal_default = true;
                    quote!(#num as _)
                } else {
                    quote!(::core::default::Default::default())
                }
            } else if is_prim_ident(&field.ty, "bool") {
                if let Ok(b) = trimmed.parse::<bool>() {
                    has_any_literal_default = true;
                    quote!(#b)
                } else {
                    quote!(::core::default::Default::default())
                }
            } else if is_prim_ident(&field.ty, "String") {
                has_any_literal_default = true;
                let unquoted = trimmed
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(trimmed);
                quote!(::std::string::String::from(#unquoted))
            } else {
                quote!(::core::default::Default::default())
            }
        } else if is_quantity_type(&field.ty) && !unit.is_empty() {
            has_any_literal_default = true;
            quote!(::sysml_dsl::core_contracts::Quantity::new(0.0, #unit))
        } else {
            quote!(::core::default::Default::default())
        };

        field_inits.push(quote!(#fname: #init_expr));
    }

    let _ = has_any_literal_default;
    let ident = &item.ident;
    Ok(quote! {
        impl ::core::default::Default for #ident {
            fn default() -> Self {
                Self {
                    #(#field_inits),*
                }
            }
        }
    })
}

fn is_quantity_type(ty: &Type) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(last) = tp.path.segments.last() {
            return last.ident == "Quantity";
        }
    }
    false
}

fn is_prim_ident(ty: &Type, name: &str) -> bool {
    if let Type::Path(tp) = ty {
        if let Some(last) = tp.path.segments.last() {
            return last.ident == name;
        }
    }
    false
}
