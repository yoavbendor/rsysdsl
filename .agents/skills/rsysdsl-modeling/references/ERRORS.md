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
| `` evaluation panicked: `redefines` target item `x` does not exist on any specialized ancestor port `` | `#[sysml::out_item(redefines)]` names an item the base port does not have | Use the inherited item's SysML (camelCase) name, e.g. `redefines = "electrical"`, and make sure the port has `#[sysml::specializes(BasePort)]`. |
| `#[sysml::interface] struct must declare two 'end' port fields ...` | Concrete interface with no / one `#[sysml::end]` field | Declare exactly two `#[sysml::end]` port fields. Only an `#[sysml::interface(abstract)]` classification base may declare none. |
| `evaluation panicked: concrete #[sysml::interface] 'X' inherits no 'end' ports from its abstract base interface` | Concrete interface specializes an end-less abstract base and declares no ends itself | Add two `#[sysml::end]` port fields to `X`. |
| `the trait bound 'MyItem: Default' is not satisfied` | A `#[sysml::item]` used inside a `#[sysml::port]` lacks `Default` | Add `#[derive(Debug, Clone, Copy, Default)]` to `MyItem`. |
| `` unknown unit symbol `kgs` in #[sysml::attr(unit = ...)]; did you mean `kg` or `kg/s`? `` | Unit string is not in the closed SI table (`sysml_core::units::UNITS`) | Use the suggested symbol, or compose OMG atoms with `*`, `/`, `^`, `( )` (e.g. `"m^3/s"`, `"J/(kg*K)"`). For a unit your own model / another library defines, add the `custom_unit` flag (skips unit + dimension checks). |
| `` unit symbol `kN` (kilonewton) is not defined in the OMG SysML v2 `SI` library; ... help: use `N` instead `` | Prefixed / non-SI symbol (`kN`, `ms`, `kPa`, `rpm`, `%`, `degC`, ...) that the strict audit cannot resolve | Use the `help:` spelling and convert the value (`45.0 kN` -> `45000.0` with `unit = "N"`; `50 ms` -> `0.050` with `unit = "s"`; `%` -> drop `unit`, store a 0..1 fraction). |
| `` dimension mismatch in `<=`: `temp_k` [K] has dimension K, but `power_w` [W] has dimension kg*m^2*s^-3 (...) `` | `#[sysml::require_constraint]` / `assume_constraint` compares or adds operands of different dimensions | Compare like with like (`K` vs `K`), or convert explicitly with a ratio attribute that carries the bridging unit (e.g. `power_w / conductance_w_per_k <= temp_rise_k` with `unit = "W/K"`). Numeric literals are dimensionless wildcards. |
| `` error[E0080]: evaluation panicked: dimension mismatch in `>`: `event.cavity_temp` [K] ... `subject.max_rf_power` [W] ... `` | Same mismatch inside `#[sysml::transition(guard = ...)]` (operand units live on the `#[sysml::item]` / subject `#[sysml::part]`, so it is reported by a `const` assertion) | Fix the guard operands or their `unit = ...` on the item / part so both sides share a dimension. |

---

## 2. Open-MBEE `rsysdsl audit --stdlib --strict` Findings -> 1-Line Fix

