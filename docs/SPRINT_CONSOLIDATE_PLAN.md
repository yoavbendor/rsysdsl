# Sprint plan — Consolidate & Deepen (no new subcommands)

Basis: review of `main` at `bf21107` (all 147 tests + 21 e2e green) and the external assessment recommending consolidation. Every item below fixes something verified by running the code; evidence is quoted inline.

**Rule for this sprint: zero new CLI subcommands.** Depth and correctness of the existing 15 only.

## Verified problems this sprint fixes

| # | Problem | Evidence |
| --- | --- | --- |
| P1 | Operators find values by **attribute-name lists** (`"mass"`, `"ratedPower"`, `"powerDraw"`, `"minRuntimeS"`, `"maxNoiseDb"`) and `slice` by **English keywords** | `budget.rs:95,100`, `trade_study.rs:88,141`, `refute.rs:79`, `refactor.rs:64-88` |
| P2 | **False PASS**: `budget --max-power-w 100` on a model with a 2000 W element (`ratedPowerW [W]`) → total 0 W, `pass: true`, margin 100 W, exit 0 | Haiku trial-3 water heater; also ventilation (2 × 170 W fans → 0 W) |
| P3 | `refute` on a model with no variations and no recognised requirements → **`"feasible": false`** with an empty core | Haiku trial-3 water heater |
| P4 | `refute`/`trade-study` only know 5 hard-coded metrics (runtime = energy/power, mass, power, noise, cost); they never evaluate the model's own `require_constraint` expressions | `refute.rs` `RefuteLimits` |
| P5 | Two authoring paths: `SwHwSystem` builder (string names, raw `_ms` floats outside the unit system, own `Asil` enum) re-checks at runtime (`E001`/`E002`) what the macro path makes `rustc` enforce | `sysml-ir/src/swhw.rs`, `sysml-emit/src/ir_validate.rs` |
| P6 | Docs overstate speed: "sub-millisecond" (`0.83 ms @ 2,040 elements`) is `validate_ir` on a synthetic builder model, not the authoring loop (macro path = `rustc`, seconds) | `scale_check.rs`; AGENTS.md §7 |

## Design decisions (made here, so implementers don't have to)

- **D1 — Role + dimension, never names.** An attribute contributes to a budget only if it has a *role*, from either (a) inheritance: it is or redefines a `sysml-std` role attribute (`PowerElement::ratedPower` → power, `PhysicalComponent::mass` → mass, `wasteHeat` → waste heat), or (b) an explicit tag `#[sysml::attr(unit = "W", budget = "power")]` (roles: `mass`, `power`, `waste_heat`, `cost`, `latency`). The tag is emitted as SysML metadata (`@BudgetRole { role = "power"; }`) so IR operators read it. The unit's **dimension must match** the role (power ⇒ W) — compile error otherwise.
- **D2 — No silent pass.** Every operator reports `untracked`: attributes whose dimension matches a budget domain but have no role (e.g. `ratedPowerW [W]` untagged). Budget `status` ∈ `pass | fail | incomplete`; `incomplete` when a limit is set and there are untracked candidates or zero contributors. `--strict` exits 1 on `fail` or `incomplete`.
- **D3 — Refute verdicts are three-valued:** `feasible | refuted | not_applicable` (no constraints or no design space). `not_applicable` never reports `feasible: false`.
- **D4 — Operators run on the IR, with an IR expression evaluator.** Constraints are stored as strings in the IR (`require_constraints: Vec<String>`), produced by our own closed transpiler (`expr_transpile.rs`). Write a small parser/evaluator for exactly that subset (arithmetic, comparisons, `and`/`or`/`not`, attribute refs, calls to `calc def`s present in the IR). Operators then work on `.sysml` files from any source (authored, lifted, brownfield). In-crate compiled closures stay for `sysml-sim` dynamics.
- **D5 — One authoring surface.** Humans and agents author with macros only. SW/HW concepts become a `SysmlStdSoftware` package in `sysml-std` (`SwPartition`, `SwTask`, `ComputeNode`, `BusChannel` as part defs; timing in `s`; ASIL via existing ISO 26262 metadata). `E003`/`E004`/`W001` checks move to IR-level lint/audit rules so they apply to macro-authored models. `SwHwSystem` stays as an internal lowering/brownfield-import library, not documented for authoring.

## Stages (each lands as its own commits, pushed after `cargo test --workspace` is green)

| Stage | Work | Owner |
| --- | --- | --- |
| **C1** | D1 + D2 + D3: role tags and std role attributes, `untracked` reporting, budget `status`, refute `not_applicable`; remove name lists from `budget`, `trade-study`, `refute`; `slice` keyed on flow item types / port types / dimensions instead of keywords | Opus agent |
| **C1-T** | Regression corpus + tests written against the D2/D3 JSON contract: copy the 12 trial-3 `.sysml` files to `tests/fixtures/haiku_t3/`; negative-name fixtures; tests asserting no false pass, `incomplete` on untracked watts, `not_applicable` on empty refute | **Haiku** (well-defined, contract given) |
| **C2** | D4: IR expression parser/evaluator for the transpiler subset, property-tested against the compiled `EvaluableRequirement` results for every example requirement | Opus agent |
| **C3** | Generalize `refute`/`trade-study`/`what-if` onto C2: any `require_constraint` + `calc`, discrete variations **and** continuous ranges (`--range attr=lo..hi`; interval/grid + bisection for the boundary); MUC and minimum relaxations over real constraints | Opus agent |
| **C4** | D5: `SysmlStdSoftware` package + IR lint rules for timing/ASIL guardrails; migrate `automotive_progressive` and `px4_fmuv6x_brownfield` examples to macros | Opus (package + rules), **Haiku** (example migration from a worked reference) |
| **C5** | Docs truthfulness: AGENTS.md §7 / README speed claims (P6), operator docs for role tags and `status` values, remove builder-authoring guidance | **Haiku** (exact edit list given) |
| **T5** | Real-world end-to-end trial: one unfamiliar messy corpus (PDF table + block diagram + conflicting requirements) → docmem → `reconcile` → model → `refute` → `sim` → HTML review, with no hand-written aliases. Pre-registered protocol, graded from source by a separate agent | Haiku runs (subjects), Opus grader |

Order: C1 ∥ C1-T → C2 → C3 → C4 ∥ C5 → T5. C1-T runs in parallel with C1 because the JSON contract is fixed above.

## JSON contract for C1 / C1-T (fixed)

- `budget`: each domain object gains `"status": "pass" | "fail" | "incomplete"` and `"untracked": [{"element": str, "attribute": str, "unit": str, "value": number|null}]`; top-level `"ok"` is false unless every limited domain is `pass`. With `--strict`, exit 1 when `ok` is false.
- `refute`: top-level `"status": "feasible" | "refuted" | "not_applicable"`; `"feasible"` stays as a bool for compatibility but is `null` when `not_applicable`.
- `trade-study`: `"untracked"` list as above; `"status": "ok" | "not_applicable"` (no variations).

## Delegation notes

- Haiku gets tasks with a fixed contract, exact files, and a worked example; Opus gets design-bearing work. Every delegated task: commit per item, push, `cargo test --workspace` green, never force-push.
- Trial agents follow the canonical trial rules in `HAIKU_TRIAL.md` (incl. own-directory-only).

## Open question for the owner (T5)
Which corpus? It must be shareable in this repo or fetchable by script (like `scripts/fetch-px4-corpus.sh`), contain at least one real PDF table, one block diagram, and at least one requirement conflict.
