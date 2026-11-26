# Apollo 11 SysML v2 Coverage & Roadmap (`COVERAGE.md`)

This report audits `rsysdsl` against all **28 `.sysml` files (7,163 lines)** in Airbus Central R&T's [`airbus/apollo-11-sysml-v2`](https://github.com/airbus/apollo-11-sysml-v2) model (pinned at commit `0e8a7d6790cfa7e15b991eb745e9a38f9bed0474`).

Every construct encountered during lifting is either faithfully represented in `sysml-ir` and re-emitted deterministically, or explicitly logged in `LiftReport.unsupported` — **zero silent drops**.

## Summary Metrics (Full 28-File Airbus Apollo 11 Corpus)

Reproducible via:
```bash
cargo run -q -p sysml-cli -- lift tests/fixtures/apollo-11-sysml-v2/*.sysml tests/fixtures/apollo-11-sysml-v2/*/*.sysml | jq '.stats'
```

| Category | Count Lifted | Round-Trip Status |
| --- | ---: | --- |
| Files / Packages (`package`, `library package`) | 28 / 28 | 100% lossless, 0 parse errors |
| Imports (`private import ...::*` / `::**`) | 119 | 100% lossless |
| `comment` (`comment /* ... */`) | 3 | 100% lossless |
| `part def` (incl. `abstract`, `individual`, `:>`) | 194 | 100% lossless |
| `part` usages (incl. multiplicity, `ordered`, `:>`, `:>>`, nested items/reqs) | 208 | 100% lossless |
| `attribute` usages (typed values, units, `:>>`, `:>`, `<short>`, `nonunique`, nested) | 446 | 100% lossless |
| `port def` & `port` usages (incl. `~` conjugation) | 8 / 20 | 100% lossless |
| `interface def` & `connection def` & `interface`/`connection` usages | 8 / 1 / 36 | 100% lossless |
| `item def` & `enum def` | 25 / 2 | 100% lossless |
| `action def` & `perform action` allocations | 110 / 91 | 100% lossless |
| `state def` & `transition`s | 16 / 14 | 100% lossless |
| `requirement def` (`<'ID'>`, `@Rationale`, `require constraint`, `#refinement`) | 296 | 100% lossless |
| `requirement` usages (specification groups & stakeholder needs) | 293 | 100% lossless |
| `satisfy` traceability links | 272 | 100% lossless |
| `#refinement dependency` links on `part def`s (`CapabilitiesPackage`) | 42 | 100% lossless |
| `calc def` & `calc` usages (`CalculationsPackage`, `CoSMAQuantitiesAndUnitsPackage`) | 15 | 100% lossless |
| `analysis def` & `analysis` usages (`AnalysisPackage`) | 3 / 5 | 100% lossless |
| 4D temporal `timeslice` & `snapshot` portions (`Apollo11MissionExecutionPackage`) | 73 | 100% lossless |
| Diagram `view` usages (`CoSMAViewsPackage`) | 1 | 100% lossless |
| **Total Core MBSE Elements Lifted** | **2,330** | **2,916 / 2,916 semantic facts matched (0 missing / 0 extra)** |
| **Explicitly Logged Unsupported Constructs** | **0** | **100.0% corpus coverage (28 / 28 files at 0 unsupported)** |

### Compiled End-to-End Round-Trip Vertical Slice (Gate 3)

The 13-package vertical slice spanning all 5 CoSMA layers plus Capabilities, Calculations, and Analysis (`CoSMAPackage`, `OperationsPackage`, `CapabilitiesPackage`, `MissionPhasesPackage`, `FunctionsPackage`, `LogicalComponentsPackage`, `TechnicalPortsPackage`, `TechnicalComponentsPackage`, `MissionRequirementsPackage`, `TechnicalRequirementsPackage`, `SystemSpecificationPackage`, `CalculationsPackage`, `AnalysisPackage`) executes the full pipeline with **0 unsupported constructs**:

```text
Apollo 11 .sysml (13 files, 0 unsupported)
  -> sysml-lift
  -> Generated Rust DSL (#[sysml::*], incl. #[sysml::calc] & #[sysml::analysis])
  -> cargo run (rustc compile-time type checks + execution)
  -> sysml-emit
  -> Re-emitted .sysml
  -> Open-MBEE sysml-toolkit validation (0 errors)
  -> Semantic AST/graph diff: 100% match (0 missing, 0 extra)
```

## Per-File Apollo 11 Coverage Breakdown (28 / 28 Files at 0 Unsupported)

