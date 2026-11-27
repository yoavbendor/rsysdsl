# Haiku-Class Authoring Trial (S5, baseline)

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
