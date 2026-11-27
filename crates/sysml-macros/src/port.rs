//! Proc-macro expansions for `#[sysml::port]`, `#[sysml::interface]`,
//! `#[sysml::item]`, `#[sysml::enum_def]`, and `#[sysml::connection]`.
//!
//! All compile-time contracts (`SysmlItemCore`, `SysmlPortCore`, `PortConnectable`,
//! `SymmetricPort`, `SysmlInterfaceCore`, `Specializes`) are `#![no_std]` zero-alloc;
//! IR reflection impls are gated under `#[cfg(feature = "reflect")]` when `no_std` is set.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemEnum, ItemStruct};

use crate::util::{
    analyze_rust_type, build_attr_usage_tokens, extract_doc, extract_sysml_name_override,
    has_no_std_mode, is_sysml_attr, parse_multiplicity_str, path_to_sysml_string, quote_opt_mult,
    quote_opt_str, quote_vec_str, reflect_cfg, snake_to_camel, strip_sysml_attrs, KvArgs, PathList,
};

pub fn expand_item(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &item.attrs);
    let rcfg = reflect_cfg(is_no_std);
    let doc = extract_doc(&item.attrs);
    let is_abstract = kv.has_flag("abstract");
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| item.ident.to_string());

    let mut specializes_paths = Vec::new();
    for attr in &item.attrs {
        if is_sysml_attr(attr, "specializes") {
            let list: PathList = attr.parse_args()?;
            specializes_paths.extend(list.paths.into_iter());
        }
    }
    let spec_names: Vec<String> = specializes_paths.iter().map(path_to_sysml_string).collect();

    let field_unit_fn = crate::units_check::field_unit_fn(&item.ident, &item.fields)?;
    let mut attr_tokens = Vec::new();
    if let Fields::Named(fields) = &mut item.fields {
        for field in &mut fields.named {
            let fname = field.ident.as_ref().unwrap().to_string();
            attr_tokens.push(build_attr_usage_tokens(&fname, &field.ty, &field.attrs)?);
            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let ident = &item.ident;
    let doc_tok = quote_opt_str(&doc);
    let spec_tok = quote_vec_str(&spec_names);
    let spec_impls = specializes_paths.iter().map(|base| {
        quote! {
            impl ::sysml_dsl::core_contracts::Specializes<#base> for #ident {}
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_item_core::<#base>(); };
        }
    });

    Ok(quote! {
        #item

        impl ::sysml_dsl::core_contracts::SysmlItemCore for #ident {
            const SYSML_NAME: &'static str = #sysml_name;
        }

        #field_unit_fn

        #rcfg
        impl ::sysml_dsl::ir::SysmlItem for #ident {
            fn item_def() -> ::sysml_dsl::ir::ItemDef {
                ::sysml_dsl::ir::ItemDef {
                    name: ::std::string::String::from(#sysml_name),
                    is_abstract: #is_abstract,
                    doc: #doc_tok,
                    specializes: #spec_tok,
                    attributes: ::std::vec![#(#attr_tokens),*],
                }
            }
        }

        #(#spec_impls)*
    })
}

pub fn expand_enum_def(args: TokenStream, mut item: ItemEnum) -> syn::Result<TokenStream> {
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

    let mut variant_names = Vec::new();
    for v in &mut item.variants {
        let vname = extract_sysml_name_override(&v.attrs)?
            .unwrap_or_else(|| snake_to_camel(&v.ident.to_string().to_lowercase()));
        variant_names.push(vname);
        v.attrs = strip_sysml_attrs(std::mem::take(&mut v.attrs));
    }

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let ident = &item.ident;
    let doc_tok = quote_opt_str(&doc);
    let variants_tok = quote_vec_str(&variant_names);

    Ok(quote! {
        #item

        impl ::sysml_dsl::core_contracts::SysmlEnumCore for #ident {
            const SYSML_NAME: &'static str = #sysml_name;
        }

        #rcfg
        impl ::sysml_dsl::ir::SysmlEnum for #ident {
            fn enum_def() -> ::sysml_dsl::ir::EnumDef {
                ::sysml_dsl::ir::EnumDef {
                    name: ::std::string::String::from(#sysml_name),
                    doc: #doc_tok,
                    variants: #variants_tok,
                }
            }
        }
    })
}

