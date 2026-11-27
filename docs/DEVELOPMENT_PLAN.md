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
- **Status: done.** Model: `#[sysml::safety_goal(asil = AsilD, safe_state = "doorOpenCutoff", ftti_ms = 20)]`
  on MW-004 and `#[sysml::fmea(failure_mode = "ContactsWeldedClosed", ...)]` on both door interlocks
  (strict audit 0/0/0, lint CLEAN). FTTI 20 ms is MW-004's own `max_cutoff_latency_s`; the ASIL and
  the FMEA severity/occurrence/detection ratings (10/10/10) are **PLACEHOLDER — no source** and are
  marked as such in the model. Runner `examples/microwave_sim` registers four scenarios with
  `rsysdsl verify`: `microwave_reheat_nominal` (MW-001 pass), `microwave_door_open_mid_cook`
  (MW-004 pass, 13 ms = primary interlock 5 ms + HV relay 8 ms), `microwave_door_open_primary_interlock_welded`
  (MW-004 pass, 16 ms via the monitor interlock) and the deliberately broken
  `microwave_door_open_both_interlocks_welded` (`expect_pass: false`; MW-004 fails, first violation
  t = 45.021 s). Faults are looked up from the model's `PartDef::fmea`; verdicts call the model's
  `evaluate()`, the safe state / FTTI come from its `SafetyGoal` impl, and the door-open reaction is
  the model's `MicrowaveState::step_with`. `trace-matrix --model microwave --verify` shows MW-001 and
  MW-004 as PASS with the verifying scenario names (expected-pass scenarios merged; the broken variant
  is a monitor self-test, not evidence). The 120 s duration and t = 45 s door opening are scenario
  stimulus from this plan (the model has no cook-time attribute). Not covered: MW-002 / MW-003 (the
  model has no load/air thermal-mass or specific-heat attributes, so only absorbed RF energy is
  integrated), MCU processing time (taken as 0). Assumptions and parameter sources are listed in
  `examples/microwave_sim/src/lib.rs`. **Limitation:** the simulation verifies the model as written,
  not the hardware.

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
- **Gate:** `cargo test --workspace` green; all examples strict audit 0/0/0 and lint CLEAN.
- **Status: done.** Built:
  - S6.1 cross-package `satisfy`/`verify`: `emit_model` qualifies requirement references the owning package cannot see (`satisfy requirement : OtherPkg::Req;`), so no manual import is needed; emit + strict-audit test. Pattern 2 no longer names its package `Requirements` (shadows the OMG library package; new ERRORS.md row).
  - S6.2 interfaces: `#[sysml::interface(abstract)] pub struct EnergyChannel {}` (no ends) is accepted as a pure classification base; a concrete interface inheriting no ends is a compile error (UI test). New lint `untyped_connections` (connects without an interface type in a model that declares/uses typed interfaces); `unclassified_interfaces` suggestion names the working pattern; ERRORS.md lint table; Pattern 1 fully typed and classified.
  - S6.3 `#[sysml::out_item(redefines)]` / `(redefines = "name")` (also `in_item`/`inout_item`) emits `out item :>> x : T;`, targets checked at compile time; strict-audit test against sysml-std `ElectricalPowerPort` and ERRORS.md entry for `validateNamespaceDistinguishibility`.
  - S6.4 std defaults made visible rather than deleted (110 numeric defaults across the 7 std packages: comms 22, electronics 6, flows 14, hmi 5, mechanics 25, natural 16, power 22): every std package documents them as library placeholders, and lint info `inherited_std_default` lists per model definition each numeric std default it inherits without redefining (counted separately from smell infos; examples currently inherit 0–17 such definitions each). `emit --omit-std` / `--std-out` (shared `sysml_emit::emit_cli`, used by every example emit bin and the `rsysdsl new` template) keeps std out of the model file; `rsysdsl audit|lint|trace-matrix|view|validate|safety-report --files` auto-load the bundled sysml-std when the files reference it.
  - S6.5 limits vs evidence: see design note below.
  - S6.6 `cargo install --path crates/sysml-cli` documented (AGENTS.md step 0, README, SKILL.md); docs use `rsysdsl <cmd>` with `cargo run -q -p sysml-cli -- <cmd>` as the stated no-install equivalent.
  - Gate: `cargo test --workspace` green; all 7 examples with an emit bin (with and without `--omit-std`) plus built-in apollo11-model, embedded-flight-controller and microwave: strict audit 0 errors / 0 warnings / 0 orphans, lint `--strict` CLEAN; a fresh `rsysdsl new --no-workspace` crate passes its tests, audit and lint.
  - Deferred: trace-matrix architecture counts (`part defs`, `modeled calcs`) still include sysml-std elements when std is loaded; std numeric defaults were not removed (would change example numbers; revisit with S7 `tbd_value`); `untyped_connections` cannot catch a model that deletes its interface defs as well as their uses.
