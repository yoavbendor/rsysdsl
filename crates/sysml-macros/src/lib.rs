//! Procedural macro entry points for `rsysdsl`.
//!
//! Primary element macros:
//! - `#[sysml::package]` (on `mod`)
//! - `#[sysml::part]` (on `struct`)
//! - `#[sysml::port]` (on `struct`)
//! - `#[sysml::interface]` (on `struct`)
//! - `#[sysml::connection]` (on `struct`)
//! - `#[sysml::item]` (on `struct`)
//! - `#[sysml::enum_def]` (on `enum`)
//! - `#[sysml::action]` (on `struct`)
//! - `#[sysml::state]` (on `struct` or `enum`)
//! - `#[sysml::requirement]` (on `struct`)
//! - `#[sysml::specification]` (on `struct`)
//! - `#[sysml::verification]` (on `struct`)
//!
//! Relationship / traceability macros (may appear below OR above a primary macro):
//! - `#[sysml::specializes(...)]`
//! - `#[sysml::perform(...)]`
//! - `#[sysml::satisfy(...)]`
//! - `#[sysml::verify(...)]`
//! - `#[sysml::connect(...)]`

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Item, ItemEnum, ItemMod, ItemStruct};

mod analysis;
mod behavior;
mod calc;
mod expr_transpile;
mod package;
mod part;
mod port;
mod requirement;
mod units_check;
mod util;

use util::{is_sysml_attr, PathList};

