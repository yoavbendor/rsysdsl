# `rsysdsl` Development Work Plan

Derived from the November 2026 review of the MVP (commit `7be3f0c`). Each stage is
one or more self-contained commits on the work branch, gated by
`cargo test --workspace` plus the strict audit loop in [`AGENTS.md`](../AGENTS.md).

## Review findings this plan addresses

| # | Finding (verified by running the code) | Severity | Stage |
| --- | --- | --- | --- |
| F1 | 4 of 18 trybuild snapshots fail on rustc 1.97 (diagnostic wording drift only; CI floats on `stable`) | CI red | S0 |
| F2 | Units are a runtime `&'static str` on `Quantity`; `#[sysml::attr(unit = "furlongs")]` compiles and is only caught by the OMG audit | High | S1 |
| F3 | Dimensionally wrong constraints compile and emit silently, e.g. `require_constraint(temp_k <= power_w)` with `K` vs `W` | **Critical for simulation** | S1 |
| F4 | `rsysdsl verify` is hard-coded to the Apollo 11 scenario; no generic scenario runner | High | S2 |
| F5 | `#[sysml::satisfy]` is an unchecked claim; no simulation evidence links back to requirements | Medium | S2, S3 |
| F6 | No fault-injection execution of `#[sysml::fmea]` / `#[sysml::safety_goal]` FTTI | High (ISO 26262 value) | S3 |
| F7 | Large authoring surface for small models; no scaffold forces a good hierarchy | Medium (ergonomics) | S4 |
| F8 | No measured evidence that Haiku-class models can author passing models | Unknown | S5 |

## Stages

### S0 — Green CI baseline
- Re-bless the 4 drifted trybuild `.stderr` snapshots (reviewed: path-qualification and span-rendering changes only).
- Pin `rust-toolchain.toml` so compiler-diagnostic snapshots stop drifting with `stable`.
- **Gate:** `cargo test --workspace` fully green.

### S1 — Compile-time unit & dimension safety (F2, F3)
- Closed SI unit table in `sysml-core` (symbol → base-dimension exponents: L, M, T, I, Θ, N, J), covering every unit used in `sysml-std` and the examples.
- `#[sysml::attr(unit = ...)]` rejects unknown units at macro expansion with a "did you mean" suggestion.
- `#[sysml::require_constraint]` and transition guards: both sides of each comparison / `+` / `-` must have the same dimension; mismatch is a compile error naming both units.
- New trybuild compile-fail cases: `bad_unit_symbol.rs`, `bad_constraint_dimension.rs`.
- **Gate:** all examples still compile and audit at 0/0/0; new UI tests pass.
- *Later (not in this stage):* type-level dimensions (`Quantity<f64, Dim>`) if the macro-level check proves insufficient.
- **Status: done.** Built:
  - `sysml_core::units` (`#![no_std]`, all `const fn`): closed atom table (symbol → `[L, M, T, I, Θ, N, J]` exponents) with a parser for `*`, `/`, `^<int>`, `( )` composites; `sysml-macros` now depends on `sysml-core` and uses the same table at expansion time.
  - Each atom is flagged OMG-resolvable or not; non-OMG atoms (`kN`, `ms`, `kPa`, `rpm`, `%`, `degC`, ...) are rejected with the exact accepted spelling (``help: use `N` instead ...``). Unknown symbols get a "did you mean" (edit distance). `custom_unit` flag opts out for model-defined units (used by `sysml-lift` codegen for foreign corpora).
  - `require_constraint` / `assume_constraint`: dimension check at expansion time (`syn::Error` on the expression). Transition guards: operand units live on other types, so `#[sysml::item]`/`#[sysml::part]` expose a hidden `const fn __sysml_field_unit` and the state macro emits `const` assertions (`E0080 evaluation panicked: dimension mismatch ...`). Literals are wildcards; unit-less fields skip the check.
  - `sysml-emit` emits composite units as unquoted SysML expressions over `SI` atoms (`[m^3/s]`, `[J/(kg*K)]`, `[W/'°C']`); new test `crates/sysml-std/tests/derived_units_audit.rs` proves a strict 0-warning audit.
  - UI tests: `bad_unit_symbol`, `bad_unit_not_in_omg`, `bad_constraint_dimension`, `bad_guard_dimension`. No dimension bugs found in existing examples; all 7 emit-bin examples still audit 0/0.

### S2 — Generic scenario runner, `sysml-sim` crate (F4, F5)
- New crate `crates/sysml-sim` (runner tier; models never depend on it):
  - `Scenario` trait: initial state, stimulus, fixed-step `step(dt)`, monitors.
  - Fixed-step RK4 integrator + event detection for state-machine transitions.
  - Requirement monitors built from `EvaluableRequirement::evaluate()` sampled every step; records first violation time and min margin.
  - Deterministic JSON trace + `Verdict { requirement_id, pass, evidence }`.
