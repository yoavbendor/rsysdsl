//! Closed SI unit table and `const fn` dimensional analysis (`#![no_std]`, zero-alloc).
//!
//! Every unit symbol accepted by `#[sysml::attr(unit = "...")]` is either an *atom* from
//! [`UNITS`] or a product/quotient of atoms built with `*`, `/`, `^<int>` and parentheses
//! (e.g. `"N*m"`, `"m^3/s"`, `"W/(m^2*K)"`, `"1/h"`). Each atom maps to its base-dimension
//! exponents over the seven SI base quantities `[L, M, T, I, Θ, N, J]`
//! (length, mass, time, electric current, thermodynamic temperature, amount of substance,
//! luminous intensity). Dimensionless units (`"1"`, `"%"`, `"dB"`, `"rad"`, ...) have all-zero
//! exponents.
//!
//! The same table is used by `sysml-macros` at expansion time (unknown unit symbols and
//! dimensionally inconsistent `#[sysml::require_constraint]` expressions are compile errors) and
//! by generated `const` assertions for `#[sysml::transition(guard = ...)]`, whose operand units
//! live on other `#[sysml::item]` / `#[sysml::part]` types.

/// Base-dimension exponents `[L, M, T, I, Θ, N, J]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dim(pub [i8; 7]);

impl Dim {
    pub const ONE: Dim = Dim([0; 7]);

    pub const fn mul(self, o: Dim) -> Dim {
        let mut out = [0i8; 7];
        let mut i = 0;
        while i < 7 {
            out[i] = self.0[i] + o.0[i];
            i += 1;
        }
        Dim(out)
    }

    pub const fn div(self, o: Dim) -> Dim {
        self.mul(o.powi(-1))
    }

    pub const fn powi(self, p: i8) -> Dim {
        let mut out = [0i8; 7];
        let mut i = 0;
        while i < 7 {
            out[i] = self.0[i] * p;
            i += 1;
        }
        Dim(out)
    }

    pub const fn eq(self, o: Dim) -> bool {
        let mut i = 0;
        while i < 7 {
            if self.0[i] != o.0[i] {
                return false;
            }
            i += 1;
        }
        true
    }

    pub const fn is_dimensionless(self) -> bool {
        self.eq(Dim::ONE)
    }
}

const fn d(l: i8, m: i8, t: i8, i: i8, th: i8, n: i8, j: i8) -> Dim {
    Dim([l, m, t, i, th, n, j])
}

/// One entry of the closed unit table.
#[derive(Debug, Clone, Copy)]
pub struct UnitDef {
    pub symbol: &'static str,
    pub dim: Dim,
    pub name: &'static str,
    /// `""` when the symbol resolves in the OMG `SI` standard library (accepted by
    /// `#[sysml::attr(unit = ...)]`); otherwise the accepted OMG spelling to use instead.
    pub use_instead: &'static str,
}

impl UnitDef {
    /// `true` if the symbol resolves in the OMG standard library (`private import SI::*;`).
    pub const fn is_omg(&self) -> bool {
        self.use_instead.is_empty()
    }
}

const fn u(symbol: &'static str, dim: Dim, name: &'static str) -> UnitDef {
    UnitDef {
        symbol,
        dim,
        name,
        use_instead: "",
    }
}

/// A symbol that is dimensionally understood but NOT defined in the OMG standard library
/// (emitting it would fail `rsysdsl audit --stdlib --strict` with an unresolved reference).
const fn x(
    symbol: &'static str,
    dim: Dim,
    name: &'static str,
    use_instead: &'static str,
) -> UnitDef {
    UnitDef {
        symbol,
        dim,
        name,
        use_instead,
    }
}

