//! Closed, 1:1 SysML v2 <-> Rust expression transpiler for `#[sysml::require_constraint]`,
//! `#[sysml::transition(..., guard = ...)]`, and `#[sysml::calc]`.
//!
//! Any Rust construct that cannot be faithfully emitted to OMG SysML v2 (arbitrary
//! function calls, method calls, closures, macros, `unsafe`, `async`, loops, etc.)
//! is rejected at `rustc` compile time with a `syn::Error`, preventing semantic leakage.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeMap;
use syn::{BinOp, Expr, Lit, Member, Path, UnOp};

use crate::util::snake_to_camel;

/// Transpilation target context controlling identifier resolution and allowed callees.
pub enum ExprContext<'a> {
    /// Inside `#[sysml::require_constraint(...)]` on a requirement struct.
    /// Maps Rust field name -> SysML field name.
    Requirement {
        fields: &'a BTreeMap<String, String>,
    },
    /// Inside `#[sysml::transition(..., guard = ...)]` on a state machine enum.
    /// Allows `event.<field>` and (if `has_subject`) `subject.<field>`.
    TransitionGuard { has_subject: bool },
    /// Inside `#[sysml::calc] fn`.
    /// Maps Rust local/param name -> SysML name.
    Calc {
        bindings: &'a BTreeMap<String, String>,
    },
}

/// Result of transpiling one closed expression into both OMG SysML v2 and Rust.
pub struct TranspiledExpr {
    pub sysml: String,
    pub rust: TokenStream,
    /// Callee paths invoked inside `#[sysml::calc]` — each must be a modeled `#[sysml::calc]`
    /// (verified at compile time via `__sysml_calc_marker_<name>: SysmlCalc`).
    pub calc_callees: Vec<Path>,
}

pub fn transpile_expr(expr: &Expr, ctx: &ExprContext<'_>) -> syn::Result<TranspiledExpr> {
    let mut callees = Vec::new();
    let (sysml, rust) = visit_expr(expr, ctx, &mut callees)?;
    Ok(TranspiledExpr {
        sysml,
        rust,
        calc_callees: callees,
    })
}

fn is_numeric_binop(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::Add(_)
            | BinOp::Sub(_)
            | BinOp::Mul(_)
            | BinOp::Div(_)
            | BinOp::Lt(_)
            | BinOp::Le(_)
            | BinOp::Gt(_)
            | BinOp::Ge(_)
    )
}

fn is_bool_binop(op: &BinOp) -> bool {
    matches!(op, BinOp::And(_) | BinOp::Or(_))
}

