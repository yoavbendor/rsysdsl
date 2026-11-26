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
| `part` usages (incl. multiplicity, `ordered`, `:>`, `:>>`, nested items/reqs) | 148 | 100% lossless |
| `attribute` usages (typed values, units, `:>>`, `:>`, keyword-less bindings) | 351 | 100% lossless |
| `port def` & `port` usages (incl. `~` conjugation) | 8 / 20 | 100% lossless |
| `interface def` & `connection def` & `interface`/`connection` usages | 8 / 1 / 36 | 100% lossless |
| `item def` & `enum def` | 25 / 2 | 100% lossless |
| `action def` & `perform action` allocations | 110 / 53 | 100% lossless |
| `state def` & `transition`s | 16 / 14 | 100% lossless |
| `requirement def` (`<'ID'>`, `@Rationale`, `require constraint`, `#refinement`) | 296 | 100% lossless |
| `requirement` usages (specification groups & stakeholder needs) | 293 | 100% lossless |
| `satisfy` traceability links | 272 | 100% lossless |
| `#refinement dependency` links on `part def`s (`CapabilitiesPackage`) | 42 | 100% lossless |
| `calc def` parametric calculations (`CalculationsPackage`) | 14 | 100% lossless |
| `analysis def` & `analysis` usages (`AnalysisPackage`) | 3 / 5 | 100% lossless |
| **Total Core MBSE Elements Lifted** | **2,061** | **2,693 / 2,693 semantic facts matched (0 missing / 0 extra)** |
| **Explicitly Logged Unsupported Constructs** | **32** | **98.5% corpus coverage (25 / 28 files at 0 unsupported)** |

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

## Per-File Apollo 11 Coverage Breakdown

| Apollo 11 File | Lifted Elements | Logged Unsupported | Unsupported Construct Kinds |
| --- | ---: | ---: | --- |
| `Apollo11Model.sysml` | 0 | 0 | — (root aggregator package with imports only) |
| `Analysis/AnalysisPackage.sysml` | 8 | 0 | — |
| `Analysis/CalculationsPackage.sysml` | 14 | 0 | — |
| `CoSMA/CoSMAPackage.sysml` | 51 | 0 | — |
| `Function/FunctionsPackage.sysml` | 75 | 0 | — |
| `Function/FunctionSpecificationPackage.sysml` | 225 | 0 | — |
| `Logical/LogicalComponentsPackage.sysml` | 59 | 0 | — |
| `Operation/OperationsPackage.sysml` | 39 | 0 | — |
| `Program/ProgramPackage.sysml` | 52 | 0 | — |
| `Purpose/CapabilitiesPackage.sysml` | 67 | 0 | — |
| `Purpose/ContextPackage.sysml` | 35 | 0 | — |
| `Purpose/MissionPackage.sysml` | 85 | 0 | — |
| `Purpose/MissionPhasesPackage.sysml` | 28 | 0 | — |
| `Purpose/MissionSpecificationPackage.sysml` | 92 | 0 | — |
| `Purpose/StakeholderPackage.sysml` | 65 | 0 | — |
| `Requirements/FunctionalRequirementsPackage.sysml` | 112 | 0 | — |
| `Requirements/MissionRequirementsPackage.sysml` | 134 | 0 | — |
| `Requirements/StakeholderNeedsPackage.sysml` | 22 | 0 | — |
| `Requirements/TechnicalRequirementsPackage.sysml` | 200 | 0 | — |
| `Technical/AstronautsPackage.sysml` | 89 | 0 | — |
| `Technical/SystemPackage.sysml` | 7 | 0 | — |
| `Technical/SystemSpecificationPackage.sysml` | 219 | 0 | — |
| `Technical/TechnicalComponentsPackage.sysml` | 143 | 0 | — |
| `Technical/TechnicalIndividualsPackage.sysml` | 66 | 0 | — |
| `Technical/TechnicalPortsPackage.sysml` | 21 | 0 | — |
| `Execution/Apollo11MissionExecutionPackage.sysml` | 6 | 14 | `portion:Timeslice(14)` (temporal `timeslice` / `snapshot` blocks) |
| `CoSMA/CoSMAQuantitiesAndUnitsPackage.sysml` | 0 | 17 | `top_usage:Attribute(16)`, `top_usage:Calc(1)` |
| `CoSMA/CoSMAViewsPackage.sysml` | 0 | 1 | `top_usage:View(1)` |

