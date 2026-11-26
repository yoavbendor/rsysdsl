//! Proc-macro expansions for `#[sysml::port]`, `#[sysml::interface]`,
//! `#[sysml::item]`, `#[sysml::enum_def]`, `#[sysml::connection]`, and
//! `#[sysml::attribute_def]`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemEnum, ItemStruct};

use crate::util::{
    analyze_rust_type, build_attr_usage_tokens, extract_doc, extract_sysml_name_override,
    is_sysml_attr, parse_multiplicity_str, path_to_sysml_string, quote_opt_mult, quote_opt_str,
    quote_vec_str, snake_to_camel, strip_sysml_attrs, KvArgs, PathList,
};

pub fn expand_item(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
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
            impl ::sysml_ir::Specializes<#base> for #ident {}
            const _: fn() = || { ::sysml_ir::assert_item::<#base>(); };
        }
    });

    Ok(quote! {
        #item

        impl ::sysml_ir::SysmlItem for #ident {
            const SYSML_NAME: &'static str = #sysml_name;

            fn item_def() -> ::sysml_ir::ItemDef {
                ::sysml_ir::ItemDef {
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

        impl ::sysml_ir::SysmlEnum for #ident {
            const SYSML_NAME: &'static str = #sysml_name;

            fn enum_def() -> ::sysml_ir::EnumDef {
                ::sysml_ir::EnumDef {
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
    let doc = extract_doc(&item.attrs);
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
                    quote!(::std::option::Option::Some(::sysml_ir::Direction::In))
                } else if is_out {
                    all_inout = false;
                    quote!(::std::option::Option::Some(::sysml_ir::Direction::Out))
                } else if is_inout {
                    quote!(::std::option::Option::Some(::sysml_ir::Direction::InOut))
                } else {
                    all_inout = false;
                    quote!(::std::option::Option::None)
                };
                let (ty_name, _, mult) = analyze_rust_type(&field.ty);
                let mult_tok = quote_opt_mult(&mult);
                let fty = &field.ty;
                item_type_checks.push(quote! { ::sysml_ir::assert_item::<#fty>(); });
                item_tokens.push(quote! {
                    ::sysml_ir::DirectedItem {
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
            impl ::sysml_ir::Specializes<#base> for #ident {}
            const _: fn() = || { ::sysml_ir::assert_port::<#base>(); };
        }
    });

    // Compile-time port polarity compatibility:
    // - Directional ports only connect to their `Conjugated<Self>` counterpart.
    // - Symmetric ports (all `inout` items or marked `symmetric`) also connect `Self` <-> `Self`.
    let symmetric_impls = if is_symmetric {
        quote! {
            impl ::sysml_ir::SymmetricPort for #ident {}
            impl ::sysml_ir::PortConnectable<#ident> for #ident {}
        }
    } else {
        quote!()
    };

    Ok(quote! {
        #item

        impl ::sysml_ir::SysmlPort for #ident {
            type Conjugate = ::sysml_ir::Conjugated<#ident>;
            const SYSML_NAME: &'static str = #sysml_name;

            fn port_def() -> ::sysml_ir::PortDef {
                ::sysml_ir::PortDef {
                    name: ::std::string::String::from(#sysml_name),
                    doc: #doc_tok,
                    specializes: #spec_tok,
                    is_symmetric: #is_symmetric,
                    items: ::std::vec![#(#item_tokens),*],
                    attributes: ::std::vec![#(#attr_tokens),*],
                }
            }
        }

        impl ::sysml_ir::PortConnectable<::sysml_ir::Conjugated<#ident>> for #ident {}
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
    let doc = extract_doc(&item.attrs);
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| item.ident.to_string());

    let mut end_tokens = Vec::new();
    let mut end_types = Vec::new();

    if let Fields::Named(fields) = &mut item.fields {
        for field in &mut fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            let fname_sysml = extract_sysml_name_override(&field.attrs)?
                .unwrap_or_else(|| snake_to_camel(&fname_rust));
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
                ::sysml_ir::InterfaceEnd {
                    name: ::std::string::String::from(#fname_sysml),
                    type_name: ::std::string::String::from(#ty_name),
                    conjugated: #conjugated,
                    multiplicity: #mult_tok,
                }
            });
            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    if end_types.len() != 2 {
        return Err(syn::Error::new_spanned(
            &item.ident,
            "#[sysml::interface] struct must declare exactly two `end` port fields",
        ));
    }

    let end_a = &end_types[0];
    let end_b = &end_types[1];

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let ident = &item.ident;
    let doc_tok = quote_opt_str(&doc);

    Ok(quote! {
        #item

        impl ::sysml_ir::SysmlInterface for #ident {
            type EndA = #end_a;
            type EndB = #end_b;
            const SYSML_NAME: &'static str = #sysml_name;

            fn interface_def() -> ::sysml_ir::InterfaceDef {
                ::sysml_ir::InterfaceDef {
                    name: ::std::string::String::from(#sysml_name),
                    doc: #doc_tok,
                    specializes: ::std::vec![],
                    ends: ::std::vec![#(#end_tokens),*],
                }
            }
        }

        // Compile-time check: interface ends must be compatible SysML ports!
        const _: fn() = || {
            ::sysml_ir::assert_interface_ends::<#end_a, #end_b>();
        };
    })
}

pub fn expand_connection(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
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
                ::sysml_ir::ConnectionEndDecl {
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

        impl #ident {
            pub fn connection_def() -> ::sysml_ir::ConnectionDef {
                ::sysml_ir::ConnectionDef {
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
