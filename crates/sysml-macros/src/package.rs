//! Proc-macro expansion for `#[sysml::package]` on Rust modules.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Item, ItemMod, LitStr};

use crate::util::{
    extract_doc, extract_sysml_name_override, is_sysml_attr, quote_opt_str, snake_to_pascal,
    strip_sysml_attrs, KvArgs,
};

fn parse_import_spec(spec: &str) -> (String, TokenStream) {
    if let Some(base) = spec.strip_suffix("::**") {
        (
            base.to_string(),
            quote!(::sysml_ir::ImportWildcard::RecursiveStar),
        )
    } else if let Some(base) = spec.strip_suffix("::*") {
        (base.to_string(), quote!(::sysml_ir::ImportWildcard::Star))
    } else {
        (spec.to_string(), quote!(::sysml_ir::ImportWildcard::None))
    }
}

pub fn expand_package(args: TokenStream, mut item: ItemMod) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let doc = extract_doc(&item.attrs);
    let is_library = kv.has_flag("library");
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| snake_to_pascal(&item.ident.to_string()));

    let mut import_tokens = Vec::new();
    for attr in &item.attrs {
        if is_sysml_attr(attr, "import") {
            let s: LitStr = attr.parse_args()?;
            let (target, wildcard) = parse_import_spec(&s.value());
            import_tokens.push(quote! {
                ::sysml_ir::Import {
                    visibility: ::sysml_ir::Visibility::Private,
                    target: ::std::string::String::from(#target),
                    wildcard: #wildcard,
                }
            });
        }
    }

    let mut member_tokens = Vec::new();
    if let Some((_, ref mut items)) = item.content {
        for sub in items.iter_mut() {
            match sub {
                Item::Const(c) => {
                    let mut had_import = false;
                    for attr in &c.attrs {
                        if is_sysml_attr(attr, "import") {
                            let s: LitStr = attr.parse_args()?;
                            let (target, wildcard) = parse_import_spec(&s.value());
                            import_tokens.push(quote! {
                                ::sysml_ir::Import {
                                    visibility: ::sysml_ir::Visibility::Private,
                                    target: ::std::string::String::from(#target),
                                    wildcard: #wildcard,
                                }
                            });
                            had_import = true;
                        }
                    }
                    if had_import {
                        c.attrs = strip_sysml_attrs(std::mem::take(&mut c.attrs));
                    }
                }
                Item::Struct(s) => {
                    let id = &s.ident;
                    if s.attrs.iter().any(|a| is_sysml_attr(a, "part")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::PartDef(<#id as ::sysml_ir::SysmlPart>::part_def())
                        });
                    } else if s.attrs.iter().any(|a| is_sysml_attr(a, "port")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::PortDef(<#id as ::sysml_ir::SysmlPort>::port_def())
                        });
                    } else if s.attrs.iter().any(|a| is_sysml_attr(a, "interface")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::InterfaceDef(<#id as ::sysml_ir::SysmlInterface>::interface_def())
                        });
                    } else if s.attrs.iter().any(|a| is_sysml_attr(a, "connection")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::ConnectionDef(#id::connection_def())
                        });
                    } else if s.attrs.iter().any(|a| is_sysml_attr(a, "item")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::ItemDef(<#id as ::sysml_ir::SysmlItem>::item_def())
                        });
                    } else if s.attrs.iter().any(|a| is_sysml_attr(a, "action")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::ActionDef(<#id as ::sysml_ir::SysmlAction>::action_def())
                        });
                    } else if s.attrs.iter().any(|a| is_sysml_attr(a, "state")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::StateDef(<#id as ::sysml_ir::SysmlState>::state_def())
                        });
                    } else if s.attrs.iter().any(|a| is_sysml_attr(a, "requirement")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::RequirementDef(<#id as ::sysml_ir::SysmlRequirement>::requirement_def())
                        });
                    } else if s.attrs.iter().any(|a| is_sysml_attr(a, "specification")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::RequirementUsage(#id::requirement_usage())
                        });
                    } else if s.attrs.iter().any(|a| is_sysml_attr(a, "verification")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::VerificationDef(#id::verification_def())
                        });
                    }
                }
                Item::Enum(e) => {
                    let id = &e.ident;
                    if e.attrs.iter().any(|a| is_sysml_attr(a, "state")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::StateDef(<#id as ::sysml_ir::SysmlState>::state_def())
                        });
                    } else if e.attrs.iter().any(|a| is_sysml_attr(a, "enum_def")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::EnumDef(<#id as ::sysml_ir::SysmlEnum>::enum_def())
                        });
                    }
                }
                Item::Mod(m) => {
                    let id = &m.ident;
                    if m.attrs.iter().any(|a| is_sysml_attr(a, "package")) {
                        member_tokens.push(quote! {
                            ::sysml_ir::PackageMember::Package(#id::sysml_package())
                        });
                    }
                }
                _ => {}
            }
        }

        let doc_tok = quote_opt_str(&doc);
        let builder_fn: Item = syn::parse2(quote! {
            /// Deterministically builds the `sysml_ir::Package` IR for this module.
            pub fn sysml_package() -> ::sysml_ir::Package {
                ::sysml_ir::Package {
                    name: ::std::string::String::from(#sysml_name),
                    is_library: #is_library,
                    doc: #doc_tok,
                    imports: ::std::vec![#(#import_tokens),*],
                    members: ::std::vec![#(#member_tokens),*],
                }
            }
        })?;
        items.push(builder_fn);
    }

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));

    Ok(quote!(#item))
}
