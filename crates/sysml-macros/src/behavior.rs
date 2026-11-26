//! Proc-macro expansions for `#[sysml::action]` and `#[sysml::state]`.
//!
//! `#[sysml::state]` supports two idiomatic forms:
//! 1. On a `struct` — defines a SysML `state def` (such as CoSMA mission phases
//!    with `#[sysml::specializes(Phase)]` and `#[sysml::do_action(...)]`).
//! 2. On an `enum` — defines an **executable state machine** that BOTH emits
//!    valid SysML v2 `state def` with `state` usages + `transition`s AND
//!    generates compile-time checked transitions (`to = Variant` must exist,
//!    `on = Trigger` must implement `SysmlItem`) plus an executable `step()` API!

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{bracketed, Expr, ExprLit, Fields, Ident, Item, ItemEnum, ItemStruct, Lit, LitStr, Path, Token};

use crate::expr_transpile::{transpile_expr, ExprContext};
use crate::util::{
    analyze_rust_type, extract_doc, extract_refinements, extract_sysml_name_override,
    is_sysml_attr, path_to_sysml_string, quote_opt_str, quote_vec_str, snake_to_camel,
    strip_sysml_attrs, KvArgs, PathList,
};

/// Step declaration `step_name: ActionType` inside `#[sysml::do_action(...)]` or `#[sysml::step(...)]`.
struct StepDecl {
    name: Ident,
    action_type: Option<Path>,
}

impl Parse for StepDecl {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let action_type = if input.peek(Token![:]) {
            let _colon: Token![:] = input.parse()?;
            Some(input.parse::<Path>()?)
        } else {
            None
        };
        Ok(Self { name, action_type })
    }
}

/// Parses `#[sysml::do_action(name = "poweredDescentOperations", steps = [step1: Op1, step2: Op2])]`.
struct DoActionArgs {
    name: String,
    steps: Vec<StepDecl>,
}

impl Parse for DoActionArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name = None;
        let mut steps = Vec::new();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;
            match key.to_string().as_str() {
                "name" => {
                    let s: LitStr = input.parse()?;
                    name = Some(s.value());
                }
                "steps" => {
                    let content;
                    bracketed!(content in input);
                    let list: Punctuated<StepDecl, Token![,]> =
                        Punctuated::parse_terminated(&content)?;
                    steps.extend(list.into_iter());
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown key `{other}` in #[sysml::do_action(...)]"),
                    ));
                }
            }
            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
            }
        }
        Ok(Self {
            name: name.ok_or_else(|| {
                syn::Error::new(input.span(), "missing `name = \"...\"` in #[sysml::do_action(...)]")
            })?,
            steps,
        })
    }
}

enum GuardSpec {
    RawString(String),
    NativeExpr(Expr),
}

/// Parses `#[sysml::transition(on = TriggerItem, guard = event.v <= subject.max_v, to = TargetVariant)]`
/// or `#[sysml::transition(to = TargetVariant)]`.
struct TransitionArgs {
    trigger: Option<Path>,
    to: Ident,
    guard: Option<GuardSpec>,
    effect: Option<String>,
    doc: Option<String>,
}

impl Parse for TransitionArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut trigger = None;
        let mut to = None;
        let mut guard = None;
        let mut effect = None;
        let mut doc = None;
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            let _eq: Token![=] = input.parse()?;
            match key.to_string().as_str() {
                "on" | "accept" => {
                    trigger = Some(input.parse::<Path>()?);
                }
                "to" | "then" => {
                    to = Some(input.parse::<Ident>()?);
                }
                "guard" | "if_guard" => {
                    let expr: Expr = input.parse()?;
                    if let Expr::Lit(ExprLit {
                        lit: Lit::Str(s), ..
                    }) = expr
                    {
                        guard = Some(GuardSpec::RawString(s.value()));
                    } else {
                        guard = Some(GuardSpec::NativeExpr(expr));
                    }
                }
                "effect" => {
                    let s: LitStr = input.parse()?;
                    effect = Some(s.value());
                }
                "doc" => {
                    let s: LitStr = input.parse()?;
                    doc = Some(s.value());
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("unknown key `{other}` in #[sysml::transition(...)]"),
                    ));
                }
            }
            if input.peek(Token![,]) {
                let _comma: Token![,] = input.parse()?;
            }
        }
        Ok(Self {
            trigger,
            to: to.ok_or_else(|| {
                syn::Error::new(input.span(), "missing `to = TargetVariant` in #[sysml::transition(...)]")
            })?,
            guard,
            effect,
            doc,
        })
    }
}