- Port `apollo11_sim` onto it without changing its verdicts.
- `rsysdsl verify` becomes generic (scenario registry) instead of Apollo-only.
- **Gate:** Apollo verdicts unchanged; new unit tests for integrator accuracy and monitor timing.
- **Status: done.** `crates/sysml-sim` ships `Scenario`, `run`, `rk4_step`, `ZeroCrossing`, `Monitor`
  (closure or `EvaluableRequirement`; every-step or at-end; min margin + first violation time),
  time-triggered `Fault`s, a deterministic JSON `RunResult` trace and a `Registry`.
  `apollo11-sim` runs on it (`PoweredDescentScenario`); `rsysdsl verify` output for
  `nominal`, `fault`, `all` and the default is byte-identical to the pre-port output
  (HLR-R002, CLR-R050, CLR-R057, CLR-R058 unchanged). New flags: `--scenario <name|alias|all>`,
  `--list`, `--trace <file>`. API summary: [`SIMULATION_VISION.md` §4](SIMULATION_VISION.md).

### S3 — FMEA-driven fault injection on the microwave (F6)
- `examples/microwave_sim`: 120 s reheat scenario + door-opened-mid-cook fault at t = 45 s.
- Faults come from the model's `#[sysml::fmea]` entries; verdict checks the safety-goal FTTI and the interlock state transition.
- Verdicts feed `trace-matrix --verify` so requirements show "verified by scenario" rather than only "satisfied".
- **Gate:** scenario verdicts pass; a deliberately broken interlock variant fails.
- Note: simulation shows the model behaves as written; it does not validate the model against hardware. Reports must list assumptions and parameter sources.

### S4 — Authoring ergonomics: `rsysdsl new` scaffold (F7)
- `rsysdsl new <name> [--layers types,subsystems,context]` generates a compiling crate with the three-layer skeleton (types/ports → subsystems → top context), emit binary and tests, registered in the workspace.
- Generated crate passes `cargo test`, strict audit and strict lint out of the box.
- **Gate:** CLI e2e test creates, builds and audits a scaffolded crate.
- **Status: done.** `rsysdsl new <name> [--dir <path>] [--no-workspace]` (`crates/sysml-cli/src/scaffold.rs`, templates in `crates/sysml-cli/templates/new/`) generates `<Name>Types` / `<Name>Subsystems` / `<Name>Requirements` / `<Name>Context` packages with one placeholder requirement (`Source: TODO`), refuses to overwrite, and registers the crate in the workspace. Generated crate passes `cargo test`, strict stdlib audit (0/0/0) and strict lint (CLEAN). The layer list is fixed (no `--layers` flag yet). CLI e2e: `cli_new_scaffolds_layered_crate_and_refuses_overwrite` (default) and `cli_new_scaffold_builds_audits_and_lints_clean` (full build + emit + audit + lint, `#[ignore]`d for speed, ~40 s; run with `-- --ignored`).

### S5 — Haiku-class authoring trial (F8)
- Give a Haiku-class model 3 small written specs plus `AGENTS.md`; record first-pass compile rate, iterations to green audit, lint findings and error classes.
- Output: `docs/HAIKU_TRIAL.md` with the measured results and concrete doc/DSL fixes they imply.
- Re-run after S1/S4 to measure improvement.
- **Status: baseline done.** All 3 specs reached clean audit + lint in 8 / 17 / 24 invocations; see [`HAIKU_TRIAL.md`](HAIKU_TRIAL.md). Quick doc/lint fixes landed with the report.

### S6 — Fix authoring friction found by the Haiku trial
- Cross-package `satisfy`: auto-import or rustc error instead of an audit-only failure.
- `unclassified_interfaces` lint: workable minimal abstract-interface pattern, documented; also flag untyped connects so removing interfaces is never the "fix".
- `#[sysml::out_item(redefines)]` and an ERRORS.md entry for duplicate inherited members.
- sysml-std numeric-default policy (no unstated numbers leaking into models) and an emit option to omit std packages from output.
- Separate requirement limits from measured/evidence values; tie evidence to verification cases.
- Document installing the CLI as `rsysdsl`.
- **Gate:** re-run the 3 trial specs; target ≤ 10 invocations each.

### Deferred (not scheduled)
- Bond-graph / acausal connections and FMI 3.0 export (`SIMULATION_VISION.md` Tier 3).
- Monte Carlo parameter sweeps and HTML trace playback (Tier 4 UI).
- Type-level dimensional analysis.

## Execution notes
- S1, S2, S4, S5 are independent and run in parallel on separate worktrees; S3 depends on S2.
- Each stage lands as its own commit(s) and is pushed after `cargo test --workspace` is green.
