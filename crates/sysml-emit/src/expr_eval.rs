//! C2 / D4 — IR expression parser and evaluator.
//!
//! Constraints, calc bodies and derived attribute values are stored in the IR as SysML
//! expression **strings** (`require_constraints: Vec<String>`, `CalcDef::result_expr`, ...),
//! produced by our closed transpiler (`sysml-macros/src/expr_transpile.rs`) or lifted from a
//! `.sysml` file. This module parses exactly that subset and evaluates it on IR values, so the
//! operators (`audit`, later `refute` / `trade-study`) work on `.sysml` files from any source.
//!
//! # Grammar (lowest to highest precedence)
//!
//! ```text
//! expr    := 'if' or ('?' | 'then') expr 'else' expr | or
//! or      := and (('or' | '|') and)*
//! and     := eq (('and' | '&') eq)*
//! eq      := rel (('==' | '!=') rel)*
//! rel     := add (('<' | '<=' | '>' | '>=') add)?
//! add     := mul (('+' | '-') mul)*
//! mul     := unary (('*' | '/') unary)*
//! unary   := ('-' | '+' | 'not' | '!') unary | postfix
//! postfix := primary ('[' unit ']')?
//! primary := number | 'true' | 'false' | string | '(' expr ')' | expr-starting-with-'if'
//!          | name ('.' name)*  | name '(' (expr (',' expr)*)? ')'
//! name    := identifier | 'quoted name'
//! ```
//!
//! `if c ? a else b` is what the transpiler emits for a Rust `if c { a } else { b }`;
//! `if c then a else b` is accepted as well.
//!
//! # Values, units and unknowns
//!
//! Numbers carry an SI dimension ([`DimTag`]): an attribute with a unit has a known dimension
//! (from the closed `sysml_core::units` table), a bare literal is a wildcard (compatible with
//! anything, as in the compile-time check), an attribute without a unit is unknown-dimension
//! (checks skipped). `+`, `-`, comparisons, `==` and `if` branches on two known, different
//! dimensions are an [`EvalError::Dimension`]; so are two direct attribute references of one
//! dimension whose units have different known scales (`g` vs `kg`), because the compiled code
//! compares the raw numbers. Values are never converted.
//!
//! Evaluation is three-valued: an operand without a value (a `tbd_value` attribute, an
//! `evidence` attribute nobody has measured, a declaration without a default) yields an
//! unknown number/truth carrying the reasons, never a guess. `and` / `or` use Kleene logic
//! (`false and unknown = false`), so a constraint is decided as soon as its known part decides.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use sysml_core::units::{parse_unit, Dim};
use sysml_ir::{
    AttributeUsage, CalcDef, Model, Package, PackageMember, PartDef, RequirementDef, TbdKind,
};

use crate::roles::{attr_key, base_scale, RoleIndex};

// ---------------------------------------------------------------------------------------------
// AST + parser
// ---------------------------------------------------------------------------------------------

/// Binary operators of the subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::And => "and",
            BinOp::Or => "or",
        }
    }
}

/// Parsed expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Numeric literal; `int` when written without `.` or exponent.
    Num { value: f64, int: bool },
    Bool(bool),
    Str(String),
    /// Attribute reference, plain (`maxPower`) or dotted (`motor.ratedPower`).
    Ref(Vec<String>),
    /// Literal or expression with a unit suffix (`170.0 [W]`).
    WithUnit(Box<Expr>, String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    /// Call of a `calc def` present in the IR.
    Call(String, Vec<Expr>),
}

/// A syntax error, with the byte offset where it was detected.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at byte {}: {}", self.offset, self.message)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64, bool),
    Str(String),
    Name(String),
    /// Keyword or operator.
    Sym(&'static str),
    Unit(String),
}

fn lex(src: &str) -> Result<Vec<(usize, Tok)>, ParseError> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    let err = |offset: usize, m: &str| ParseError {
        offset,
        message: m.to_string(),
    };
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_digit() || (c == b'.' && i + 1 < b.len() && b[i + 1].is_ascii_digit()) {
            let mut int = true;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            if i < b.len() && b[i] == b'.' && !(i + 1 < b.len() && b[i + 1] == b'.') {
                int = false;
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
                let mut j = i + 1;
                if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
                    j += 1;
                }
                if j < b.len() && b[j].is_ascii_digit() {
                    int = false;
                    i = j;
                    while i < b.len() && b[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let text = &src[start..i];
            let v: f64 = text.parse().map_err(|_| err(start, "bad number"))?;
            out.push((start, Tok::Num(v, int)));
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            let word = &src[start..i];
            let tok = match word {
                "and" => Tok::Sym("and"),
                "or" => Tok::Sym("or"),
                "not" => Tok::Sym("not"),
                "if" => Tok::Sym("if"),
                "then" => Tok::Sym("then"),
                "else" => Tok::Sym("else"),
                "true" => Tok::Sym("true"),
                "false" => Tok::Sym("false"),
                _ => Tok::Name(word.to_string()),
            };
            out.push((start, tok));
            continue;
        }
        if c == b'\'' {
            i += 1;
            let s0 = i;
            while i < b.len() && b[i] != b'\'' {
                i += 1;
            }
            if i >= b.len() {
                return Err(err(start, "unterminated quoted name"));
            }
            out.push((start, Tok::Name(src[s0..i].to_string())));
            i += 1;
            continue;
        }
        if c == b'"' {
            i += 1;
            let mut s = String::new();
            while i < b.len() && b[i] != b'"' {
                if b[i] == b'\\' && i + 1 < b.len() {
                    i += 1;
                }
                s.push(b[i] as char);
                i += 1;
            }
            if i >= b.len() {
                return Err(err(start, "unterminated string"));
            }
            i += 1;
            out.push((start, Tok::Str(s)));
            continue;
        }
        if c == b'[' {
            let close = src[i..]
                .find(']')
                .ok_or_else(|| err(start, "unterminated unit `[...`"))?;
            out.push((start, Tok::Unit(src[i + 1..i + close].trim().to_string())));
            i += close + 1;
            continue;
        }
        let two = if i + 1 < b.len() { &src[i..i + 2] } else { "" };
        let sym: &'static str = match two {
            "<=" => "<=",
            ">=" => ">=",
            "==" => "==",
            "!=" => "!=",
            "&&" => "and",
            "||" => "or",
            _ => match c {
                b'+' => "+",
                b'-' => "-",
                b'*' => "*",
                b'/' => "/",
                b'<' => "<",
                b'>' => ">",
                b'(' => "(",
                b')' => ")",
                b',' => ",",
                b'.' => ".",
                b'?' => "?",
                b'!' => "not",
                b'&' => "and",
                b'|' => "or",
                _ => return Err(err(start, &format!("unexpected character `{}`", c as char))),
            },
        };
        i += if matches!(two, "<=" | ">=" | "==" | "!=" | "&&" | "||") { 2 } else { 1 };
        out.push((start, Tok::Sym(sym)));
    }
    Ok(out)
}