#[proc_macro_attribute]
pub fn calc(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    calc::expand_calc(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn analysis(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    analysis::expand_analysis(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn package(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemMod);
    package::expand_package(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn part(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    part::expand_part(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn port(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    port::expand_port(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn interface(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    port::expand_interface(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn connection(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    port::expand_connection(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn item(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    port::expand_item(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn enum_def(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemEnum);
    port::expand_enum_def(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn action(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    behavior::expand_action(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn state(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as Item);
    behavior::expand_state(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn requirement(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    requirement::expand_requirement(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn specification(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    requirement::expand_specification(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn verification(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemStruct);
    requirement::expand_verification(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Helper: check if an item still carries a primary `#[sysml::*]` attribute below
/// the current macro; if so, reorder the primary attribute to the top so it
/// consumes this helper attribute cleanly regardless of user attribute ordering.
fn reorder_below_primary(
    helper_name: &str,
    args: proc_macro2::TokenStream,
    item: &mut Item,
) -> Option<proc_macro2::TokenStream> {
    const PRIMARY: &[&str] = &[
        "part",
        "variation",
        "port",
        "interface",
        "connection",
        "item",
        "action",
        "state",
        "requirement",
        "specification",
        "verification",
        "calc",
        "analysis",
    ];
    let attrs = match item {
        Item::Struct(s) => &mut s.attrs,
        Item::Enum(e) => &mut e.attrs,
        _ => return None,
    };
    let idx = attrs
        .iter()
        .position(|a| PRIMARY.iter().any(|p| is_sysml_attr(a, p)))?;
    let primary_attr: Attribute = attrs.remove(idx);
    let helper_ident = syn::Ident::new(helper_name, proc_macro2::Span::call_site());
    let helper_attr: Attribute = syn::parse_quote!(#[::sysml_macros::#helper_ident(#args)]);
    attrs.insert(0, helper_attr);
    attrs.insert(0, primary_attr);
    Some(quote!(#item))
}

#[proc_macro_attribute]
pub fn specializes(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2: proc_macro2::TokenStream = args.clone().into();
    let mut item = parse_macro_input!(input as Item);
    if let Some(reordered) = reorder_below_primary("specializes", args2.clone(), &mut item) {
        return reordered.into();
    }
    let list = match syn::parse2::<PathList>(args2) {
        Ok(l) => l,
        Err(e) => return e.to_compile_error().into(),
    };
    let ident = match &item {
        Item::Struct(s) => s.ident.clone(),
        Item::Enum(e) => e.ident.clone(),
        _ => {
            return syn::Error::new_spanned(
                item,
                "#[sysml::specializes] can only be applied to structs or enums",
            )
            .to_compile_error()
            .into();
        }
    };
    let impls = list.paths.iter().map(|base| {
        quote! { impl ::sysml_dsl::core_contracts::Specializes<#base> for #ident {} }
    });
    quote! {
        #item
        #(#impls)*
    }
    .into()
}

#[proc_macro_attribute]
pub fn perform(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2: proc_macro2::TokenStream = args.into();
    let mut item = parse_macro_input!(input as Item);
    if let Some(reordered) = reorder_below_primary("perform", args2.clone(), &mut item) {
        return reordered.into();
    }
    let parsed = match syn::parse2::<part::PerformArgs>(args2) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };
    let ident = match &item {
        Item::Struct(s) => s.ident.clone(),
        _ => {
            return syn::Error::new_spanned(item, "#[sysml::perform] requires a struct")
                .to_compile_error()
                .into();
        }
    };
    let apath = parsed.action_path;
    quote! {
        #item
        impl ::sysml_dsl::core_contracts::Performs<#apath> for #ident {}
        const _: fn() = || { ::sysml_dsl::core_contracts::assert_action_core::<#apath>(); };
    }
    .into()
}

#[proc_macro_attribute]
pub fn satisfy(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2: proc_macro2::TokenStream = args.into();
    let mut item = parse_macro_input!(input as Item);
    if let Some(reordered) = reorder_below_primary("satisfy", args2.clone(), &mut item) {
        return reordered.into();
    }
    let parsed = match syn::parse2::<part::ReqLinkArgs>(args2) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };
    let ident = match &item {
        Item::Struct(s) => s.ident.clone(),
        _ => {
            return syn::Error::new_spanned(item, "#[sysml::satisfy] requires a struct")
                .to_compile_error()
                .into();
        }
    };
    let rpath = parsed.req_path;
    let id_check = if let Some(exp_id) = parsed.expected_id {
        quote! {
            const _: () = {
                if !::sysml_dsl::core_contracts::const_str_eq(<#rpath as ::sysml_dsl::core_contracts::SysmlRequirementCore>::REQ_ID, #exp_id) {
                    panic!("SysML requirement ID mismatch in #[sysml::satisfy]");
                }
            };
        }
    } else {
        quote!()
    };
    quote! {
        #item
        impl ::sysml_dsl::core_contracts::Satisfies<#rpath> for #ident {}
        const _: fn() = || { ::sysml_dsl::core_contracts::assert_requirement_core::<#rpath>(); };
        #id_check
    }
    .into()
}

#[proc_macro_attribute]
pub fn verify(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2: proc_macro2::TokenStream = args.into();
    let mut item = parse_macro_input!(input as Item);
    if let Some(reordered) = reorder_below_primary("verify", args2.clone(), &mut item) {
        return reordered.into();
    }
    let parsed = match syn::parse2::<part::ReqLinkArgs>(args2) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error().into(),
    };
    let ident = match &item {
        Item::Struct(s) => s.ident.clone(),
        _ => {
            return syn::Error::new_spanned(item, "#[sysml::verify] requires a struct")
                .to_compile_error()
                .into();
        }
    };
    let rpath = parsed.req_path;
    quote! {
        #item
        impl ::sysml_dsl::core_contracts::Verifies<#rpath> for #ident {}
        const _: fn() = || { ::sysml_dsl::core_contracts::assert_requirement_core::<#rpath>(); };
    }
    .into()
}

#[proc_macro_attribute]
pub fn connect(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2: proc_macro2::TokenStream = args.into();
    let mut item = parse_macro_input!(input as Item);
    if let Some(reordered) = reorder_below_primary("connect", args2, &mut item) {
        return reordered.into();
    }
    syn::Error::new_spanned(
        item,
        "#[sysml::connect] must be attached to a #[sysml::part] struct",
    )
    .to_compile_error()
    .into()
}

#[proc_macro_attribute]
pub fn safety_goal(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2: proc_macro2::TokenStream = args.into();
    let mut item = parse_macro_input!(input as Item);
    if let Some(reordered) = reorder_below_primary("safety_goal", args2, &mut item) {
        return reordered.into();
    }
    syn::Error::new_spanned(
        item,
        "#[sysml::safety_goal] must be attached to a #[sysml::requirement] struct",
    )
    .to_compile_error()
    .into()
}

#[proc_macro_attribute]
pub fn fmea(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2: proc_macro2::TokenStream = args.into();
    let mut item = parse_macro_input!(input as Item);
    if let Some(reordered) = reorder_below_primary("fmea", args2, &mut item) {
        return reordered.into();
    }
    syn::Error::new_spanned(
        item,
        "#[sysml::fmea] must be attached to a #[sysml::part] struct",
    )
    .to_compile_error()
    .into()
}

#[proc_macro_attribute]
pub fn independent(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2: proc_macro2::TokenStream = args.into();
    let mut item = parse_macro_input!(input as Item);
    if let Some(reordered) = reorder_below_primary("independent", args2, &mut item) {
        return reordered.into();
    }
    syn::Error::new_spanned(
        item,
        "#[sysml::independent] must be attached to a #[sysml::part] struct",
    )
    .to_compile_error()
    .into()
}

#[proc_macro_attribute]
pub fn asil_decompose(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2: proc_macro2::TokenStream = args.into();
    let mut item = parse_macro_input!(input as Item);
    if let Some(reordered) = reorder_below_primary("asil_decompose", args2, &mut item) {
        return reordered.into();
    }
    syn::Error::new_spanned(
        item,
        "#[sysml::asil_decompose] must be attached to a #[sysml::part] struct",
    )
    .to_compile_error()
    .into()
}

#[proc_macro_attribute]
pub fn variation(args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemEnum);
    part::expand_variation(args.into(), item)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

#[proc_macro_attribute]
pub fn allocate(args: TokenStream, input: TokenStream) -> TokenStream {
    let args2: proc_macro2::TokenStream = args.into();
    let mut item = parse_macro_input!(input as Item);
    if let Some(reordered) = reorder_below_primary("allocate", args2, &mut item) {
        return reordered.into();
    }
    syn::Error::new_spanned(
        item,
        "#[sysml::allocate] must be attached to a #[sysml::part] struct",
    )
    .to_compile_error()
    .into()
}

#[proc_macro_attribute]
pub fn import(_args: TokenStream, input: TokenStream) -> TokenStream {
    input
}