- **Design note — limits vs evidence (S6.5).** Built: `#[sysml::attr(..., evidence)]` marks a measured/achieved requirement attribute (IR `AttributeUsage::is_evidence`, emitted as a `doc /* [evidence] ... */` tag and lifted back); an evidence attribute without a default is NaN, so `evaluate()` is false until a verification case sets it; `trace-matrix` gives each row an `evidence_source` (`verification` only when a `VerificationReport` verdict exists, otherwise `model default (placeholder)` / `unmarked`); `audit` reports `hollow-requirement` warnings (constraint operands with the same model value, i.e. a limit copied into the measured value) without failing, so authors are not pushed to invent a different number; the microwave example marks its four measured values. Open / deferred: (1) sysml-sim monitors could assert that every `evidence` field of the requirement they evaluate was actually overwritten by the scenario (needs a generated `evidence_fields()` accessor on requirements); (2) tie evidence to `#[sysml::verification]` / `verify` defs in SysML (verification case `subject` + `objective`) instead of a doc tag, ideally via a small `RsysdslMetadata::Evidence` metadata def once S7 adds the `@TBD` metadata package (same mechanism); (3) whether `audit --strict` should eventually fail on unmarked constrained requirements — decide after T2/T3 show whether agents use the marker honestly.

### T2 — Haiku trial 2 (measures S6 alone)
- Same 3 specs, same instructions and cap as the baseline; **2–3 runs per spec**, report medians.
- Metric: invocations to clean audit + lint, compared with baseline (8 / 17 / 24, single runs).
- Kept separate from S7 so friction reduction is not confounded with TBD behavior.
- **Status: done (8 of 9 runs; e-bike r3 cut off by a usage limit).** Medians 5 / 9.5 / 6 vs baseline 8 / 17 / 24; only 4 failing invocations in 8 runs. Measures S4 + S6 together; 6/8 runs avoided sysml-std. New findings N1–N11 in [`HAIKU_TRIAL.md`](HAIKU_TRIAL.md#trial-2-after-s4--s6).

### S6b — Scaffold and doc fixes from trial 2 (before S7)
- N1 scaffold layer order `Types → Requirements → Subsystems → Context`; N2 no invented number in the scaffold placeholder; N3 standalone scaffold adds sysml-std and correct next steps; N5 distinct "no evidence supplied" trace label; N6 `step_with` ignored-event outcome; N7/N11 PATTERNS updates.
- Lint `unreachable_state` / `overlapping_guards` (N8) if small; N9 n-ary bus and N10 guard→requirement references go to the backlog.
- Trial hygiene: trial prompts must forbid reading `docs/HAIKU_TRIAL.md`.
- **Status: done.**
  - N1: `rsysdsl new` scaffolds `Types → Requirements → Subsystems → Context`; the placeholder requirement is satisfied by a subsystem part (`Controller`). CLI help, layer comments and AGENTS.md updated; e2e test checks the order.
  - N2: the placeholder requirement is text-only (`Source: TODO`, no constraint, no number); the limit/evidence constraint shape is a comment with `<value from the document>`.
  - N3: scaffolds (workspace and `--no-workspace`) depend on `sysml-std` and prepend `sysml_std::std_packages()`; emit steps use `--omit-std`; standalone `lib.rs` header and JSON next steps never use `-p`. AGENTS.md documents the absolute path deps and warns that `rsysdsl new` without `--dir` writes into the repo's `examples/`.
  - N5: trace matrix `evidence_source = "no evidence supplied"` (evidence attr without default; not counted as placeholder evidence) vs `"model default (placeholder)"`; markdown labels `no evidence supplied yet` / `evidence is a model default (placeholder)`.
  - N6: generated state machines add `dispatch_with` / `dispatch_event` returning `StepOutcome::{Moved(state), Ignored}` (std and no_std); `step_with` / `step_event` unchanged. Documented in Pattern 4, AGENTS Phase 3 gate and SKILL.
  - N7/N11: PATTERNS documents cross-package `satisfy` (Rust `use` needed, SysML import not), exact guard grammar (bool fields, `!`/`&&`/`||`/`==`, no calls/casts/paths), field-less event items, and `#[sysml::port(redefines)]` to connect inherited ports; all snippets compiled + strict-audited in `crates/sysml-dsl/tests/patterns_s6b.rs`.
  - N8: `StateDef::initial_state` (emitted as `entry; then <state>;`, lifted back) enables lint `unreachable_state` (warning) plus `dead_end_state` and `overlapping_guards` (info, counted as `state_review_count`, never fail `--strict`). All examples stay audit 0/0/0 and `lint --strict` CLEAN (dishwasher, power_tools_family and microwave show intended prioritized-guard / latched-fault review items).
  - Trial hygiene: "Running an authoring trial" section in `docs/HAIKU_TRIAL.md` (no reading trial reports/plan; report failing invocations and verify rounds separately).
  - Not done here: N4 (inherited std default "not specified") is S7 `tbd_value`; N9 and N10 moved to the Backlog below.

### S7 — TBD / technical-debt suite (controlled escape hatches)
Goal: support gradual, cyclical design without silent holes. Every escape hatch costs a written reason, is visible in the emitted SysML, is counted, and must shrink as maturity rises.
1. **Close the free hatch:** a `Quantity` attribute must declare exactly one of `unit = "..."`, `dimensionless`, or `tbd_unit = "reason"`; none is a compile error (today an omitted unit silently skips the dimension check). `custom_unit` requires a reason too.
2. **Visible debt:** every hatch emits `@TBD { reason; owner; resolveBy; }` metadata; `audit` and `trace-matrix` report a TBD ledger; `audit --maturity concept|preliminary|detailed|release` sets the allowed TBD budget (release = 0).
3. **Ratchet:** committed `tbd-baseline.json`; CI fails if the TBD count rises without the baseline being updated in the same commit.
4. **Typed placeholders:** `TbdChannel` interface (connects any two ports, payload untyped) for "some comms link, protocol TBD", and `tbd_value = "reason"` on attributes for parameters not yet in the source (no invented numbers).
- **Gate:** existing examples unchanged or migrated; ratchet demonstrated in CI.

### T3 — Haiku trial 3 (measures S7)
- Same 3 specs + 1 new spec with deliberate gaps (unstated protocol, missing parameter value, non-SI unit); 2–3 runs each.
- Metrics: honest TBD use (real gaps → TBD with true reason), misuse (TBD where the spec gives the value, or TBD masking a real error), leaked numbers not in any source, invocations.
- The hand-labelled results become ground truth for S8.

### S8 — Adversarial AI reviewer (`rsysdsl review`)
- Advisory only (never a blocking gate): separate context sees spec + diff + TBD ledger, never the author's reasoning.
- Checks: number provenance against the spec, TBD-reason honesty, hollow requirements (measured value copied from limit).
- Structured findings that must cite a spec section (uncited findings dropped); run at phase gates / PRs, ideally a different model or prompt from the author.
- **Gate:** scored against T3 ground truth (recall on labelled misuse, false-positive rate).

### Backlog (from trial 2, not scheduled)
- **N9 n-ary bus:** a shared bus (CAN, I2C) with >2 members can only be modelled as point-to-point connects. Needs a bus connection with >2 ends (SysML `connection` with n `end`s) in the IR, macro, emitter, viewer and lint. Open question: model it as an n-ary `connection def` or as a bus part with one port per member.
- **N10 guard→requirement references:** guards cannot read requirement limits, so limits get duplicated into part attributes (drift risk). Options: allow `req.<Requirement>.<attr>` operands in guards, or a lint for numeric literals duplicated between requirement and part attributes.

### Deferred (not scheduled)
- Bond-graph / acausal connections and FMI 3.0 export (`SIMULATION_VISION.md` Tier 3).
- Monte Carlo parameter sweeps and HTML trace playback (Tier 4 UI).
- Type-level dimensional analysis.

## Execution notes
- S0–S5 done (S1, S2, S4, S5 ran in parallel worktrees; S3 after S2).
- Order from here: S6 → T2 → S7 → T3 → S8. Trials are separated so each change's effect can be measured on its own.
- Push after every significant item (not only per stage), each after `cargo test --workspace` is green, so work survives an interrupted session.
