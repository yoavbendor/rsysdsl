# Handoff — state of work and next steps

Written at the end of a Claude Code session (November 2026) for whoever continues (Antigravity or another agent). Branch: `claude/antigravity-rust-review-j6vns0` (not yet merged to `main`; no PR opened).

Read in this order: this file → [`DEVELOPMENT_PLAN.md`](DEVELOPMENT_PLAN.md) (stages, status notes, backlog) → [`HAIKU_TRIAL.md`](HAIKU_TRIAL.md) (evidence for every fix) → [`trials/`](trials/) (trial 3 protocol and per-run grading).

## 1. Where things stand

| Stage | What | Status |
| --- | --- | --- |
| S0 | Green CI; `rust-toolchain.toml` pinned to 1.97.0 | done |
| S1 | Compile-time SI unit table + dimensional analysis | done |
| S2 | `crates/sysml-sim` scenario runner; Apollo ported; generic `rsysdsl verify` | done |
| S3 | Microwave FMEA fault injection (`examples/microwave_sim`), verdicts in trace-matrix | done (ASIL D and FMEA 10/10/10 are labelled PLACEHOLDERs — need a real hazard analysis) |
| S4 | `rsysdsl new` layered scaffold | done |
| S5/T2/T3 | Haiku authoring trials (baseline, after S6, after S7) | done |
| S6, S6b | Friction fixes from trials 1–2 (satisfy, interfaces, evidence vs limit, scaffold order, `dispatch_with`, state lints) | done |
| S7 | TBD suite (mandatory units, `tbd_value`/`tbd_unit`/`#[sysml::tbd]`/TBD channel, `@TBD` metadata, `--maturity`, ratchet + CI) | done |
| **S7b** | **Fixes from trial 3** | **next** |
| T4 | Haiku trial 4 (measures S7b) | after S7b |
| S8 | Adversarial AI reviewer (`rsysdsl review`), scored on T3/T4 ground truth | after T4 |

Everything on the branch passes `cargo test --workspace`; all 7 examples with emit bins pass `rsysdsl audit --stdlib --strict` (0/0/0), `lint --strict` and `tbd --check`.

## 2. Next steps (in order)

### Step 1 — S7b: fixes from trial 3 (details and evidence: HAIKU_TRIAL.md "Trial 3", findings T3-1..T3-10)
1. **T3-1 (bug, do first):** PATTERNS Pattern 8 and ERRORS rows 23/27 recommend `unit = "rpm", custom_unit = "..."`, which fails strict audit (`unresolved reference 'rpm'`). Fix the docs immediately (convert exactly, e.g. 2400 rpm → 40 Hz, or `tbd_unit`). Then add a way to define a unit in-model (e.g. `#[sysml::unit_def(name = "rpm", ...)]` emitting a SysML unit definition) or emit custom units in a form the audit resolves.
2. **T3-3:** add a `not_modeled = "reason"` marker (own ledger kind, not maturity-gated) for inherited sysml-std parameters that are out of scope (e.g. supply mass). Align the `inherited_std_default` lint suggestion with AGENTS Golden Rule 3 (they currently disagree).
3. **T3-4:** built-in TBD trigger for transitions whose event the source does not define (agents invent `BoostRequest`/`FrostEvent`); a `#[sysml::tbd]` state should satisfy `unreachable_state`.
4. **T3-5:** extend `inherited_std_default` to port, item and interface types (12 V / 1 A on `ElectricalPowerPort`, 0.98 `ElectricalLink` efficiency, 293.15 K thermal ports currently enter silently).
5. **T3-6:** auto-import `ScalarValues::*` when a package has numeric attributes (today: compiles, then audit fails with `unresolved reference 'Real'`); report audit line numbers relative to the emitted file, not the stdlib-inclusive text.
6. **T3-2 / T3-9:** AGENTS Phase 3 checklist line "every trigger not in the source is a TBD transition"; doc gaps (`dimensionless` + `tbd_value`, `tbd_unit` without default, subject-less state template, camelCase names in `redefines`, grandparent defaults count, "100% linkage" = satisfied not verified); a rule for **design value vs requirement bound** (water heater cutoff trip point — graded borderline in T3).
7. **T3-7 / T3-8:** narrow `overlapping_guards` (only same-field or unguarded overlaps); clean up sysml-std `BatteryPack` (own 18 V `pack_voltage` plus inherited 12 V `nominalVoltage`); document explicit paths when the prelude glob collides with user names.
8. **T3-10:** add to the canonical trial rules: every file the agent creates stays inside its own crate directory.

