# `rsysdsl` — Bidirectional Rust Macro/DSL for SysML v2 ("System as Code")

`rsysdsl` is a Cargo workspace MVP that lets systems engineers and AI coding agents author systems in **idiomatic, strongly typed Rust** (`struct`, `enum`, traits, and proc-macros) and deterministically compile them into **OMG-valid SysML v2 (`.sysml`)** — with a **SysML v2 -> Rust DSL lifter**, **compile-time MBSE & anti-semantic-leakage safety enforced by `rustc`**, and an **executable verification loop** validated end-to-end against **Airbus Central R&T's Apollo 11 SysML v2 mission model**.

## 3-Tier Anti-Semantic-Leakage Architecture

To prevent AI coding agents from burying hidden safety `if`s in unconstrained Rust that never appear in `.sysml` or the ASPICE traceability matrix, `rsysdsl` enforces three strict tiers at `rustc` compile time:

1. **Tier 1 — Declarative Structure (`#[sysml::package]`):** Raw `fn`, `impl`, `static`, `trait`, `macro_rules!`, non-import `const`, and unannotated `struct`/`enum`/`mod` inside a `#[sysml::package]` module are **compile-time `rustc` errors**. Parameter defaults (`impl Default`) are auto-generated from `#[sysml::attr(value = ..., unit = ...)]`; requirement predicates (`#[sysml::require_constraint(...)]`) compile into **both** OMG SysML v2 `require constraint { ... }` **and** `EvaluableRequirement::evaluate(&self)`; state guards (`#[sysml::transition(..., guard = ...)]`) compile into **both** SysML `if <guard>` **and** `step_with(&event, &subject)`.
2. **Tier 2 — Modeled Behavior (`#[sysml::calc] fn`):** Functions inside `#[sysml::package]` are allowed **only** when annotated `#[sysml::calc]`. Every statement/expression is validated against the 1:1 SysML-transpilable subset and every callee must itself be a modeled `#[sysml::calc]` (opaque Rust calls fail `rustc`).
3. **Tier 3 — Unconstrained Execution Runners (separate crate):** Physics integrators, fault injectors, and test harnesses live in a separate crate (e.g., `apollo11-sim -> apollo11-model`) so Cargo itself enforces a one-way dependency from runner to model.

## Workspace Crates

| Crate | Purpose |
| --- | --- |
| [`crates/sysml-ir`](crates/sysml-ir) | Strongly typed SysML v2 IR (including `CalcDef`), compile-time MBSE traits (`SysmlPart`, `SysmlPort`, `Conjugated<P>`, `PortConnectable`, `Specializes`, `Performs`, `Satisfies`, `Verifies`, `EvaluableRequirement`, `SysmlCalc`, `ToScalar`), `Quantity<T>`, and JSON verification types. |
| [`crates/sysml-macros`](crates/sysml-macros) | Proc-macros (`#[sysml::package]`, `#[sysml::part]`, `#[sysml::port]`, `#[sysml::interface]`, `#[sysml::item]`, `#[sysml::enum_def]`, `#[sysml::action]`, `#[sysml::state]`, `#[sysml::requirement]`, `#[sysml::specification]`, `#[sysml::verification]`, `#[sysml::calc]`, `#[sysml::specializes]`, `#[sysml::perform]`, `#[sysml::satisfy]`, `#[sysml::verify]`, `#[sysml::connect]`) + closed 1:1 expression transpiler. |
| [`crates/sysml-dsl`](crates/sysml-dsl) | Ergonomic user-facing facade (`sysml_dsl::prelude::*`) + Gate 1 `trybuild` compile-fail safety & anti-leakage test suite (12 compile-fail tests + 1 compile-pass). |
| [`crates/sysml-emit`](crates/sysml-emit) | 1:1 deterministic IR -> formatted OMG SysML v2 `.sysml` emitter + embedded `Open-MBEE/sysml-toolkit` validator (`sysmlv2-syntax` + `sysmlv2-model`, with OMG Standard Library `sysml.library` resolution) + traceability audit (`audit_model`) + ASPICE bidirectional traceability matrix (`build_trace_matrix`, JSON/Markdown/CSV). |
| [`crates/sysml-lift`](crates/sysml-lift) | `Open-MBEE/sysml-toolkit` AST -> IR lifter, compilable Rust DSL code generator, semantic AST/graph round-trip comparator, and explicit unsupported-construct ledger (zero silent drops). |
| [`crates/sysml-cli`](crates/sysml-cli) | Agent-first CLI (`rsysdsl`): `emit`, `lift`, `roundtrip-check`, `verify`, `audit`, `trace-matrix`, `validate` (compact JSON on `stdout`, diagnostics on `stderr`, exit codes `0` = ok, `1` = validation/audit/roundtrip/verification failure, `2` = IO/parse error). |
| [`examples/apollo11_model`](examples/apollo11_model) | Pure 100% declarative 5-layer Apollo 11 Powered Descent model crate (zero raw `impl`/`fn`, auto-`Default`, compiled constraints, guarded `Apollo11MissionPhases`, and `#[sysml::calc]` functions). |
| [`examples/apollo11_sim`](examples/apollo11_sim) | Closed-loop physics, fault-injection, and verification runner crate depending one-way on `apollo11-model`. |

## Quickstart

### Run All 5 Acceptance Gates in One Command

```bash
cargo test --workspace
```

