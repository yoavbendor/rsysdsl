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
- C1.6 (additive): `budget` gains top-level `"rollup": "usage_tree" | "per_definition"` and `"roots": [str]` (top-level context parts the usage tree starts from); each mass/power/waste-heat/cost domain gains `"uninstantiated": [{"element", "attribute", "unit", "value"}]` (role values of concrete definitions never instantiated from a root; `value` in the domain unit) and `"reasons": [str]` (why a limited domain is or would be `incomplete`). `contributors` now counts instances; `hot_spots[].element` is a usage path (`Context.supplyFan`) instead of a definition name.

## C1 implementation notes (decisions and deferred items)

- **Roles beyond D1:** besides `mass | power | waste_heat | cost | latency`, two roles exist: `efficiency` (dimensionless; waste heat of a part with a power role and no waste-heat role is `power * (1 - efficiency)`) and `none` (explicit opt-out, silences `untracked`, e.g. supply capacities, aggregates of child loads, consumable loads, external actors, natural bodies).
- **Cost unit:** the SI table has no currency, so `budget = "cost"` requires `dimensionless` (an amount in the model's single currency; the CLI flag keeps its `_usd` name and the report unit `"USD"`) or `unit = "<currency>", custom_unit = "reason"` (which is TBD debt by design). Cost has no dimension, so it can never be `untracked`.
- **Metadata package:** roles are emitted as `@SysmlStdBudget::BudgetRole { role = "..."; }`; the package is named with the `SysmlStd` prefix so it travels with the sysml-std half under `--omit-std` and is re-attached by the CLI like the rest of sysml-std.
- **Exclusions:** only non-abstract, non-variation part definitions of the model's own packages are scanned (with inherited values), so requirement-def attributes never count; `evidence` attributes are skipped entirely; `tbd_value` attributes are never contributions nor `untracked` — when role-tagged they are listed in an extra `unresolved` list (with expression values and units of unknown scale) and make a limited domain `incomplete`. Untagged watts are listed under both `power` and `waste_heat` (same dimension). Unvalued inherited declarations (abstract std bases) are ignored.
- **Domains without a limit** report `status: "pass"` (nothing to check); `ok` only considers limited domains.
- **Latency:** a stage's latency is the sum of its latency-role attributes (s). The old LLM formula `ttft + tokensOut / tokPerS` needed names; a decode time must be its own latency attribute until C2 can evaluate `expr = "tokensOut / tokPerS"`.
- **refute / trade-study (interim until C3):** design-point metrics = mass/power/cost roles, energy = attributes of dimension J, runtime = energy / power. Requirements are mapped structurally (comparison of two requirement attributes; bound dimension mass `<=` / power `<=` / time `>=`); everything else, and any constraint whose metric no point has, is listed under `unevaluated`. Noise has no structural source: `--max-noise-db` is kept for compatibility but reported as `unevaluated`; the noise metric was removed from `TradePoint` and the Pareto dominance.
- **slice:** a part's aspects come from ports declared in the model (not generic ports inherited from std bases such as every element's `heatOut`), std base types, valued attribute dimensions and roles, and safety metadata. Open question: whether connected ports (connection-level item flows) should also classify parts; not needed for the current e2e expectations.

## C1.6 — budget multiplicity (usage-tree rollup)

- **Bug fixed:** budget counted each concrete part *definition* once, so two 170 W fans of one `Fan` definition totalled 170 W. Mass, power, waste heat and cost now roll up over part **usages**: every owned usage is an instance, times a fixed multiplicity (`[n]`, `[n..n]`; Rust `[T; n]` fields emit `[n]`). Trial-3 `ventilation_r2` now totals 340 W (was 170 W); fixture `tests/fixtures/budget_multiplicity/two_fans.sysml` (2 fans + `heaters[2]`) totals 440 W.
- **Roots:** concrete (non-abstract, non-variation) part definitions of the model's own packages that are not the type of an owned (non-`ref`) part usage or a variant anywhere in those packages, **and** not specialized by another user definition (a concrete base such as `PlanetaryGearbox` reaches the tree through its specializations), plus package-level part usages. An `individual part def` (e.g. Apollo's `'LM-5' :> ApolloLunarModule`) is a root only when no non-individual root exists — otherwise it would count the same flight article twice (once as itself, once as `ApolloSpacecraft.lunarModule`). Several roots are summed; they are listed in `roots`.
- **Walk:** inherited part usages come along the specialization chain (most-derived redefinition wins, missing type/multiplicity filled from the redefined usage); usage-level attribute redefinitions (`part f : Fan { attribute :>> ratedPower = 200 [W]; }`) and nested parts override the definition's. `ref` usages are references, not instances (skipped). A usage typed by a **variation** definition is a choice budget does not bind: listed as `unresolved` ("variation X (variant not bound)"). A range/open multiplicity (`[1..*]`) adds its **lower bound** (so `fail` stays sound) and an `unresolved` entry (so `pass` is impossible). Optional recursion (`part subcomponents[*] : MassedComponent` inside `MassedComponent`) is not expanded (zero instances required; `unresolved` if the recursive definition has role values of its own); a mandatory composition cycle is reported in `reasons` and makes limited domains `incomplete`.
- **Uninstantiated:** concrete user definitions that are neither reached nor an ancestor of a reached definition; their role values are listed per domain, never summed, and make a limited domain `incomplete`. **No root at all** (e.g. a composition cycle between every definition) falls back to one count per definition, `rollup: "per_definition"`, with a reason, and limited domains are `incomplete`.
- **Timing** stays per definition (a critical path is not multiplied by instance count). `what-if` inherits the fix (it re-runs `budget`).
- **Example fix:** `power_tools_family` composed `part base : CordlessToolBase` inside each tool that already *is* a `CordlessToolBase` (`:>`), only so Rust `connect` paths could reach `base.motor`; that doubled every tool's platform in SysML. It is now `#[sysml::part(ref)]` (`ref part base`). Workshop totals: 12.03 kg / 7864.5 W (per-definition counting gave 10.02 kg / 4261.5 W).
- **Deferred / open:** (1) `trade-study` / `refute` design points still read only the chosen variant types' own role attributes, once per variation — they do not weight a variation by how many usages of it exist, nor include the variant's children; C3 rebuilds design points on the usage tree + evaluator. (2) Aggregate vs. leaf roles: `power_tools_family` tags each tool's *complete* mass (`mass` role, 1.95 kg) and its children's masses too, so the workshop mass double-counts tool internals (this was also true per definition). Roles carry no "aggregate of children" meaning yet; the C1 convention is `budget = "none"` on aggregates, but trade-study needs the tool total. Decide in C3 (e.g. an `aggregate` role that replaces, rather than adds to, the subtree).

## Delegation notes

- Haiku gets tasks with a fixed contract, exact files, and a worked example; Opus gets design-bearing work. Every delegated task: commit per item, push, `cargo test --workspace` green, never force-push.
- Trial agents follow the canonical trial rules in `HAIKU_TRIAL.md` (incl. own-directory-only).

## T5 corpus (decided)
The owner chose the **PX4 + Pixhawk 6X corpus**, pinned to commit SHAs. Protocol, contamination controls, prerequisites and grading: [`trials/T5_PROTOCOL.md`](trials/T5_PROTOCOL.md). Note: docmem lives in the separate `yoavbendor/cppognee` repo and the current driver script embeds a hand-written alias table, which T5 forbids.