const ONE: Dim = Dim::ONE;
const LEN: Dim = d(1, 0, 0, 0, 0, 0, 0);
const MASS: Dim = d(0, 1, 0, 0, 0, 0, 0);
const TIME: Dim = d(0, 0, 1, 0, 0, 0, 0);
const CURR: Dim = d(0, 0, 0, 1, 0, 0, 0);
const TEMP: Dim = d(0, 0, 0, 0, 1, 0, 0);
const AMT: Dim = d(0, 0, 0, 0, 0, 1, 0);
const LUM: Dim = d(0, 0, 0, 0, 0, 0, 1);
const AREA: Dim = d(2, 0, 0, 0, 0, 0, 0);
const VOL: Dim = d(3, 0, 0, 0, 0, 0, 0);
const FREQ: Dim = d(0, 0, -1, 0, 0, 0, 0);
const FORCE: Dim = d(1, 1, -2, 0, 0, 0, 0);
const PRESS: Dim = d(-1, 1, -2, 0, 0, 0, 0);
const ENERGY: Dim = d(2, 1, -2, 0, 0, 0, 0);
const POWER: Dim = d(2, 1, -3, 0, 0, 0, 0);
const CHARGE: Dim = d(0, 0, 1, 1, 0, 0, 0);
const VOLT: Dim = d(2, 1, -3, -1, 0, 0, 0);
const OHM: Dim = d(2, 1, -3, -2, 0, 0, 0);
const SIEMENS: Dim = d(-2, -1, 3, 2, 0, 0, 0);
const FARAD: Dim = d(-2, -1, 4, 2, 0, 0, 0);
const HENRY: Dim = d(2, 1, -2, -2, 0, 0, 0);
const WEBER: Dim = d(2, 1, -2, -1, 0, 0, 0);
const TESLA: Dim = d(0, 1, -2, -1, 0, 0, 0);
const LUX: Dim = d(-2, 0, 0, 0, 0, 0, 1);

