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
| [`crates/sysml-core`](crates/sysml-core) | Zero-dependency `#![no_std]` compile-time MBSE contracts (`SysmlPartCore`, `SysmlPortCore`, `Conjugated<P>`, `PortConnectable`, `Specializes`, `Performs`, `Satisfies`, `Verifies`, `EvaluableRequirement`, `SysmlCalcCore`, `Quantity<T>`, `NoStdTransitionError`) and ISO 26262-9 safety traits (`Asil`, `AsilA`..`AsilD`, `SafetyGoal`, `IndependentFrom<Other>`, `AsilDecomposable<L, R>`). |
| [`crates/sysml-ir`](crates/sysml-ir) | Strongly typed SysML v2 IR (including `CalcDef`, `AnalysisDef`, `MetadataDef`, `SafetyGoalMeta`, `FmeaEntry`, `AsilDecompositionMeta`), host reflection traits (`SysmlPart`, `SysmlPort`, `SysmlState`, `SysmlRequirement`, …), and JSON verification types. |
| [`crates/sysml-macros`](crates/sysml-macros) | Proc-macros (`#[sysml::package]` incl. `no_std`, `#[sysml::part]`, `#[sysml::port]`, `#[sysml::interface]`, `#[sysml::item]`, `#[sysml::enum_def]`, `#[sysml::action]`, `#[sysml::state]`, `#[sysml::requirement]`, `#[sysml::specification]`, `#[sysml::verification]`, `#[sysml::calc]`, `#[sysml::analysis]`, `#[sysml::specializes]`, `#[sysml::perform]`, `#[sysml::satisfy]`, `#[sysml::verify]`, `#[sysml::connect]`, `#[sysml::safety_goal]`, `#[sysml::fmea]`, `#[sysml::independent]`, `#[sysml::asil_decompose]`) + closed 1:1 expression transpiler. |
| [`crates/sysml-dsl`](crates/sysml-dsl) | Ergonomic `#![no_std]`-capable facade (`sysml_dsl::prelude::*`, optional `reflect` feature) + Gate 1 `trybuild` compile-fail safety, anti-leakage & ISO 26262 test suite (14 compile-fail tests + 1 compile-pass). |
| [`crates/sysml-emit`](crates/sysml-emit) | 1:1 deterministic IR -> formatted OMG SysML v2 `.sysml` emitter (incl. `iso26262_metadata_package()` for `@SafetyGoal`/`@FMEA`/`@AsilDecomposition`) + embedded `Open-MBEE/sysml-toolkit` validator (`sysmlv2-syntax` + `sysmlv2-model`, with OMG Standard Library `sysml.library` resolution) + traceability audit (`audit_model`) + ASPICE bidirectional traceability matrix (`build_trace_matrix`, JSON/Markdown/CSV). |
| [`crates/sysml-lift`](crates/sysml-lift) | `Open-MBEE/sysml-toolkit` AST -> IR lifter, compilable Rust DSL code generator, semantic AST/graph round-trip comparator, and explicit unsupported-construct ledger (zero silent drops). |
| [`crates/sysml-cli`](crates/sysml-cli) | Agent-first CLI (`rsysdsl`): `emit`, `lift`, `roundtrip-check`, `verify`, `audit`, `trace-matrix`, `validate` (compact JSON on `stdout`, diagnostics on `stderr`, exit codes `0` = ok, `1` = validation/audit/roundtrip/verification failure, `2` = IO/parse error). |
| [`examples/apollo11_model`](examples/apollo11_model) | Pure 100% declarative 5-layer Apollo 11 Powered Descent model crate (zero raw `impl`/`fn`, auto-`Default`, compiled constraints, guarded `Apollo11MissionPhases`, and `#[sysml::calc]` functions). |
| [`examples/apollo11_sim`](examples/apollo11_sim) | Closed-loop physics, fault-injection, and verification runner crate depending one-way on `apollo11-model`. |
| [`examples/embedded_flight_controller`](examples/embedded_flight_controller) | Ferrocene-ready `#![no_std]` ISO 26262 ASIL-D dual-channel flight controller (`ASIL_D -> ASIL_B(D) + ASIL_B(D)` with compile-time independence witnesses, `@FMEA`, guarded `#![no_std]` state machine, and 0-warning OMG stdlib validation). |
| [`examples/starter_model`](examples/starter_model) | Minimal 1-file Visio-to-`rsysdsl` starter template with standalone `emit` binary (`cargo run -q -p starter-model --bin emit`) ready for VS Code SysML v2 / `sysml-reactflow`. |