fn visit_expr(
    expr: &Expr,
    ctx: &ExprContext<'_>,
    callees: &mut Vec<Path>,
) -> syn::Result<(String, TokenStream)> {
    match expr {
        Expr::Paren(p) => {
            let (s, r) = visit_expr(&p.expr, ctx, callees)?;
            Ok((format!("({s})"), quote!((#r))))
        }
        Expr::Lit(l) => match &l.lit {
            Lit::Int(i) => {
                let s = i.base10_digits().to_string();
                let f: f64 = s.parse().unwrap_or(0.0);
                let r = match ctx {
                    ExprContext::Calc { .. } => quote!(#l),
                    _ => quote!(#f),
                };
                Ok((s, r))
            }
            Lit::Float(f) => {
                let s = f.base10_digits().to_string();
                Ok((s, quote!(#l)))
            }
            Lit::Bool(b) => Ok((b.value.to_string(), quote!(#l))),
            Lit::Str(st) => {
                let escaped = st.value().replace('\\', "\\\\").replace('"', "\\\"");
                Ok((format!("\"{escaped}\""), quote!(#l)))
            }
            _ => Err(syn::Error::new_spanned(
                l,
                "unsupported literal kind in SysML expression (semantic leakage prevented)",
            )),
        },
        Expr::Path(ep) => {
            let ident = ep.path.get_ident().ok_or_else(|| {
                syn::Error::new_spanned(
                    ep,
                    "qualified paths are not allowed as values in SysML expressions; reference local fields or parameters",
                )
            })?;
            let rname = ident.to_string();
            match ctx {
                ExprContext::Requirement { fields } => {
                    let sysml_name = fields.get(&rname).ok_or_else(|| {
                        syn::Error::new_spanned(
                            ident,
                            format!(
                                "unknown requirement attribute `{rname}` in #[sysml::require_constraint]; expected one of: {:?}",
                                fields.keys().collect::<Vec<_>>()
                            ),
                        )
                    })?;
                    Ok((sysml_name.clone(), quote!(self.#ident)))
                }
                ExprContext::TransitionGuard { has_subject } => {
                    if rname == "event" {
                        Err(syn::Error::new_spanned(
                            ident,
                            "use `event.<field>` to access a trigger item attribute in #[sysml::transition] guard",
                        ))
                    } else if rname == "subject" && *has_subject {
                        Err(syn::Error::new_spanned(
                            ident,
                            "use `subject.<field>` to access a subject part attribute in #[sysml::transition] guard",
                        ))
                    } else {
                        Err(syn::Error::new_spanned(
                            ident,
                            format!(
                                "unqualified identifier `{rname}` is not allowed in #[sysml::transition] guard; use `event.<field>`{}",
                                if *has_subject { " or `subject.<field>`" } else { "" }
                            ),
                        ))
                    }
                }
                ExprContext::Calc { bindings } => {
                    let sysml_name = bindings.get(&rname).ok_or_else(|| {
                        syn::Error::new_spanned(
                            ident,
                            format!(
                                "unknown variable `{rname}` in #[sysml::calc]; only declared parameters and `let` locals are allowed"
                            ),
                        )
                    })?;
                    Ok((sysml_name.clone(), quote!(#ident)))
                }
            }
        }
        Expr::Field(ef) => {
            let member_ident = match &ef.member {
                Member::Named(id) => id,
                Member::Unnamed(idx) => {
                    return Err(syn::Error::new_spanned(
                        idx,
                        "tuple index access is not supported in SysML expressions",
                    ));
                }
            };
            let m_rust = member_ident.to_string();
            let m_sysml = snake_to_camel(&m_rust);

            // Check root of field chain
            match &*ef.base {
                Expr::Path(root_p) => {
                    let root_ident = root_p.path.get_ident().ok_or_else(|| {
                        syn::Error::new_spanned(root_p, "expected simple identifier at field chain root")
                    })?;
                    let root_str = root_ident.to_string();
                    match ctx {
                        ExprContext::Requirement { fields } => {
                            if root_str == "self" {
                                let sysml_name = fields.get(&m_rust).ok_or_else(|| {
                                    syn::Error::new_spanned(
                                        member_ident,
                                        format!("unknown requirement field `self.{m_rust}`"),
                                    )
                                })?;
                                Ok((sysml_name.clone(), quote!(self.#member_ident)))
                            } else {
                                Err(syn::Error::new_spanned(
                                    root_ident,
                                    "in #[sysml::require_constraint], access requirement attributes as `<field>` or `self.<field>`",
                                ))
                            }
                        }
                        ExprContext::TransitionGuard { has_subject } => {
                            if root_str == "event" {
                                Ok((m_sysml, quote!(__sysml_event.#member_ident)))
                            } else if root_str == "subject" {
                                if !*has_subject {
                                    return Err(syn::Error::new_spanned(
                                        root_ident,
                                        "`subject.<field>` requires `#[sysml::state(subject = PartType)]` on the state machine enum",
                                    ));
                                }
                                Ok((m_sysml, quote!(__sysml_subject.#member_ident)))
                            } else {
                                Err(syn::Error::new_spanned(
                                    root_ident,
                                    format!(
                                        "guard root must be `event`{} (found `{root_str}`)",
                                        if *has_subject { " or `subject`" } else { "" }
                                    ),
                                ))
                            }
                        }
                        ExprContext::Calc { bindings } => {
                            let base_sysml = bindings.get(&root_str).ok_or_else(|| {
                                syn::Error::new_spanned(
                                    root_ident,
                                    format!("unknown variable `{root_str}` in #[sysml::calc]"),
                                )
                            })?;
                            Ok((format!("{base_sysml}.{m_sysml}"), quote!(#root_ident.#member_ident)))
                        }
                    }
                }
                Expr::Field(_) => {
                    let (base_s, base_r) = visit_expr(&ef.base, ctx, callees)?;
                    Ok((format!("{base_s}.{m_sysml}"), quote!(#base_r.#member_ident)))
                }
                other => Err(syn::Error::new_spanned(
                    other,
                    "unsupported receiver in SysML field access",
                )),
            }
        }
        Expr::Unary(u) => {
            let (s, r) = visit_expr(&u.expr, ctx, callees)?;
            match u.op {
                UnOp::Not(_) => Ok((format!("not {s}"), quote!(!(#r)))),
                UnOp::Neg(_) => {
                    let r_num = wrap_scalar(r, ctx);
                    Ok((format!("-{s}"), quote!(-(#r_num))))
                }
                _ => Err(syn::Error::new_spanned(
                    u,
                    "only `!` and unary `-` are supported in SysML expressions",
                )),
            }
        }
        Expr::Binary(b) => {
            let (ls, lr) = visit_expr(&b.left, ctx, callees)?;
            let (rs, rr) = visit_expr(&b.right, ctx, callees)?;
            let sysml_op = match b.op {
                BinOp::Add(_) => "+",
                BinOp::Sub(_) => "-",
                BinOp::Mul(_) => "*",
                BinOp::Div(_) => "/",
                BinOp::Lt(_) => "<",
                BinOp::Le(_) => "<=",
                BinOp::Gt(_) => ">",
                BinOp::Ge(_) => ">=",
                BinOp::Eq(_) => "==",
                BinOp::Ne(_) => "!=",
                BinOp::And(_) => "and",
                BinOp::Or(_) => "or",
                _ => {
                    return Err(syn::Error::new_spanned(
                        b.op,
                        "unsupported binary operator in SysML expression",
                    ));
                }
            };
            let op = &b.op;
            let rust_expr = if is_numeric_binop(&b.op) {
                let l = wrap_scalar(lr, ctx);
                let r = wrap_scalar(rr, ctx);
                quote!((#l #op #r))
            } else if is_bool_binop(&b.op) {
                quote!(((#lr) #op (#rr)))
            } else {
                // `==` / `!=`: compare directly
                quote!(((#lr) #op (#rr)))
            };
            Ok((format!("{ls} {sysml_op} {rs}"), rust_expr))
        }
        Expr::If(eif) => {
            // SysML v2 conditional expression: `if cond ? then_expr else else_expr`
            let (cs, cr) = visit_expr(&eif.cond, ctx, callees)?;
            let then_expr = extract_single_block_expr(&eif.then_branch)?;
            let (_, else_box) = eif.else_branch.as_ref().ok_or_else(|| {
                syn::Error::new_spanned(
                    eif,
                    "SysML conditional expressions require an `else` branch (`if cond { a } else { b }`)",
                )
            })?;
            let else_expr = match &**else_box {
                Expr::Block(b) => extract_single_block_expr(&b.block)?,
                other => other,
            };
            let (ts, tr) = visit_expr(then_expr, ctx, callees)?;
            let (es, er) = visit_expr(else_expr, ctx, callees)?;
            Ok((
                format!("if {cs} ? {ts} else {es}"),
                quote!(if #cr { #tr } else { #er }),
            ))
        }
        Expr::Call(call) => {
            // Only allowed inside `#[sysml::calc]` — and every callee MUST be another
            // modeled `#[sysml::calc]` function (enforced via `__sysml_calc_marker_<fn>`!).
            if !matches!(ctx, ExprContext::Calc { .. }) {
                return Err(syn::Error::new_spanned(
                    call,
                    "arbitrary function calls are forbidden in SysML constraints/guards (semantic leakage prevented)",
                ));
            }
            let fn_path = match &*call.func {
                Expr::Path(p) => p.path.clone(),
                other => {
                    return Err(syn::Error::new_spanned(
                        other,
                        "only direct calls to modeled `#[sysml::calc]` functions are allowed inside `#[sysml::calc]`",
                    ));
                }
            };
            let mut arg_sysml = Vec::new();
            let mut arg_rust = Vec::new();
            for arg in &call.args {
                let (as_, ar) = visit_expr(arg, ctx, callees)?;
                arg_sysml.push(as_);
                arg_rust.push(ar);
            }
            callees.push(fn_path.clone());
            let last_name = fn_path
                .segments
                .last()
                .map(|s| snake_to_camel(&s.ident.to_string()))
                .unwrap_or_default();
            Ok((
                format!("{last_name}({})", arg_sysml.join(", ")),
                quote!(#fn_path(#(#arg_rust),*)),
            ))
        }
        other => Err(syn::Error::new_spanned(
            other,
            "untranspilable Rust expression forbidden inside SysML model (semantic leakage prevented: move imperative logic to an external runner crate)",
        )),
    }
}

fn extract_single_block_expr(block: &syn::Block) -> syn::Result<&Expr> {
    if block.stmts.len() != 1 {
        return Err(syn::Error::new_spanned(
            block,
            "SysML conditional branch must contain a single result expression",
        ));
    }
    match &block.stmts[0] {
        syn::Stmt::Expr(e, None) => Ok(e),
        other => Err(syn::Error::new_spanned(
            other,
            "SysML conditional branch must evaluate to a single expression without semicolon",
        )),
    }
}

fn wrap_scalar(tokens: TokenStream, ctx: &ExprContext<'_>) -> TokenStream {
    match ctx {
        // Inside `#[sysml::calc]`, parameters are already typed Rust scalars (e.g. `f64`).
        ExprContext::Calc { .. } => tokens,
        // In requirements and transition guards, fields may be `Quantity<f64>` or primitives.
        _ => quote!(::sysml_ir::ToScalar::to_scalar(#tokens)),
    }
}

/// Build the hidden marker struct path `__sysml_calc_marker_<fn>` corresponding to a callee path.
pub fn calc_marker_path(callee: &Path) -> Path {
    let mut marker = callee.clone();
    if let Some(last) = marker.segments.last_mut() {
        last.ident = format_ident!("__sysml_calc_marker_{}", last.ident);
    }
    marker
}