This single command automatically runs:
1. **Gate 1 — Rust Compile-Time Safety & Anti-Leakage (`crates/sysml-dsl/tests/gate1_compile_safety.rs`):** `trybuild` verifies 1 valid model compiles and **12** invalid/leaky models fail `rustc` (port polarity/mismatch, cross-kind `:>` specialization, wrong requirement ID, non-action `perform`, missing state target, raw `fn`/`impl`/unannotated `struct` inside `#[sysml::package]`, typo in `#[sysml::require_constraint]`, opaque Rust call in `#[sysml::calc]`, nonexistent field in `#[sysml::transition]` guard).
2. **Gate 2 — Normative SysML v2 Validation & Audit (`crates/sysml-lift/tests/apollo11_roundtrip.rs`):** Validates `apollo11-model` (including emitted `calc def`s and guarded transitions) against the full official **OMG SysML v2 Standard Library (`sysml.library`)** via `Open-MBEE/sysml-toolkit` with **0 errors AND 0 warnings**, and runs `audit_model_with_stdlib(..., strict = true)` (**0 orphan requirements, 0 unverified constrained requirements**).
3. **Gate 3 — Semantic Round-Trip Fidelity (`crates/sysml-lift/tests/apollo11_roundtrip.rs`):** Lifts a 10-package Airbus Apollo 11 vertical slice (1,242 semantic facts across all 5 CoSMA layers) -> generates Rust DSL -> compiles & executes it with `cargo run` -> re-emits `.sysml` -> validates with Open-MBEE -> verifies **1,242 / 1,242 facts match (0 missing, 0 extra)**; plus audits all 28 Airbus Apollo 11 files.
4. **Gate 4 — Closed-Loop Executable Verification (`examples/apollo11_sim/src/lib.rs`):** Executes terminal powered-descent physics and DPS stuck-throttle fault injection directly on `apollo11-model` via compiled `EvaluableRequirement::evaluate()`, `#[sysml::calc]`, and guarded `Apollo11MissionPhases::step_with()`, emitting JSON verdicts linked to `HLR-R002`, `CLR-R050`, `CLR-R057`, and `CLR-R058`.
5. **Gate 5 — Reproducible Docs & Coverage (`README.md`, [`COVERAGE.md`](COVERAGE.md)):** Full per-file Airbus Apollo 11 element coverage report and roadmap.

### Agent-First CLI (`rsysdsl`)

```bash
# Emit OMG-valid SysML v2 from the pure declarative 5-layer Apollo 11 Rust DSL model
cargo run -q -p sysml-cli -- emit --model apollo11-model --pretty

# Emit raw .sysml text to a file
cargo run -q -p sysml-cli -- emit --model apollo11-model --raw -o /tmp/apollo11_model.sysml

# Run strict 0-warning OMG Standard Library semantic validation + orphan-requirement + verification audit
cargo run -q -p sysml-cli -- audit --model apollo11-model --verify --stdlib --strict --pretty

# Export ASPICE bidirectional traceability matrix (JSON, Markdown, or RFC-4180 CSV)
cargo run -q -p sysml-cli -- trace-matrix --model apollo11-model --verify --format markdown
cargo run -q -p sysml-cli -- trace-matrix --model apollo11-model --verify --format csv -o /tmp/trace.csv

# Generate a traceability matrix directly from existing Airbus Apollo 11 .sysml files
cargo run -q -p sysml-cli -- trace-matrix --format markdown --files \
  tests/fixtures/apollo-11-sysml-v2/Requirements/TechnicalRequirementsPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Technical/SystemSpecificationPackage.sysml

# Lift real Airbus Apollo 11 .sysml files into compilable Rust DSL
cargo run -q -p sysml-cli -- lift \
  tests/fixtures/apollo-11-sysml-v2/CoSMA/CoSMAPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Logical/LogicalComponentsPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Technical/TechnicalPortsPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Technical/TechnicalComponentsPackage.sysml \
  -o /tmp/apollo11_lifted.rs

# Run bidirectional round-trip check (exit 0 = valid + 100% semantic fidelity)
cargo run -q -p sysml-cli -- roundtrip-check --pretty \
  tests/fixtures/apollo-11-sysml-v2/CoSMA/CoSMAPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Logical/LogicalComponentsPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Technical/TechnicalPortsPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Technical/TechnicalComponentsPackage.sysml

# Execute closed-loop Powered Descent verification (nominal + fault injection)
cargo run -q -p sysml-cli -- verify --scenario all --pretty

# Validate any .sysml file with the embedded Open-MBEE sysml-toolkit oracle + OMG stdlib
cargo run -q -p sysml-cli -- validate --stdlib --strict --pretty /tmp/apollo11_model.sysml
```

## References & Provenance

- **OMG SysML v2 / KerML Specification:** [systems-modeling/sysml-v2-release](https://github.com/systems-modeling/sysml-v2-release)
- **Primary Parser & Validation Oracle:** [Open-MBEE/sysml-toolkit](https://github.com/Open-MBEE/sysml-toolkit) (pinned rev `c8d256bdf6638d3f867e61850df332da43a0f9e9`)
- **CLI Design Inspiration & Secondary Oracle:** [nomograph-ai/sysml](https://github.com/nomograph-ai/sysml)
- **Round-Trip Benchmark Model:** [airbus/apollo-11-sysml-v2](https://github.com/airbus/apollo-11-sysml-v2) (pinned rev `0e8a7d6790cfa7e15b991eb745e9a38f9bed0474`, MPL-2.0, see [`tests/fixtures/apollo-11-sysml-v2/PROVENANCE.md`](tests/fixtures/apollo-11-sysml-v2/PROVENANCE.md))

## License

Dual-licensed under MIT or Apache-2.0. The Airbus Apollo 11 test fixtures under `tests/fixtures/apollo-11-sysml-v2/` retain their upstream MPL-2.0 license.