| Audit Rule / Message | Cause | Exact Fix |
| --- | --- | --- |
| `orphan-requirement` | A `#[sysml::requirement]` has no `#[sysml::satisfy(...)]` link | Add `#[sysml::satisfy(req = MyReq, id = "REQ-...")]` to the part that realizes it. |
| `unresolved reference 'ms'` (or `'kN'`, `'%'`) | Non-base SI unit symbol not exported by `SI::*` (now rejected at compile time unless marked `custom_unit`) | Convert to canonical OMG `SI::*` base unit (see Section 3 below), e.g. `0.050` with `unit = "s"` instead of `50.0` with `unit = "ms"`. |
| `unresolved reference 'Rationale'` | Used `#[sysml::rationale(...)]` without importing `ModelingMetadata::*` | Add `#[sysml::import("ModelingMetadata::*")]` on that package. |
| `unresolved reference 'SafetyGoal'` / `'FMEA'` / `'AsilDecomposition'` | Used ISO 26262 macros without `ISO26262Metadata` | Include `sysml_emit::iso26262_metadata_package()` in `Model::new(...)` and add `#[sysml::import("ISO26262Metadata::*")]` to the package. |
| `validateNamespaceDistinguishibility` (duplicate / indistinguishable inherited member, e.g. `electrical`) | A port (or item / interface) specializing a base (often sysml-std) re-declares a member the base already has | Mark the re-declaration as a redefinition: port items `#[sysml::out_item(redefines)]` (same name; also `in_item` / `inout_item`) or `#[sysml::out_item(redefines = "electrical")]` (new local name); attributes `#[sysml::attr(redefines = "voltage", ...)]`; part ports `#[sysml::port(redefines)]`. Or drop the field and use the inherited one. |
| `hollow-requirement` (warning, does not fail the audit) | A requirement constraint compares two attributes with the same model value: the "measured" value was copied from the limit | Do not invent a different number. Mark the measured attribute `#[sysml::attr(unit = "W", evidence)]` with **no default** and supply it from a verification case (test / sysml-sim monitor). |
| ``trace-matrix`` shows `unverified — no evidence supplied yet` | Constrained requirement whose `evidence` attributes have no value and no `VerificationReport` verdict exists. This is the honest starting state, not an error | Verify it with a `<system>_sim` monitor (`rsysdsl verify`, `trace-matrix --verify`) when a verification case exists. Never add a default to the evidence attribute to make it look measured. |
| ``trace-matrix`` shows `unverified — evidence is a model default (placeholder)` / `— no evidence attribute` | An `evidence` attribute carries a model default (a placeholder, never verification), or the constraint has no attribute marked `evidence` at all | Remove the invented default (`#[sysml::attr(unit = "W", evidence)]`, no `default`) unless it is a documented model estimate; mark measured values `evidence`. Only `VerificationReport` verdicts count as verification. |
| `root package 'Requirements' shadows the standard library package 'Requirements'` (then `unresolved reference 'Requirements::X'`) | A `#[sysml::package(name = ...)]` reuses an OMG library package name (`Requirements`, `Parts`, `Ports`, `Items`, `Actions`, `States`, ...) | Rename the package, e.g. `<System>Requirements`. |
| `unused private import` warning | Imported `SI::*`, `ISQ::*`, or `ModelingMetadata::*` in a package that doesn't reference it | Remove unused `#[sysml::import(...)]` attributes from that package. |

---

## 2b. `rsysdsl lint --strict` Findings -> Fix

| Lint code | Cause | Exact Fix |
| --- | --- | --- |
| `unclassified_interfaces` | 3+ concrete `#[sysml::interface]`s with no parent | **Keep the interfaces** and classify them. Smallest working pattern: `#[sysml::interface(abstract)] pub struct EnergyChannel {}` (no `end` fields needed), then `#[sysml::specializes(EnergyChannel)]` on each concrete interface, which keeps its own two `#[sysml::end]` ports. Reuse a sysml-std base (`InfoChannel`, `ThermalLink`, `ElectricalLink`, `DigitalBusChannel`) only when its end port types match yours. |
| `untyped_connections` | A `#[sysml::connect(...)]` without `interface = ...` in a model that declares or uses typed interfaces (typically interfaces stripped to silence `unclassified_interfaces`) | Add `interface = SomeLink` to the connect; declare the link with two `#[sysml::end]` ports (`Port` + `Conjugated<Port>`, or the same `#[sysml::port(symmetric)]` type on both ends for a bus). |
| `duplicate_part_shape` | Two concrete parts share >= 3 port/attribute names with no common ancestor | Extract an `#[sysml::part(abstract)]` base with the shared features and `#[sysml::specializes(Base)]` both. |
| `inherited_std_default` (info, does not fail `--strict`) | A model definition specializing a sysml-std element inherits numeric library defaults it never redefines | For each value your source document gives, `#[sysml::attr(redefines = "mainsFrequency", default = 60.0, unit = "Hz")]`. Leave a default only if it is irrelevant to the model; never copy a library number into a requirement. |
| `flat_god_assembly` | A part owns > 10 leaf parts directly | Group them into domain subsystem parts (Power, Electronics, Mechanics, HMI). |

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
| Volume flow / density | `"m^3/s"`, `"kg/m^3"`, `"kg/s"` | `#[sysml::attr(default = 0.002, unit = "m^3/s")]` |
| Torque / thermal | `"N*m"`, `"W/K"`, `"J/(kg*K)"`, `"°C"` | `#[sysml::attr(default = 1.5, unit = "N*m")]` |

Composite units are written in plain ASCII (`*`, `/`, `^<int>`, parentheses) over OMG `SI` atoms; they
emit as unquoted SysML unit expressions (e.g. `[m^3/s]`) that resolve against `SI::*` with 0 warnings.
Prefixed atoms that the OMG `SI` library *does* define are also accepted: `km`, `cm`, `mm`, `nm`, `mL`,
`L`, `g`, `kJ`, `MJ`, `kW`, `min`, `h`, `d`, `eV`.

**Rust -> SysML v2 Scalar Types** (requires `#[sysml::import("ScalarValues::*")]`):
- `f64` / `f32` / `Quantity<f64>` -> `Real`
- `i32` / `i64` / `u32` / `u64` -> `Integer`
- `bool` -> `Boolean`
- `String` / `&'static str` -> `String`
