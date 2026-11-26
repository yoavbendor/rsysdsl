//! Proc-macro expansion for `#[sysml::part]`.
//!
//! Enforces at `rustc` compile time:
//! - Every `#[sysml::specializes(Base1, Base2)]` target implements `SysmlPart`.
//! - Every `#[sysml::perform(name: ActionTy)]` target implements `SysmlAction`.
//! - Every `#[sysml::satisfy(req = ReqTy, ...)]` target implements `SysmlRequirement`,
//!   and any explicit `id = "..."` matches `ReqTy::REQ_ID` at compile time.
//! - Every `#[sysml::verify(req = ReqTy, ...)]` target implements `SysmlRequirement`.
//! - Every `#[sysml::connect(from = a.p1, to = b.p2)]` (with optional `interface = Iface`)
//!   references real Rust fields on `Self` whose port types satisfy `PortConnectable`
//!   (and match `Iface::EndA` / `Iface::EndB` when an interface is given).

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Expr, Fields, Ident, ItemStruct, LitStr, Meta, Path, Token};

use crate::util::{
    analyze_rust_type, build_attr_usage_tokens, build_auto_default_impl, extract_comments,
    extract_doc, extract_refinements, extract_sysml_name_override, has_no_std_mode, is_sysml_attr,
    parse_multiplicity_str, path_to_sysml_string, quote_opt_mult, quote_opt_str, quote_vec_str,
    reflect_cfg, snake_to_camel, strip_sysml_attrs, KvArgs, PathList,
};

/// Parses `#[sysml::perform(execute_descent_burn: ExecuteDescentBurn)]`
/// or `#[sysml::perform(execute_descent_burn: ExecuteDescentBurn, name = "executeDescentBurn")]`.
pub struct PerformArgs {
    pub local_name: Ident,
    pub action_path: Path,
    pub sysml_name: Option<String>,
}

impl Parse for PerformArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let local_name: Ident = input.parse()?;
        let _colon: Token![:] = input.parse()?;
        let action_path: Path = input.parse()?;
        let mut sysml_name = None;
        if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            let kv: KvArgs = input.parse()?;
            sysml_name = kv.get_str("name");
        }
        Ok(Self {
            local_name,
            action_path,
            sysml_name,
        })
    }
}

/// Parses `#[sysml::satisfy(req = ReqPath, id = "HLR-R002", by = "path")]`
/// or shorthand `#[sysml::satisfy(ReqPath)]`.
pub struct ReqLinkArgs {
    pub req_path: Path,
    pub expected_id: Option<String>,
    pub by: Option<String>,
    pub asserted: bool,
    pub raw_req_name: Option<String>,
}

impl Parse for ReqLinkArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Support shorthand `#[sysml::satisfy(MyReq)]` or key-value `#[sysml::satisfy(req = MyReq, ...)]`
        let lookahead = input.fork();
        let is_kv = lookahead.parse::<Ident>().is_ok() && lookahead.peek(Token![=]);
        if !is_kv {
            let req_path: Path = input.parse()?;
            let mut expected_id = None;
            let mut by = None;
            let mut asserted = false;
            let mut raw_req_name = None;
            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
                let kv: KvArgs = input.parse()?;
                expected_id = kv.get_str("id");
                by = kv.get_str("by");
                asserted = kv.has_flag("assert");
                raw_req_name = kv.get_str("requirement");
            }
            return Ok(Self {
                req_path,
                expected_id,
                by,
                asserted,
                raw_req_name,
            });
        }
        let kv: KvArgs = input.parse()?;
        let req_path = kv.get_path("req").ok_or_else(|| {
            syn::Error::new(
                input.span(),
                "expected `req = RequirementType` in #[sysml::satisfy(...)] / #[sysml::verify(...)]",
            )
        })?;
        Ok(Self {
            req_path,
            expected_id: kv.get_str("id"),
            by: kv.get_str("by"),
            asserted: kv.has_flag("assert"),
            raw_req_name: kv.get_str("requirement"),
        })
    }
}