| Apollo 11 File | Lifted Elements | Logged Unsupported | Status |
| --- | ---: | ---: | --- |
| `Apollo11Model.sysml` | 0 | 0 | 100% (root aggregator package with imports) |
| `Analysis/AnalysisPackage.sysml` | 8 | 0 | 100% |
| `Analysis/CalculationsPackage.sysml` | 14 | 0 | 100% |
| `CoSMA/CoSMAPackage.sysml` | 51 | 0 | 100% |
| `CoSMA/CoSMAQuantitiesAndUnitsPackage.sysml` | 20 | 0 | 100% (custom ISQ units, `nonunique`, `calc <ln>`) |
| `CoSMA/CoSMAViewsPackage.sysml` | 1 | 0 | 100% (`view RequirementsTableView`) |
| `Execution/Apollo11MissionExecutionPackage.sysml` | 252 | 0 | 100% (14 `timeslice` + 59 `snapshot` 4D timeline) |
| `Function/FunctionsPackage.sysml` | 75 | 0 | 100% |
| `Function/FunctionSpecificationPackage.sysml` | 225 | 0 | 100% |
| `Logical/LogicalComponentsPackage.sysml` | 59 | 0 | 100% |
| `Operation/OperationsPackage.sysml` | 39 | 0 | 100% |
| `Program/ProgramPackage.sysml` | 52 | 0 | 100% |
| `Purpose/CapabilitiesPackage.sysml` | 67 | 0 | 100% |
| `Purpose/ContextPackage.sysml` | 35 | 0 | 100% |
| `Purpose/MissionPackage.sysml` | 85 | 0 | 100% |
| `Purpose/MissionPhasesPackage.sysml` | 28 | 0 | 100% |
| `Purpose/MissionSpecificationPackage.sysml` | 92 | 0 | 100% |
| `Purpose/StakeholderPackage.sysml` | 65 | 0 | 100% |
| `Requirements/FunctionalRequirementsPackage.sysml` | 112 | 0 | 100% |
| `Requirements/MissionRequirementsPackage.sysml` | 134 | 0 | 100% |
| `Requirements/StakeholderNeedsPackage.sysml` | 22 | 0 | 100% |
| `Requirements/TechnicalRequirementsPackage.sysml` | 200 | 0 | 100% |
| `Technical/AstronautsPackage.sysml` | 89 | 0 | 100% |
| `Technical/SystemPackage.sysml` | 7 | 0 | 100% |
| `Technical/SystemSpecificationPackage.sysml` | 219 | 0 | 100% |
| `Technical/TechnicalComponentsPackage.sysml` | 143 | 0 | 100% |
| `Technical/TechnicalIndividualsPackage.sysml` | 66 | 0 | 100% |
| `Technical/TechnicalPortsPackage.sysml` | 21 | 0 | 100% |

## Completed Milestones (Sprints 1–5)

1. **Anti-Semantic-Leakage 3-Tier Architecture (Sprint 1 — Completed):**
   - `#[sysml::package]` rejects raw `fn`/`impl`/unannotated structs at `rustc` compile time; auto-generated `Default` from `#[sysml::attr(default = ...)]`; single-source compiled `#[sysml::require_constraint(...)]`, guarded `#[sysml::transition(..., guard = ...)]`, and pure `#[sysml::calc] fn`; one-way `apollo11-sim -> apollo11-model` crate split.
2. **ASPICE Traceability Matrix & 0-Warning OMG Stdlib Validation (Sprint 2 — Completed):**
   - `rsysdsl trace-matrix` (JSON/Markdown/CSV over Rust models or raw `.sysml` files) and `rsysdsl audit --stdlib --strict` validating with 0 errors and 0 warnings against `sysml.library`.
3. **Apollo 11 Corpus Expansion to 98.5% (Sprint 3 — Completed):**
   - Added `#[sysml::calc]`, `#[sysml::analysis]`, `#refinement` dependencies, keyword-less feature bindings, and expanded the compiled Gate 3 slice to 13 packages.
4. **ISO 26262 Safety Contracts & Ferrocene `#![no_std]` Compatibility (Sprint 4 — Completed):**
   - Zero-dependency `#![no_std]` [`crates/sysml-core`](crates/sysml-core) with compile-time ISO 26262-9 Table 1 ASIL decomposition (`AsilDecomposable<L, R>`) and freedom-from-interference independence witnesses (`IndependentFrom<Other>`).
   - `#[sysml::safety_goal]`, `#[sysml::fmea]`, `#[sysml::independent]`, `#[sysml::asil_decompose]`, and `iso26262_metadata_package()` emitting `@SafetyGoal`, `@FMEA`, and `@AsilDecomposition` with 0-error / 0-warning OMG stdlib validation.
   - Ferrocene-ready `#![no_std]` [`examples/embedded_flight_controller`](examples/embedded_flight_controller) (`cargo check -p embedded-flight-controller --no-default-features` pulls zero `std`/`alloc`/`serde` target dependencies).
5. **100.0% Apollo 11 Coverage, Safety/Cut-Set Analysis & LLM Authoring Kit (Sprint 5 — Completed):**
   - Reached **100.0% corpus coverage (28 / 28 Airbus Apollo 11 files, 0 unsupported constructs, 2,916 / 2,916 semantic facts matched)** by lifting 4D `timeslice`/`snapshot` execution timelines, custom ISQ quantity/unit definitions (`nonunique`, `<short>`, nested attributes, `calc` usages), and `view` usages.
   - Added ISO 26262 Safety, FMEA RPN ranking & minimal fault cut-set analysis (`rsysdsl safety-report --model embedded-flight-controller` and `--files <model.sysml>`, JSON/Markdown) plus `@SafetyGoal`/`@FMEA`/`@AsilDecomposition` metadata lifting.
   - Added lean [`AGENTS.md`](AGENTS.md), progressive-disclosure [`.agents/skills/rsysdsl-modeling/`](.agents/skills/rsysdsl-modeling/SKILL.md), [`examples/starter_model/`](examples/starter_model/), `rsysdsl audit --files`, and air-gapped [`scripts/vendor-offline.sh`](scripts/vendor-offline.sh).

## Next Milestones (Future)

- **SMT / Z3 Constraint Solving & Fuzzing:** Integrate Open-MBEE's `sysmlv2-solve` (Z3) alongside Rust `proptest` to formally prove and fuzz `require constraint { ... }` predicates.
- **Knowledge-Graph Export:** Optional `nomograph-ai/sysml` graph export for multi-hop AI agent queries across large multi-crate system models.
