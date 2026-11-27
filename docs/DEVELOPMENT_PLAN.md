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

### S2 — Generic scenario runner, `sysml-sim` crate (F4, F5)
- New crate `crates/sysml-sim` (runner tier; models never depend on it):
  - `Scenario` trait: initial state, stimulus, fixed-step `step(dt)`, monitors.
  - Fixed-step RK4 integrator + event detection for state-machine transitions.
  - Requirement monitors built from `EvaluableRequirement::evaluate()` sampled every step; records first violation time and min margin.
  - Deterministic JSON trace + `Verdict { requirement_id, pass, evidence }`.
- Port `apollo11_sim` onto it without changing its verdicts.
- `rsysdsl verify` becomes generic (scenario registry) instead of Apollo-only.
- **Gate:** Apollo verdicts unchanged; new unit tests for integrator accuracy and monitor timing.

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

### S5 — Haiku-class authoring trial (F8)
- Give a Haiku-class model 3 small written specs plus `AGENTS.md`; record first-pass compile rate, iterations to green audit, lint findings and error classes.
- Output: `docs/HAIKU_TRIAL.md` with the measured results and concrete doc/DSL fixes they imply.
- Re-run after S1/S4 to measure improvement.

### Deferred (not scheduled)
- Bond-graph / acausal connections and FMI 3.0 export (`SIMULATION_VISION.md` Tier 3).
- Monte Carlo parameter sweeps and HTML trace playback (Tier 4 UI).
- Type-level dimensional analysis.

## Execution notes
- S1, S2, S4, S5 are independent and run in parallel on separate worktrees; S3 depends on S2.
- Each stage lands as its own commit(s) and is pushed after `cargo test --workspace` is green.