/// Parses `#[sysml::connect(from = a.p1, to = b.p2)]` or
/// `#[sysml::connect(interface = IfaceTy, name = "lmInterstage", from = a.p1, to = b.p2)]`.
pub struct ConnectArgs {
    pub interface_path: Option<Path>,
    pub connection_type: Option<String>,
    pub name: Option<String>,
    pub from_expr: Expr,
    pub to_expr: Expr,
    pub from_sysml: Option<String>,
    pub to_sysml: Option<String>,
}

impl Parse for ConnectArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut interface_path = None;
        let mut connection_type = None;
        let mut name = None;
        let mut from_expr = None;
        let mut to_expr = None;
        let mut from_sysml = None;
        let mut to_sysml = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;
            match key.to_string().as_str() {
                "interface" => {
                    interface_path = Some(input.parse::<Path>()?);
                }
                "connection" => {
                    let s: LitStr = input.parse()?;
                    connection_type = Some(s.value());
                }
                "name" => {
                    let s: LitStr = input.parse()?;
                    name = Some(s.value());
                }
                "from" => {
                    from_expr = Some(input.parse::<Expr>()?);
                }
                "to" => {
                    to_expr = Some(input.parse::<Expr>()?);
                }
                "from_sysml" => {
                    let s: LitStr = input.parse()?;
                    from_sysml = Some(s.value());
                }
                "to_sysml" => {
                    let s: LitStr = input.parse()?;
                    to_sysml = Some(s.value());
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown key `{other}` in #[sysml::connect(...)]"),
                    ));
                }
            }
            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
            }
        }

        Ok(Self {
            interface_path,
            connection_type,
            name,
            from_expr: from_expr.ok_or_else(|| {
                syn::Error::new(input.span(), "missing `from = ...` in #[sysml::connect(...)]")
            })?,
            to_expr: to_expr.ok_or_else(|| {
                syn::Error::new(input.span(), "missing `to = ...` in #[sysml::connect(...)]")
            })?,
            from_sysml,
            to_sysml,
        })
    }
}