### Step 2 — T4: Haiku trial 4
Same 4 specs and prompt as T3 (`docs/trials/T3_PROTOCOL.md`), 3 runs each, canonical rules in HAIKU_TRIAL.md plus T3-10. Pre-register before running. Grade from source with a separate agent (as in `trials/T3_GRADING.md`). Primary metrics: unmarked invented triggers (T3: 5/12 runs), std-default use vs avoidance (T3: 8/12 avoided std parts), ledger noise, friction failures.

### Step 3 — S8: AI reviewer
Advisory only (never a blocking gate). Separate context sees spec + diff + TBD ledger. Score it on the T3 (and T4) labelled set: it must flag the 5 runs with unmarked invented triggers and the 5 runs with port-level std defaults, and must not flag the 75 honest TBDs. Uncited findings are dropped.

### Step 4 — open the pull request
Nothing on this branch is merged. Open a PR to `main` when S7b lands (or earlier if you want review); the commit history is per-stage and readable.

## 3. Problems found along the way (process and environment)

- **Disk fills fast.** `target/` reaches 17–26 GB; parallel agents with separate target dirs filled the disk once. Run `cargo clean` between big phases; give parallel trial agents one shared `CARGO_TARGET_DIR`; delete agent worktree `target/` dirs after merging.
- **trybuild snapshots drift with rustc.** That is why `rust-toolchain.toml` pins 1.97.0 (CI used floating `stable`). When bumping the toolchain, re-bless with `TRYBUILD=overwrite` and read the diffs (they should be wording-only).
- **Usage limits cut agents mid-run.** Commit and push after every item, not per stage. Trial runs cut off by a limit are excluded, not counted as failures.
- **Parallel agents:** worktree isolation works but agents start from an old commit unless told to rebase onto the branch tip; merge by cherry-pick. A single agent working directly on the branch and pushing per item was the most robust pattern.
- **Trial contamination:** a trial agent once read `HAIKU_TRIAL.md` and changed its design; agents wrote helper scripts into the shared trial folder and overwrote each other's. The canonical trial rules (HAIKU_TRIAL.md) now forbid both; keep them.
- **Invocation counts measure diligence, not friction.** Report failing invocations and verify rounds separately (canonical rule 3).

## 4. Open decisions for the project owner

1. **`custom_unit` default phase:** it is due by `detailed` like every TBD, so lifted external corpora (Apollo 11 uses psi, fps, kB) fail `--maturity detailed` until mapped. Give custom units their own default phase?
2. **Strict on evidence:** should `audit --strict` fail on constrained requirements with no evidence marker? (Deferred in S6.)
3. **sysml-std defaults:** keep library numeric defaults (blank or `not_modeled` per model), or remove numeric defaults from abstract std bases entirely?
4. **Grading rule:** is a TBD on a design parameter whose spec gives only a bound (cutoff trip point ≤ 368 K) honest or misuse? T3 graded it borderline.
5. **S3 safety placeholders:** microwave ASIL D and FMEA ratings need a real hazard analysis before they mean anything.

## 5. Smaller leftovers (from the original review and stage notes)

- `Quantity::kn()` / `Quantity::percent()` runtime constructors produce units (`kN`, `%`) that `unit = ...` now rejects; remove or convert.
- `cli_new_scaffold_builds_audits_and_lints_clean` is `#[ignore]`d (~40 s); CI does not run it.
- Deferred in S2: zero-crossing time is estimated but the trace records the end of step; evidence captured at end of run, not at first violation; HLR-R002 has no margin (mixed m/s and m).
- Deferred in S6/S7: sim monitors do not check that `evidence` fields were actually written; no TBD markers on action/calc/analysis/enum/verification defs; lift codegen drops TBDs on states/transitions; integer `tbd_value` defaults to 0; trace-matrix links TBDs to requirements by simple name.
- Backlog (plan): n-ary bus connections (N9); guards referencing requirement limits (N10 — agents duplicate limits into part attributes); bond-graph/FMI export; Monte Carlo and HTML trace playback.