/// The closed table of atomic unit symbols (SI base, SI derived, prefixed forms and a few
/// accepted non-SI units). Composite units are formed from these with `*`, `/`, `^`, `( )`.
///
/// Entries built with `u(...)` resolve in the OMG `SI` library and are accepted by
/// `#[sysml::attr(unit = ...)]`; entries built with `x(...)` are known (so dimension checks and
/// diagnostics understand them) but rejected with their `use_instead` spelling, because the OMG
/// library does not define them (e.g. `kN`, `ms`, `%`).
pub const UNITS: &[UnitDef] = &[
    // Dimensionless
    u("1", ONE, "one"),
    x(
        "%",
        ONE,
        "percent",
        "1 (omit `unit`; store a 0..1 fraction or name the field `*_pct`)",
    ),
    x(
        "ppm",
        ONE,
        "parts per million",
        "1 (omit `unit` and document the scale)",
    ),
    u("dB", ONE, "decibel"),
    u("rad", ONE, "radian"),
    x("deg", ONE, "degree (angle)", "°"),
    u("°", ONE, "degree (angle)"),
    u("sr", ONE, "steradian"),
    // Length
    u("m", LEN, "metre"),
    u("km", LEN, "kilometre"),
    u("cm", LEN, "centimetre"),
    u("mm", LEN, "millimetre"),
    x("μm", LEN, "micrometre", "m"),
    x("um", LEN, "micrometre", "m"),
    u("nm", LEN, "nanometre"),
    x("au", LEN, "astronomical unit", "m"),
    // Area / volume
    x("ha", AREA, "hectare", "m^2"),
    u("L", VOL, "litre"),
    x("l", VOL, "litre", "L"),
    u("mL", VOL, "millilitre"),
    // Mass
    u("kg", MASS, "kilogram"),
    u("g", MASS, "gram"),
    x("mg", MASS, "milligram", "g"),
    x("t", MASS, "tonne", "kg"),
    // Time
    u("s", TIME, "second"),
    x("ms", TIME, "millisecond", "s"),
    x("μs", TIME, "microsecond", "s"),
    x("us", TIME, "microsecond", "s"),
    x("ns", TIME, "nanosecond", "s"),
    u("min", TIME, "minute"),
    u("h", TIME, "hour"),
    u("d", TIME, "day"),
    // Electric current
    u("A", CURR, "ampere"),
    x("mA", CURR, "milliampere", "A"),
    x("kA", CURR, "kiloampere", "A"),
    // Temperature
    u("K", TEMP, "kelvin"),
    u("°C", TEMP, "degree Celsius"),
    x("degC", TEMP, "degree Celsius", "°C"),
    // Amount of substance / luminous intensity
    u("mol", AMT, "mole"),
    u("cd", LUM, "candela"),
    u("lm", LUM, "lumen"),
    u("lx", LUX, "lux"),
    // Frequency / rotation rate
    u("Hz", FREQ, "hertz"),
    x("kHz", FREQ, "kilohertz", "Hz"),
    x("MHz", FREQ, "megahertz", "Hz"),
    x("GHz", FREQ, "gigahertz", "Hz"),
    x("rpm", FREQ, "revolutions per minute", "1/min"),
    // Force / pressure
    u("N", FORCE, "newton"),
    x("kN", FORCE, "kilonewton", "N"),
    x("MN", FORCE, "meganewton", "N"),
    u("Pa", PRESS, "pascal"),
    x("kPa", PRESS, "kilopascal", "Pa"),
    x("MPa", PRESS, "megapascal", "Pa"),
    x("bar", PRESS, "bar", "Pa"),
    // Energy / power
    u("J", ENERGY, "joule"),
    u("kJ", ENERGY, "kilojoule"),
    u("MJ", ENERGY, "megajoule"),
    x("Wh", ENERGY, "watt hour", "J"),
    x("kWh", ENERGY, "kilowatt hour", "J"),
    u("eV", ENERGY, "electronvolt"),
    u("W", POWER, "watt"),
    x("mW", POWER, "milliwatt", "W"),
    u("kW", POWER, "kilowatt"),
    x("MW", POWER, "megawatt", "W"),
    // Electromagnetism
    u("C", CHARGE, "coulomb"),
    x("Ah", CHARGE, "ampere hour", "C"),
    x("mAh", CHARGE, "milliampere hour", "C"),
    u("V", VOLT, "volt"),
    x("mV", VOLT, "millivolt", "V"),
    x("kV", VOLT, "kilovolt", "V"),
    u("Ω", OHM, "ohm"),
    x("ohm", OHM, "ohm", "Ω"),
    u("S", SIEMENS, "siemens"),
    u("F", FARAD, "farad"),
    u("H", HENRY, "henry"),
    u("Wb", WEBER, "weber"),
    u("T", TESLA, "tesla"),
];

/// Common composite spellings offered as "did you mean" suggestions (all parse via [`parse_unit`]).
pub const COMMON_COMPOSITES: &[&str] = &[
    "m/s", "m/s^2", "rad/s", "rad/s^2", "kg/s", "m^2", "m^3", "m^3/s", "kg/m^3", "N*m", "W/K",
    "W/m^2", "J/K", "J/kg", "J/(kg*K)", "W/(m*K)", "1/s", "1/h", "km/h", "L/min", "V/m", "A/m",
];

const fn bytes_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Look up an atomic unit symbol in the closed [`UNITS`] table.
pub const fn lookup_atom(sym: &[u8]) -> Option<&'static UnitDef> {
    let mut i = 0;
    while i < UNITS.len() {
        if bytes_eq(UNITS[i].symbol.as_bytes(), sym) {
            return Some(&UNITS[i]);
        }
        i += 1;
    }
    None
}

const fn is_delim(b: u8) -> bool {
    matches!(b, b'*' | b'/' | b'^' | b'(' | b')' | b' ')
}

const fn skip_ws(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && s[i] == b' ' {
        i += 1;
    }
    i
}

/// Parse result: `Ok((dim, next_index))` or `Err(())`.
type PRes = Result<(Dim, usize), ()>;

