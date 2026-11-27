# Handoff — state of work and next steps

Written at the end of a Claude Code session (November 2026) for whoever continues (Antigravity or another agent). Branch: `claude/antigravity-rust-review-j6vns0` (not yet merged to `main`; no PR opened).

Read in this order: this file → [`DEVELOPMENT_PLAN.md`](DEVELOPMENT_PLAN.md) (stages, status notes, backlog) → [`HAIKU_TRIAL.md`](HAIKU_TRIAL.md) (evidence for every fix) → [`trials/`](trials/) (trial 3 protocol and per-run grading).

## 1. Where things stand

| Stage | What | Status |
| --- | --- | --- |
| S0 | Green CI; Rust 1.97.0 pinned in `.github/workflows/ci-pages.yml` (root `rust-toolchain.toml` removed for air-gapped hosts) | done |
| S1 | Compile-time SI unit table + dimensional analysis | done |
| S2 | `crates/sysml-sim` scenario runner; Apollo ported; generic `rsysdsl verify` | done |
| S3 | Microwave FMEA fault injection (`examples/microwave_sim`), verdicts in trace-matrix | done (ASIL D and FMEA 10/10/10 are labelled PLACEHOLDERs — need a real hazard analysis) |
| S4 | `rsysdsl new` layered scaffold | done |
| S5/T2/T3 | Haiku authoring trials (baseline, after S6, after S7) | done |
| S6, S6b | Friction fixes from trials 1–2 (satisfy, interfaces, evidence vs limit, scaffold order, `dispatch_with`, state lints) | done |
| S7 | TBD suite (mandatory units, `tbd_value`/`tbd_unit`/`#[sysml::tbd]`/TBD channel, `@TBD` metadata, `--maturity`, ratchet + CI) | done |
| S7b (core) | T3-1 (`CustomUnitMetadata` emission + lifting + docs), T3-6 (`ScalarValues::*` auto-import), root-cause fix for T3-3/T3-5/T3-8 (stripped fictional defaults from `sysml-std` abstract bases/flows/links & removed duplicate `BatteryPack::pack_voltage`), moved 1.97.0 pin to CI | done |
| **Simulation** | **Closed-loop simulation, physical ODEs, live HTML trace playback & verification** | **next (after PR merge)** |
| S7b (remaining) / T4 / S8 | Deferred: TBD transition trigger (T3-4), narrow `overlapping_guards` (T3-7), Haiku trial 4, advisory AI reviewer | deferred |

Everything on the branch passes `cargo test --workspace` (tested on Rust 1.97.0); all 7 examples with emit bins pass `rsysdsl audit --stdlib --strict` (0/0/0), `lint --strict` and `tbd --check`.

## 2. Next steps (in order)

### Step 1 — Merge PR #7 to `main` and move to Simulation
Core S7b friction/leakage root causes (T3-1, T3-3/T3-5/T3-8, T3-6, offline toolchain pin) are resolved without adding `not_modeled` complexity. Next priority is extending `crates/sysml-sim` (real plant dynamics, multi-system simulation coverage, HTML trace playback, evidence-write enforcement).

### Step 2 — Deferred S7b / T4 / S8 items (return after Simulation)
1. **T3-4:** built-in TBD trigger for transitions whose event the source does not define; a `#[sysml::tbd]` state should satisfy `unreachable_state`.
2. **T3-2 / T3-7 / T3-9 / T3-10:** narrow `overlapping_guards`, minor doc clarifications, trial workspace rule.
3. **T4 & S8:** Haiku trial 4 and advisory AI reviewer (`rsysdsl review`), scored on T3/T4 ground truth.

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