## Authoring New Systems with AI Agents (Cursor-CLI / Claude Code / Codex)

`rsysdsl` ships with a lean, cost-aware LLM authoring workflow so engineers and coding agents can draft a system from a Visio diagram and incrementally crystallize it from a document corpus into a fully traceable SysML v2 model:

- **Agent instructions:** [`AGENTS.md`](AGENTS.md) — rules, 1-page Visio/MBSE -> `rsysdsl` concept map, and 4-phase incremental crystallization workflow.
- **On-demand modeling skill:** [`.agents/skills/rsysdsl-modeling/`](.agents/skills/rsysdsl-modeling/SKILL.md) (`PATTERNS.md` copy-pasteable Rust templates + `ERRORS.md` `rustc`/Open-MBEE fix table).
- **Corporate offline & VS Code viewer guide:** [`docs/CORPORATE_OFFLINE_SETUP.md`](docs/CORPORATE_OFFLINE_SETUP.md) + [`scripts/vendor-offline.sh`](scripts/vendor-offline.sh).

## Quickstart

### Run All 5 Acceptance Gates + Bare-Metal `#![no_std]` Check

```bash
# Full workspace test suite (all 5 gates + ISO 26262 + flight controller stdlib validation)
cargo test --workspace

# Verify true bare-metal #![no_std] build (zero std / alloc / serde target dependencies)
cargo check -p sysml-core
cargo check -p embedded-flight-controller --no-default-features
```

`cargo test --workspace` automatically runs:
1. **Gate 1 — Rust Compile-Time Safety, Anti-Leakage & ISO 26262 (`crates/sysml-dsl/tests/gate1_compile_safety.rs`):** `trybuild` verifies 1 valid model compiles and **14** invalid/leaky models fail `rustc` (port polarity/mismatch, cross-kind `:>` specialization, wrong requirement ID, non-action `perform`, missing state target, raw `fn`/`impl`/unannotated `struct` inside `#[sysml::package]`, typo in `#[sysml::require_constraint]`, opaque Rust call in `#[sysml::calc]`, nonexistent field in `#[sysml::transition]` guard, illegal ISO 26262 `ASIL_D -> (ASIL_B, ASIL_A)` decomposition, and missing `#[sysml::independent(...)]` freedom-from-interference witness).
2. **Gate 2 — Normative SysML v2 Validation & Audit (`crates/sysml-lift/tests/apollo11_roundtrip.rs`, `examples/embedded_flight_controller/src/lib.rs`):** Validates both `apollo11-model` and `embedded-flight-controller` (including `@SafetyGoal`, `@FMEA`, `@AsilDecomposition`, `calc def`s, and guarded transitions) against the full official **OMG SysML v2 Standard Library (`sysml.library`)** via `Open-MBEE/sysml-toolkit` with **0 errors AND 0 warnings**, and runs `audit_model_with_stdlib(..., strict = true)` (**0 orphan requirements, 0 unverified constrained requirements**).
3. **Gate 3 — Semantic Round-Trip Fidelity (`crates/sysml-lift/tests/apollo11_roundtrip.rs`):** Lifts a 13-package Airbus Apollo 11 vertical slice (all 5 CoSMA layers + `CapabilitiesPackage`, `CalculationsPackage`, `AnalysisPackage`, 0 unsupported) -> generates Rust DSL -> compiles & executes it with `cargo run` -> re-emits `.sysml` -> validates with Open-MBEE -> verifies **100% semantic match (0 missing, 0 extra)**; plus audits all 28 Airbus Apollo 11 files (**2,916 / 2,916 facts matched, 100.0% corpus coverage, 28 / 28 files at 0 unsupported**).
4. **Gate 4 — Closed-Loop Executable Verification (`examples/apollo11_sim/src/lib.rs`, `examples/embedded_flight_controller/src/lib.rs`):** Executes terminal powered-descent physics and DPS stuck-throttle fault injection directly on `apollo11-model` (emitting JSON verdicts linked to `HLR-R002`, `CLR-R050`, `CLR-R057`, `CLR-R058`) and exercises the `#![no_std]` ISO 26262 flight controller safety state machine and FTTI requirement constraints.
5. **Gate 5 — Reproducible Docs & Coverage (`README.md`, [`COVERAGE.md`](COVERAGE.md)):** Full per-file Airbus Apollo 11 element coverage report and roadmap.