const fn parse_product(s: &[u8], i: usize) -> PRes {
    let (mut acc, mut i) = match parse_factor(s, i) {
        Ok(v) => v,
        Err(()) => return Err(()),
    };
    loop {
        i = skip_ws(s, i);
        if i >= s.len() || (s[i] != b'*' && s[i] != b'/') {
            return Ok((acc, i));
        }
        let is_div = s[i] == b'/';
        let (rhs, next) = match parse_factor(s, i + 1) {
            Ok(v) => v,
            Err(()) => return Err(()),
        };
        acc = if is_div { acc.div(rhs) } else { acc.mul(rhs) };
        i = next;
    }
}

const fn parse_factor(s: &[u8], i: usize) -> PRes {
    let mut i = skip_ws(s, i);
    if i >= s.len() {
        return Err(());
    }
    let base = if s[i] == b'(' {
        let (inner, next) = match parse_product(s, i + 1) {
            Ok(v) => v,
            Err(()) => return Err(()),
        };
        let next = skip_ws(s, next);
        if next >= s.len() || s[next] != b')' {
            return Err(());
        }
        i = next + 1;
        inner
    } else {
        let start = i;
        while i < s.len() && !is_delim(s[i]) {
            i += 1;
        }
        if i == start {
            return Err(());
        }
        let (_, tail) = s.split_at(start);
        let (atom, _) = tail.split_at(i - start);
        match lookup_atom(atom) {
            Some(def) => def.dim,
            None => return Err(()),
        }
    };
    let j = skip_ws(s, i);
    if j < s.len() && s[j] == b'^' {
        let mut k = skip_ws(s, j + 1);
        let neg = k < s.len() && s[k] == b'-';
        if neg {
            k += 1;
        }
        let start = k;
        let mut p: i32 = 0;
        while k < s.len() && s[k].is_ascii_digit() {
            p = p * 10 + (s[k] - b'0') as i32;
            if p > 16 {
                return Err(());
            }
            k += 1;
        }
        if k == start {
            return Err(());
        }
        let p = if neg { -(p as i8) } else { p as i8 };
        return Ok((base.powi(p), k));
    }
    Ok((base, i))
}

/// Parse a unit symbol (atom or composite) into its base dimension. `None` = unknown unit.
pub const fn parse_unit(unit: &str) -> Option<Dim> {
    let s = unit.as_bytes();
    match parse_product(s, 0) {
        Ok((dim, i)) => {
            if skip_ws(s, i) == s.len() {
                Some(dim)
            } else {
                None
            }
        }
        Err(()) => None,
    }
}

/// Outcome of checking a unit symbol for use in `#[sysml::attr(unit = ...)]`.
#[derive(Debug, Clone, Copy)]
pub enum UnitCheck {
    /// Well-formed and every atom resolves in the OMG standard library.
    Ok(Dim),
    /// Not a well-formed product/quotient of known atoms.
    Unknown,
    /// A bare `"1"`: dimensionless quantities carry no unit in SysML v2.
    BareOne,
    /// Well-formed but uses an atom absent from the OMG library.
    NotInOmg(&'static UnitDef),
}

/// Check a unit symbol against the closed table *and* OMG standard-library resolvability.
pub const fn check_unit(unit: &str) -> UnitCheck {
    let dim = match parse_unit(unit) {
        Some(d) => d,
        None => return UnitCheck::Unknown,
    };
    let s = unit.as_bytes();
    let mut i = 0;
    let mut only_one = true;
    while i < s.len() {
        let c = s[i];
        if c == b'^' {
            i = skip_ws(s, i + 1);
            if i < s.len() && s[i] == b'-' {
                i += 1;
            }
            while i < s.len() && s[i].is_ascii_digit() {
                i += 1;
            }
            continue;
        }
        if is_delim(c) {
            i += 1;
            continue;
        }
        let start = i;
        while i < s.len() && !is_delim(s[i]) {
            i += 1;
        }
        let (_, tail) = s.split_at(start);
        let (atom, _) = tail.split_at(i - start);
        if !bytes_eq(atom, b"1") {
            only_one = false;
        }
        if let Some(def) = lookup_atom(atom) {
            if !def.is_omg() {
                return UnitCheck::NotInOmg(def);
            }
        }
    }
    if only_one {
        return UnitCheck::BareOne;
    }
    UnitCheck::Ok(dim)
}

/// Returns `true` if `unit` is a known atom or a well-formed composite of known atoms.
pub const fn is_known_unit(unit: &str) -> bool {
    parse_unit(unit).is_some()
}

// ---------------------------------------------------------------------------------------------
// Dimension values for expression checking
// ---------------------------------------------------------------------------------------------

/// Kind of a [`DimVal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimKind {
    /// Operand has no (or an unparseable) unit: dimension checks involving it are skipped.
    Unknown,
    /// Numeric literal: compatible with any dimension (`x <= 100.0`, `2.0 * x`).
    Wildcard,
    /// Operand has a known dimension.
    Known,
}