struct Parser {
    toks: Vec<(usize, Tok)>,
    pos: usize,
    len: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|(_, t)| t)
    }
    fn offset(&self) -> usize {
        self.toks.get(self.pos).map(|(o, _)| *o).unwrap_or(self.len)
    }
    fn is(&self, s: &str) -> bool {
        matches!(self.peek(), Some(Tok::Sym(x)) if *x == s)
    }
    fn eat(&mut self, s: &str) -> bool {
        if self.is(s) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn expect(&mut self, s: &str) -> Result<(), ParseError> {
        if self.eat(s) {
            Ok(())
        } else {
            Err(self.error(&format!("expected `{s}`")))
        }
    }
    fn error(&self, m: &str) -> ParseError {
        ParseError {
            offset: self.offset(),
            message: match self.peek() {
                Some(t) => format!("{m}, found {t:?}"),
                None => format!("{m}, found end of expression"),
            },
        }
    }

    fn expr(&mut self) -> Result<Expr, ParseError> {
        if self.eat("if") {
            let cond = self.or()?;
            if !(self.eat("?") || self.eat("then")) {
                return Err(self.error("expected `?` or `then` after `if` condition"));
            }
            let then = self.expr()?;
            self.expect("else")?;
            let els = self.expr()?;
            return Ok(Expr::If(Box::new(cond), Box::new(then), Box::new(els)));
        }
        self.or()
    }

    fn or(&mut self) -> Result<Expr, ParseError> {
        let mut l = self.and()?;
        while self.eat("or") {
            let r = self.and()?;
            l = Expr::Bin(BinOp::Or, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn and(&mut self) -> Result<Expr, ParseError> {
        let mut l = self.eq()?;
        while self.eat("and") {
            let r = self.eq()?;
            l = Expr::Bin(BinOp::And, Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn eq(&mut self) -> Result<Expr, ParseError> {
        let mut l = self.rel()?;
        loop {
            let op = if self.eat("==") {
                BinOp::Eq
            } else if self.eat("!=") {
                BinOp::Ne
            } else {
                return Ok(l);
            };
            let r = self.rel()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
    }

    fn rel(&mut self) -> Result<Expr, ParseError> {
        let l = self.add()?;
        let op = if self.eat("<=") {
            BinOp::Le
        } else if self.eat(">=") {
            BinOp::Ge
        } else if self.eat("<") {
            BinOp::Lt
        } else if self.eat(">") {
            BinOp::Gt
        } else {
            return Ok(l);
        };
        let r = self.add()?;
        Ok(Expr::Bin(op, Box::new(l), Box::new(r)))
    }

    fn add(&mut self) -> Result<Expr, ParseError> {
        let mut l = self.mul()?;
        loop {
            let op = if self.eat("+") {
                BinOp::Add
            } else if self.eat("-") {
                BinOp::Sub
            } else {
                return Ok(l);
            };
            let r = self.mul()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
    }

    fn mul(&mut self) -> Result<Expr, ParseError> {
        let mut l = self.unary()?;
        loop {
            let op = if self.eat("*") {
                BinOp::Mul
            } else if self.eat("/") {
                BinOp::Div
            } else {
                return Ok(l);
            };
            let r = self.unary()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        if self.eat("-") {
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        if self.eat("+") {
            return self.unary();
        }
        if self.eat("not") {
            return Ok(Expr::Not(Box::new(self.unary()?)));
        }
        let p = self.primary()?;
        if let Some(Tok::Unit(u)) = self.peek().cloned() {
            self.pos += 1;
            return Ok(Expr::WithUnit(Box::new(p), u));
        }
        Ok(p)
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        if self.is("if") {
            return self.expr();
        }
        if self.eat("(") {
            let e = self.expr()?;
            self.expect(")")?;
            return Ok(e);
        }
        if self.eat("true") {
            return Ok(Expr::Bool(true));
        }
        if self.eat("false") {
            return Ok(Expr::Bool(false));
        }
        match self.peek().cloned() {
            Some(Tok::Num(v, int)) => {
                self.pos += 1;
                Ok(Expr::Num { value: v, int })
            }
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Ok(Expr::Str(s))
            }
            Some(Tok::Name(n)) => {
                self.pos += 1;
                if self.eat("(") {
                    let mut args = Vec::new();
                    if !self.eat(")") {
                        loop {
                            args.push(self.expr()?);
                            if self.eat(")") {
                                break;
                            }
                            self.expect(",")?;
                        }
                    }
                    return Ok(Expr::Call(n, args));
                }
                let mut path = vec![n];
                while self.eat(".") {
                    match self.peek().cloned() {
                        Some(Tok::Name(m)) => {
                            self.pos += 1;
                            path.push(m);
                        }
                        _ => return Err(self.error("expected a name after `.`")),
                    }
                }
                Ok(Expr::Ref(path))
            }
            _ => Err(self.error("expected an operand")),
        }
    }
}

/// Parse one expression of the subset (the whole string must be consumed).
pub fn parse_expr(src: &str) -> Result<Expr, ParseError> {
    let toks = lex(src)?;
    let mut p = Parser {
        toks,
        pos: 0,
        len: src.len(),
    };
    let e = p.expr()?;
    if p.pos != p.toks.len() {
        return Err(p.error("unexpected trailing input"));
    }
    Ok(e)
}

// ---------------------------------------------------------------------------------------------
// Values
// ---------------------------------------------------------------------------------------------

/// Dimension of a numeric value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DimTag {
    /// No (or no parseable) unit: dimension checks involving it are skipped.
    Unknown,
    /// Bare literal: compatible with any dimension.
    Wildcard,
    Known(Dim),
}

impl DimTag {
    pub fn of_unit(unit: Option<&str>) -> DimTag {
        match unit.map(str::trim).filter(|u| !u.is_empty()).and_then(parse_unit) {
            Some(d) => DimTag::Known(d),
            None => DimTag::Unknown,
        }
    }

    fn mul(self, o: DimTag) -> DimTag {
        match (self, o) {
            (DimTag::Unknown, _) | (_, DimTag::Unknown) => DimTag::Unknown,
            (DimTag::Wildcard, x) | (x, DimTag::Wildcard) => x,
            (DimTag::Known(a), DimTag::Known(b)) => DimTag::Known(a.mul(b)),
        }
    }

    fn div(self, o: DimTag) -> DimTag {
        match (self, o) {
            (DimTag::Unknown, _) | (_, DimTag::Unknown) => DimTag::Unknown,
            (DimTag::Wildcard, DimTag::Known(b)) => DimTag::Known(b.powi(-1)),
            (x, DimTag::Wildcard) => x,
            (DimTag::Known(a), DimTag::Known(b)) => DimTag::Known(a.div(b)),
        }
    }

    /// Result of `a + b` / `if` branches (after [`Self::compatible`]).
    fn join(self, o: DimTag) -> DimTag {
        match (self, o) {
            (DimTag::Unknown, _) | (_, DimTag::Unknown) => DimTag::Unknown,
            (DimTag::Known(a), _) | (_, DimTag::Known(a)) => DimTag::Known(a),
            _ => DimTag::Wildcard,
        }
    }

    fn compatible(self, o: DimTag) -> bool {
        match (self, o) {
            (DimTag::Known(a), DimTag::Known(b)) => a.eq(b),
            _ => true,
        }
    }

    fn render(self) -> String {
        match self {
            DimTag::Unknown => "unknown dimension".into(),
            DimTag::Wildcard => "dimensionless literal".into(),
            DimTag::Known(d) => {
                const NAMES: [&str; 7] = ["m", "kg", "s", "A", "K", "mol", "cd"];
                let parts: Vec<String> = d
                    .0
                    .iter()
                    .zip(NAMES)
                    .filter(|(e, _)| **e != 0)
                    .map(|(e, n)| if *e == 1 { n.to_string() } else { format!("{n}^{e}") })
                    .collect();
                if parts.is_empty() {
                    "1".into()
                } else {
                    parts.join("*")
                }
            }
        }
    }
}

/// A number, possibly unknown, with its dimension.
#[derive(Debug, Clone, PartialEq)]
pub struct Num {
    /// `None` = unknown (see `why`).
    pub value: Option<f64>,
    pub dim: DimTag,
    /// Unit as written on a directly referenced attribute (`None` for compound expressions).
    pub unit: Option<String>,
    /// Integer literal (only meaningful inside calc bodies, where Rust integer division applies).
    pub int: bool,
    pub why: Vec<String>,
}

/// Result of evaluating an expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Num(Num),
    /// `None` = unknown truth (see `why`).
    Bool(Option<bool>, Vec<String>),
    Str(String),
    /// Unknown value of unknown type (e.g. an untyped attribute with no value).
    Unknown(Vec<String>),
}

impl Value {
    pub fn number(v: f64, unit: Option<&str>) -> Value {
        Value::Num(Num {
            value: Some(v),
            dim: DimTag::of_unit(unit),
            unit: unit.map(str::to_string),
            int: false,
            why: Vec::new(),
        })
    }

    pub fn unknown_number(unit: Option<&str>, why: impl Into<String>) -> Value {
        Value::Num(Num {
            value: None,
            dim: DimTag::of_unit(unit),
            unit: unit.map(str::to_string),
            int: false,
            why: vec![why.into()],
        })
    }

    /// Three-valued truth of a boolean result (`None` when unknown or not a boolean).
    pub fn truth(&self) -> Option<bool> {
        match self {
            Value::Bool(b, _) => *b,
            _ => None,
        }
    }

    /// Known numeric value.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Num(n) => n.value,
            _ => None,
        }
    }

    /// Why the value is unknown (empty when known).
    pub fn reasons(&self) -> Vec<String> {
        match self {
            Value::Num(n) => n.why.clone(),
            Value::Bool(_, w) | Value::Unknown(w) => w.clone(),
            Value::Str(_) => Vec::new(),
        }
    }
}

/// An evaluation that cannot produce even an unknown value.
#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    Parse(ParseError),
    /// Mismatched dimensions (or scales) in `+`, `-`, a comparison or `if` branches.
    Dimension(String),
    /// A reference that does not resolve in the scope.
    Unresolved(String),
    /// A call of something that is not a `calc def` in the IR, or with the wrong arity.
    Call(String),
    /// An operator applied to the wrong kind of value (e.g. `not 3.0`).
    Type(String),
    /// Calc recursion or a cyclic attribute definition.
    Recursion(String),
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::Parse(e) => write!(f, "parse error {e}"),
            EvalError::Dimension(m) => write!(f, "dimension error: {m}"),
            EvalError::Unresolved(m) => write!(f, "unresolved reference `{m}`"),
            EvalError::Call(m) => write!(f, "call error: {m}"),
            EvalError::Type(m) => write!(f, "type error: {m}"),
            EvalError::Recursion(m) => write!(f, "recursion: {m}"),
        }
    }
}

/// Where references are resolved.
pub trait Scope {
    /// Value of a plain (`[name]`) or dotted reference; `None` when it does not resolve.
    fn lookup(&self, path: &[String], ev: &Evaluator<'_>) -> Option<Result<Value, EvalError>>;
}

/// A flat name -> value scope (calc parameters and locals, tests).
#[derive(Debug, Clone, Default)]
pub struct MapScope(pub BTreeMap<String, Value>);

impl Scope for MapScope {
    fn lookup(&self, path: &[String], _ev: &Evaluator<'_>) -> Option<Result<Value, EvalError>> {
        match path {
            [name] => self.0.get(name).cloned().map(Ok),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------------------------

const MAX_CALL_DEPTH: usize = 64;

/// Evaluates parsed expressions; `calc def`s of the model are callable by name.
pub struct Evaluator<'m> {
    calcs: BTreeMap<String, &'m CalcDef>,
    depth: RefCell<usize>,
}

fn simple(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name).trim()
}

fn merge(a: &[String], b: &[String]) -> Vec<String> {
    let mut out = a.to_vec();
    for x in b {
        if !out.contains(x) {
            out.push(x.clone());
        }
    }
    out
}

impl<'m> Evaluator<'m> {
    /// An evaluator whose callable calcs are every `calc def` in `model` (library included).
    pub fn new(model: &'m Model) -> Self {
        fn walk<'m>(pkg: &'m Package, out: &mut BTreeMap<String, &'m CalcDef>) {
            for m in &pkg.members {
                match m {
                    PackageMember::Package(sub) => walk(sub, out),
                    PackageMember::CalcDef(c) => {
                        out.entry(c.name.clone()).or_insert(c);
                    }
                    _ => {}
                }
            }
        }
        let mut calcs = BTreeMap::new();
        for p in &model.packages {
            walk(p, &mut calcs);
        }
        Evaluator {
            calcs,
            depth: RefCell::new(0),
        }
    }

    /// An evaluator without calcs.
    pub fn empty() -> Self {
        Evaluator {
            calcs: BTreeMap::new(),
            depth: RefCell::new(0),
        }
    }

    pub fn calc(&self, name: &str) -> Option<&'m CalcDef> {
        self.calcs.get(simple(name)).copied()
    }

    /// Parse and evaluate `src` (requirement semantics: all arithmetic in `f64`).
    pub fn eval_str(&self, src: &str, scope: &dyn Scope) -> Result<Value, EvalError> {
        let e = parse_expr(src).map_err(EvalError::Parse)?;
        self.eval(&e, scope, false)
    }

    /// Evaluate `e`. `int_semantics` applies Rust integer division to two integer literals
    /// (calc bodies, where the transpiler keeps integer literals as integers).
    pub fn eval(&self, e: &Expr, scope: &dyn Scope, int_semantics: bool) -> Result<Value, EvalError> {
        match e {
            Expr::Num { value, int } => Ok(Value::Num(Num {
                value: Some(*value),
                dim: DimTag::Wildcard,
                unit: None,
                int: *int && int_semantics,
                why: Vec::new(),
            })),
            Expr::Bool(b) => Ok(Value::Bool(Some(*b), Vec::new())),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Ref(path) => match scope.lookup(path, self) {
                Some(r) => r,
                None => Err(EvalError::Unresolved(path.join("."))),
            },
            Expr::WithUnit(inner, unit) => {
                let v = self.eval(inner, scope, int_semantics)?;
                let n = self.num(v, "[unit]")?;
                Ok(Value::Num(Num {
                    dim: DimTag::of_unit(Some(unit)),
                    unit: Some(unit.clone()),
                    int: false,
                    ..n
                }))
            }
            Expr::Neg(inner) => {
                let v = self.eval(inner, scope, int_semantics)?;
                let n = self.num(v, "unary -")?;
                Ok(Value::Num(Num {
                    value: n.value.map(|x| -x),
                    unit: None,
                    ..n
                }))
            }
            Expr::Not(inner) => {
                let v = self.eval(inner, scope, int_semantics)?;
                let (b, why) = self.truth(v, "not")?;
                Ok(Value::Bool(b.map(|x| !x), why))
            }
            Expr::If(c, t, f) => {
                let cv = self.eval(c, scope, int_semantics)?;
                let (cb, cwhy) = self.truth(cv, "if")?;
                let tv = self.eval(t, scope, int_semantics)?;
                let fv = self.eval(f, scope, int_semantics)?;
                if let (Value::Num(a), Value::Num(b)) = (&tv, &fv) {
                    if !a.dim.compatible(b.dim) {
                        return Err(EvalError::Dimension(format!(
                            "`if` branches have different dimensions ({} vs {})",
                            a.dim.render(),
                            b.dim.render()
                        )));
                    }
                }
                match cb {
                    Some(true) => Ok(tv),
                    Some(false) => Ok(fv),
                    None => Ok(match (tv, fv) {
                        (Value::Num(a), Value::Num(b)) => Value::Num(Num {
                            value: (a.value.is_some() && a.value == b.value)
                                .then_some(a.value)
                                .flatten(),
                            dim: a.dim.join(b.dim),
                            unit: None,
                            int: a.int && b.int,
                            why: merge(&cwhy, &merge(&a.why, &b.why)),
                        }),
                        (Value::Bool(a, aw), Value::Bool(b, bw)) => {
                            let v = if a.is_some() && a == b { a } else { None };
                            Value::Bool(v, merge(&cwhy, &merge(&aw, &bw)))
                        }
                        (x, y) if x == y => x,
                        _ => Value::Unknown(cwhy),
                    }),
                }
            }
            Expr::Call(name, args) => {
                let calc = self
                    .calc(name)
                    .ok_or_else(|| EvalError::Call(format!("`{name}` is not a calc def in the model")))?;
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a, scope, int_semantics)?);
                }
                self.call_calc(calc, vals)
            }
            Expr::Bin(op, l, r) => {
                let lv = self.eval(l, scope, int_semantics)?;
                // Short-circuit is not used: an error on either side is always reported.
                let rv = self.eval(r, scope, int_semantics)?;
                self.binary(*op, lv, rv, l, r)
            }
        }
    }

    /// Call `calc` with positional argument values (dimensions flow from the arguments).
    pub fn call_calc(&self, calc: &CalcDef, args: Vec<Value>) -> Result<Value, EvalError> {
        if args.len() != calc.params.len() {
            return Err(EvalError::Call(format!(
                "`{}` takes {} argument(s), got {}",
                calc.name,
                calc.params.len(),
                args.len()
            )));
        }
        {
            let mut d = self.depth.borrow_mut();
            if *d >= MAX_CALL_DEPTH {
                return Err(EvalError::Recursion(format!("calc `{}` nests too deep", calc.name)));
            }
            *d += 1;
        }
        let result = (|| {
            let mut scope = MapScope::default();
            for (p, v) in calc.params.iter().zip(args) {
                scope.0.insert(p.name.clone(), v);
            }
            for local in &calc.locals {
                let e = parse_expr(&local.expr).map_err(EvalError::Parse)?;
                let v = self.eval(&e, &scope, true)?;
                scope.0.insert(local.name.clone(), v);
            }
            let e = parse_expr(&calc.result_expr).map_err(EvalError::Parse)?;
            self.eval(&e, &scope, true)
        })();
        *self.depth.borrow_mut() -= 1;
        result
    }

    fn num(&self, v: Value, op: &str) -> Result<Num, EvalError> {
        match v {
            Value::Num(n) => Ok(n),
            Value::Unknown(why) => Ok(Num {
                value: None,
                dim: DimTag::Unknown,
                unit: None,
                int: false,
                why,
            }),
            Value::Bool(..) => Err(EvalError::Type(format!("`{op}` needs a number, got a boolean"))),
            Value::Str(_) => Err(EvalError::Type(format!("`{op}` needs a number, got a string"))),
        }
    }

    fn truth(&self, v: Value, op: &str) -> Result<(Option<bool>, Vec<String>), EvalError> {
        match v {
            Value::Bool(b, why) => Ok((b, why)),
            Value::Unknown(why) => Ok((None, why)),
            Value::Num(_) => Err(EvalError::Type(format!("`{op}` needs a boolean, got a number"))),
            Value::Str(_) => Err(EvalError::Type(format!("`{op}` needs a boolean, got a string"))),
        }
    }

    fn check_dims(&self, op: BinOp, a: &Num, b: &Num, l: &Expr, r: &Expr) -> Result<(), EvalError> {
        if !a.dim.compatible(b.dim) {
            return Err(EvalError::Dimension(format!(
                "`{}` {} `{}`: {} vs {}",
                render(l),
                op.symbol(),
                render(r),
                a.dim.render(),
                b.dim.render()
            )));
        }
        if let (Some(ua), Some(ub), DimTag::Known(_)) = (&a.unit, &b.unit, a.dim) {
            if ua.trim() != ub.trim() {
                if let (Some(sa), Some(sb)) = (base_scale(Some(ua)), base_scale(Some(ub))) {
                    if (sa - sb).abs() > f64::EPSILON * sa.abs().max(sb.abs()) {
                        return Err(EvalError::Dimension(format!(
                            "`{}` {} `{}`: units `{ua}` and `{ub}` differ in scale and values are not converted",
                            render(l),
                            op.symbol(),
                            render(r)
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    fn binary(&self, op: BinOp, lv: Value, rv: Value, l: &Expr, r: &Expr) -> Result<Value, EvalError> {
        match op {
            BinOp::And | BinOp::Or => {
                let (a, aw) = self.truth(lv, op.symbol())?;
                let (b, bw) = self.truth(rv, op.symbol())?;
                let (dominant, v) = if op == BinOp::And { (false, a.zip(b).map(|(x, y)| x && y)) } else { (true, a.zip(b).map(|(x, y)| x || y)) };
                Ok(if a == Some(dominant) || b == Some(dominant) {
                    Value::Bool(Some(dominant), Vec::new())
                } else {
                    Value::Bool(v, if v.is_some() { Vec::new() } else { merge(&aw, &bw) })
                })
            }
            BinOp::Eq | BinOp::Ne => {
                let eq = match (&lv, &rv) {
                    (Value::Str(a), Value::Str(b)) => Some(a == b),
                    (Value::Bool(a, _), Value::Bool(b, _)) => a.zip(*b).map(|(x, y)| x == y),
                    (Value::Num(_), _) | (_, Value::Num(_)) | (Value::Unknown(_), Value::Unknown(_)) => {
                        let a = self.num(lv.clone(), op.symbol())?;
                        let b = self.num(rv.clone(), op.symbol())?;
                        self.check_dims(op, &a, &b, l, r)?;
                        a.value.zip(b.value).map(|(x, y)| x == y)
                    }
                    (Value::Unknown(_), _) | (_, Value::Unknown(_)) => None,
                    _ => {
                        return Err(EvalError::Type(format!(
                            "`{}` compares values of different kinds",
                            op.symbol()
                        )))
                    }
                };
                let why = merge(&lv.reasons(), &rv.reasons());
                Ok(Value::Bool(
                    eq.map(|x| if op == BinOp::Eq { x } else { !x }),
                    if eq.is_some() { Vec::new() } else { why },
                ))
            }
            _ => {
                let a = self.num(lv, op.symbol())?;
                let b = self.num(rv, op.symbol())?;
                let why = merge(&a.why, &b.why);
                match op {
                    BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                        self.check_dims(op, &a, &b, l, r)?;
                        let v = a.value.zip(b.value).map(|(x, y)| match op {
                            BinOp::Lt => x < y,
                            BinOp::Le => x <= y,
                            BinOp::Gt => x > y,
                            _ => x >= y,
                        });
                        Ok(Value::Bool(v, if v.is_some() { Vec::new() } else { why }))
                    }
                    BinOp::Add | BinOp::Sub => {
                        self.check_dims(op, &a, &b, l, r)?;
                        let int = a.int && b.int;
                        let v = a.value.zip(b.value).map(|(x, y)| {
                            let s = if op == BinOp::Add { x + y } else { x - y };
                            if int { s.trunc() } else { s }
                        });
                        Ok(Value::Num(Num {
                            value: v,
                            dim: a.dim.join(b.dim),
                            unit: None,
                            int,
                            why: if v.is_some() { Vec::new() } else { why },
                        }))
                    }
                    _ => {
                        let int = a.int && b.int;
                        let v = a.value.zip(b.value).map(|(x, y)| {
                            if op == BinOp::Mul {
                                x * y
                            } else if int && y != 0.0 {
                                (x / y).trunc()
                            } else {
                                x / y
                            }
                        });
                        Ok(Value::Num(Num {
                            value: v,
                            dim: if op == BinOp::Mul { a.dim.mul(b.dim) } else { a.dim.div(b.dim) },
                            unit: None,
                            int,
                            why: if v.is_some() { Vec::new() } else { why },
                        }))
                    }
                }
            }
        }
    }
}

/// Render an expression back to SysML text (diagnostics; parenthesised where needed).
pub fn render(e: &Expr) -> String {
    match e {
        Expr::Num { value, int } => {
            if *int {
                format!("{value:.0}")
            } else {
                format!("{value}")
            }
        }
        Expr::Bool(b) => b.to_string(),
        Expr::Str(s) => format!("\"{s}\""),
        Expr::Ref(p) => p.join("."),
        Expr::WithUnit(x, u) => format!("{} [{u}]", render(x)),
        Expr::Neg(x) => format!("-{}", render(x)),
        Expr::Not(x) => format!("not {}", render(x)),
        Expr::Bin(op, l, r) => format!("({} {} {})", render(l), op.symbol(), render(r)),
        Expr::If(c, t, f) => format!("(if {} ? {} else {})", render(c), render(t), render(f)),
        Expr::Call(n, a) => format!("{n}({})", a.iter().map(render).collect::<Vec<_>>().join(", ")),
    }
}

// ---------------------------------------------------------------------------------------------
// IR scopes
// ---------------------------------------------------------------------------------------------

/// Scalar kind of an attribute, from its type (`Boolean`, `Integer` / `Natural` / `Positive`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrKind {
    Real,
    Int,
    Bool,
}

fn kind_of(type_name: Option<&str>) -> Option<AttrKind> {
    match simple(type_name?) {
        "Boolean" => Some(AttrKind::Bool),
        "Integer" | "Natural" | "Positive" => Some(AttrKind::Int),
        "Real" | "Rational" | "Number" => Some(AttrKind::Real),
        _ => None,
    }
}

/// One resolved attribute: its value expression (or why it has none), unit and kind.
#[derive(Debug, Clone)]
struct AttrSource {
    expr: Option<String>,
    why: &'static str,
    unit: Option<String>,
    kind: AttrKind,
}

/// Resolved attributes of one definition (own and inherited, most-derived wins).
#[derive(Debug, Clone, Default)]
struct AttrTable {
    attrs: BTreeMap<String, AttrSource>,
}

impl AttrTable {
    /// `chain`: the definition's attribute lists, most-derived first.
    fn from_chain(chain: &[&[AttributeUsage]]) -> Self {
        let mut decls: BTreeMap<String, Vec<&AttributeUsage>> = BTreeMap::new();
        for attrs in chain {
            for a in attrs.iter() {
                if let Some(k) = attr_key(a) {
                    decls.entry(k).or_default().push(a);
                }
            }
        }
        let attrs = decls
            .into_iter()
            .map(|(k, list)| {
                let unit = list.iter().find_map(|a| a.unit.clone());
                let kind = list
                    .iter()
                    .find_map(|a| kind_of(a.type_name.as_deref()))
                    .unwrap_or(AttrKind::Real);
                let decider = list
                    .iter()
                    .find(|a| a.value.is_some() || a.tbd.iter().any(|m| m.kind == TbdKind::Value))
                    .copied();
                let expr = decider.and_then(|a| a.value.as_deref()).map(|v| v.trim().to_string());
                let why = if expr.is_some() {
                    ""
                } else if decider.is_some() {
                    "tbd_value (open in the source)"
                } else if list.iter().any(|a| a.is_evidence) {
                    "evidence without a value"
                } else {
                    "no value"
                };
                (k, AttrSource { expr, why, unit, kind })
            })
            .collect();
        AttrTable { attrs }
    }
}

/// Resolves plain references against a requirement definition's attributes (own, inherited
/// from specialized requirement definitions, redefined), with optional value overrides.
pub struct RequirementScope<'m> {
    table: AttrTable,
    overrides: BTreeMap<String, Value>,
    evaluating: RefCell<BTreeSet<String>>,
    _m: std::marker::PhantomData<&'m ()>,
}

impl<'m> RequirementScope<'m> {
    pub fn new(model: &'m Model, req: &'m RequirementDef) -> Self {
        let mut index: BTreeMap<String, &RequirementDef> = BTreeMap::new();
        fn walk<'m>(pkg: &'m Package, out: &mut BTreeMap<String, &'m RequirementDef>) {
            for m in &pkg.members {
                match m {
                    PackageMember::Package(sub) => walk(sub, out),
                    PackageMember::RequirementDef(r) => {
                        out.entry(r.name.clone()).or_insert(r);
                        if let Some(s) = &r.short_name {
                            out.entry(s.clone()).or_insert(r);
                        }
                    }
                    _ => {}
                }
            }
        }
        for p in &model.packages {
            walk(p, &mut index);
        }
        let mut chain: Vec<&[AttributeUsage]> = Vec::new();
        let mut seen = BTreeSet::new();
        let mut queue = std::collections::VecDeque::from([req]);
        while let Some(r) = queue.pop_front() {
            if !seen.insert(r.name.clone()) {
                continue;
            }
            chain.push(&r.attributes);
            for s in &r.specializes {
                if let Some(p) = index.get(simple(s)) {
                    queue.push_back(p);
                }
            }
        }
        RequirementScope {
            table: AttrTable::from_chain(&chain),
            overrides: BTreeMap::new(),
            evaluating: RefCell::new(BTreeSet::new()),
            _m: std::marker::PhantomData,
        }
    }

    /// Replace an attribute's value (in its declared unit) for this evaluation.
    pub fn set(&mut self, name: &str, v: f64) {
        let v = match self.table.attrs.get(name) {
            Some(a) if a.kind == AttrKind::Bool => Value::Bool(Some(v != 0.0), Vec::new()),
            Some(a) => Value::number(v, a.unit.as_deref()),
            None => Value::number(v, None),
        };
        self.overrides.insert(name.to_string(), v);
    }

    /// Attribute names (resolved keys) of the requirement.
    pub fn attribute_names(&self) -> Vec<String> {
        self.table.attrs.keys().cloned().collect()
    }

    /// Declared unit of an attribute, if any.
    pub fn unit_of(&self, name: &str) -> Option<String> {
        self.table.attrs.get(name)?.unit.clone()
    }

    /// Scalar kind of an attribute (from its declared type).
    pub fn kind_of(&self, name: &str) -> Option<AttrKind> {
        Some(self.table.attrs.get(name)?.kind)
    }

    /// The attribute's own value (its default), when it is a plain literal.
    pub fn literal_of(&self, name: &str) -> Option<f64> {
        literal_value(self.table.attrs.get(name)?.expr.as_deref()?)
    }
}

/// The number of a literal value expression (`170.0`, `-3`, `170.0 [W]`).
pub fn literal_value(src: &str) -> Option<f64> {
    match parse_expr(src).ok()? {
        Expr::Num { value, .. } => Some(value),
        Expr::Neg(x) => match *x {
            Expr::Num { value, .. } => Some(-value),
            _ => None,
        },
        Expr::WithUnit(x, _) => match *x {
            Expr::Num { value, .. } => Some(value),
            _ => None,
        },
        _ => None,
    }
}

fn resolve_attr(
    table: &AttrTable,
    name: &str,
    scope: &dyn Scope,
    evaluating: &RefCell<BTreeSet<String>>,
    ev: &Evaluator<'_>,
) -> Option<Result<Value, EvalError>> {
    let src = table.attrs.get(name)?;
    let Some(raw) = src.expr.as_deref() else {
        let why = format!("`{name}`: {}", src.why);
        return Some(Ok(if src.kind == AttrKind::Bool {
            Value::Bool(None, vec![why])
        } else {
            Value::unknown_number(src.unit.as_deref(), why)
        }));
    };
    let unit = &src.unit;
    if !evaluating.borrow_mut().insert(name.to_string()) {
        return Some(Err(EvalError::Recursion(format!("attribute `{name}` depends on itself"))));
    }
    let r = parse_expr(raw).map_err(EvalError::Parse).and_then(|e| {
        let is_literal = matches!(&e, Expr::Num { .. } | Expr::Neg(_) | Expr::WithUnit(..));
        let v = ev.eval(&e, scope, false)?;
        Ok(match v {
            // A literal (or a wildcard result) takes the attribute's declared unit.
            Value::Num(n) if is_literal || (unit.is_some() && n.dim == DimTag::Wildcard) => {
                let u = unit.clone().or(n.unit.clone());
                Value::Num(Num {
                    dim: DimTag::of_unit(u.as_deref()),
                    unit: u,
                    int: false,
                    ..n
                })
            }
            // A derived value keeps the computed dimension, labelled with the declared unit.
            Value::Num(n) => Value::Num(Num {
                unit: unit.clone(),
                ..n
            }),
            other => other,
        })
    });
    evaluating.borrow_mut().remove(name);
    Some(r)
}

impl Scope for RequirementScope<'_> {
    fn lookup(&self, path: &[String], ev: &Evaluator<'_>) -> Option<Result<Value, EvalError>> {
        let [name] = path else { return None };
        if let Some(v) = self.overrides.get(name) {
            return Some(Ok(v.clone()));
        }
        resolve_attr(&self.table, name, self, &self.evaluating, ev)
    }
}

/// Resolves references against a part definition: plain names are its effective attributes
/// (own, inherited, redefined), dotted names walk its effective part usages
/// (`motor.ratedPower`).
pub struct PartScope<'i, 'a> {
    idx: &'i RoleIndex<'a>,
    def: &'a PartDef,
    table: AttrTable,
    evaluating: RefCell<BTreeSet<String>>,
}

impl<'i, 'a> PartScope<'i, 'a> {
    pub fn new(idx: &'i RoleIndex<'a>, def: &'a PartDef) -> Self {
        let lineage = idx.lineage(def);
        let chain: Vec<&[AttributeUsage]> = lineage.iter().map(|d| d.attributes.as_slice()).collect();
        PartScope {
            idx,
            def,
            table: AttrTable::from_chain(&chain),
            evaluating: RefCell::new(BTreeSet::new()),
        }
    }
}

impl Scope for PartScope<'_, '_> {
    fn lookup(&self, path: &[String], ev: &Evaluator<'_>) -> Option<Result<Value, EvalError>> {
        match path {
            [] => None,
            [name] => resolve_attr(&self.table, name, self, &self.evaluating, ev),
            [head, rest @ ..] => {
                let part = self
                    .idx
                    .effective_parts(self.def)
                    .into_iter()
                    .find(|p| &p.key == head)?;
                let child = self.idx.get(part.type_name.as_deref()?)?;
                PartScope::new(self.idx, child).lookup(rest, ev)
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Requirement verdicts
// ---------------------------------------------------------------------------------------------

/// Three-valued verdict of one constraint (plus `error` when it cannot be evaluated at all).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
    Unknown,
    Error,
}

/// Outcome of evaluating one constraint string.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ConstraintOutcome {
    pub constraint: String,
    pub verdict: Verdict,
    /// Why the verdict is `unknown` (operands without values) or `error` (the message).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
}

/// Evaluate one boolean constraint in `scope`.
pub fn evaluate_constraint(ev: &Evaluator<'_>, src: &str, scope: &dyn Scope) -> ConstraintOutcome {
    let (verdict, reasons) = match ev.eval_str(src, scope) {
        Ok(Value::Bool(Some(true), _)) => (Verdict::Pass, Vec::new()),
        Ok(Value::Bool(Some(false), _)) => (Verdict::Fail, Vec::new()),
        Ok(Value::Bool(None, why)) | Ok(Value::Unknown(why)) => (Verdict::Unknown, why),
        Ok(other) => (
            Verdict::Error,
            vec![format!("constraint is not boolean (got {other:?})")],
        ),
        Err(e) => (Verdict::Error, vec![e.to_string()]),
    };
    ConstraintOutcome {
        constraint: src.to_string(),
        verdict,
        reasons,
    }
}

/// Combine constraint verdicts of one requirement: any `error` -> error, any `fail` -> fail,
/// any `unknown` -> unknown, else pass (an empty list is `unknown`: nothing decides it).
pub fn combine(outcomes: &[ConstraintOutcome]) -> Verdict {
    if outcomes.is_empty() {
        return Verdict::Unknown;
    }
    if outcomes.iter().any(|o| o.verdict == Verdict::Error) {
        Verdict::Error
    } else if outcomes.iter().any(|o| o.verdict == Verdict::Fail) {
        Verdict::Fail
    } else if outcomes.iter().any(|o| o.verdict == Verdict::Unknown) {
        Verdict::Unknown
    } else {
        Verdict::Pass
    }
}

// ---------------------------------------------------------------------------------------------
// Agreement with the compiled code (test support)
// ---------------------------------------------------------------------------------------------

/// Checks that the evaluator agrees with the compiled Rust behind a model's requirements and
/// calcs (`sysml_ir::CompiledProbes`, generated by the macros). Used by the workspace's
/// agreement tests; not part of any operator.
pub mod agreement {
    use super::*;
    use sysml_ir::CompiledProbes;

    /// Summary of one agreement run.
    #[derive(Debug, Clone, Default)]
    pub struct AgreementReport {
        pub requirements_compared: usize,
        pub calcs_compared: usize,
        /// Value sets on which both sides produced a decided result and were compared.
        pub cases_compared: usize,
        /// Value sets on which the evaluator was `unknown` (defaults with evidence/TBD operands).
        pub cases_unknown: usize,
        /// Compared requirement cases by outcome (both must occur, or the sets do not
        /// discriminate).
        pub requirement_passes: usize,
        pub requirement_fails: usize,
        /// Requirement definitions in the model with constraints but no compiled probe
        /// (string-literal constraints, or requirements from other crates).
        pub requirements_without_probe: Vec<String>,
        pub disagreements: Vec<String>,
        /// Probes whose requirement / calc definition is not in the model.
        pub unmatched_probes: Vec<String>,
    }

    /// Deterministic xorshift generator (no external crates).
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    fn all_requirements(model: &Model) -> Vec<&RequirementDef> {
        fn walk<'m>(pkg: &'m Package, out: &mut Vec<&'m RequirementDef>) {
            for m in &pkg.members {
                match m {
                    PackageMember::Package(sub) => walk(sub, out),
                    PackageMember::RequirementDef(r) => out.push(r),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        for p in &model.packages {
            walk(p, &mut out);
        }
        out
    }

    /// Compare every probe on its defaults plus `random_sets` perturbed value sets (and a
    /// boundary set where unvalued attributes equal a same-unit limit).
    pub fn check(model: &Model, probes: &CompiledProbes, random_sets: usize) -> AgreementReport {
        let ev = Evaluator::new(model);
        let reqs = all_requirements(model);
        let mut rep = AgreementReport::default();
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);

        for probe in &probes.requirements {
            let Some(def) = reqs.iter().find(|r| {
                r.name == probe.name
                    && probe.constraints.iter().all(|c| r.require_constraints.iter().any(|x| x == c))
            }) else {
                rep.unmatched_probes.push(format!("requirement {}", probe.name));
                continue;
            };
            rep.requirements_compared += 1;
            let base = RequirementScope::new(model, def);
            let names = base.attribute_names();
            let defaults: Vec<(String, Option<f64>)> =
                names.iter().map(|n| (n.clone(), base.literal_of(n))).collect();
            // Base magnitude for an attribute without a default: a same-unit default, else the
            // mean magnitude of the requirement's defaults, else 1.
            let mean = {
                let v: Vec<f64> = defaults.iter().filter_map(|(_, d)| d.map(f64::abs)).collect();
                if v.is_empty() { 1.0 } else { (v.iter().sum::<f64>() / v.len() as f64).max(1e-9) }
            };
            let base_of = |n: &str| -> f64 {
                if let Some(d) = base.literal_of(n) {
                    return d;
                }
                let u = base.unit_of(n);
                defaults
                    .iter()
                    .find(|(m, d)| d.is_some() && base.unit_of(m) == u)
                    .and_then(|(_, d)| *d)
                    .unwrap_or(mean)
            };

            // Value sets: None = defaults; Some(map) = every attribute assigned.
            let mut sets: Vec<Option<BTreeMap<String, f64>>> = vec![None];
            let boundary: BTreeMap<String, f64> = names
                .iter()
                .map(|n| {
                    let v = match base.kind_of(n) {
                        Some(AttrKind::Bool) => 1.0,
                        Some(AttrKind::Int) => base_of(n).round(),
                        _ => base_of(n),
                    };
                    (n.clone(), v)
                })
                .collect();
            sets.push(Some(boundary));
            for _ in 0..random_sets {
                let set = names
                    .iter()
                    .map(|n| {
                        let r = rng.next();
                        let v = match base.kind_of(n) {
                            Some(AttrKind::Bool) => (r < 0.5) as u8 as f64,
                            Some(AttrKind::Int) => (base_of(n) * 2.0 * r).round(),
                            _ => base_of(n) * (0.25 + 1.5 * r),
                        };
                        (n.clone(), v)
                    })
                    .collect();
                sets.push(Some(set));
            }

            for set in sets {
                let mut scope = RequirementScope::new(model, def);
                if let Some(m) = &set {
                    for (k, v) in m {
                        scope.set(k, *v);
                    }
                }
                let outcomes: Vec<ConstraintOutcome> = probe
                    .constraints
                    .iter()
                    .map(|c| evaluate_constraint(&ev, c, &scope))
                    .collect();
                let verdict = combine(&outcomes);
                let lookup = |name: &str| set.as_ref().and_then(|m| m.get(name).copied());
                let compiled = (probe.eval)(&lookup);
                match verdict {
                    Verdict::Pass | Verdict::Fail => {
                        rep.cases_compared += 1;
                        if verdict == Verdict::Pass {
                            rep.requirement_passes += 1;
                        } else {
                            rep.requirement_fails += 1;
                        }
                        if compiled != (verdict == Verdict::Pass) {
                            rep.disagreements.push(format!(
                                "requirement {}: IR {verdict:?}, compiled {compiled} on {}",
                                probe.name,
                                set.as_ref().map(|m| format!("{m:?}")).unwrap_or_else(|| "defaults".into())
                            ));
                        }
                    }
                    Verdict::Unknown if set.is_none() => rep.cases_unknown += 1,
                    _ => rep.disagreements.push(format!(
                        "requirement {}: IR {verdict:?} ({:?}) on {}",
                        probe.name,
                        outcomes.iter().flat_map(|o| o.reasons.clone()).collect::<Vec<_>>(),
                        set.as_ref().map(|m| format!("{m:?}")).unwrap_or_else(|| "defaults".into())
                    )),
                }
            }
        }

        for r in &reqs {
            if !r.require_constraints.is_empty()
                && !probes.requirements.iter().any(|p| p.name == r.name)
            {
                rep.requirements_without_probe.push(r.name.clone());
            }
        }

        for probe in &probes.calcs {
            let Some(calc) = ev.calc(probe.name) else {
                rep.unmatched_probes.push(format!("calc {}", probe.name));
                continue;
            };
            if calc.params.len() != probe.arity {
                rep.disagreements.push(format!(
                    "calc {}: IR has {} params, compiled {}",
                    probe.name,
                    calc.params.len(),
                    probe.arity
                ));
                continue;
            }
            rep.calcs_compared += 1;
            let mut arg_sets: Vec<Vec<f64>> = vec![vec![1.0; probe.arity], vec![2.5; probe.arity]];
            for k in 0..random_sets.max(1) {
                arg_sets.push(
                    (0..probe.arity)
                        .map(|_| {
                            let r = rng.next();
                            if k % 4 == 3 { 200.0 * r - 100.0 } else { 0.1 + 99.9 * r }
                        })
                        .collect(),
                );
            }
            for args in arg_sets {
                let compiled = (probe.eval)(&args);
                let vals = args.iter().map(|a| Value::number(*a, None)).collect();
                match ev.call_calc(calc, vals) {
                    Ok(v) => match v.as_f64() {
                        Some(ir) => {
                            rep.cases_compared += 1;
                            let same = (ir.is_nan() && compiled.is_nan())
                                || ir == compiled
                                || (ir - compiled).abs() <= 1e-9 * ir.abs().max(compiled.abs());
                            if !same {
                                rep.disagreements.push(format!(
                                    "calc {}({args:?}): IR {ir}, compiled {compiled}",
                                    probe.name
                                ));
                            }
                        }
                        None => rep.disagreements.push(format!(
                            "calc {}({args:?}): IR gave no number ({v:?})",
                            probe.name
                        )),
                    },
                    Err(e) => rep
                        .disagreements
                        .push(format!("calc {}({args:?}): IR error {e}", probe.name)),
                }
            }
        }
        rep
    }

    /// Every expression string the IR stores for `model`'s own and library packages:
    /// requirement require/assume constraints, calc locals and results, transition guards,
    /// analysis/portion assert constraints and non-literal attribute values. Returns
    /// (location, expression) pairs.
    pub fn expression_strings(model: &Model) -> Vec<(String, String)> {
        fn attrs(loc: &str, list: &[AttributeUsage], out: &mut Vec<(String, String)>) {
            for a in list {
                if let Some(v) = a.value.as_deref() {
                    if literal_value(v).is_none() {
                        let k = attr_key(a).unwrap_or_default();
                        out.push((format!("{loc}.{k} (value)"), v.trim().to_string()));
                    }
                }
                attrs(loc, &a.nested_attributes, out);
            }
        }
        fn walk(pkg: &Package, out: &mut Vec<(String, String)>) {
            for m in &pkg.members {
                match m {
                    PackageMember::Package(sub) => walk(sub, out),
                    PackageMember::RequirementDef(r) => {
                        for c in r.require_constraints.iter().chain(&r.assume_constraints) {
                            out.push((format!("requirement {}", r.name), c.clone()));
                        }
                        attrs(&r.name, &r.attributes, out);
                    }
                    PackageMember::CalcDef(c) | PackageMember::CalcUsage(c) => {
                        for l in &c.locals {
                            out.push((format!("calc {}.{}", c.name, l.name), l.expr.clone()));
                        }
                        if !c.result_expr.trim().is_empty() {
                            out.push((format!("calc {} result", c.name), c.result_expr.clone()));
                        }
                    }
                    PackageMember::StateDef(sd) => {
                        for t in &sd.transitions {
                            if let Some(g) = &t.guard {
                                out.push((format!("state {} guard", sd.name), g.clone()));
                            }
                        }
                    }
                    PackageMember::AnalysisDef(a) => {
                        for c in &a.assert_constraints {
                            out.push((format!("analysis {}", a.name), c.clone()));
                        }
                    }
                    PackageMember::PartDef(p) => attrs(&p.name, &p.attributes, out),
                    PackageMember::ItemDef(i) => attrs(&i.name, &i.attributes, out),
                    _ => {}
                }
            }
        }
        let mut out = Vec::new();
        for p in &model.packages {
            walk(p, &mut out);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: f64, u: &str) -> Value {
        Value::number(v, if u.is_empty() { None } else { Some(u) })
    }

    fn scope(pairs: &[(&str, Value)]) -> MapScope {
        MapScope(pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect())
    }

    #[test]
    fn parses_the_transpiler_subset() {
        for src in [
            "measuredAirflowM3s >= minAirflowM3s",
            "a + b * c - d / e <= 3.5E-2",
            "not (a < b) and c or not d",
            "if x > 0 ? x else -x",
            "if x > 0 then x else 0.0",
            "-a * -2",
            "batteryRuntimeS(energyJ, loadPowerW) >= minRuntimeS",
            "payload.pressureDrop > target.alarmTriggerPa",
            "x == true",
            "name != \"idle\"",
            "'LM-5'.mass > 0",
            "170.0 [W]",
            "(a)",
        ] {
            parse_expr(src).unwrap_or_else(|e| panic!("{src}: {e}"));
        }
        assert!(parse_expr("a +").is_err());
        assert!(parse_expr("a b").is_err());
        assert!(parse_expr("f(a,").is_err());
    }

    #[test]
    fn precedence_matches_rust_for_transpiled_strings() {
        let ev = Evaluator::empty();
        let s = scope(&[]);
        let eval = |src: &str| ev.eval_str(src, &s).unwrap();
        assert_eq!(eval("1 + 2 * 3").as_f64(), Some(7.0));
        assert_eq!(eval("(1 + 2) * 3").as_f64(), Some(9.0));
        assert_eq!(eval("10 - 4 - 3").as_f64(), Some(3.0));
        assert_eq!(eval("8 / 4 / 2").as_f64(), Some(1.0));
        assert_eq!(eval("-2 * 3").as_f64(), Some(-6.0));
        assert_eq!(eval("true or false and false").truth(), Some(true));
        assert_eq!(eval("not false and false").truth(), Some(false));
        assert_eq!(eval("if 1 < 2 ? 10 else 20").as_f64(), Some(10.0));
        assert_eq!(eval("1 + if 1 > 2 ? 10 else 20").as_f64(), Some(21.0));
        // Requirement semantics are f64; calc bodies use Rust integer division for int literals.
        assert_eq!(eval("1 / 2").as_f64(), Some(0.5));
    }

    #[test]
    fn dimensions_are_checked_and_carried() {
        let ev = Evaluator::empty();
        let s = scope(&[
            ("p", n(100.0, "W")),
            ("t", n(2.0, "s")),
            ("e", n(150.0, "J")),
            ("m", n(80.0, "g")),
            ("mMax", n(0.5, "kg")),
            ("x", n(1.0, "")),
        ]);
        assert_eq!(ev.eval_str("p * t <= e", &s).unwrap().truth(), Some(false));
        assert_eq!(ev.eval_str("p <= 120", &s).unwrap().truth(), Some(true));
        assert_eq!(ev.eval_str("x <= p", &s).unwrap().truth(), Some(true));
        assert!(matches!(ev.eval_str("p <= t", &s), Err(EvalError::Dimension(_))));
        assert!(matches!(ev.eval_str("p + e", &s), Err(EvalError::Dimension(_))));
        assert!(matches!(ev.eval_str("if x > 0 ? p else t", &s), Err(EvalError::Dimension(_))));
        // Same dimension, different known scale: not silently compared.
        assert!(matches!(ev.eval_str("m <= mMax", &s), Err(EvalError::Dimension(_))));
    }

    #[test]
    fn unknowns_are_three_valued_never_guessed() {
        let ev = Evaluator::empty();
        let s = scope(&[
            ("measured", Value::unknown_number(Some("W"), "evidence without a value")),
            ("limit", n(100.0, "W")),
            ("flag", Value::Bool(Some(false), vec![])),
        ]);
        let v = ev.eval_str("measured <= limit", &s).unwrap();
        assert_eq!(v.truth(), None);
        assert!(v.reasons()[0].contains("evidence"));
        // Kleene: false and unknown = false; true or unknown = true.
        assert_eq!(ev.eval_str("flag and measured <= limit", &s).unwrap().truth(), Some(false));
        assert_eq!(ev.eval_str("not flag or measured <= limit", &s).unwrap().truth(), Some(true));
        // A dimension error is reported even when the value is unknown.
        assert!(matches!(ev.eval_str("measured <= 3 [s]", &s), Err(EvalError::Dimension(_))));
        assert!(matches!(ev.eval_str("missing > 0", &s), Err(EvalError::Unresolved(_))));
    }

    #[test]
    fn calcs_are_called_with_locals_nesting_and_int_division() {
        let model = Model::new(vec![Package {
            name: "P".into(),
            members: vec![
                PackageMember::CalcDef(CalcDef {
                    name: "runtime".into(),
                    params: vec![
                        sysml_ir::CalcParam { name: "energy".into(), type_name: "Real".into(), ..Default::default() },
                        sysml_ir::CalcParam { name: "power".into(), type_name: "Real".into(), ..Default::default() },
                    ],
                    locals: vec![sysml_ir::CalcLocal { name: "r".into(), expr: "energy / power".into() }],
                    return_name: "result".into(),
                    return_type: "Real".into(),
                    result_expr: "r".into(),
                    ..Default::default()
                }),
                PackageMember::CalcDef(CalcDef {
                    name: "twice".into(),
                    params: vec![sysml_ir::CalcParam { name: "x".into(), type_name: "Real".into(), ..Default::default() }],
                    result_expr: "runtime(x, 1) * 2 + 1 / 2".into(),
                    ..Default::default()
                }),
            ],
            ..Default::default()
        }]);
        let ev = Evaluator::new(&model);
        let s = scope(&[("e", n(3600.0, "J")), ("p", n(100.0, "W")), ("tMin", n(30.0, "s"))]);
        assert_eq!(ev.eval_str("runtime(e, p) >= tMin", &s).unwrap().truth(), Some(true));
        // The calc result carries J / W = s; comparing it with W is a dimension error.
        assert!(matches!(ev.eval_str("runtime(e, p) >= p", &s), Err(EvalError::Dimension(_))));
        // `1 / 2` inside a calc body is Rust integer division.
        assert_eq!(ev.eval_str("twice(5.0)", &s).unwrap().as_f64(), Some(10.0));
        assert!(matches!(ev.eval_str("runtime(e)", &s), Err(EvalError::Call(_))));
        assert!(matches!(ev.eval_str("nope(e)", &s), Err(EvalError::Call(_))));
    }

    #[test]
    fn requirement_scope_resolves_inherited_redefined_and_tbd() {
        let base = RequirementDef {
            name: "Base".into(),
            attributes: vec![AttributeUsage {
                name: Some("limit".into()),
                value: Some("100.0".into()),
                unit: Some("W".into()),
                ..Default::default()
            }],
            ..Default::default()
        };
        let derived = RequirementDef {
            name: "Derived".into(),
            specializes: vec!["Base".into()],
            attributes: vec![
                AttributeUsage {
                    redefines: vec!["limit".into()],
                    value: Some("80.0".into()),
                    ..Default::default()
                },
                AttributeUsage {
                    name: Some("measured".into()),
                    unit: Some("W".into()),
                    is_evidence: true,
                    ..Default::default()
                },
                AttributeUsage {
                    name: Some("margin".into()),
                    value: Some("limit - 10".into()),
                    unit: Some("W".into()),
                    ..Default::default()
                },
            ],
            require_constraints: vec!["measured <= limit".into()],
            ..Default::default()
        };
        let model = Model::new(vec![Package {
            name: "R".into(),
            members: vec![PackageMember::RequirementDef(base), PackageMember::RequirementDef(derived)],
            ..Default::default()
        }]);
        let req = match &model.packages[0].members[1] {
            PackageMember::RequirementDef(r) => r,
            _ => unreachable!(),
        };
        let ev = Evaluator::new(&model);
        let mut scope = RequirementScope::new(&model, req);
        let o = evaluate_constraint(&ev, "measured <= limit", &scope);
        assert_eq!(o.verdict, Verdict::Unknown);
        assert!(o.reasons[0].contains("evidence"));
        assert_eq!(scope.unit_of("limit").as_deref(), Some("W"));
        assert_eq!(evaluate_constraint(&ev, "margin == 70", &scope).verdict, Verdict::Pass);
        scope.set("measured", 90.0);
        assert_eq!(evaluate_constraint(&ev, "measured <= limit", &scope).verdict, Verdict::Fail);
        scope.set("measured", 80.0);
        assert_eq!(evaluate_constraint(&ev, "measured <= limit", &scope).verdict, Verdict::Pass);
        assert_eq!(evaluate_constraint(&ev, "measured <= 3 [s]", &scope).verdict, Verdict::Error);
    }
}