pub fn expand_port(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &item.attrs);
    let rcfg = reflect_cfg(is_no_std);
    let doc = extract_doc(&item.attrs);
    let is_abstract = kv.has_flag("abstract");
    let explicit_symmetric = kv.has_flag("symmetric");
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| item.ident.to_string());

    let mut specializes_paths = Vec::new();
    for attr in &item.attrs {
        if is_sysml_attr(attr, "specializes") {
            let list: PathList = attr.parse_args()?;
            specializes_paths.extend(list.paths.into_iter());
        }
    }
    let spec_names: Vec<String> = specializes_paths.iter().map(path_to_sysml_string).collect();

    let mut item_tokens = Vec::new();
    let mut attr_tokens = Vec::new();
    let mut item_type_checks = Vec::new();
    let mut all_inout = true;
    let mut has_any_item = false;

    if let Fields::Named(fields) = &mut item.fields {
        for field in &mut fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            let fname_sysml = extract_sysml_name_override(&field.attrs)?
                .unwrap_or_else(|| snake_to_camel(&fname_rust));
            let is_in = field
                .attrs
                .iter()
                .any(|a| is_sysml_attr(a, "in_item") || is_sysml_attr(a, "in_param"));
            let is_out = field
                .attrs
                .iter()
                .any(|a| is_sysml_attr(a, "out") || is_sysml_attr(a, "out_item") || is_sysml_attr(a, "out_param"));
            let is_inout = field
                .attrs
                .iter()
                .any(|a| is_sysml_attr(a, "inout") || is_sysml_attr(a, "inout_item") || is_sysml_attr(a, "inout_param"));
            let is_attr = field.attrs.iter().any(|a| is_sysml_attr(a, "attr"));

            if is_attr {
                attr_tokens.push(build_attr_usage_tokens(&fname_rust, &field.ty, &field.attrs)?);
            } else {
                has_any_item = true;
                let dir_tok = if is_in {
                    all_inout = false;
                    quote!(::std::option::Option::Some(::sysml_dsl::ir::Direction::In))
                } else if is_out {
                    all_inout = false;
                    quote!(::std::option::Option::Some(::sysml_dsl::ir::Direction::Out))
                } else if is_inout {
                    quote!(::std::option::Option::Some(::sysml_dsl::ir::Direction::InOut))
                } else {
                    all_inout = false;
                    quote!(::std::option::Option::None)
                };
                let (ty_name, _, mult) = analyze_rust_type(&field.ty);
                let mult_tok = quote_opt_mult(&mult);
                let fty = &field.ty;
                item_type_checks.push(quote! { ::sysml_dsl::core_contracts::assert_item_core::<#fty>(); });
                item_tokens.push(quote! {
                    ::sysml_dsl::ir::DirectedItem {
                        direction: #dir_tok,
                        name: ::std::string::String::from(#fname_sysml),
                        type_name: ::std::string::String::from(#ty_name),
                        multiplicity: #mult_tok,
                        subsets: ::std::vec![],
                        doc: ::std::option::Option::None,
                    }
                });
            }
            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    let is_symmetric = explicit_symmetric || (has_any_item && all_inout);

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let ident = &item.ident;
    let doc_tok = quote_opt_str(&doc);
    let spec_tok = quote_vec_str(&spec_names);

    let spec_impls = specializes_paths.iter().map(|base| {
        quote! {
            impl ::sysml_dsl::core_contracts::Specializes<#base> for #ident {}
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_port_core::<#base>(); };
        }
    });

    let symmetric_impls = if is_symmetric {
        quote! {
            impl ::sysml_dsl::core_contracts::SymmetricPort for #ident {}
            impl ::sysml_dsl::core_contracts::PortConnectable<#ident> for #ident {}
        }
    } else {
        quote!()
    };

    Ok(quote! {
        #item

        impl ::sysml_dsl::core_contracts::SysmlPortCore for #ident {
            type Conjugate = ::sysml_dsl::core_contracts::Conjugated<#ident>;
            const SYSML_NAME: &'static str = #sysml_name;
        }

        #rcfg
        impl ::sysml_dsl::ir::SysmlPort for #ident {
            fn port_def() -> ::sysml_dsl::ir::PortDef {
                ::sysml_dsl::ir::PortDef {
                    name: ::std::string::String::from(#sysml_name),
                    is_abstract: #is_abstract,
                    doc: #doc_tok,
                    specializes: #spec_tok,
                    is_symmetric: #is_symmetric,
                    items: ::std::vec![#(#item_tokens),*],
                    attributes: ::std::vec![#(#attr_tokens),*],
                }
            }
        }

        impl ::sysml_dsl::core_contracts::PortConnectable<::sysml_dsl::core_contracts::Conjugated<#ident>> for #ident {}
        #symmetric_impls

        const _: fn() = || {
            #(#item_type_checks)*
        };

        #(#spec_impls)*
    })
}

pub fn expand_interface(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &item.attrs);
    let rcfg = reflect_cfg(is_no_std);
    let doc = extract_doc(&item.attrs);
    let is_abstract = kv.has_flag("abstract");
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| item.ident.to_string());

    let mut specializes_paths = Vec::new();
    for attr in &item.attrs {
        if is_sysml_attr(attr, "specializes") {
            let list: PathList = attr.parse_args()?;
            specializes_paths.extend(list.paths.into_iter());
        }
    }
    let spec_names: Vec<String> = specializes_paths.iter().map(path_to_sysml_string).collect();

    let auto_default_impl = crate::util::build_auto_default_impl(&item)?;

    let mut end_tokens = Vec::new();
    let mut end_types = Vec::new();
    let mut attr_tokens = Vec::new();
    let mut redef_checks = Vec::new();
    let mut declared_field_names: Vec<String> = Vec::new();

    if let Fields::Named(fields) = &mut item.fields {
        for field in &mut fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            let fname_sysml_default = extract_sysml_name_override(&field.attrs)?
                .unwrap_or_else(|| snake_to_camel(&fname_rust));
            declared_field_names.push(fname_rust.clone());
            declared_field_names.push(fname_sysml_default.clone());
            let is_attr = field.attrs.iter().any(|a| is_sysml_attr(a, "attr"));
            if is_attr {
                if !specializes_paths.is_empty() {
                    let mut akv = KvArgs::default();
                    for a in &field.attrs {
                        if is_sysml_attr(a, "attr") {
                            if let syn::Meta::List(_) = &a.meta {
                                akv = a.parse_args()?;
                            }
                        }
                    }
                    let mut redefs = akv.get_all_str("redefines");
                    if redefs.is_empty() && akv.has_flag("redefines") {
                        redefs.push(fname_sysml_default.clone());
                    }
                    for r in &redefs {
                        if !r.contains("::") && !r.contains('.') {
                            let r_str = r.as_str();
                            let ancestor_checks = specializes_paths.iter().map(|base_path| {
                                quote!(#base_path::__sysml_has_field(#r_str))
                            });
                            redef_checks.push(quote! {
                                const _: () = assert!(
                                    #(#ancestor_checks)||*,
                                    concat!("`redefines` target field `", #r_str, "` does not exist on any specialized ancestor interface")
                                );
                            });
                        }
                    }
                }
                attr_tokens.push(build_attr_usage_tokens(&fname_rust, &field.ty, &field.attrs)?);
                field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
                continue;
            }

            let fname_sysml = fname_sysml_default;
            let mut fkv = KvArgs::default();
            for a in &field.attrs {
                if is_sysml_attr(a, "end") {
                    if let syn::Meta::List(_) = &a.meta {
                        fkv = a.parse_args()?;
                    }
                }
            }
            let (ty_name, conjugated, rust_mult) = analyze_rust_type(&field.ty);
            let mult = fkv
                .get_str("mult")
                .map(|m| parse_multiplicity_str(&m))
                .or(rust_mult);
            let mult_tok = quote_opt_mult(&mult);
            end_types.push(field.ty.clone());
            end_tokens.push(quote! {
                ::sysml_dsl::ir::InterfaceEnd {
                    name: ::std::string::String::from(#fname_sysml),
                    type_name: ::std::string::String::from(#ty_name),
                    conjugated: #conjugated,
                    multiplicity: #mult_tok,
                }
            });
            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    let (end_a_tok, end_b_tok, ends_check_tok) = if end_types.len() == 2 {
        let end_a = &end_types[0];
        let end_b = &end_types[1];
        (
            quote!(#end_a),
            quote!(#end_b),
            quote! {
                const _: fn() = || {
                    ::sysml_dsl::core_contracts::assert_interface_ends_core::<#end_a, #end_b>();
                };
            },
        )
    } else if end_types.is_empty() && !specializes_paths.is_empty() {
        let base = &specializes_paths[0];
        (
            quote!(<#base as ::sysml_dsl::core_contracts::SysmlInterfaceCore>::EndA),
            quote!(<#base as ::sysml_dsl::core_contracts::SysmlInterfaceCore>::EndB),
            quote!(),
        )
    } else {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "#[sysml::interface] struct must declare two `end` port fields (or inherit them via #[sysml::specializes(BaseInterface)])",
        ));
    };

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let ident = &item.ident;
    let doc_tok = quote_opt_str(&doc);
    let spec_tok = quote_vec_str(&spec_names);

    let spec_impls = specializes_paths.iter().map(|base| {
        quote! {
            impl ::sysml_dsl::core_contracts::Specializes<#base> for #ident {}
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_interface_core::<#base>(); };
        }
    });

    let own_field_checks = declared_field_names.iter().map(|n| {
        quote!(::sysml_dsl::core_contracts::const_str_eq(name, #n))
    });
    let parent_field_checks = specializes_paths.iter().map(|base| {
        quote!(#base::__sysml_has_field(name))
    });
    let has_field_expr = quote!(false #(|| #own_field_checks)* #(|| #parent_field_checks)*);

    Ok(quote! {
        #item

        #auto_default_impl

        impl #ident {
            #[doc(hidden)]
            #[allow(dead_code)]
            pub const fn __sysml_has_field(name: &str) -> bool {
                #has_field_expr
            }
        }

        impl ::sysml_dsl::core_contracts::SysmlInterfaceCore for #ident {
            type EndA = #end_a_tok;
            type EndB = #end_b_tok;
            const SYSML_NAME: &'static str = #sysml_name;
        }

        #rcfg
        impl ::sysml_dsl::ir::SysmlInterface for #ident {
            fn interface_def() -> ::sysml_dsl::ir::InterfaceDef {
                ::sysml_dsl::ir::InterfaceDef {
                    name: ::std::string::String::from(#sysml_name),
                    is_abstract: #is_abstract,
                    doc: #doc_tok,
                    specializes: #spec_tok,
                    attributes: ::std::vec![#(#attr_tokens),*],
                    ends: ::std::vec![#(#end_tokens),*],
                }
            }
        }

        #ends_check_tok
        #(#spec_impls)*
        #(#redef_checks)*
    })
}

pub fn expand_connection(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &item.attrs);
    let rcfg = reflect_cfg(is_no_std);
    let doc = extract_doc(&item.attrs);
    let is_abstract = kv.has_flag("abstract");
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| item.ident.to_string());

    let mut end_tokens = Vec::new();
    if let Fields::Named(fields) = &mut item.fields {
        for field in &mut fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            let fname_sysml = extract_sysml_name_override(&field.attrs)?
                .unwrap_or_else(|| snake_to_camel(&fname_rust));
            let mut fkv = KvArgs::default();
            for a in &field.attrs {
                if is_sysml_attr(a, "end") || is_sysml_attr(a, "part") || is_sysml_attr(a, "requirement_ref") {
                    if let syn::Meta::List(_) = &a.meta {
                        fkv = a.parse_args()?;
                    }
                }
            }
            let kind_kw = fkv.get_str("kind").unwrap_or_else(|| "part".to_string());
            let is_ref = fkv.has_flag("ref") || true;
            let subsets = fkv.get_all_str("subsets");
            let subsets_tok = quote_vec_str(&subsets);
            let (ty_name, _, rust_mult) = analyze_rust_type(&field.ty);
            let mult = fkv
                .get_str("mult")
                .map(|m| parse_multiplicity_str(&m))
                .or(rust_mult);
            let mult_tok = quote_opt_mult(&mult);
            end_tokens.push(quote! {
                ::sysml_dsl::ir::ConnectionEndDecl {
                    is_ref: #is_ref,
                    kind_keyword: ::std::string::String::from(#kind_kw),
                    name: ::std::string::String::from(#fname_sysml),
                    type_name: ::std::string::String::from(#ty_name),
                    multiplicity: #mult_tok,
                    subsets: #subsets_tok,
                }
            });
            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let ident = &item.ident;
    let doc_tok = quote_opt_str(&doc);

    Ok(quote! {
        #item

        #rcfg
        impl #ident {
            pub fn connection_def() -> ::sysml_dsl::ir::ConnectionDef {
                ::sysml_dsl::ir::ConnectionDef {
                    name: ::std::string::String::from(#sysml_name),
                    is_abstract: #is_abstract,
                    doc: #doc_tok,
                    specializes: ::std::vec![],
                    ends: ::std::vec![#(#end_tokens),*],
                }
            }
        }
    })
}