/// Dimension of one (sub)expression, plus the unit label of a direct field reference.
#[derive(Debug, Clone, Copy)]
pub struct DimVal {
    pub kind: DimKind,
    pub dim: Dim,
    /// Unit symbol as written on the field (empty for compound expressions / literals).
    pub label: &'static str,
}

impl DimVal {
    pub const UNKNOWN: DimVal = DimVal {
        kind: DimKind::Unknown,
        dim: Dim::ONE,
        label: "",
    };
    pub const WILDCARD: DimVal = DimVal {
        kind: DimKind::Wildcard,
        dim: Dim::ONE,
        label: "",
    };

    /// Dimension of a field whose `#[sysml::attr(unit = ...)]` is `unit` (`""` = no unit).
    pub const fn from_unit(unit: &'static str) -> DimVal {
        if unit.is_empty() {
            return DimVal::UNKNOWN;
        }
        match parse_unit(unit) {
            Some(dim) => DimVal {
                kind: DimKind::Known,
                dim,
                label: unit,
            },
            None => DimVal::UNKNOWN,
        }
    }

    pub const fn known(dim: Dim) -> DimVal {
        DimVal {
            kind: DimKind::Known,
            dim,
            label: "",
        }
    }

    /// `a * b`
    pub const fn mul(self, o: DimVal) -> DimVal {
        match (self.kind, o.kind) {
            (DimKind::Unknown, _) | (_, DimKind::Unknown) => DimVal::UNKNOWN,
            (DimKind::Wildcard, DimKind::Wildcard) => DimVal::WILDCARD,
            (DimKind::Wildcard, DimKind::Known) => DimVal::known(o.dim),
            (DimKind::Known, DimKind::Wildcard) => DimVal::known(self.dim),
            (DimKind::Known, DimKind::Known) => DimVal::known(self.dim.mul(o.dim)),
        }
    }

    /// `a / b`
    pub const fn div(self, o: DimVal) -> DimVal {
        match (self.kind, o.kind) {
            (DimKind::Unknown, _) | (_, DimKind::Unknown) => DimVal::UNKNOWN,
            (DimKind::Wildcard, DimKind::Wildcard) => DimVal::WILDCARD,
            (DimKind::Wildcard, DimKind::Known) => DimVal::known(o.dim.powi(-1)),
            (DimKind::Known, DimKind::Wildcard) => DimVal::known(self.dim),
            (DimKind::Known, DimKind::Known) => DimVal::known(self.dim.div(o.dim)),
        }
    }

    /// `a + b`, `a - b`, `if c { a } else { b }` (assumes [`compatible`] was checked).
    pub const fn add(self, o: DimVal) -> DimVal {
        match (self.kind, o.kind) {
            (DimKind::Unknown, _) | (_, DimKind::Unknown) => DimVal::UNKNOWN,
            (DimKind::Known, _) => DimVal::known(self.dim),
            (_, DimKind::Known) => DimVal::known(o.dim),
            _ => DimVal::WILDCARD,
        }
    }
}

