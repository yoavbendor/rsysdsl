# `rsysdsl` — Bidirectional Rust Macro/DSL for SysML v2 ("System as Code")

`rsysdsl` is a Cargo workspace MVP that lets systems engineers and AI coding agents author systems in **idiomatic, strongly typed Rust** (`struct`, `enum`, traits, and proc-macros) and deterministically compile them into **OMG-valid SysML v2 (`.sysml`)** — with a **SysML v2 -> Rust DSL lifter**, **compile-time MBSE safety enforced by `rustc`**, and an **executable verification loop** validated end-to-end against **Airbus Central R&T's Apollo 11 SysML v2 mission model**.

## Architecture

| Crate | Purpose |
| --- | --- |
| [`crates/sysml-ir`](crates/sysml-ir) | Strongly typed SysML v2 IR, compile-time MBSE traits (`SysmlPart`, `SysmlPort`, `Conjugated<P>`, `PortConnectable`, `Specializes`, `Performs`, `Satisfies`, `Verifies`), `Quantity<T>`, and JSON verification report types. |
| [`crates/sysml-macros`](crates/sysml-macros) | Proc-macros: `#[sysml::package]`, `#[sysml::part]`, `#[sysml::port]`, `#[sysml::interface]`, `#[sysml::item]`, `#[sysml::enum_def]`, `#[sysml::action]`, `#[sysml::state]`, `#[sysml::requirement]`, `#[sysml::specification]`, `#[sysml::verification]`, `#[sysml::specializes]`, `#[sysml::perform]`, `#[sysml::satisfy]`, `#[sysml::verify]`, `#[sysml::connect]`. |
| [`crates/sysml-dsl`](crates/sysml-dsl) | Ergonomic user-facing facade re-exporting `sysml-ir` and `sysml-macros` under `sysml_dsl::prelude::*`, plus Gate 1 `trybuild` compile-fail safety tests. |
| [`crates/sysml-emit`](crates/sysml-emit) | 1:1 deterministic IR -> formatted OMG SysML v2 `.sysml` emitter + embedded `Open-MBEE/sysml-toolkit` parser & normative validator (`sysmlv2-syntax` + `sysmlv2-model`). |
| [`crates/sysml-lift`](crates/sysml-lift) | `Open-MBEE/sysml-toolkit` AST -> IR lifter, compilable Rust DSL code generator, semantic AST/graph round-trip comparator, and explicit unsupported-construct ledger (zero silent drops). |
| [`crates/sysml-cli`](crates/sysml-cli) | Agent-first CLI (`rsysdsl`): `emit`, `lift`, `roundtrip-check`, `verify`, `validate` (compact JSON on `stdout`, diagnostics on `stderr`, exit codes `0` = ok, `1` = validation/roundtrip/verification mismatch, `2` = IO/parse error). |
| [`examples/apollo11_slice`](examples/apollo11_slice) | Complete 5-layer Apollo 11 Powered Descent vertical slice in idiomatic Rust DSL + closed-loop nominal & fault-injection executable verification. |

## Quickstart

### Run All 5 Acceptance Gates in One Command

```bash
cargo test --workspace
```