## Explicitly Unsupported Constructs Remaining (32 Total — Logged, Never Silently Dropped)

| Construct Kind | Count in Apollo 11 | Target Sprint |
| --- | ---: | --- |
| `top_usage:Attribute` / `top_usage:Calc` | 17 | Sprint 5 — Custom ISQ quantity & unit algebra definitions (`CoSMAQuantitiesAndUnitsPackage`). |
| `portion:Timeslice` | 14 | Sprint 5 — 4D temporal execution timelines (`timeslice` / `snapshot` in `Apollo11MissionExecutionPackage`). |
| `top_usage:View` | 1 | Sprint 5 — Diagram rendering `view` definitions (`CoSMAViewsPackage`). |

## Completed Milestones (Sprints 1–4)

1. **Anti-Semantic-Leakage 3-Tier Architecture (Sprint 1 — Completed):**
   - `#[sysml::package]` rejects raw `fn`/`impl`/unannotated structs at `rustc` compile time; auto-generated `Default` from `#[sysml::attr(default = ...)]`; single-source compiled `#[sysml::require_constraint(...)]`, guarded `#[sysml::transition(..., guard = ...)]`, and pure `#[sysml::calc] fn`; one-way `apollo11-sim -> apollo11-model` crate split.
2. **ASPICE Traceability Matrix & 0-Warning OMG Stdlib Validation (Sprint 2 — Completed):**
   - `rsysdsl trace-matrix` (JSON/Markdown/CSV over Rust models or raw `.sysml` files) and `rsysdsl audit --stdlib --strict` validating with 0 errors and 0 warnings against `sysml.library`.
3. **Apollo 11 Corpus Expansion to 98.5% (Sprint 3 — Completed):**
   - Reduced unsupported constructs from 273 -> 32 (**25 / 28 files at 0 unsupported**, **2,693 / 2,693 semantic facts matched**), adding `#[sysml::calc]`, `#[sysml::analysis]`, `#refinement` dependencies, keyword-less feature bindings, and expanding the compiled Gate 3 slice to 13 packages.
4. **ISO 26262 Safety Contracts & Ferrocene `#![no_std]` Compatibility (Sprint 4 — Completed):**
   - Zero-dependency `#![no_std]` [`crates/sysml-core`](crates/sysml-core) with compile-time ISO 26262-9 Table 1 ASIL decomposition (`AsilDecomposable<L, R>`) and freedom-from-interference independence witnesses (`IndependentFrom<Other>`).
   - `#[sysml::safety_goal]`, `#[sysml::fmea]`, `#[sysml::independent]`, `#[sysml::asil_decompose]`, and `iso26262_metadata_package()` emitting `@SafetyGoal`, `@FMEA`, and `@AsilDecomposition` with 0-error / 0-warning OMG stdlib validation.
   - Ferrocene-ready `#![no_std]` [`examples/embedded_flight_controller`](examples/embedded_flight_controller) (`cargo check -p embedded-flight-controller --no-default-features` pulls zero `std`/`alloc`/`serde` target dependencies).

## Next Milestones (Sprint 5+)

1. **Remaining 32 Apollo 11 Constructs (98.5% -> 100%):**
   - Lift `timeslice` / `snapshot` 4D timelines -> executable scenario test harnesses (`Apollo11MissionExecutionPackage`, 14 constructs).
   - Lift top-level custom ISQ quantity & unit algebra definitions (`CoSMAQuantitiesAndUnitsPackage`, 17 constructs) and diagram `view` definitions (`CoSMAViewsPackage`, 1 construct).
2. **Fault-Tree / Cut-Set Extraction & Dual-Oracle SMT Integration:**
   - Extract minimal cut sets directly from `#[sysml::fmea]` + `#[sysml::state]` fault-mode transitions.
   - Integrate Open-MBEE's `sysmlv2-solve` (Z3) alongside Rust property-based testing (`proptest`) to both prove and fuzz `require constraint { ... }` predicates.
   - Optional `nomograph-ai/sysml` knowledge-graph export for multi-hop agent queries over Rust-authored systems.
3. **LLM-Guided Authoring & Corporate Workflow (`AGENTS.md` + `starter_model`):**
   - Dedicated `AGENTS.md` guide and starter scaffold enabling cost-aware coding agents (Cursor-CLI) to draft systems from diagrams/specs and view `.sysml` in VS Code SysML v2 extensions.