/// Convert a Rust field-access chain like `lunar_module_descent_stage.upper_stage_port`
/// into SysML camelCase chain `lunarModuleDescentStage.upperStagePort`.
fn expr_chain_to_sysml(expr: &Expr) -> String {
    match expr {
        Expr::Field(ef) => {
            let base = expr_chain_to_sysml(&ef.base);
            let member = match &ef.member {
                syn::Member::Named(id) => snake_to_camel(&id.to_string()),
                syn::Member::Unnamed(idx) => idx.index.to_string(),
            };
            format!("{base}.{member}")
        }
        Expr::Path(ep) => {
            if let Some(id) = ep.path.get_ident() {
                snake_to_camel(&id.to_string())
            } else {
                path_to_sysml_string(&ep.path)
            }
        }
        other => quote!(#other).to_string(),
    }
}

/// Prefix a field-access expression `a.b.c` with `s.` -> `s.a.b.c` for compile-time checking.
fn prefix_with_self_param(expr: &Expr) -> TokenStream {
    quote!(s.#expr)
}

pub fn expand_part(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let is_no_std = has_no_std_mode(&kv, &item.attrs);
    let rcfg = reflect_cfg(is_no_std);
    let doc = extract_doc(&item.attrs);
    let is_abstract = kv.has_flag("abstract");
    let is_individual = kv.has_flag("individual");
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| item.ident.to_string());

    // Collect `#[sysml::specializes(...)]`
    let mut specializes_paths = Vec::new();
    // Collect `#[sysml::perform(...)]`
    let mut performs = Vec::new();
    // Collect `#[sysml::satisfy(...)]`
    let mut satisfies = Vec::new();
    // Collect `#[sysml::verify(...)]`
    let mut verifies = Vec::new();
    // Collect `#[sysml::connect(...)]`
    let mut connects = Vec::new();
    // Collect `#[sysml::exhibit(name = "phases", state = StateType)]`
    let mut exhibits = Vec::new();
    // Collect ISO 26262 `#[sysml::fmea(...)]`, `#[sysml::independent(...)]`, `#[sysml::asil_decompose(...)]`
    let mut fmea_tokens = Vec::new();
    let mut independent_paths = Vec::new();
    let mut asil_decomp_tokens = Vec::new();
    let mut asil_decomp_checks = Vec::new();

    for attr in &item.attrs {
        if is_sysml_attr(attr, "specializes") {
            let list: PathList = attr.parse_args()?;
            specializes_paths.extend(list.paths.into_iter());
        } else if is_sysml_attr(attr, "perform") {
            let p: PerformArgs = attr.parse_args()?;
            performs.push(p);
        } else if is_sysml_attr(attr, "satisfy") {
            let s: ReqLinkArgs = attr.parse_args()?;
            satisfies.push(s);
        } else if is_sysml_attr(attr, "verify") {
            let v: ReqLinkArgs = attr.parse_args()?;
            verifies.push(v);
        } else if is_sysml_attr(attr, "connect") {
            let c: ConnectArgs = attr.parse_args()?;
            connects.push(c);
        } else if is_sysml_attr(attr, "exhibit") {
            let ekv: KvArgs = attr.parse_args()?;
            let ename = ekv.get_str("name").unwrap_or_else(|| "stateMachine".to_string());
            let estate = ekv.get_path("state").ok_or_else(|| {
                syn::Error::new_spanned(attr, "expected `state = StateType` in #[sysml::exhibit(...)]")
            })?;
            let inline = ekv.has_flag("inline");
            exhibits.push((ename, estate, inline));
        } else if is_sysml_attr(attr, "fmea") {
            let fkv: KvArgs = attr.parse_args()?;
            let failure_mode = fkv.get_str("failure_mode").ok_or_else(|| {
                syn::Error::new_spanned(attr, "missing `failure_mode = \"...\"` in #[sysml::fmea(...)]")
            })?;
            let effect = fkv.get_str("effect").ok_or_else(|| {
                syn::Error::new_spanned(attr, "missing `effect = \"...\"` in #[sysml::fmea(...)]")
            })?;
            let parse_1_10 = |key: &str| -> syn::Result<u8> {
                let raw = fkv.get_str(key).ok_or_else(|| {
                    syn::Error::new_spanned(attr, format!("missing `{key} = 1..=10` in #[sysml::fmea(...)]"))
                })?;
                let val: u8 = raw.parse().map_err(|_| {
                    syn::Error::new_spanned(attr, format!("`{key}` must be an integer 1..=10 in #[sysml::fmea(...)]"))
                })?;
                if !(1..=10).contains(&val) {
                    return Err(syn::Error::new_spanned(
                        attr,
                        format!("`{key}` must be in 1..=10 (got {val}) in #[sysml::fmea(...)]"),
                    ));
                }
                Ok(val)
            };
            let severity = parse_1_10("severity")?;
            let occurrence = parse_1_10("occurrence")?;
            let detection = parse_1_10("detection")?;
            let asil_tok = if let Some(asil_str) = fkv.get_str("asil") {
                quote!(::std::option::Option::Some(::std::string::String::from(#asil_str)))
            } else if let Some(asil_path) = fkv.get_path("asil") {
                quote!(::std::option::Option::Some(::std::string::String::from(
                    <#asil_path as ::sysml_dsl::core_contracts::AsilLevel>::LABEL
                )))
            } else {
                quote!(::std::option::Option::None)
            };
            fmea_tokens.push(quote! {
                ::sysml_dsl::ir::FmeaEntry {
                    failure_mode: ::std::string::String::from(#failure_mode),
                    effect: ::std::string::String::from(#effect),
                    severity: #severity,
                    occurrence: #occurrence,
                    detection: #detection,
                    asil: #asil_tok,
                }
            });
        } else if is_sysml_attr(attr, "independent") {
            let list: PathList = attr.parse_args()?;
            independent_paths.extend(list.paths.into_iter());
        } else if is_sysml_attr(attr, "asil_decompose") {
            let dkv: KvArgs = attr.parse_args()?;
            let goal = dkv.get_path("goal").ok_or_else(|| {
                syn::Error::new_spanned(attr, "expected `goal = SafetyGoalReq` in #[sysml::asil_decompose(...)]")
            })?;
            let left = dkv.get_path("left").ok_or_else(|| {
                syn::Error::new_spanned(attr, "expected `left = LeftPart` in #[sysml::asil_decompose(...)]")
            })?;
            let left_asil = dkv.get_path("left_asil").ok_or_else(|| {
                syn::Error::new_spanned(attr, "expected `left_asil = AsilB` in #[sysml::asil_decompose(...)]")
            })?;
            let right = dkv.get_path("right").ok_or_else(|| {
                syn::Error::new_spanned(attr, "expected `right = RightPart` in #[sysml::asil_decompose(...)]")
            })?;
            let right_asil = dkv.get_path("right_asil").ok_or_else(|| {
                syn::Error::new_spanned(attr, "expected `right_asil = AsilB` in #[sysml::asil_decompose(...)]")
            })?;
            asil_decomp_checks.push(quote! {
                const _: fn() = || {
                    ::sysml_dsl::core_contracts::assert_asil_decomposition::<
                        #goal, #left, #left_asil, #right, #right_asil
                    >();
                };
            });
            asil_decomp_tokens.push(quote! {
                ::sysml_dsl::ir::AsilDecompositionMeta {
                    goal: ::std::string::String::from(<#goal as ::sysml_dsl::core_contracts::SysmlRequirementCore>::REQ_ID),
                    left_part: ::std::string::String::from(<#left as ::sysml_dsl::core_contracts::SysmlPartCore>::SYSML_NAME),
                    left_asil: ::std::string::String::from(<#left_asil as ::sysml_dsl::core_contracts::AsilLevel>::LABEL),
                    right_part: ::std::string::String::from(<#right as ::sysml_dsl::core_contracts::SysmlPartCore>::SYSML_NAME),
                    right_asil: ::std::string::String::from(<#right_asil as ::sysml_dsl::core_contracts::AsilLevel>::LABEL),
                }
            });
        }
    }

    let spec_names: Vec<String> = specializes_paths.iter().map(path_to_sysml_string).collect();

    let auto_default_impl = build_auto_default_impl(&item)?;

    // Parse fields into attributes, nested parts, ports, items, requirement usages
    let mut attr_tokens = Vec::new();
    let mut part_tokens = Vec::new();
    let mut port_tokens = Vec::new();
    let mut item_tokens = Vec::new();
    let mut req_tokens = Vec::new();
    let mut field_type_checks = Vec::new();

    if let Fields::Named(fields) = &mut item.fields {
        for field in &mut fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            let fdoc = extract_doc(&field.attrs);
            let fdoc_tok = quote_opt_str(&fdoc);

            let is_part = field.attrs.iter().any(|a| is_sysml_attr(a, "part"));
            let is_port = field.attrs.iter().any(|a| is_sysml_attr(a, "port"));
            let is_item = field.attrs.iter().any(|a| is_sysml_attr(a, "item"));
            let is_req = field.attrs.iter().any(|a| is_sysml_attr(a, "requirement_ref"));

            if is_part {
                let mut fkv = KvArgs::default();
                for a in &field.attrs {
                    if is_sysml_attr(a, "part") {
                        if let Meta::List(_) = &a.meta {
                            fkv = a.parse_args()?;
                        }
                    }
                }
                let is_anonymous = fkv.has_flag("anonymous");
                let fname_sysml_opt = if is_anonymous {
                    None
                } else {
                    Some(
                        extract_sysml_name_override(&field.attrs)?
                            .or_else(|| fkv.get_str("name"))
                            .unwrap_or_else(|| snake_to_camel(&fname_rust)),
                    )
                };
                let fname_tok = quote_opt_str(&fname_sysml_opt);
                let (ty_name, _, rust_mult) = analyze_rust_type(&field.ty);
                let explicit_ty = fkv.get_str("ty");
                let ty_vec = if let Some(et) = explicit_ty {
                    if et.is_empty() {
                        vec![]
                    } else {
                        et.split(',').map(|s| s.trim().to_string()).collect()
                    }
                } else {
                    vec![ty_name]
                };
                let mult = fkv
                    .get_str("mult")
                    .map(|m| parse_multiplicity_str(&m))
                    .or(rust_mult);
                let is_ordered = fkv.has_flag("ordered");
                let is_ind = fkv.has_flag("individual");
                let is_ref = fkv.has_flag("ref");
                let subsets = fkv.get_all_str("subsets");
                let redefines = fkv.get_all_str("redefines");
                let default_val = fkv.get_str("default");

                // Compile-time check: inner type of a #[sysml::part] field must implement SysmlPartCore
                if let Some(inner_ty) = unwrap_container_type(&field.ty) {
                    if !fkv.has_flag("untyped") {
                        field_type_checks.push(quote! { ::sysml_dsl::core_contracts::assert_part_core::<#inner_ty>(); });
                    }
                }

                let ty_tok = quote_vec_str(&ty_vec);
                let subsets_tok = quote_vec_str(&subsets);
                let redefines_tok = quote_vec_str(&redefines);
                let mult_tok = quote_opt_mult(&mult);
                let def_tok = quote_opt_str(&default_val);

                part_tokens.push(quote! {
                    ::sysml_dsl::ir::PartUsage {
                        name: #fname_tok,
                        is_individual: #is_ind,
                        is_ref: #is_ref,
                        type_names: #ty_tok,
                        subsets: #subsets_tok,
                        redefines: #redefines_tok,
                        multiplicity: #mult_tok,
                        is_ordered: #is_ordered,
                        default_value: #def_tok,
                        doc: #fdoc_tok,
                        attributes: ::std::vec![],
                        nested_parts: ::std::vec![],
                        ports: ::std::vec![],
                        items: ::std::vec![],
                        performs: ::std::vec![],
                        connections: ::std::vec![],
                        requirements: ::std::vec![],
                    }
                });
            } else if is_port {
                let mut fkv = KvArgs::default();
                for a in &field.attrs {
                    if is_sysml_attr(a, "port") {
                        if let Meta::List(_) = &a.meta {
                            fkv = a.parse_args()?;
                        }
                    }
                }
                let fname_sysml = extract_sysml_name_override(&field.attrs)?
                    .or_else(|| fkv.get_str("name"))
                    .unwrap_or_else(|| snake_to_camel(&fname_rust));
                let (ty_name, conjugated, rust_mult) = analyze_rust_type(&field.ty);
                let mult = fkv
                    .get_str("mult")
                    .map(|m| parse_multiplicity_str(&m))
                    .or(rust_mult);
                let mult_tok = quote_opt_mult(&mult);
                let fty = &field.ty;
                field_type_checks.push(quote! { ::sysml_dsl::core_contracts::assert_port_core::<#fty>(); });
                port_tokens.push(quote! {
                    ::sysml_dsl::ir::PortUsage {
                        name: ::std::string::String::from(#fname_sysml),
                        type_name: ::std::string::String::from(#ty_name),
                        conjugated: #conjugated,
                        multiplicity: #mult_tok,
                        doc: #fdoc_tok,
                    }
                });
            } else if is_item {
                let mut fkv = KvArgs::default();
                for a in &field.attrs {
                    if is_sysml_attr(a, "item") {
                        if let Meta::List(_) = &a.meta {
                            fkv = a.parse_args()?;
                        }
                    }
                }
                let fname_sysml = extract_sysml_name_override(&field.attrs)?
                    .or_else(|| fkv.get_str("name"))
                    .unwrap_or_else(|| snake_to_camel(&fname_rust));
                let (ty_name, _, rust_mult) = analyze_rust_type(&field.ty);
                let mult_tok = quote_opt_mult(&rust_mult);
                let subsets = fkv.get_all_str("subsets");
                let subsets_tok = quote_vec_str(&subsets);
                if let Some(inner_ty) = unwrap_container_type(&field.ty) {
                    field_type_checks.push(quote! { ::sysml_dsl::core_contracts::assert_item_core::<#inner_ty>(); });
                }
                item_tokens.push(quote! {
                    ::sysml_dsl::ir::DirectedItem {
                        direction: ::std::option::Option::None,
                        name: ::std::string::String::from(#fname_sysml),
                        type_name: ::std::string::String::from(#ty_name),
                        multiplicity: #mult_tok,
                        subsets: #subsets_tok,
                        doc: #fdoc_tok,
                    }
                });
            } else if is_req {
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
                let (ty_name, _, rust_mult) = analyze_rust_type(&field.ty);
                let mult = fkv
                    .get_str("mult")
                    .map(|m| parse_multiplicity_str(&m))
                    .or(rust_mult);
                let mult_tok = quote_opt_mult(&mult);
                let subsets = fkv.get_all_str("subsets");
                let subsets_tok = quote_vec_str(&subsets);
                if let Some(inner_ty) = unwrap_container_type(&field.ty) {
                    field_type_checks.push(quote! { ::sysml_dsl::core_contracts::assert_requirement_core::<#inner_ty>(); });
                }
                req_tokens.push(quote! {
                    ::sysml_dsl::ir::RequirementUsage {
                        short_name: ::std::option::Option::None,
                        name: ::std::string::String::from(#fname_sysml),
                        type_name: ::std::option::Option::Some(::std::string::String::from(#ty_name)),
                        multiplicity: #mult_tok,
                        subsets: #subsets_tok,
                        doc: #fdoc_tok,
                        subject: ::std::option::Option::None,
                        nested_requirements: ::std::vec![],
                        satisfies: ::std::vec![],
                        verifies: ::std::vec![],
                    }
                });
            } else {
                // Default: typed attribute usage
                attr_tokens.push(build_attr_usage_tokens(&fname_rust, &field.ty, &field.attrs)?);
            }

            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    // Build IR & compile-time checks for `perform action`
    let mut perform_tokens = Vec::new();
    let mut perform_impls = Vec::new();
    let ident = &item.ident;
    for p in &performs {
        let pname = p
            .sysml_name
            .clone()
            .unwrap_or_else(|| snake_to_camel(&p.local_name.to_string()));
        let apath = &p.action_path;
        let aname = path_to_sysml_string(apath);
        perform_tokens.push(quote! {
            ::sysml_dsl::ir::PerformAction {
                name: ::std::option::Option::Some(::std::string::String::from(#pname)),
                action_type: ::std::option::Option::Some(::std::string::String::from(#aname)),
                references: ::std::option::Option::None,
                redefines: ::std::vec![],
            }
        });
        perform_impls.push(quote! {
            impl ::sysml_dsl::core_contracts::Performs<#apath> for #ident {}
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_action_core::<#apath>(); };
        });
    }

    // Build IR & compile-time checks for `satisfy`
    let mut satisfy_tokens = Vec::new();
    let mut satisfy_impls = Vec::new();
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
        satisfy_tokens.push(quote! {
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
        satisfy_impls.push(quote! {
            impl ::sysml_dsl::core_contracts::Satisfies<#rpath> for #ident {}
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_requirement_core::<#rpath>(); };
            #id_check
        });
    }

    // Build IR & compile-time checks for `verify`
    let mut verify_tokens = Vec::new();
    let mut verify_impls = Vec::new();
    for v in &verifies {
        let rpath = &v.req_path;
        let by_tok = quote_opt_str(&v.by);
        verify_tokens.push(quote! {
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
        verify_impls.push(quote! {
            impl ::sysml_dsl::core_contracts::Verifies<#rpath> for #ident {}
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_requirement_core::<#rpath>(); };
            #id_check
        });
    }

    // Build IR & compile-time checks for `connect` / `interface ... connect`
    let mut conn_tokens = Vec::new();
    let mut conn_checks = Vec::new();
    for c in &connects {
        let from_str = c
            .from_sysml
            .clone()
            .unwrap_or_else(|| expr_chain_to_sysml(&c.from_expr));
        let to_str = c
            .to_sysml
            .clone()
            .unwrap_or_else(|| expr_chain_to_sysml(&c.to_expr));
        let cname_tok = quote_opt_str(&c.name);
        let from_access = prefix_with_self_param(&c.from_expr);
        let to_access = prefix_with_self_param(&c.to_expr);

        if let Some(iface) = &c.interface_path {
            let iface_name = path_to_sysml_string(iface);
            conn_tokens.push(quote! {
                ::sysml_dsl::ir::ConnectionUsage {
                    kind: ::sysml_dsl::ir::ConnectionKind::Interface,
                    is_abstract: false,
                    name: #cname_tok,
                    type_name: ::std::option::Option::Some(::std::string::String::from(#iface_name)),
                    multiplicity: ::std::option::Option::None,
                    subsets: ::std::vec![],
                    from: ::std::option::Option::Some(::std::string::String::from(#from_str)),
                    to: ::std::option::Option::Some(::std::string::String::from(#to_str)),
                    doc: ::std::option::Option::None,
                }
            });
            conn_checks.push(quote! {
                const _: fn(&#ident) = |s: &#ident| {
                    ::sysml_dsl::core_contracts::assert_interface_connect_core::<#iface>(&#from_access, &#to_access);
                };
            });
        } else {
            let ctype_tok = quote_opt_str(&c.connection_type);
            conn_tokens.push(quote! {
                ::sysml_dsl::ir::ConnectionUsage {
                    kind: ::sysml_dsl::ir::ConnectionKind::Connection,
                    is_abstract: false,
                    name: #cname_tok,
                    type_name: #ctype_tok,
                    multiplicity: ::std::option::Option::None,
                    subsets: ::std::vec![],
                    from: ::std::option::Option::Some(::std::string::String::from(#from_str)),
                    to: ::std::option::Option::Some(::std::string::String::from(#to_str)),
                    doc: ::std::option::Option::None,
                }
            });
            conn_checks.push(quote! {
                const _: fn(&#ident) = |s: &#ident| {
                    ::sysml_dsl::core_contracts::assert_ports_connect_core(&#from_access, &#to_access);
                };
            });
        }
    }

    // Build IR & compile-time checks for `exhibit state`
    let mut exhibit_tokens = Vec::new();
    let mut exhibit_checks = Vec::new();
    for (ename, estate, inline) in &exhibits {
        let state_ty_name = path_to_sysml_string(estate);
        exhibit_checks.push(quote! {
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_state_core::<#estate>(); };
        });
        if *inline {
            exhibit_tokens.push(quote! {
                {
                    let sd = <#estate as ::sysml_dsl::ir::SysmlState>::state_def();
                    ::sysml_dsl::ir::StateUsage {
                        name: ::std::string::String::from(#ename),
                        type_name: ::std::option::Option::None,
                        is_exhibit: true,
                        doc: sd.doc,
                        entry_action: sd.entry_action,
                        do_action: sd.do_action,
                        exit_action: sd.exit_action,
                        states: sd.states,
                        transitions: sd.transitions,
                    }
                }
            });
        } else {
            exhibit_tokens.push(quote! {
                ::sysml_dsl::ir::StateUsage {
                    name: ::std::string::String::from(#ename),
                    type_name: ::std::option::Option::Some(::std::string::String::from(#state_ty_name)),
                    is_exhibit: true,
                    doc: ::std::option::Option::None,
                    entry_action: ::std::option::Option::None,
                    do_action: ::std::option::Option::None,
                    exit_action: ::std::option::Option::None,
                    states: ::std::vec![],
                    transitions: ::std::vec![],
                }
            });
        }
    }

    let comments = extract_comments(&item.attrs)?;
    let comment_tokens: Vec<TokenStream> = comments
        .iter()
        .map(|(cname, body)| {
            let cname_tok = quote_opt_str(cname);
            quote! {
                ::sysml_dsl::ir::Comment {
                    name: #cname_tok,
                    body: ::std::string::String::from(#body),
                }
            }
        })
        .collect();

    let refinements = extract_refinements(&item.attrs, &sysml_name)?;
    let ref_tokens: Vec<TokenStream> = refinements
        .iter()
        .map(|(c, s)| {
            quote! {
                ::sysml_dsl::ir::RefinementDependency {
                    client: ::std::string::String::from(#c),
                    supplier: ::std::string::String::from(#s),
                }
            }
        })
        .collect();

    let spec_impls = specializes_paths.iter().map(|base| {
        quote! {
            impl ::sysml_dsl::core_contracts::Specializes<#base> for #ident {}
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_part_core::<#base>(); };
        }
    });

    let independent_impls = independent_paths.iter().map(|other| {
        quote! {
            impl ::sysml_dsl::core_contracts::IndependentFrom<#other> for #ident {}
            const _: fn() = || { ::sysml_dsl::core_contracts::assert_part_core::<#other>(); };
        }
    });

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let doc_tok = quote_opt_str(&doc);
    let spec_tok = quote_vec_str(&spec_names);

    Ok(quote! {
        #item

        #auto_default_impl

        impl ::sysml_dsl::core_contracts::SysmlPartCore for #ident {
            const SYSML_NAME: &'static str = #sysml_name;
        }

        #rcfg
        impl ::sysml_dsl::ir::SysmlPart for #ident {
            fn part_def() -> ::sysml_dsl::ir::PartDef {
                ::sysml_dsl::ir::PartDef {
                    name: ::std::string::String::from(#sysml_name),
                    is_abstract: #is_abstract,
                    is_individual: #is_individual,
                    doc: #doc_tok,
                    comments: ::std::vec![#(#comment_tokens),*],
                    specializes: #spec_tok,
                    attributes: ::std::vec![#(#attr_tokens),*],
                    parts: ::std::vec![#(#part_tokens),*],
                    ports: ::std::vec![#(#port_tokens),*],
                    items: ::std::vec![#(#item_tokens),*],
                    performs: ::std::vec![#(#perform_tokens),*],
                    connections: ::std::vec![#(#conn_tokens),*],
                    exhibit_states: ::std::vec![#(#exhibit_tokens),*],
                    requirements: ::std::vec![#(#req_tokens),*],
                    satisfies: ::std::vec![#(#satisfy_tokens),*],
                    verifies: ::std::vec![#(#verify_tokens),*],
                    refinements: ::std::vec![#(#ref_tokens),*],
                    fmea: ::std::vec![#(#fmea_tokens),*],
                    asil_decompositions: ::std::vec![#(#asil_decomp_tokens),*],
                }
            }
        }

        const _: fn() = || {
            #(#field_type_checks)*
        };

        #(#spec_impls)*
        #(#independent_impls)*
        #(#asil_decomp_checks)*
        #(#perform_impls)*
        #(#satisfy_impls)*
        #(#verify_impls)*
        #(#conn_checks)*
        #(#exhibit_checks)*
    })
}

/// Unwrap `Vec<T>`, `Option<T>`, or `[T; N]` to inspect `T` for compile-time trait checks.
fn unwrap_container_type(ty: &syn::Type) -> Option<syn::Type> {
    if let syn::Type::Path(tp) = ty {
        if let Some(last) = tp.path.segments.last() {
            let id = last.ident.to_string();
            if id == "Vec" || id == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
                }
            }
            if id == "Box" {
                if let syn::PathArguments::AngleBracketed(args) = &last.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
                }
            }
        }
        return Some(ty.clone());
    }
    if let syn::Type::Array(arr) = ty {
        return Some((*arr.elem).clone());
    }
    None
}

// Keep unused import warning away if Punctuated isn't directly referenced
#[allow(dead_code)]
type _UnusedPunctuated = Punctuated<Path, Token![,]>;