This single command automatically runs:
1. **Gate 1 — Rust Compile-Time Safety (`crates/sysml-dsl/tests/gate1_compile_safety.rs`):** `trybuild` suite verifying 1 valid system compiles and 6 invalid models fail `rustc` compilation (`bad_port_polarity`, `bad_port_mismatch`, `bad_specialization_kind`, `bad_requirement_id`, `bad_perform_target`, `bad_state_transition`).
2. **Gate 2 — Normative SysML v2 Validation (`crates/sysml-lift/tests/apollo11_roundtrip.rs`):** Validates emitted `.sysml` (both the hand-written 5-layer `apollo11-slice` and lifted Airbus Apollo 11 packages) with `Open-MBEE/sysml-toolkit` (`sysmlv2-syntax` + `sysmlv2-model`) with **zero errors**.
3. **Gate 3 — Semantic Round-Trip Fidelity (`crates/sysml-lift/tests/apollo11_roundtrip.rs`):** Lifts a 10-package Airbus Apollo 11 vertical slice (1,242 semantic facts across all 5 CoSMA layers) -> generates Rust DSL -> compiles & executes it with `cargo run` -> re-emits `.sysml` -> validates with Open-MBEE -> verifies **1,242 / 1,242 facts match (0 missing, 0 extra)**; plus audits all 28 Airbus Apollo 11 files.
4. **Gate 4 — Closed-Loop Executable Verification (`examples/apollo11_slice/src/simulation.rs`):** Runs terminal powered-descent physics, fault injection (stuck DPS throttle valve + guidance drift), and state-machine legality checks directly on the `#[sysml::*]` Rust structs/enums, emitting JSON verdicts linked to `HLR-R002`, `CLR-R050`, `CLR-R057`, and `CLR-R058`.
5. **Gate 5 — Reproducible Docs & Coverage (`README.md`, [`COVERAGE.md`](COVERAGE.md)):** Full per-file Airbus Apollo 11 element coverage report and roadmap.

### Agent-First CLI (`rsysdsl`)

```bash
# Emit OMG-valid SysML v2 from the hand-written 5-layer Apollo 11 Rust DSL slice
cargo run -q -p sysml-cli -- emit --model apollo11-slice --pretty

# Emit raw .sysml text to a file
cargo run -q -p sysml-cli -- emit --model apollo11-slice --raw -o /tmp/apollo11_slice.sysml

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

# Validate any .sysml file with the embedded Open-MBEE sysml-toolkit oracle
cargo run -q -p sysml-cli -- validate --pretty /tmp/apollo11_slice.sysml
```

## Example: Idiomatic Rust DSL with Compile-Time Safety

```rust
use sysml_dsl::prelude::*;

#[sysml::item]
#[derive(Default)]
pub struct CommandSignal;

#[sysml::port]
#[derive(Default)]
pub struct LMStagingPort {
    #[sysml::out]
    pub commands: CommandSignal,
}

#[sysml::interface]
#[derive(Default)]
pub struct LMStagingInterface {
    #[sysml::end]
    pub platform: LMStagingPort,
    #[sysml::end]
    pub ascender: Conjugated<LMStagingPort>, // omitting Conjugated<...> fails rustc!
}

#[sysml::requirement(id = "HLR-R002")]
#[derive(Default)]
pub struct LunarLanderSoftLandingRequirement;

#[sysml::part]
#[sysml::satisfy(req = LunarLanderSoftLandingRequirement, id = "HLR-R002")] // wrong ID fails rustc!
#[sysml::connect(
    interface = LMStagingInterface,
    name = "lmInterstage",
    from = descent_stage.upper_port,
    to = ascent_stage.lower_port
)]
#[derive(Default)]
pub struct ApolloLunarModule {
    #[sysml::part]
    pub descent_stage: DescentStage,
    #[sysml::part]
    pub ascent_stage: AscentStage,
}
```

## References & Provenance

- **OMG SysML v2 / KerML Specification:** [systems-modeling/sysml-v2-release](https://github.com/systems-modeling/sysml-v2-release)
- **Primary Parser & Validation Oracle:** [Open-MBEE/sysml-toolkit](https://github.com/Open-MBEE/sysml-toolkit) (pinned rev `c8d256bdf6638d3f867e61850df332da43a0f9e9`)
- **CLI Design Inspiration & Secondary Oracle:** [nomograph-ai/sysml](https://github.com/nomograph-ai/sysml)
- **Round-Trip Benchmark Model:** [airbus/apollo-11-sysml-v2](https://github.com/airbus/apollo-11-sysml-v2) (pinned rev `0e8a7d6790cfa7e15b991eb745e9a38f9bed0474`, MPL-2.0, see [`tests/fixtures/apollo-11-sysml-v2/PROVENANCE.md`](tests/fixtures/apollo-11-sysml-v2/PROVENANCE.md))

## License

Dual-licensed under MIT or Apache-2.0. The Airbus Apollo 11 test fixtures under `tests/fixtures/apollo-11-sysml-v2/` retain their upstream MPL-2.0 license.