pub fn expand_action(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
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
    let mut step_decls = Vec::new();
    for attr in &item.attrs {
        if is_sysml_attr(attr, "specializes") {
            let list: PathList = attr.parse_args()?;
            specializes_paths.extend(list.paths.into_iter());
        } else if is_sysml_attr(attr, "step") {
            let s: StepDecl = attr.parse_args()?;
            step_decls.push(s);
        }
    }
    let refinements = extract_refinements(&item.attrs, &sysml_name)?;
    let spec_names: Vec<String> = specializes_paths.iter().map(path_to_sysml_string).collect();

    let mut param_tokens = Vec::new();
    if let Fields::Named(fields) = &mut item.fields {
        for field in &mut fields.named {
            let fname_rust = field.ident.as_ref().unwrap().to_string();
            let fname_sysml = extract_sysml_name_override(&field.attrs)?
                .unwrap_or_else(|| snake_to_camel(&fname_rust));
            let is_out = field
                .attrs
                .iter()
                .any(|a| is_sysml_attr(a, "out") || is_sysml_attr(a, "out_param"));
            let is_inout = field
                .attrs
                .iter()
                .any(|a| is_sysml_attr(a, "inout") || is_sysml_attr(a, "inout_param"));
            let dir_tok = if is_out {
                quote!(::sysml_ir::Direction::Out)
            } else if is_inout {
                quote!(::sysml_ir::Direction::InOut)
            } else {
                quote!(::sysml_ir::Direction::In)
            };
            let (ty_name, _, _) = analyze_rust_type(&field.ty);
            param_tokens.push(quote! {
                ::sysml_ir::ActionParam {
                    direction: #dir_tok,
                    name: ::std::string::String::from(#fname_sysml),
                    type_name: ::std::string::String::from(#ty_name),
                }
            });
            field.attrs = strip_sysml_attrs(std::mem::take(&mut field.attrs));
        }
    }

    let mut step_tokens = Vec::new();
    let mut step_checks = Vec::new();
    for s in &step_decls {
        let sname = s.name.to_string();
        let stype = s.action_type.as_ref().map(path_to_sysml_string);
        let stype_tok = quote_opt_str(&stype);
        if let Some(p) = &s.action_type {
            step_checks.push(quote! { ::sysml_ir::assert_action::<#p>(); });
        }
        step_tokens.push(quote! {
            ::sysml_ir::ActionStep {
                name: ::std::string::String::from(#sname),
                type_name: #stype_tok,
                is_then: true,
            }
        });
    }

    let has_steps = !step_decls.is_empty();
    let ref_tokens = refinements.iter().map(|(c, s)| {
        quote! {
            ::sysml_ir::RefinementDependency {
                client: ::std::string::String::from(#c),
                supplier: ::std::string::String::from(#s),
            }
        }
    });

    let ident = &item.ident;
    let spec_impls = specializes_paths.iter().map(|base| {
        quote! {
            impl ::sysml_ir::Specializes<#base> for #ident {}
            const _: fn() = || { ::sysml_ir::assert_action::<#base>(); };
        }
    });

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let doc_tok = quote_opt_str(&doc);
    let spec_tok = quote_vec_str(&spec_names);

    Ok(quote! {
        #item

        impl ::sysml_ir::SysmlAction for #ident {
            const SYSML_NAME: &'static str = #sysml_name;

            fn action_def() -> ::sysml_ir::ActionDef {
                ::sysml_ir::ActionDef {
                    name: ::std::string::String::from(#sysml_name),
                    is_abstract: #is_abstract,
                    doc: #doc_tok,
                    specializes: #spec_tok,
                    params: ::std::vec![#(#param_tokens),*],
                    subactions: ::std::vec![],
                    has_initial_start: #has_steps,
                    steps: ::std::vec![#(#step_tokens),*],
                    has_done: #has_steps,
                    refinements: ::std::vec![#(#ref_tokens),*],
                }
            }
        }

        const _: fn() = || {
            #(#step_checks)*
        };

        #(#spec_impls)*
    })
}

pub fn expand_state(args: TokenStream, item: Item) -> syn::Result<TokenStream> {
    match item {
        Item::Struct(s) => expand_state_struct(args, s),
        Item::Enum(e) => expand_state_enum(args, e),
        other => Err(syn::Error::new_spanned(
            other,
            "#[sysml::state] can only be applied to a `struct` or `enum`",
        )),
    }
}

fn expand_state_struct(args: TokenStream, mut item: ItemStruct) -> syn::Result<TokenStream> {
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
    let mut do_action_args: Option<DoActionArgs> = None;

    for attr in &item.attrs {
        if is_sysml_attr(attr, "specializes") {
            let list: PathList = attr.parse_args()?;
            specializes_paths.extend(list.paths.into_iter());
        } else if is_sysml_attr(attr, "do_action") {
            do_action_args = Some(attr.parse_args()?);
        }
    }

    let mut action_checks = Vec::new();
    let do_action_tok = if let Some(da) = &do_action_args {
        let daname = &da.name;
        let step_toks: Vec<TokenStream> = da
            .steps
            .iter()
            .map(|s| {
                let sname = s.name.to_string();
                let stype = s.action_type.as_ref().map(path_to_sysml_string);
                let stype_tok = quote_opt_str(&stype);
                if let Some(p) = &s.action_type {
                    action_checks.push(quote! { ::sysml_ir::assert_action::<#p>(); });
                }
                quote! {
                    ::sysml_ir::ActionStep {
                        name: ::std::string::String::from(#sname),
                        type_name: #stype_tok,
                        is_then: true,
                    }
                }
            })
            .collect();
        let has_steps = !da.steps.is_empty();
        quote! {
            ::std::option::Option::Some(::sysml_ir::ActionUsage {
                name: ::std::string::String::from(#daname),
                type_name: ::std::option::Option::None,
                multiplicity: ::std::option::Option::None,
                redefines: ::std::vec![],
                has_initial_start: #has_steps,
                steps: ::std::vec![#(#step_toks),*],
                has_done: #has_steps,
                doc: ::std::option::Option::None,
            })
        }
    } else {
        quote!(::std::option::Option::None)
    };

    let spec_names: Vec<String> = specializes_paths.iter().map(path_to_sysml_string).collect();
    let ident = &item.ident;
    let spec_impls = specializes_paths.iter().map(|base| {
        quote! {
            impl ::sysml_ir::Specializes<#base> for #ident {}
            const _: fn() = || { ::sysml_ir::assert_state::<#base>(); };
        }
    });

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let doc_tok = quote_opt_str(&doc);
    let spec_tok = quote_vec_str(&spec_names);

    Ok(quote! {
        #item

        impl ::sysml_ir::SysmlState for #ident {
            const SYSML_NAME: &'static str = #sysml_name;

            fn state_def() -> ::sysml_ir::StateDef {
                ::sysml_ir::StateDef {
                    name: ::std::string::String::from(#sysml_name),
                    is_abstract: #is_abstract,
                    doc: #doc_tok,
                    specializes: #spec_tok,
                    subject_type: ::std::option::Option::None,
                    entry_action: ::std::option::Option::None,
                    do_action: #do_action_tok,
                    exit_action: ::std::option::Option::None,
                    states: ::std::vec![],
                    transitions: ::std::vec![],
                }
            }
        }

        const _: fn() = || {
            #(#action_checks)*
        };

        #(#spec_impls)*
    })
}

fn expand_state_enum(args: TokenStream, mut item: ItemEnum) -> syn::Result<TokenStream> {
    let kv: KvArgs = if args.is_empty() {
        KvArgs::default()
    } else {
        syn::parse2(args)?
    };
    let doc = extract_doc(&item.attrs);
    let sysml_name = extract_sysml_name_override(&item.attrs)?
        .or_else(|| kv.get_str("name"))
        .unwrap_or_else(|| item.ident.to_string());
    let subject_ty: Option<Path> = kv.get_path("subject");
    let has_subject = subject_ty.is_some();

    let mut specializes_paths = Vec::new();
    for attr in &item.attrs {
        if is_sysml_attr(attr, "specializes") {
            let list: PathList = attr.parse_args()?;
            specializes_paths.extend(list.paths.into_iter());
        }
    }
    let spec_names: Vec<String> = specializes_paths.iter().map(path_to_sysml_string).collect();

    let ident = &item.ident;
    let mut state_usage_tokens = Vec::new();
    let mut transition_tokens = Vec::new();
    let mut compile_checks = Vec::new();
    let mut step_match_arms = Vec::new();
    let mut guarded_event_checks = Vec::new();
    let mut allowed_match_arms = Vec::new();
    let mut name_match_arms = Vec::new();
    let mut initial_variant: Option<Ident> = None;

    if let Some(subj) = &subject_ty {
        compile_checks.push(quote! { ::sysml_ir::assert_part::<#subj>(); });
    }

    for variant in &mut item.variants {
        let v_ident = variant.ident.clone();
        let v_doc = extract_doc(&variant.attrs);
        let v_doc_tok = quote_opt_str(&v_doc);
        let v_sysml_name = extract_sysml_name_override(&variant.attrs)?
            .unwrap_or_else(|| lower_first(&v_ident.to_string()));

        let mut state_ty: Option<Path> = None;
        let mut variant_transitions = Vec::new();

        for attr in &variant.attrs {
            if is_sysml_attr(attr, "initial") {
                initial_variant = Some(v_ident.clone());
            } else if is_sysml_attr(attr, "transition") {
                let t: TransitionArgs = attr.parse_args()?;
                variant_transitions.push(t);
            } else if is_sysml_attr(attr, "state_type") {
                let p: Path = attr.parse_args()?;
                state_ty = Some(p);
            }
        }

        let state_ty_str = state_ty.as_ref().map(path_to_sysml_string);
        if let Some(st) = &state_ty {
            compile_checks.push(quote! { ::sysml_ir::assert_state::<#st>(); });
        }
        let state_ty_tok = quote_opt_str(&state_ty_str);
        state_usage_tokens.push(quote! {
            ::sysml_ir::StateUsage {
                name: ::std::string::String::from(#v_sysml_name),
                type_name: #state_ty_tok,
                is_exhibit: false,
                doc: #v_doc_tok,
                entry_action: ::std::option::Option::None,
                do_action: ::std::option::Option::None,
                exit_action: ::std::option::Option::None,
                states: ::std::vec![],
                transitions: ::std::vec![],
            }
        });

        name_match_arms.push(quote! {
            #ident::#v_ident => #v_sysml_name,
        });

        let mut allowed_triggers = Vec::new();
        for t in &variant_transitions {
            let target_ident = &t.to;
            let target_sysml = lower_first(&target_ident.to_string());
            let trigger_str = t.trigger.as_ref().map(path_to_sysml_string);
            let trigger_tok = quote_opt_str(&trigger_str);
            let effect_tok = quote_opt_str(&t.effect);
            let tdoc_tok = quote_opt_str(&t.doc);

            // Compile-time check 1: Target variant MUST exist on `Self`!
            compile_checks.push(quote! {
                let _target_check: #ident = #ident::#target_ident;
            });
            // Compile-time check 2: Trigger type MUST implement `SysmlItem`!
            if let Some(trig_path) = &t.trigger {
                compile_checks.push(quote! {
                    ::sysml_ir::assert_item::<#trig_path>();
                });
            }

            let (guard_sysml, guard_rust_opt) = match &t.guard {
                None => (None, None),
                Some(GuardSpec::RawString(s)) => (Some(s.clone()), None),
                Some(GuardSpec::NativeExpr(expr)) => {
                    let ctx = ExprContext::TransitionGuard { has_subject };
                    let transpiled = transpile_expr(expr, &ctx)?;
                    (Some(transpiled.sysml), Some(transpiled.rust))
                }
            };
            let guard_tok = quote_opt_str(&guard_sysml);

            let payload_name_tok = if guard_rust_opt.is_some() && t.trigger.is_some() {
                quote!(::std::option::Option::Some(::std::string::String::from("payload")))
            } else {
                quote!(::std::option::Option::None)
            };

            transition_tokens.push(quote! {
                ::sysml_ir::Transition {
                    name: ::std::option::Option::None,
                    source: ::std::string::String::from(#v_sysml_name),
                    trigger: #trigger_tok,
                    trigger_payload_name: #payload_name_tok,
                    guard: #guard_tok,
                    effect: #effect_tok,
                    target: ::std::string::String::from(#target_sysml),
                    doc: #tdoc_tok,
                }
            });

            let trig_key = trigger_str.unwrap_or_else(|| "auto".to_string());
            if !allowed_triggers.contains(&trig_key) {
                allowed_triggers.push(trig_key.clone());
            }

            if guard_rust_opt.is_none() {
                step_match_arms.push(quote! {
                    (#ident::#v_ident, #trig_key) => {
                        *self = #ident::#target_ident;
                        ::std::result::Result::Ok(*self)
                    }
                });
            }

            // Generate typed payload + guard dispatch arm for `step_event` / `step_with`
            if let Some(trig_path) = &t.trigger {
                if let Some(guard_expr) = &guard_rust_opt {
                    guarded_event_checks.push(quote! {
                        if *self == #ident::#v_ident {
                            if let ::std::option::Option::Some(__sysml_event) =
                                (__any_event as &dyn ::std::any::Any).downcast_ref::<#trig_path>()
                            {
                                if #guard_expr {
                                    *self = #ident::#target_ident;
                                    return ::std::result::Result::Ok(*self);
                                }
                            }
                        }
                    });
                } else {
                    guarded_event_checks.push(quote! {
                        if *self == #ident::#v_ident {
                            if (__any_event as &dyn ::std::any::Any).is::<#trig_path>() {
                                *self = #ident::#target_ident;
                                return ::std::result::Result::Ok(*self);
                            }
                        }
                    });
                }
            }
        }

        allowed_match_arms.push(quote! {
            #ident::#v_ident => ::std::vec![#(::std::string::String::from(#allowed_triggers)),*],
        });

        variant.attrs = strip_sysml_attrs(std::mem::take(&mut variant.attrs));
        variant.attrs.retain(|a| !is_sysml_attr(a, "state_type"));
    }

    let initial_fn = if let Some(init) = initial_variant.or_else(|| item.variants.first().map(|v| v.ident.clone())) {
        quote! {
            pub fn initial_state() -> Self {
                Self::#init
            }
        }
    } else {
        quote!()
    };

    let typed_step_fn = if let Some(subj) = &subject_ty {
        quote! {
            /// Evaluates typed trigger payload and compiled SysML v2 transition guards
            /// against both `event` and `subject`, advancing the state machine atomically.
            pub fn step_with<E: ::sysml_ir::SysmlItem + 'static>(
                &mut self,
                __any_event: &E,
                __sysml_subject: &#subj,
            ) -> ::std::result::Result<Self, ::sysml_ir::TransitionError> {
                let _ = __sysml_subject;
                #(#guarded_event_checks)*
                ::std::result::Result::Err(::sysml_ir::TransitionError {
                    state_machine: ::std::string::String::from(#sysml_name),
                    from: ::std::string::String::from(self.sysml_state_name()),
                    trigger: ::std::string::String::from(<E as ::sysml_ir::SysmlItem>::SYSML_NAME),
                    allowed_triggers: self.allowed_triggers(),
                })
            }
        }
    } else {
        quote! {
            /// Evaluates typed trigger payload and compiled SysML v2 transition guards,
            /// advancing the state machine atomically.
            pub fn step_event<E: ::sysml_ir::SysmlItem + 'static>(
                &mut self,
                __any_event: &E,
            ) -> ::std::result::Result<Self, ::sysml_ir::TransitionError> {
                #(#guarded_event_checks)*
                ::std::result::Result::Err(::sysml_ir::TransitionError {
                    state_machine: ::std::string::String::from(#sysml_name),
                    from: ::std::string::String::from(self.sysml_state_name()),
                    trigger: ::std::string::String::from(<E as ::sysml_ir::SysmlItem>::SYSML_NAME),
                    allowed_triggers: self.allowed_triggers(),
                })
            }
        }
    };

    let spec_impls = specializes_paths.iter().map(|base| {
        quote! {
            impl ::sysml_ir::Specializes<#base> for #ident {}
            const _: fn() = || { ::sysml_ir::assert_state::<#base>(); };
        }
    });

    item.attrs = strip_sysml_attrs(std::mem::take(&mut item.attrs));
    let doc_tok = quote_opt_str(&doc);
    let spec_tok = quote_vec_str(&spec_names);
    let subject_type_str = subject_ty.as_ref().map(path_to_sysml_string);
    let subject_type_tok = quote_opt_str(&subject_type_str);

    Ok(quote! {
        #item

        impl #ident {
            #initial_fn

            /// Returns the SysML v2 state name of the current variant.
            pub fn sysml_state_name(&self) -> &'static str {
                match self {
                    #(#name_match_arms)*
                }
            }

            /// Returns the list of legal trigger names from the current state.
            pub fn allowed_triggers(&self) -> ::std::vec::Vec<::std::string::String> {
                match self {
                    #(#allowed_match_arms)*
                }
            }

            /// Executes an unguarded transition by trigger name (or `"auto"` for unconditional
            /// transitions), returning `Err(TransitionError)` if the transition is illegal.
            pub fn step(&mut self, trigger: &str) -> ::std::result::Result<Self, ::sysml_ir::TransitionError> {
                match (*self, trigger) {
                    #(#step_match_arms)*
                    _ => ::std::result::Result::Err(::sysml_ir::TransitionError {
                        state_machine: ::std::string::String::from(#sysml_name),
                        from: ::std::string::String::from(self.sysml_state_name()),
                        trigger: ::std::string::String::from(trigger),
                        allowed_triggers: self.allowed_triggers(),
                    }),
                }
            }

            /// Executes an unguarded strongly typed transition using a `SysmlItem` trigger type.
            pub fn step_on<T: ::sysml_ir::SysmlItem>(&mut self) -> ::std::result::Result<Self, ::sysml_ir::TransitionError> {
                self.step(<T as ::sysml_ir::SysmlItem>::SYSML_NAME)
            }

            #typed_step_fn
        }

        impl ::sysml_ir::SysmlState for #ident {
            const SYSML_NAME: &'static str = #sysml_name;

            fn state_def() -> ::sysml_ir::StateDef {
                ::sysml_ir::StateDef {
                    name: ::std::string::String::from(#sysml_name),
                    is_abstract: false,
                    doc: #doc_tok,
                    specializes: #spec_tok,
                    subject_type: #subject_type_tok,
                    entry_action: ::std::option::Option::None,
                    do_action: ::std::option::Option::None,
                    exit_action: ::std::option::Option::None,
                    states: ::std::vec![#(#state_usage_tokens),*],
                    transitions: ::std::vec![#(#transition_tokens),*],
                }
            }
        }

        const _: fn() = || {
            #(#compile_checks)*
        };

        #(#spec_impls)*
    })
}

fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_ascii_lowercase().to_string() + chars.as_str(),
    }
}