/// `true` unless both sides have a known dimension and the dimensions differ.
pub const fn compatible(a: DimVal, b: DimVal) -> bool {
    !(matches!(a.kind, DimKind::Known) && matches!(b.kind, DimKind::Known)) || a.dim.eq(b.dim)
}

// ---------------------------------------------------------------------------------------------
// Fixed-capacity message buffer (const-evaluable diagnostics)
// ---------------------------------------------------------------------------------------------

/// Fixed-capacity UTF-8 message buffer usable in `const` evaluation.
#[derive(Clone, Copy)]
pub struct MsgBuf {
    buf: [u8; 512],
    len: usize,
}

impl MsgBuf {
    pub const fn new() -> Self {
        MsgBuf {
            buf: [0; 512],
            len: 0,
        }
    }

    pub const fn push(mut self, s: &str) -> Self {
        let b = s.as_bytes();
        // Never split a multi-byte char: only append whole strings that fit.
        if self.len + b.len() > self.buf.len() {
            return self;
        }
        let mut i = 0;
        while i < b.len() {
            self.buf[self.len] = b[i];
            self.len += 1;
            i += 1;
        }
        self
    }

    const fn push_i8(self, v: i8) -> Self {
        const DIGITS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
        let mut s = self;
        let mut n = v as i32;
        if n < 0 {
            s = s.push("-");
            n = -n;
        }
        if n >= 10 {
            s = s.push(DIGITS[(n / 10) as usize]);
        }
        s.push(DIGITS[(n % 10) as usize])
    }

    /// Append a dimension in base-SI form, e.g. `kg*m^2*s^-3`, `K`, or `1`.
    pub const fn push_dim(self, dim: Dim) -> Self {
        // Conventional order: kg, m, s, A, K, mol, cd  (indices into [L, M, T, I, Θ, N, J])
        const ORDER: [usize; 7] = [1, 0, 2, 3, 4, 5, 6];
        const SYM: [&str; 7] = ["m", "kg", "s", "A", "K", "mol", "cd"];
        let mut s = self;
        let mut first = true;
        let mut k = 0;
        while k < 7 {
            let idx = ORDER[k];
            let e = dim.0[idx];
            if e != 0 {
                if !first {
                    s = s.push("*");
                }
                first = false;
                s = s.push(SYM[idx]);
                if e != 1 {
                    s = s.push("^");
                    s = s.push_i8(e);
                }
            }
            k += 1;
        }
        if first {
            s = s.push("1");
        }
        s
    }

    pub const fn as_str(&self) -> &str {
        let (head, _) = self.buf.split_at(self.len);
        match core::str::from_utf8(head) {
            Ok(s) => s,
            Err(_) => "dimension mismatch",
        }
    }
}

impl Default for MsgBuf {
    fn default() -> Self {
        Self::new()
    }
}

const fn push_side(m: MsgBuf, src: &str, v: DimVal) -> MsgBuf {
    let mut m = m.push("`").push(src).push("`");
    if !v.label.is_empty() {
        m = m.push(" [").push(v.label).push("]");
    }
    m.push(" has dimension ").push_dim(v.dim)
}

/// Build the canonical dimension-mismatch diagnostic shared by the proc-macro (requirement
/// constraints) and the generated `const` guard assertions.
pub const fn mismatch_message(op: &str, lsrc: &str, l: DimVal, rsrc: &str, r: DimVal) -> MsgBuf {
    let m = MsgBuf::new()
        .push("dimension mismatch in `")
        .push(op)
        .push("`: ");
    let m = push_side(m, lsrc, l);
    let m = m.push(", but ");
    let m = push_side(m, rsrc, r);
    m.push(" (both sides of comparisons, `+` and `-` must have the same dimension)")
}

