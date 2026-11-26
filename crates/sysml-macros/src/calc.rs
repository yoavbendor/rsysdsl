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
use syn::{FnArg, ItemFn, LocalInit, Pat, ReturnType, Stmt};

use crate::expr_transpile::{calc_marker_path, transpile_expr, ExprContext};
use crate::util::{
    analyze_rust_type, extract_doc, extract_sysml_name_override, quote_opt_str, snake_to_camel,
    strip_sysml_attrs, KvArgs,
};

pub fn expand_calc(args: TokenStream, mut func: ItemFn) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
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
                    ::sysml_ir::CalcParam {
                        name: ::std::string::String::from(#p_sysml),
                        type_name: ::std::string::String::from(#p_ty),
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

    // Walk every statement in the function body: only `let x = <expr>;` locals
    // followed by a single trailing return expression `<expr>` are allowed!
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
                    ::sysml_ir::CalcLocal {
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

    // Compile-time check: every function called inside this #[sysml::calc] MUST itself
    // be a modeled #[sysml::calc] function!
    let callee_checks = all_callees.iter().map(|callee| {
        let marker = calc_marker_path(callee);
        quote! {
            ::sysml_ir::assert_calc::<#marker>();
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

        impl ::sysml_ir::SysmlCalc for #marker_ident {
            const SYSML_NAME: &'static str = #sysml_name;

            fn calc_def() -> ::sysml_ir::CalcDef {
                ::sysml_ir::CalcDef {
                    name: ::std::string::String::from(#sysml_name),
                    doc: #doc_tok,
                    params: ::std::vec![#(#param_tokens),*],
                    locals: ::std::vec![#(#local_tokens),*],
                    return_name: ::std::string::String::from(#return_name),
                    return_type: ::std::string::String::from(#return_type),
                    result_expr: ::std::string::String::from(#result_expr),
                }
            }
        }

        const _: fn() = || {
            #(#callee_checks)*
        };
    })
}
