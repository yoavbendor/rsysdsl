# Haiku-Class Authoring Trial (S5, baseline)

## Running an authoring trial (canonical rules, from S6b)

Every trial prompt must include these rules, and every trial report must use these metrics:

1. **No contamination.** The agent must NOT read `docs/HAIKU_TRIAL.md`, `docs/DEVELOPMENT_PLAN.md`, or any other trial report or plan (trial 2, greenhouse r2, read this file and changed its design because of the baseline findings). It may read `AGENTS.md`, `.agents/skills/`, `crates/`, `examples/` and the spec it was given.
2. **Same setup each time:** the same specs and prompt as the previous trial, a standalone crate outside the repo (`rsysdsl new <name> --no-workspace --dir <path>`), the installed `rsysdsl` CLI, a soft cap of 15 cargo/`rsysdsl` invocations, and a log of every invocation with its exit status.
3. **Report three numbers per run, separately:** total invocations; **failing invocations** (non-zero exit, or a test/audit/lint failure); and **verify rounds** (full `cargo test` + emit + `audit --strict` + `lint --strict` passes). Total invocations alone mostly measures diligence (trial 2's highest counts were voluntary re-verification); failing invocations measure friction.
4. Exclude `--help` and the `rsysdsl new` call from counts. Record runs cut off by usage limits as excluded, not as failures.
5. Label which stage(s) the trial measures; if two changes land together, say the result is confounded.

**Question:** can a Haiku-class model author a well-structured, passing `rsysdsl` model from a short written spec, using only `AGENTS.md` and the repo docs?

**Setup (November 2026, commit after S0):** three independent Haiku agents, one spec each (a few subsystems, 3 requirements with source references, a mode list). Each worked in a standalone crate outside the repo, was told to follow `AGENTS.md` phases 1–3 until `audit --stdlib --strict` and `lint --strict` were clean, with a soft cap of 15 cargo/CLI invocations, and logged every invocation.

## Results

| Spec | Final result | Invocations | Over cap? | Main cost |
| --- | --- | --- | --- | --- |
| Greenhouse climate | test 5/5, audit 0/0/0, lint CLEAN | 8 | no | derived unit `m^3/s` |
| E-bike pedal assist | test 4/4, audit 0/0/0, lint CLEAN (1 info) | 17 | yes | cross-package `satisfy`, lint interface rule |
| Water heater | test 4/4, audit 0/0/0, lint CLEAN | 24 (2 void) | yes | lint interface rule → sysml-std redesign |

**Verdict: yes, with friction.** All three reached a fully clean model. Structure was reasonable (top-level context, typed ports/interfaces, requirements with provenance, state machine, calcs). Requirement discipline was good: none invented thresholds; all flagged spec gaps and marked placeholders. The rustc-level errors were never the bottleneck. **Every expensive iteration came from late, audit/lint-only failures and doc gaps**, which are fixable.

In all three trials, the model code compiled on the first build. Failures came from (in order of cost):

1. **Lint `unclassified_interfaces`** (2/3 trials, ~10 invocations in the water heater). The suggested fix named a nonexistent type (`sysml_std::flows::EnergyLink`). The docs say to classify under `InfoChannel`/`DigitalBusChannel`, but those require std port types; an empty abstract interface base is rejected by the macro. One agent "fixed" the warning by **removing typed interfaces from its connects** — the lint rewarded a worse model.
2. **Derived units** (2/3): `m^3/s`, `m^3` compile but fail strict audit (`unresolved reference`) because the emitter quotes units containing `^`. Agents had to read `emit.rs` and `SI.sysml` to find `'m³⋅s⁻¹'` / `L`. No guidance for non-SI units (`Ah`).
3. **Cross-package `satisfy`** (2/3): needs `#[sysml::import("ReqPkg::*")]` on the satisfying package. rustc passes; strict audit fails. Undocumented, and Pattern 2 showed the form without the import.
4. **Doc bug, `step_with`** (3/3): Pattern 4 implied it returns the mode; it takes `&mut self` and returns `Result<State, NoStdTransitionError>`.
5. **Duplicate inherited member** (water heater): `validateNamespaceDistinguishibility` when specializing std ports/items; `#[sysml::out_item]` has no `redefines`. Undocumented.

## Other observations

- **sysml-std defaults leak unstated numbers.** Std parents carry defaults (50 Hz mains, 48 MHz MCU, 0.25 kg/s pump, 0.98 link efficiency) that the spec never gave. This conflicts with Golden Rule 3; one agent avoided specializing std parts for this reason. Emitted files also include all std packages and 23–24 std calcs, which a reviewer can mistake for model content.
- **"Unverified" with no path forward.** All constrained requirements show `unverified` in `trace-matrix`; nothing in the docs explains `#[sysml::verification]` / `#[sysml::verify]`. This is the hook S2/S3 simulation verdicts should fill.
- **Requirement "measured" placeholders.** Requirement defs hold both the limit and a "measured" value; with no measured value in the spec, every agent set it to the limit. The pattern conflates spec and evidence.
- **Audit wording:** findings show `severity: "error"` under rule `openmbee:strict-warning:*` while the summary counts them as warnings.
- **CLI name:** docs say `rsysdsl`; agents had to discover `cargo run -p sysml-cli --`.
- **Standalone crates:** the starter's `version.workspace = true` must be replaced outside the workspace (S4 scaffold addresses this).

## Fixed with this report
- `PATTERNS.md` Pattern 4: correct `step_with` usage with a test example.
- `PATTERNS.md` Pattern 2: cross-package `satisfy` import requirement.
- `lint.rs`: suggestion names real types (`ThermalLink` / `ElectricalLink`).

## Follow-ups (added to the plan as S6)
1. Derived-unit spellings accepted and emitted correctly (handed to S1).
2. Make cross-package `satisfy` either auto-import or a rustc error.
3. Rework `unclassified_interfaces`: allow a minimal abstract base interface, document the exact working pattern, and never reward removing interfaces (e.g. also warn on untyped connects).
4. `#[sysml::out_item(redefines)]` support, and an ERRORS.md entry for duplicate inherited members.
5. Decide policy for sysml-std defaults (no numeric defaults on abstract std bases, or explicit `TODO`-marked) and an emit option to omit std packages from the output file.
6. Separate requirement limits from measured/evidence values; tie evidence to verification cases (S2/S3).
7. Install the CLI as `rsysdsl` (`[[bin]] name = "rsysdsl"` is already set; document `cargo install --path crates/sysml-cli`).

Re-run the same three specs after S1, S4 and S6 to measure improvement against this baseline (8 / 17 / 24 invocations).

---

# Trial 2 (after S4 + S6)

**Setup:** same 3 specs, prompt, and 15-invocation soft cap as the baseline; agents followed the current `AGENTS.md` (so they had the `rsysdsl new` scaffold and installed CLI — this trial measures **S4 + S6 together**, not S6 alone). 3 runs per spec, standalone crates outside the repo, shared build cache. Count = cargo/`rsysdsl` invocations, excluding `--help` and the `rsysdsl new` scaffold call (same rule as the baseline).

## Results

| Spec | Runs (invocations) | Median | Baseline (n=1) | Invocations that failed |
| --- | --- | --- | --- | --- |
| Greenhouse | 15, 5, 5 | **5** | 8 | 2, 0, 0 |
| E-bike | 10, 9, — | **9.5** (n=2) | 17 | 2, 0 |
| Water heater | 6, 10, 5 | **6** | 24 | 0, 0, 0 |

- All 8 completed runs reached clean `cargo test`, `audit --stdlib --strict` (0/0/0, no orphans, no hollow requirements) and `lint --strict` (CLEAN). E-bike run 3 was cut off by an account usage limit mid-run and is excluded.
- **Failures nearly disappeared:** 4 failing invocations across 8 runs (baseline: most of 49). The only failures were Rust scoping in a test module (e-bike r1) and a test that assumed `step_with` returns `Ok` for an ignored event (greenhouse r1). No audit or lint failure in any run.
- The high counts (15, 10, 10) come from **voluntary repeat verify rounds** after adding phase-3 content, not from fixing errors. Invocation count now measures diligence more than friction; future trials should also report failing invocations and verify rounds.
- Every run kept measured values as `evidence` with no default; trace-matrix reported 3/3 satisfied, 0/3 verified in all runs. No invented thresholds.

## Caveats (why the improvement is real but overstated)

1. **Simpler models.** 6 of 8 runs avoided specializing sysml-std (to avoid its spec-less defaults — 50 Hz, 48 MHz, 18 V, 20 Hz…). The baseline water heater spent ~10 invocations on a sysml-std redesign. Only water heater r3 used std bases fully. Less reuse means less friction, and less structure.
2. **Contamination.** Greenhouse r2 read this file (`docs/HAIKU_TRIAL.md`) and avoided sysml-std *because of the baseline findings*. Greenhouse r3 avoided std independently after reading its sources, so the effect is not only contamination. Future trials must tell agents not to read trial reports.
3. **Confounded:** S4 (scaffold) and S6 land together here; n = 2–3 per spec.

## New findings (all 8 runs)

| # | Finding | Runs | Proposed fix |
| --- | --- | --- | --- |
| N1 | Scaffold layer order `Types → Subsystems → Requirements → Context` makes `satisfy` on realizing parts impossible under the scaffold's own "use only packages above" rule | **8/8** (6 reordered, 2 moved satisfy to Context) | Scaffold `Types → Requirements → Subsystems → Context` |
| N2 | Scaffold placeholder requirement invents a 1.0 s limit (violates Golden Rule 3) | flagged by 2, all removed it | Placeholder with no number (S7 `tbd_value`) |
| N3 | `--no-workspace` scaffold: no sysml-std dep, absolute path deps, `cargo test -p` next-steps | 5 | Add std dep; document; `--manifest-path` steps |
| N4 | No way to leave an inherited std default "not specified" → authors avoid std or keep spec-less numbers | 6 | S7 `tbd_value` |
| N5 | Trace label "evidence is a model placeholder" shown even when the evidence attr has no default | 4 | Distinct label "no evidence supplied" |
| N6 | `step_with` returns `Err` for an ignored event (a normal SysML no-op) | 1 failure | Return an explicit `Ignored` outcome, or document prominently |
| N7 | PATTERNS: cross-package `satisfy` still needs a Rust `use`; bool fields / `!` in guards and field-less event items undocumented | 4 | PATTERNS update |
| N8 | Dead-end / unreachable states and overlapping guards are not flagged | 3 | Lint rules `unreachable_state`, `overlapping_guards` |
| N9 | No n-ary bus: one shared CAN bus had to be modelled as point-to-point segments | 1 | Bus connection with >2 ends |
| N10 | Guards cannot reference requirement limits, so limits are duplicated into part attributes (drift risk) | 1 | Allow guards to reference requirement attrs, or lint duplicated literals |
| N11 | Connecting to an inherited port needs a redeclared `#[sysml::port(redefines)]` field | 1 | Document in PATTERNS |

N1–N3, N5–N7 and N11 are small (scaffold + docs); N4 is S7; N8–N10 are new features. Proposed as S6b before S7 so T3 is not dominated by N1.

---

# Trial 3 (after S7 TBD suite, on top of S6b)

**Setup:** pre-registered in [`trials/T3_PROTOCOL.md`](trials/T3_PROTOCOL.md) before any run: specs 1–3 unchanged plus a new ventilation spec with five deliberate gaps (D1 protocol, D2 exchanger efficiency, D3 "TBD W" power limit, D4 frost entry condition, D5 non-SI 2400 rpm); 3 runs each, Haiku, canonical trial rules, CLI at `bf1ceb5`. The prompt did not mention TBDs, so all TBD use came from `AGENTS.md`. **Graded from source by a separate agent, not from self-reports** — full per-run grading with file:line in [`trials/T3_GRADING.md`](trials/T3_GRADING.md). Measures S7 (with S6b).

## Results

| Spec | Invocations | Friction failures | Verify rounds | TBDs | Unmarked invented triggers |
| --- | --- | --- | --- | --- | --- |
| Greenhouse | 8, 11, 11 (median 11) | 1, 0, 0 | 1, 2, 2 | 5, 8, 10 | r1, r3 |
| E-bike | 13, 6, 10 (median 10) | 0, 1, 0 | 2, 1, 2 | 8, 3, 16 | r2 |
| Water heater | 5, 5, 6 (median 5) | 0, 0, 0 | 1, 1, 1 | 5, 12, 2 | r2, r3 (minor) |
| Ventilation (new) | 7, 10, 7 (median 7) | 1, 0, 2 | 1, 2, 1 | 8, 19, 11 | — |

- **12/12 runs** reached clean `cargo test`, `audit --stdlib --strict` and `lint --strict`, all within the 15-invocation cap (max 13). 5 friction failures in 12 runs (excluded: 4 deliberate `--maturity` probes, 1 call misrouted through another agent's shared helper script).
- **107 TBDs: 75 honest gaps, 28 blanked sysml-std defaults, 4 borderline, 0 misuse** (no TBD on a spec-given value, no TBD hiding an error, every measured value used `evidence`). **0 numeric leakage** — every non-spec number was an exact conversion (2400 rpm → 40 Hz, 80 L → 0.08 m^3) or a redefined std value; 11 `0.0` "off" initializers listed as conventions.
- **Ventilation deliberate gaps: 15/15** handled honestly (TBD channel for the protocol, `tbd_value` on HRV-3/HRV-4, TBD transition for frost entry, exact 40 Hz conversion).
- Several runs went beyond the loop unprompted: re-checked their own model and marked invented triggers (e-bike r1), ran `audit --maturity` probes (4 runs), blanked std defaults after the lint info (4 runs).

**Verdict:** the TBD suite works as intended for its main purpose — Haiku used it honestly and without prompting, and nothing was invented. The remaining honesty gap is structural, not numeric: **invented triggers left unmarked in 5 of 12 runs**, and in every spec a sibling run marked the same transitions, so it is inconsistency rather than inability. Invocation counts are not comparable to T1–T2 (more modeling work per run: TBDs, std blanking, a harder spec); friction stayed low (5 failures in 12 runs vs 4 in 8 in T2).

## Borderline items (grading rules to settle)
1. **Design value vs requirement bound** (water heater r1, r2): `tbd_value` on the cutoff trip point because WH-002 gives only "≤ 368 K"; r3 set the trip point to 368 K instead. Both readings are defensible; the docs need a rule.
2. **TBD channel where typed ports exist** (e-bike r3 torque/speed links): honest reason, but it switches off port checking that was available.

## Findings

| # | Finding | Evidence | Proposed fix |
| --- | --- | --- | --- |
| T3-1 | Documented non-SI route `unit = "rpm", custom_unit = "..."` fails strict audit (`unresolved reference 'rpm'`): nothing defines rpm | ventilation r3; PATTERNS P8, ERRORS rows 23/27 | Fix docs now; add an in-model unit definition (e.g. `#[sysml::unit_def]`) or emit custom units in a resolvable form |
| T3-2 | Invented triggers left unmarked | 5/12 runs | Phase 3 checklist line; built-in TBD trigger (T3-4); S8 reviewer target |
| T3-3 | Blanking every inherited std default floods the ledger (ventilation r2: 10 of 19 TBDs are params like supply mass) and pushes authors away from sysml-std (8/12 specialized no std part) | 4 runs | Separate `not_modeled = "reason"` marker / ledger kind, not maturity-gated; resolve the lint-vs-AGENTS wording conflict |
| T3-4 | No way to express an open trigger: agents invent placeholder events (`BoostRequest`, `FrostEvent`, `BmsExchange`); a state-level `#[sysml::tbd]` does not clear `unreachable_state` | 3 runs | Built-in `TbdTrigger`; a TBD state counts as reachable-by-TBD |
| T3-5 | sysml-std defaults enter through **ports/items/interfaces** (12 V / 1 A on `ElectricalPowerPort`, 0.98 link efficiency, 293.15 K thermal ports) and lint does not report them | 5 runs | Extend `inherited_std_default` to port/item/interface types |
| T3-6 | Numeric attr in a package without `ScalarValues::*` compiles but fails audit (`unresolved reference 'Real'`); audit line numbers point into stdlib-inclusive text | ventilation r1 | Auto-import or compile error; report lines in the emitted file |
| T3-7 | `overlapping_guards` fires on guards over different fields (band edges, fault flag vs band) | 3 runs | Only flag same-field/unguarded overlaps, or document |
| T3-8 | sysml-std `BatteryPack` carries two voltage defaults (own 18 V + inherited 12 V); prelude glob collides with authors' own names | e-bike r2, r3 | Std cleanup; document explicit paths |
| T3-9 | Doc gaps: `dimensionless` + `tbd_value`, `tbd_unit` with no default, subject-less state template, camelCase in `redefines`, grandparent defaults count, "100% linkage" means satisfied not verified | several | PATTERNS/ERRORS/AGENTS updates |
| T3-10 | Trial hygiene: agents wrote helper scripts in the shared trial folder; one overwrote another's | 2 runs | Canonical rule: every file inside the run's own crate dir |

## Ground truth for S8
The labelled set for the AI reviewer is now: 5 runs with unmarked invented triggers (should be flagged), 4 borderline items, 5 runs with port-level std defaults, 0 misuse. A useful reviewer must find the unmarked triggers and the port-level defaults without flagging the 75 honest TBDs.
