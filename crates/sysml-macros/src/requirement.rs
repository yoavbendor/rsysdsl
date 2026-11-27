//! Proc-macro expansions for `#[sysml::requirement]`, `#[sysml::specification]`,
//! and `#[sysml::verification]`.

use proc_macro2::TokenStream;
use quote::quote;
use std::collections::BTreeMap;
use syn::{Expr, ExprLit, Fields, ItemStruct, Lit, LitStr, Meta};

use crate::expr_transpile::{transpile_expr, ExprContext};
use crate::part::ReqLinkArgs;
use crate::units_check::check_requirement_dims;
use crate::util::{
    analyze_rust_type, build_attr_usage_tokens, build_auto_default_impl, extract_doc,
    extract_refinements, extract_sysml_name_override, has_no_std_mode, is_sysml_attr,
    path_to_sysml_string, quote_opt_str, quote_vec_str, reflect_cfg, snake_to_camel,
    strip_sysml_attrs, KvArgs, PathList,
};

pub fn expand_requirement(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &item.attrs);
    let rcfg = reflect_cfg(is_no_std);
    let doc = extract_doc(&item.attrs);
    let is_abstract = kv.has_flag("abstract");
    let short_name = kv.get_str("id").or_else(|| kv.get_str("short_name"));
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| item.ident.to_string());
    let req_id_const = short_name.clone().unwrap_or_else(|| sysml_name.clone());

    let auto_default_impl = build_auto_default_impl(&item)?;

    // Build map of Rust field name -> SysML attribute name for native expression transpilation
    let mut field_map: BTreeMap<String, String> = BTreeMap::new();
    // Rust field name -> declared `unit` (for compile-time dimensional analysis of constraints)
    let mut unit_map: BTreeMap<String, String> = BTreeMap::new();
    if let Fields::Named(fields) = &item.fields {
        for field in &fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            let mut fkv = KvArgs::default();
            for a in &field.attrs {
                if is_sysml_attr(a, "attr") {
                    if let Meta::List(_) = &a.meta {
                        fkv = a.parse_args()?;
                    }
                }
            }
            let fname_sysml = extract_sysml_name_override(&field.attrs)?
                .or_else(|| fkv.get_str("name"))
                .unwrap_or_else(|| snake_to_camel(&fname_rust));
            unit_map.insert(fname_rust.clone(), crate::units_check::checked_unit(&fkv));
            field_map.insert(fname_rust, fname_sysml);
        }
    }

    let mut specializes_paths = Vec::new();
    let mut rationale: Option<String> = kv.get_str("rationale");
    let mut require_constraints = Vec::new();
    let mut require_eval_exprs: Vec<TokenStream> = Vec::new();
    let mut assume_constraints = Vec::new();
    let mut subject: Option<(String, String)> = None;
    let mut subject_check = quote!();
    let mut safety_goal_impl = quote!();
    let mut safety_goal_meta_tok = quote!(::std::option::Option::None);

    let ident = item.ident.clone();

    for attr in &item.attrs {
        if is_sysml_attr(attr, "specializes") {
            let list: PathList = attr.parse_args()?;
            specializes_paths.extend(list.paths.into_iter());
        } else if is_sysml_attr(attr, "rationale") {
            let s: LitStr = attr.parse_args()?;
            rationale = Some(s.value());
        } else if is_sysml_attr(attr, "safety_goal") {
            let sg_kv: KvArgs = attr.parse_args()?;
            let asil_path = sg_kv.get_path("asil").ok_or_else(|| {
                syn::Error::new_spanned(
                    attr,
                    "expected `asil = AsilD` (or AsilC/AsilB/AsilA/Qm) in #[sysml::safety_goal(...)]",
                )
            })?;
            let safe_state = sg_kv.get_str("safe_state").ok_or_else(|| {
                syn::Error::new_spanned(
                    attr,
                    "expected `safe_state = \"...\"` in #[sysml::safety_goal(...)]",
                )
            })?;
            let ftti_ms: u32 = sg_kv
                .get_str("ftti_ms")
                .map(|s| s.parse::<u32>())
                .transpose()
                .map_err(|_| {
                    syn::Error::new_spanned(attr, "`ftti_ms` must be a non-negative integer in #[sysml::safety_goal(...)]")
                })?
                .unwrap_or(0);
            safety_goal_impl = quote! {
                impl ::sysml_dsl::core_contracts::SafetyGoal for #ident {
                    type Asil = #asil_path;
                    const SAFE_STATE: &'static str = #safe_state;
                    const FTTI_MS: u32 = #ftti_ms;
                }
            };
            safety_goal_meta_tok = quote! {
                ::std::option::Option::Some(::sysml_dsl::ir::SafetyGoalMeta {
                    asil: ::std::string::String::from(<#asil_path as ::sysml_dsl::core_contracts::AsilLevel>::LABEL),
                    safe_state: ::std::string::String::from(#safe_state),
                    ftti_ms: #ftti_ms,
                })
            };
        } else if is_sysml_attr(attr, "require_constraint") {
            let expr: Expr = attr.parse_args()?;
            if let Expr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) = expr
            {
                require_constraints.push(s.value());
            } else {
                let ctx = ExprContext::Requirement { fields: &field_map };
                let transpiled = transpile_expr(&expr, &ctx)?;
                check_requirement_dims(&expr, &unit_map)?;
                require_constraints.push(transpiled.sysml);
                require_eval_exprs.push(transpiled.rust);
            }
        } else if is_sysml_attr(attr, "assume_constraint") {
            let expr: Expr = attr.parse_args()?;
            if let Expr::Lit(ExprLit {
                lit: Lit::Str(s), ..
            }) = expr
            {
                assume_constraints.push(s.value());
            } else {
                let ctx = ExprContext::Requirement { fields: &field_map };
                let transpiled = transpile_expr(&expr, &ctx)?;
                check_requirement_dims(&expr, &unit_map)?;
                assume_constraints.push(transpiled.sysml);
            }
        } else if is_sysml_attr(attr, "subject") {
            let skv: KvArgs = attr.parse_args()?;
            let sname = skv.get_str("name").unwrap_or_else(|| "subject".to_string());
            if let Some(sp) = skv.get_path("ty") {
                let stype = path_to_sysml_string(&sp);
                subject_check = quote! { const _: fn() = || { ::sysml_dsl::core_contracts::assert_part_core::<#sp>(); }; };
                subject = Some((sname, stype));
            } else if let Some(st) = skv.get_str("ty") {
                subject = Some((sname, st));
            }
        }
    }

    let default_client = short_name.as_deref().unwrap_or(&sysml_name);
    let refinements = extract_refinements(&item.attrs, default_client)?;
    let spec_names: Vec<String> = specializes_paths.iter().map(path_to_sysml_string).collect();

    let mut attr_tokens = Vec::new();
    if let Fields::Named(fields) = &mut item.fields {
        for field in &mut fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            attr_tokens.push(build_attr_usage_tokens(&fname_rust, &field.ty, &field.attrs)?);
            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    let ref_tokens = refinements.iter().map(|(c, s)| {
        quote! {
            ::sysml_dsl::ir::RefinementDependency {
                client: ::std::string::String::from(#c),
                supplier: ::std::string::String::from(#s),
            }
        }
    });

    let spec_impls = specializes_paths.iter().map(|base| {
        quote! {
            impl ::sysml_dsl::core_contracts::Specializes<#base> for #ident {}
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_requirement_core::<#base>(); };
        }
    });

    let eval_impl = if !require_eval_exprs.is_empty() {
        quote! {
            impl ::sysml_dsl::core_contracts::EvaluableRequirement for #ident {
                fn evaluate(&self) -> bool {
                    true #(&& (#require_eval_exprs))*
                }
            }
        }
    } else {
        quote!()
    };

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let short_tok = quote_opt_str(&short_name);
    let doc_tok = quote_opt_str(&doc);
    let rat_tok = quote_opt_str(&rationale);
    let spec_tok = quote_vec_str(&spec_names);
    let req_c_tok = quote_vec_str(&require_constraints);
    let ass_c_tok = quote_vec_str(&assume_constraints);
    let subj_tok = match subject {
        Some((sn, st)) => quote! {
            ::std::option::Option::Some(::sysml_dsl::ir::SubjectDecl {
                name: ::std::string::String::from(#sn),
                type_name: ::std::string::String::from(#st),
            })
        },
        None => quote!(::std::option::Option::None),
    };

    Ok(quote! {
        #item

        #auto_default_impl

        impl ::sysml_dsl::core_contracts::SysmlRequirementCore for #ident {
            const REQ_ID: &'static str = #req_id_const;
            const SYSML_NAME: &'static str = #sysml_name;
        }

        #rcfg
        impl ::sysml_dsl::ir::SysmlRequirement for #ident {
            fn requirement_def() -> ::sysml_dsl::ir::RequirementDef {
                ::sysml_dsl::ir::RequirementDef {
                    short_name: #short_tok,
                    name: ::std::string::String::from(#sysml_name),
                    is_abstract: #is_abstract,
                    doc: #doc_tok,
                    rationale: #rat_tok,
                    safety_goal: #safety_goal_meta_tok,
                    specializes: #spec_tok,
                    subject: #subj_tok,
                    attributes: ::std::vec![#(#attr_tokens),*],
                    require_constraints: #req_c_tok,
                    assume_constraints: #ass_c_tok,
                    refinements: ::std::vec![#(#ref_tokens),*],
                }
            }
        }

        #safety_goal_impl
        #eval_impl
        #subject_check
        #(#spec_impls)*
    })
}

/// Expands `#[sysml::specification]` (a SysML v2 `requirement <name> { subject ...; requirement ...; satisfy ... by ...; }` usage).
pub fn expand_specification(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &item.attrs);
    let rcfg = reflect_cfg(is_no_std);
    let doc = extract_doc(&item.attrs);
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| lower_first(&item.ident.to_string()));

    let mut subject: Option<(String, String)> = None;
    let mut subject_check = quote!();
    let mut satisfies = Vec::new();
    let mut verifies = Vec::new();

    for attr in &item.attrs {
        if is_sysml_attr(attr, "subject") {
            let skv: KvArgs = attr.parse_args()?;
            let sname = skv.get_str("name").unwrap_or_else(|| "subject".to_string());
            if let Some(sp) = skv.get_path("ty") {
                let stype = path_to_sysml_string(&sp);
                subject_check = quote! { const _: fn() = || { ::sysml_dsl::core_contracts::assert_part_core::<#sp>(); }; };
                subject = Some((sname, stype));
            } else if let Some(st) = skv.get_str("ty") {
                subject = Some((sname, st));
            }
        } else if is_sysml_attr(attr, "satisfy") {
            let s: ReqLinkArgs = attr.parse_args()?;
            satisfies.push(s);
        } else if is_sysml_attr(attr, "verify") {
            let v: ReqLinkArgs = attr.parse_args()?;
            verifies.push(v);
        }
    }

    let mut nested_tokens = Vec::new();
    let mut field_checks = Vec::new();
    if let Fields::Named(fields) = &mut item.fields {
        for field in &mut fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            let fdoc = extract_doc(&field.attrs);
            let fdoc_tok = quote_opt_str(&fdoc);
            let mut fkv = KvArgs::default();
            for a in &field.attrs {
                if is_sysml_attr(a, "requirement_ref") {
                    if let Meta::List(_) = &a.meta {
                        fkv = a.parse_args()?;
                    }
                }
            }
            let fname_sysml = extract_sysml_name_override(&field.attrs)?
                .or_else(|| fkv.get_str("name"))
                .unwrap_or_else(|| snake_to_camel(&fname_rust));
            let (ty_name, _, _) = analyze_rust_type(&field.ty);
            let fty = &field.ty;
            field_checks.push(quote! { ::sysml_dsl::core_contracts::assert_requirement_core::<#fty>(); });
            nested_tokens.push(quote! {
                ::sysml_dsl::ir::RequirementUsage {
                    short_name: ::std::option::Option::None,
                    name: ::std::string::String::from(#fname_sysml),
                    type_name: ::std::option::Option::Some(::std::string::String::from(#ty_name)),
                    multiplicity: ::std::option::Option::None,
                    subsets: ::std::vec![],
                    doc: #fdoc_tok,
                    subject: ::std::option::Option::None,
                    nested_requirements: ::std::vec![],
                    satisfies: ::std::vec![],
                    verifies: ::std::vec![],
                }
            });
            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    let mut sat_tokens = Vec::new();
    let mut sat_checks = Vec::new();
    for s in &satisfies {
        let rpath = &s.req_path;
        let by_tok = quote_opt_str(&s.by);
        let asserted = s.asserted;
        let req_expr = if let Some(raw) = &s.raw_req_name {
            quote!(::std::string::String::from(#raw))
        } else {
            quote!(::std::string::String::from(<#rpath as ::sysml_dsl::core_contracts::SysmlRequirementCore>::SYSML_NAME))
        };
        let is_typed = s.raw_req_name.is_none();
        sat_tokens.push(quote! {
            ::sysml_dsl::ir::SatisfyStmt {
                asserted: #asserted,
                is_typed: #is_typed,
                requirement: #req_expr,
                by: #by_tok,
            }
        });
        let id_check = if let Some(exp_id) = &s.expected_id {
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
        sat_checks.push(quote! {
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_requirement_core::<#rpath>(); };
            #id_check
        });
    }

    let mut ver_tokens = Vec::new();
    let mut ver_checks = Vec::new();
    for v in &verifies {
        let rpath = &v.req_path;
        let by_tok = quote_opt_str(&v.by);
        ver_tokens.push(quote! {
            ::sysml_dsl::ir::VerifyStmt {
                requirement: ::std::string::String::from(<#rpath as ::sysml_dsl::core_contracts::SysmlRequirementCore>::SYSML_NAME),
                by: #by_tok,
            }
        });
        ver_checks.push(quote! {
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_requirement_core::<#rpath>(); };
        });
    }

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let ident = &item.ident;
    let doc_tok = quote_opt_str(&doc);
    let subj_tok = match subject {
        Some((sn, st)) => quote! {
            ::std::option::Option::Some(::sysml_dsl::ir::SubjectDecl {
                name: ::std::string::String::from(#sn),
                type_name: ::std::string::String::from(#st),
            })
        },
        None => quote!(::std::option::Option::None),
    };

    Ok(quote! {
        #item

        #rcfg
        impl #ident {
            pub fn requirement_usage() -> ::sysml_dsl::ir::RequirementUsage {
                ::sysml_dsl::ir::RequirementUsage {
                    short_name: ::std::option::Option::None,
                    name: ::std::string::String::from(#sysml_name),
                    type_name: ::std::option::Option::None,
                    multiplicity: ::std::option::Option::None,
                    subsets: ::std::vec![],
                    doc: #doc_tok,
                    subject: #subj_tok,
                    nested_requirements: ::std::vec![#(#nested_tokens),*],
                    satisfies: ::std::vec![#(#sat_tokens),*],
                    verifies: ::std::vec![#(#ver_tokens),*],
                }
            }
        }

        const _: fn() = || {
            #(#field_checks)*
        };
        #subject_check
        #(#sat_checks)*
        #(#ver_checks)*
    })
}

pub fn expand_verification(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &item.attrs);
    let rcfg = reflect_cfg(is_no_std);
    let doc = extract_doc(&item.attrs);
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| item.ident.to_string());

    let mut verifies = Vec::new();
    let mut subject: Option<(String, String)> = None;
    for attr in &item.attrs {
        if is_sysml_attr(attr, "verify") {
            let v: ReqLinkArgs = attr.parse_args()?;
            verifies.push(v);
        } else if is_sysml_attr(attr, "subject") {
            let skv: KvArgs = attr.parse_args()?;
            let sname = skv.get_str("name").unwrap_or_else(|| "subject".to_string());
            let stype = skv
                .get_path("ty")
                .map(|p| path_to_sysml_string(&p))
                .or_else(|| skv.get_str("ty"))
                .unwrap_or_default();
            subject = Some((sname, stype));
        }
    }

    let ident = &item.ident;
    let mut ver_tokens = Vec::new();
    let mut ver_impls = Vec::new();
    for v in &verifies {
        let rpath = &v.req_path;
        let by_tok = quote_opt_str(&v.by);
        ver_tokens.push(quote! {
            ::sysml_dsl::ir::VerifyStmt {
                requirement: ::std::string::String::from(<#rpath as ::sysml_dsl::core_contracts::SysmlRequirementCore>::SYSML_NAME),
                by: #by_tok,
            }
        });
        let id_check = if let Some(exp_id) = &v.expected_id {
            quote! {
                const _: () = {
                    if !::sysml_dsl::core_contracts::const_str_eq(<#rpath as ::sysml_dsl::core_contracts::SysmlRequirementCore>::REQ_ID, #exp_id) {
                        panic!("SysML requirement ID mismatch in #[sysml::verify]");
                    }
                };
            }
        } else {
            quote!()
        };
        ver_impls.push(quote! {
            impl ::sysml_dsl::core_contracts::Verifies<#rpath> for #ident {}
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_requirement_core::<#rpath>(); };
            #id_check
        });
    }

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let doc_tok = quote_opt_str(&doc);
    let subj_tok = match subject {
        Some((sn, st)) => quote! {
            ::std::option::Option::Some(::sysml_dsl::ir::SubjectDecl {
                name: ::std::string::String::from(#sn),
                type_name: ::std::string::String::from(#st),
            })
        },
        None => quote!(::std::option::Option::None),
    };

    Ok(quote! {
        #item

        #rcfg
        impl #ident {
            pub fn verification_def() -> ::sysml_dsl::ir::VerificationDef {
                ::sysml_dsl::ir::VerificationDef {
                    name: ::std::string::String::from(#sysml_name),
                    doc: #doc_tok,
                    subject: #subj_tok,
                    verifies: ::std::vec![#(#ver_tokens),*],
                }
            }
        }

        #(#ver_impls)*
    })
}

fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
    }
}