/// `const` assertion emitted for `#[sysml::transition(guard = ...)]` comparisons / `+` / `-`.
/// Panics during constant evaluation (a compile error) when the dimensions differ.
pub const fn assert_same_dim(op: &str, lsrc: &str, l: DimVal, rsrc: &str, r: DimVal) {
    if !compatible(l, r) {
        let m = mismatch_message(op, lsrc, l, rsrc, r);
        panic!("{}", m.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_atoms_and_composites() {
        assert_eq!(parse_unit("W"), Some(POWER));
        assert_eq!(parse_unit("kN"), Some(FORCE));
        assert_eq!(parse_unit("N*m"), Some(ENERGY));
        assert_eq!(parse_unit("m^3/s"), Some(d(3, 0, -1, 0, 0, 0, 0)));
        assert_eq!(parse_unit("1/h"), Some(FREQ));
        assert_eq!(parse_unit("kg/s"), Some(d(0, 1, -1, 0, 0, 0, 0)));
        assert_eq!(parse_unit("W/K"), Some(d(2, 1, -3, 0, -1, 0, 0)));
        assert_eq!(parse_unit("W/(m^2*K)"), Some(d(0, 1, -3, 0, -1, 0, 0)));
        assert_eq!(parse_unit("m/s^-2"), Some(d(1, 0, 2, 0, 0, 0, 0)));
        assert_eq!(parse_unit("rad/s"), Some(FREQ));
        assert_eq!(parse_unit("%"), Some(ONE));
        assert_eq!(parse_unit("dB"), Some(ONE));
        assert_eq!(parse_unit("L"), Some(VOL));
        assert_eq!(parse_unit("°C"), Some(TEMP));
    }

    #[test]
    fn omg_resolvability() {
        assert!(matches!(check_unit("m^3/s"), UnitCheck::Ok(_)));
        assert!(matches!(check_unit("J/(kg*K)"), UnitCheck::Ok(_)));
        assert!(matches!(check_unit("1/h"), UnitCheck::Ok(_)));
        assert!(matches!(check_unit("m/s^-2"), UnitCheck::Ok(_)));
        assert!(matches!(check_unit("1"), UnitCheck::BareOne));
        assert!(
            matches!(check_unit("kN"), UnitCheck::NotInOmg(d) if d.use_instead.as_bytes() == b"N")
        );
        assert!(matches!(check_unit("kN*m"), UnitCheck::NotInOmg(_)));
        assert!(matches!(check_unit("%"), UnitCheck::NotInOmg(_)));
        assert!(matches!(check_unit("furlongs"), UnitCheck::Unknown));
    }

    #[test]
    fn rejects_unknown() {
        assert_eq!(parse_unit("furlongs"), None);
        assert_eq!(parse_unit(""), None);
        assert_eq!(parse_unit("m/"), None);
        assert_eq!(parse_unit("m^"), None);
        assert_eq!(parse_unit("(m"), None);
        assert_eq!(parse_unit("kgs"), None);
    }

    #[test]
    fn composites_all_parse() {
        for c in COMMON_COMPOSITES {
            assert!(is_known_unit(c), "{c}");
        }
    }

    #[test]
    fn dimval_algebra() {
        let k = DimVal::from_unit("K");
        let w = DimVal::from_unit("W");
        assert!(!compatible(k, w));
        assert!(compatible(k, DimVal::WILDCARD));
        assert!(compatible(k, DimVal::UNKNOWN));
        let wk = w.div(k);
        assert!(compatible(wk.mul(k), w));
        assert!(compatible(
            DimVal::WILDCARD.div(DimVal::from_unit("s")),
            DimVal::from_unit("Hz")
        ));
    }

    #[test]
    fn message_text() {
        let m = mismatch_message(
            "<=",
            "temp_k",
            DimVal::from_unit("K"),
            "power_w",
            DimVal::from_unit("W"),
        );
        assert_eq!(
            m.as_str(),
            "dimension mismatch in `<=`: `temp_k` [K] has dimension K, but `power_w` [W] has dimension kg*m^2*s^-3 (both sides of comparisons, `+` and `-` must have the same dimension)"
        );
    }
}
