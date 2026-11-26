//! Proc-macro expansion for `#[sysml::calc]` on Rust functions.
//!
//! Enforces Tier-2 modeled behavior purity:
//! - Every parameter and return type must map to a SysML v2 type.
//! - Every statement in the body must be either a `let name = <transpilable_expr>;`
//!   binding or the trailing `<transpilable_expr>` return value.
//! - Calling any function that is NOT itself annotated `#[sysml::calc]` fails
//!   `rustc` compilation via `__sysml_calc_marker_<callee>: SysmlCalc`!

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeMap;
use syn::{Fields, FnArg, Item, ItemFn, ItemStruct, LocalInit, Meta, Pat, ReturnType, Stmt};

use crate::expr_transpile::{calc_marker_path, transpile_expr, ExprContext};
use crate::util::{
    analyze_rust_type, extract_doc, extract_sysml_name_override, has_no_std_mode, is_sysml_attr,
    parse_multiplicity_str, quote_opt_mult, quote_opt_str, reflect_cfg, snake_to_camel,
    strip_sysml_attrs, KvArgs,
};

pub fn expand_calc(args: TokenStream, item: Item) -> syn::Result<TokenStream> {
    match item {
        Item::Fn(func) => expand_calc_fn(args, func),
        Item::Struct(st) => expand_calc_struct(args, st),
        other => Err(syn::Error::new_spanned(
            other,
            "#[sysml::calc] can only be applied to a `fn` (executable calc) or `struct` (declarative lifted calc)",
        )),
    }
}