### Agent-First CLI (`rsysdsl`)

The CLI crate `crates/sysml-cli` builds a binary named `rsysdsl`. Install it once from the repo root:

```bash
cargo install --path crates/sysml-cli   # puts `rsysdsl` on PATH (~/.cargo/bin); re-run after pulling
```

Every `rsysdsl <cmd> ...` below is equivalent to `rsysdsl <cmd> ...` run from the repo root (no install needed).

```bash
# Emit OMG-valid SysML v2 from the pure declarative 5-layer Apollo 11 Rust DSL model
rsysdsl emit --model apollo11-model --pretty

# Emit raw .sysml text to a file
rsysdsl emit --model apollo11-model --raw -o /tmp/apollo11_model.sysml

# Run strict 0-warning OMG Standard Library semantic validation + orphan-requirement + verification audit
rsysdsl audit --model apollo11-model --verify --stdlib --strict --pretty

# Export ASPICE bidirectional traceability matrix (JSON, Markdown, or RFC-4180 CSV)
rsysdsl trace-matrix --model apollo11-model --verify --format markdown
rsysdsl trace-matrix --model apollo11-model --verify --format csv -o /tmp/trace.csv

# Export ISO 26262 Safety, FMEA RPN Ranking & Minimal Fault Cut-Set Report (JSON or Markdown)
rsysdsl safety-report --model embedded-flight-controller --format markdown

# Generate a traceability matrix directly from existing Airbus Apollo 11 .sysml files
rsysdsl trace-matrix --format markdown --files \
  tests/fixtures/apollo-11-sysml-v2/Requirements/TechnicalRequirementsPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Technical/SystemSpecificationPackage.sysml

# Lift real Airbus Apollo 11 .sysml files into compilable Rust DSL
rsysdsl lift \
  tests/fixtures/apollo-11-sysml-v2/CoSMA/CoSMAPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Logical/LogicalComponentsPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Technical/TechnicalPortsPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Technical/TechnicalComponentsPackage.sysml \
  -o /tmp/apollo11_lifted.rs

# Run bidirectional round-trip check (exit 0 = valid + 100% semantic fidelity)
rsysdsl roundtrip-check --pretty \
  tests/fixtures/apollo-11-sysml-v2/CoSMA/CoSMAPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Logical/LogicalComponentsPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Technical/TechnicalPortsPackage.sysml \
  tests/fixtures/apollo-11-sysml-v2/Technical/TechnicalComponentsPackage.sysml

# Execute closed-loop Powered Descent verification (nominal + fault injection)
rsysdsl verify --scenario all --pretty

# Validate any .sysml file with the embedded Open-MBEE sysml-toolkit oracle + OMG stdlib
rsysdsl validate --stdlib --strict --pretty /tmp/apollo11_model.sysml
```

## References & Provenance

- **OMG SysML v2 / KerML Specification:** [systems-modeling/sysml-v2-release](https://github.com/systems-modeling/sysml-v2-release)
- **Primary Parser & Validation Oracle:** [Open-MBEE/sysml-toolkit](https://github.com/Open-MBEE/sysml-toolkit) (pinned rev `c8d256bdf6638d3f867e61850df332da43a0f9e9`)
- **CLI Design Inspiration & Secondary Oracle:** [nomograph-ai/sysml](https://github.com/nomograph-ai/sysml)
- **Round-Trip Benchmark Model:** [airbus/apollo-11-sysml-v2](https://github.com/airbus/apollo-11-sysml-v2) (pinned rev `0e8a7d6790cfa7e15b991eb745e9a38f9bed0474`, MPL-2.0, see [`tests/fixtures/apollo-11-sysml-v2/PROVENANCE.md`](tests/fixtures/apollo-11-sysml-v2/PROVENANCE.md))

## License

Dual-licensed under MIT or Apache-2.0. The Airbus Apollo 11 test fixtures under `tests/fixtures/apollo-11-sysml-v2/` retain their upstream MPL-2.0 license.