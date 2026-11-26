//! Proc-macro expansion for `#[sysml::analysis]` (`analysis def` and `analysis` usages).

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemStruct, LitStr, Meta};

use crate::util::{
    analyze_rust_type, extract_doc, extract_sysml_name_override, has_no_std_mode, is_sysml_attr,
    parse_multiplicity_str, path_to_sysml_string, quote_opt_mult, quote_opt_str, quote_vec_str,
    reflect_cfg, snake_to_camel, strip_sysml_attrs, KvArgs, PathList,
};

pub fn expand_analysis(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &item.attrs);
    let rcfg = reflect_cfg(is_no_std);
    let doc = extract_doc(&item.attrs);
    let is_usage = kv.has_flag("usage");
    let usage_ty = kv.get_str("ty").or_else(|| kv.get_str("type"));
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| item.ident.to_string());

    let mut specializes = Vec::new();
    let mut subject_tok = quote!(::std::option::Option::None);
    let mut assert_constraints = Vec::new();

    for attr in &item.attrs {
        if is_sysml_attr(attr, "specializes") {
            let list: PathList = attr.parse_args()?;
            for p in list.paths {
                specializes.push(path_to_sysml_string(&p));
            }
        } else if is_sysml_attr(attr, "subject") {
            let skv: KvArgs = attr.parse_args()?;
            let sname = skv.get_str("name").unwrap_or_else(|| "subject".to_string());
            let sty = skv
                .get_str("ty")
                .or_else(|| skv.get_path("ty").map(|p| path_to_sysml_string(&p)))
                .unwrap_or_default();
            subject_tok = quote!(::std::option::Option::Some(::sysml_dsl::ir::SubjectDecl {
                name: ::std::string::String::from(#sname),
                type_name: ::std::string::String::from(#sty),
            }));
        } else if is_sysml_attr(attr, "assert_constraint") {
            let s: LitStr = attr.parse_args()?;
            assert_constraints.push(s.value());
        }
    }

    let mut param_tokens = Vec::new();
    if let Fields::Named(fields) = &mut item.fields {
        for field in &mut fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            let mut dir_tok = quote!(::std::option::Option::None);
            let mut fkv = KvArgs::default();
            for attr in &field.attrs {
                if is_sysml_attr(attr, "in_param") {
                    dir_tok = quote!(::std::option::Option::Some(::sysml_dsl::ir::Direction::In));
                    if let Meta::List(_) = &attr.meta {
                        fkv = attr.parse_args()?;
                    }
                } else if is_sysml_attr(attr, "out_param") {
                    dir_tok = quote!(::std::option::Option::Some(::sysml_dsl::ir::Direction::Out));
                    if let Meta::List(_) = &attr.meta {
                        fkv = attr.parse_args()?;
                    }
                } else if is_sysml_attr(attr, "inout_param") {
                    dir_tok = quote!(::std::option::Option::Some(::sysml_dsl::ir::Direction::InOut));
                    if let Meta::List(_) = &attr.meta {
                        fkv = attr.parse_args()?;
                    }
                } else if is_sysml_attr(attr, "attr") {
                    if let Meta::List(_) = &attr.meta {
                        fkv = attr.parse_args()?;
                    }
                }
            }

            let pname = extract_sysml_name_override(&field.attrs)?
                .or_else(|| fkv.get_str("name"))
                .unwrap_or_else(|| snake_to_camel(&fname_rust));
            let (_, _, rust_mult) = analyze_rust_type(&field.ty);
            let mult = fkv
                .get_str("mult")
                .map(|m| parse_multiplicity_str(&m))
                .or(rust_mult);
            let subsets = fkv.get_all_str("subsets");
            let explicit_ty = fkv.get_str("ty").or_else(|| fkv.get_str("type"));
            let type_name = explicit_ty.filter(|t| !t.is_empty());
            let value = fkv.get_str("value").or_else(|| fkv.get_str("expr"));
            let unit = fkv.get_str("unit");

            let ty_tok = quote_opt_str(&type_name);
            let subsets_tok = quote_vec_str(&subsets);
            let mult_tok = quote_opt_mult(&mult);
            let val_tok = quote_opt_str(&value);
            let unit_tok = quote_opt_str(&unit);

            param_tokens.push(quote! {
                ::sysml_dsl::ir::AnalysisParam {
                    direction: #dir_tok,
                    name: ::std::string::String::from(#pname),
                    type_name: #ty_tok,
                    subsets: #subsets_tok,
                    multiplicity: #mult_tok,
                    value: #val_tok,
                    unit: #unit_tok,
                }
            });
            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let ident = &item.ident;
    let doc_tok = quote_opt_str(&doc);
    let spec_tok = quote_vec_str(&specializes);
    let cons_tok = quote_vec_str(&assert_constraints);

    let member_body = if is_usage {
        let usage_ty_tok = quote_opt_str(&usage_ty);
        quote! {
            ::sysml_dsl::ir::PackageMember::AnalysisUsage(::sysml_dsl::ir::AnalysisUsage {
                name: ::std::string::String::from(#sysml_name),
                type_name: #usage_ty_tok,
                doc: #doc_tok,
                subject: #subject_tok,
                params: ::std::vec![#(#param_tokens),*],
                assert_constraints: #cons_tok,
            })
        }
    } else {
        quote! {
            ::sysml_dsl::ir::PackageMember::AnalysisDef(::sysml_dsl::ir::AnalysisDef {
                name: ::std::string::String::from(#sysml_name),
                doc: #doc_tok,
                specializes: #spec_tok,
                subject: #subject_tok,
                params: ::std::vec![#(#param_tokens),*],
                assert_constraints: #cons_tok,
            })
        }
    };

    Ok(quote! {
        #item

        #rcfg
        impl #ident {
            pub fn analysis_member() -> ::sysml_dsl::ir::PackageMember {
                #member_body
            }
        }
    })
}
