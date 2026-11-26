# `rsysdsl` Error -> Exact Fix Lookup & SI Units Cheat Sheet

## 1. Compiler (`rustc`) & Macro Errors -> 1-Line Fix

| Error Message | Cause | Exact Fix |
| --- | --- | --- |
| `the trait bound 'PortA: PortConnectable<PortA>' is not satisfied` | Connecting two same-polarity directed ports (`out` -> `out`) | Wrap the receiving field's port type in `Conjugated<PortA>` (or mark a bidirectional port `#[sysml::port(symmetric)]`). |
| `the trait bound 'PortA: PortConnectable<Conjugated<PortB>>' is not satisfied` | Connecting incompatible port types | Ensure both ends of `#[sysml::connect]` use the same port type (one plain, one `Conjugated<...>`). |
| `unmodeled 'fn' is forbidden inside #[sysml::package]` | Raw Rust function inside a model package | Annotate pure formulas with `#[sysml::calc]`, or move execution/test runners to a separate `<system>_sim` crate or `#[cfg(test)]`. |
| `raw 'impl' block is forbidden inside #[sysml::package]` | Custom `impl` block inside a model package | Remove `impl Default` (auto-generated from `#[sysml::attr(default = ...)]`) or move logic into `#[sysml::calc]` / `#[sysml::require_constraint]` / `#[sysml::transition]`. |
| `unannotated 'struct' / 'enum' is forbidden inside #[sysml::package]` | Plain Rust type without `#[sysml::*]` | Add `#[sysml::part]`, `#[sysml::item]`, `#[sysml::port]`, `#[sysml::requirement]`, `#[sysml::state]`, etc. |
| `SysML requirement ID mismatch in #[sysml::satisfy]` | `id = "..."` does not match target `#[sysml::requirement(id = "...")]` | Copy the exact `id` string from the requirement definition. |
| `the trait bound 'X: SysmlActionCore' is not satisfied` | `#[sysml::perform(...)]` points to a non-action struct | Annotate `X` with `#[sysml::action]` or reference the right action type. |
| `the trait bound 'X: SysmlPartCore' is not satisfied` | `#[sysml::specializes(X)]` or `#[sysml::part]` field points to a non-part type | Annotate `X` with `#[sysml::part]` (cross-kind specialization is illegal). |
| `unknown field 'foo' in #[sysml::require_constraint]` | Typo in constraint expression | Use the exact Rust field name declared on that `#[sysml::requirement]` struct. |
| `cannot call unmodeled function '...' inside #[sysml::calc]` | Calling stdlib/external Rust fn in a modeled calc | Replace with arithmetic (`+`, `-`, `*`, `/`) or call another `#[sysml::calc]` function. |
| `the trait bound 'AsilD: AsilDecomposable<AsilB, AsilA>' is not satisfied` | Illegal ISO 26262-9 ASIL decomposition (rank sum < parent) | Use a legal pair from ISO 26262-9 Table 1 (e.g., `AsilD -> (AsilB, AsilB)` or `(AsilC, AsilA)`). |
| `the trait bound 'PartA: IndependentFrom<PartB>' is not satisfied` | Missing freedom-from-interference witness for `#[sysml::asil_decompose]` | Add `#[sysml::independent(PartB)]` on `PartA` (and `#[sysml::independent(PartA)]` on `PartB`). |
| `the trait bound 'MyItem: Default' is not satisfied` | A `#[sysml::item]` used inside a `#[sysml::port]` lacks `Default` | Add `#[derive(Debug, Clone, Copy, Default)]` to `MyItem`. |

---

## 2. Open-MBEE `rsysdsl audit --stdlib --strict` Findings -> 1-Line Fix

| Audit Rule / Message | Cause | Exact Fix |
| --- | --- | --- |
| `orphan-requirement` | A `#[sysml::requirement]` has no `#[sysml::satisfy(...)]` link | Add `#[sysml::satisfy(req = MyReq, id = "REQ-...")]` to the part that realizes it. |
| `unresolved reference 'ms'` (or `'kN'`, `'%'`, `'kW'`) | Non-base SI unit symbol not directly exported by `SI::*` | Convert to canonical OMG `SI::*` base unit (see Section 3 below), e.g. `0.050` with `unit = "s"` instead of `50.0` with `unit = "ms"`. |
| `unresolved reference 'Rationale'` | Used `#[sysml::rationale(...)]` without importing `ModelingMetadata::*` | Add `#[sysml::import("ModelingMetadata::*")]` on that package. |
| `unresolved reference 'SafetyGoal'` / `'FMEA'` / `'AsilDecomposition'` | Used ISO 26262 macros without `ISO26262Metadata` | Include `sysml_emit::iso26262_metadata_package()` in `Model::new(...)` and add `#[sysml::import("ISO26262Metadata::*")]` to the package. |
| `unused private import` warning | Imported `SI::*`, `ISQ::*`, or `ModelingMetadata::*` in a package that doesn't reference it | Remove unused `#[sysml::import(...)]` attributes from that package. |

---

## 3. Canonical OMG Standard Library Units & Scalar Types (0-Warning Safe)

Use these exact unit strings with `#[sysml::import("SI::*")]` and `Quantity<f64>`:

| Quantity | Unit String (`unit = "..."`) | Example |
| --- | --- | --- |
| Time | `"s"` | `#[sysml::attr(default = 0.050, unit = "s")]` |
| Length / Distance | `"m"` | `#[sysml::attr(default = 1.5, unit = "m")]` |
| Mass | `"kg"` | `#[sysml::attr(default = 42.0, unit = "kg")]` |
| Force / Thrust | `"N"` | `#[sysml::attr(default = 45000.0, unit = "N")]` |
| Velocity | `"m/s"` | `#[sysml::attr(default = 3.0, unit = "m/s")]` |
| Acceleration | `"m/s^2"` | `#[sysml::attr(default = 9.81, unit = "m/s^2")]` |
| Power | `"W"` | `#[sysml::attr(default = 250.0, unit = "W")]` |
| Voltage / Current | `"V"`, `"A"` | `#[sysml::attr(default = 28.0, unit = "V")]` |
| Temperature | `"K"` | `#[sysml::attr(default = 310.0, unit = "K")]` |
| Pressure | `"Pa"` | `#[sysml::attr(default = 101325.0, unit = "Pa")]` |
| Frequency | `"Hz"` | `#[sysml::attr(default = 100.0, unit = "Hz")]` |
| Energy / Angle | `"J"`, `"rad"` | `#[sysml::attr(default = 1000.0, unit = "J")]` |

**Rust -> SysML v2 Scalar Types** (requires `#[sysml::import("ScalarValues::*")]`):
- `f64` / `f32` / `Quantity<f64>` -> `Real`
- `i32` / `i64` / `u32` / `u64` -> `Integer`
- `bool` -> `Boolean`
- `String` / `&'static str` -> `String`