fn expand_calc_struct(args: TokenStream, mut st: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &st.attrs);
    let rcfg = reflect_cfg(is_no_std);
    let doc = extract_doc(&st.attrs);
    let sysml_name = extract_sysml_name_override(&st.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| st.ident.to_string());

    let mut local_tokens = Vec::new();
    let mut return_name = String::new();
    let mut return_type = String::new();
    let mut return_subsets: Option<String> = None;
    let mut result_expr = String::new();

    for attr in &st.attrs {
        if is_sysml_attr(attr, "local") {
            let lkv: KvArgs = attr.parse_args()?;
            let lname = lkv.get_str("name").unwrap_or_default();
            let lexpr = lkv.get_str("expr").unwrap_or_default();
            local_tokens.push(quote! {
                ::sysml_dsl::ir::CalcLocal {
                    name: ::std::string::String::from(#lname),
                    expr: ::std::string::String::from(#lexpr),
                }
            });
        } else if is_sysml_attr(attr, "return_expr") {
            let rkv: KvArgs = attr.parse_args()?;
            return_name = rkv.get_str("name").unwrap_or_default();
            return_type = rkv
                .get_str("ty")
                .or_else(|| rkv.get_str("type"))
                .unwrap_or_default();
            return_subsets = rkv.get_str("subsets");
            result_expr = rkv.get_str("expr").unwrap_or_default();
        }
    }

    let mut param_tokens = Vec::new();
    if let Fields::Named(fields) = &mut st.fields {
        for field in &mut fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            let mut fkv = KvArgs::default();
            for attr in &field.attrs {
                if is_sysml_attr(attr, "in_param") {
                    if let Meta::List(_) = &attr.meta {
                        fkv = attr.parse_args()?;
                    }
                }
            }
            let pname = extract_sysml_name_override(&field.attrs)?
                .or_else(|| fkv.get_str("name"))
                .unwrap_or_else(|| snake_to_camel(&fname_rust));
            let (rust_ty, _, rust_mult) = analyze_rust_type(&field.ty);
            let subsets = fkv.get_str("subsets");
            let type_name = fkv
                .get_str("ty")
                .or_else(|| fkv.get_str("type"))
                .unwrap_or_else(|| if subsets.is_some() { String::new() } else { rust_ty });
            let mult = fkv
                .get_str("mult")
                .map(|m| parse_multiplicity_str(&m))
                .or(rust_mult);
            let subsets_tok = quote_opt_str(&subsets);
            let mult_tok = quote_opt_mult(&mult);
            param_tokens.push(quote! {
                ::sysml_dsl::ir::CalcParam {
                    name: ::std::string::String::from(#pname),
                    type_name: ::std::string::String::from(#type_name),
                    subsets: #subsets_tok,
                    multiplicity: #mult_tok,
                }
            });
            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    st.attrs = strip_sysml_attrs(std::mem::take(&mut st.attrs));
    let ident = &st.ident;
    let doc_tok = quote_opt_str(&doc);
    let ret_sub_tok = quote_opt_str(&return_subsets);

    Ok(quote! {
        #st

        impl ::sysml_dsl::core_contracts::SysmlCalcCore for #ident {
            const SYSML_NAME: &'static str = #sysml_name;
        }

        #rcfg
        impl ::sysml_dsl::ir::SysmlCalc for #ident {
            fn calc_def() -> ::sysml_dsl::ir::CalcDef {
                ::sysml_dsl::ir::CalcDef {
                    name: ::std::string::String::from(#sysml_name),
                    doc: #doc_tok,
                    params: ::std::vec![#(#param_tokens),*],
                    locals: ::std::vec![#(#local_tokens),*],
                    return_name: ::std::string::String::from(#return_name),
                    return_type: ::std::string::String::from(#return_type),
                    return_subsets: #ret_sub_tok,
                    result_expr: ::std::string::String::from(#result_expr),
                }
            }
        }
    })
}

fn expand_calc_fn(args: TokenStream, mut func: ItemFn) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &func.attrs);
    let rcfg = reflect_cfg(is_no_std);
    if func.sig.asyncness.is_some() {
        return Err(syn::Error::new_spanned(
            func.sig.asyncness,
            "`async fn` is forbidden in #[sysml::calc] (semantic leakage prevented)",
        ));
    }
    if func.sig.unsafety.is_some() {
        return Err(syn::Error::new_spanned(
            func.sig.unsafety,
            "`unsafe fn` is forbidden in #[sysml::calc] (semantic leakage prevented)",
        ));
    }

    let doc = extract_doc(&func.attrs);
    let fn_ident = func.sig.ident.clone();
    let sysml_name = extract_sysml_name_override(&func.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| snake_to_camel(&fn_ident.to_string()));
    let return_name = kv.get_str("return_name").unwrap_or_else(|| "result".to_string());

    let mut bindings: BTreeMap<String, String> = BTreeMap::new();
    let mut param_tokens = Vec::new();
    for arg in &func.sig.inputs {
        match arg {
            FnArg::Typed(pt) => {
                let pname_ident = match &*pt.pat {
                    Pat::Ident(pi) => &pi.ident,
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "#[sysml::calc] parameters must be simple identifiers",
                        ));
                    }
                };
                let p_rust = pname_ident.to_string();
                let p_sysml = snake_to_camel(&p_rust);
                let (p_ty, _, _) = analyze_rust_type(&pt.ty);
                bindings.insert(p_rust, p_sysml.clone());
                param_tokens.push(quote! {
                    ::sysml_dsl::ir::CalcParam {
                        name: ::std::string::String::from(#p_sysml),
                        type_name: ::std::string::String::from(#p_ty),
                        subsets: ::std::option::Option::None,
                        multiplicity: ::std::option::Option::None,
                    }
                });
            }
            FnArg::Receiver(r) => {
                return Err(syn::Error::new_spanned(
                    r,
                    "`self` receiver is not allowed on #[sysml::calc] free functions",
                ));
            }
        }
    }

    let return_type = match &func.sig.output {
        ReturnType::Type(_, ty) => analyze_rust_type(ty).0,
        ReturnType::Default => {
            return Err(syn::Error::new_spanned(
                &func.sig,
                "#[sysml::calc] function must declare an explicit return type",
            ));
        }
    };

    let mut local_tokens = Vec::new();
    let mut all_callees = Vec::new();
    let mut result_expr_sysml: Option<String> = None;

    let stmts = &func.block.stmts;
    for (idx, stmt) in stmts.iter().enumerate() {
        let is_last = idx + 1 == stmts.len();
        match stmt {
            Stmt::Local(loc) => {
                if is_last {
                    return Err(syn::Error::new_spanned(
                        loc,
                        "#[sysml::calc] must end with a return expression",
                    ));
                }
                let pat_ident = match &loc.pat {
                    Pat::Ident(pi) => &pi.ident,
                    Pat::Type(pt) => match &*pt.pat {
                        Pat::Ident(pi) => &pi.ident,
                        other => {
                            return Err(syn::Error::new_spanned(
                                other,
                                "#[sysml::calc] `let` binding must use a simple identifier",
                            ));
                        }
                    },
                    other => {
                        return Err(syn::Error::new_spanned(
                            other,
                            "#[sysml::calc] `let` binding must use a simple identifier",
                        ));
                    }
                };
                let LocalInit { expr, diverge, .. } = loc.init.as_ref().ok_or_else(|| {
                    syn::Error::new_spanned(loc, "#[sysml::calc] `let` binding must be initialized")
                })?;
                if let Some((else_tok, _)) = diverge {
                    return Err(syn::Error::new_spanned(
                        else_tok,
                        "`let ... else` is forbidden in #[sysml::calc]",
                    ));
                }
                let ctx = ExprContext::Calc {
                    bindings: &bindings,
                };
                let transpiled = transpile_expr(expr, &ctx)?;
                all_callees.extend(transpiled.calc_callees);
                let l_rust = pat_ident.to_string();
                let l_sysml = snake_to_camel(&l_rust);
                let expr_str = transpiled.sysml;
                bindings.insert(l_rust, l_sysml.clone());
                local_tokens.push(quote! {
                    ::sysml_dsl::ir::CalcLocal {
                        name: ::std::string::String::from(#l_sysml),
                        expr: ::std::string::String::from(#expr_str),
                    }
                });
            }
            Stmt::Expr(expr, None) if is_last => {
                let ctx = ExprContext::Calc {
                    bindings: &bindings,
                };
                let transpiled = transpile_expr(expr, &ctx)?;
                all_callees.extend(transpiled.calc_callees);
                result_expr_sysml = Some(transpiled.sysml);
            }
            other => {
                return Err(syn::Error::new_spanned(
                    other,
                    "forbidden imperative statement inside #[sysml::calc]; only `let` bindings and a trailing SysML-transpilable return expression are permitted",
                ));
            }
        }
    }

    let result_expr = result_expr_sysml.ok_or_else(|| {
        syn::Error::new_spanned(
            &func.block,
            "#[sysml::calc] body must end with a SysML-transpilable return expression",
        )
    })?;

    let callee_checks = all_callees.iter().map(|callee| {
        let marker = calc_marker_path(callee);
        quote! {
            ::sysml_dsl::core_contracts::assert_calc_core::<#marker>();
        }
    });

    func.attrs = strip_sysml_attrs(std::mem::take(&mut func.attrs));
    let marker_ident = format_ident!("__sysml_calc_marker_{}", fn_ident);
    let doc_tok = quote_opt_str(&doc);

    Ok(quote! {
        #func

        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        pub struct #marker_ident;

        impl ::sysml_dsl::core_contracts::SysmlCalcCore for #marker_ident {
            const SYSML_NAME: &'static str = #sysml_name;
        }

        #rcfg
        impl ::sysml_dsl::ir::SysmlCalc for #marker_ident {
            fn calc_def() -> ::sysml_dsl::ir::CalcDef {
                ::sysml_dsl::ir::CalcDef {
                    name: ::std::string::String::from(#sysml_name),
                    doc: #doc_tok,
                    params: ::std::vec![#(#param_tokens),*],
                    locals: ::std::vec![#(#local_tokens),*],
                    return_name: ::std::string::String::from(#return_name),
                    return_type: ::std::string::String::from(#return_type),
                    return_subsets: ::std::option::Option::None,
                    result_expr: ::std::string::String::from(#result_expr),
                }
            }
        }

        const _: fn() = || {
            #(#callee_checks)*
        };
    })
}
