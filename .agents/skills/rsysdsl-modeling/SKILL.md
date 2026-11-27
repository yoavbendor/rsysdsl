---
name: rsysdsl-modeling
description: Author, extend, and validate SysML v2 system architectures in strongly typed Rust using rsysdsl (Visio diagrams, system specs with provenance, requirements, guarded state machines, calculations, and ISO 26262 safety contracts). Use when creating or crystallizing a system model in this repository or fixing rsysdsl rustc/Open-MBEE audit errors.
---

# `rsysdsl` System Modeling Skill

Use this skill to incrementally build and validate **System as Code** models in Rust (`#[sysml::*]`) that emit OMG-valid `.sysml` for VS Code SysML v2 extensions (`JamieD.sysml-v2-support` / `sysml-reactflow`).

`rsysdsl` is the CLI binary of `crates/sysml-cli`: install it with `cargo install --path crates/sysml-cli` from the repo root, or run it in place as `cargo run -q -p sysml-cli -- <cmd>`.

## Progressive Disclosure — Read Only What You Need

1. **Need copy-pasteable Rust macro templates?** -> Read [`references/PATTERNS.md`](references/PATTERNS.md)
   - Pattern 1: Visio Box-and-Arrow Architecture (IBD-ready)
   - Pattern 2: Requirements, Compiled Constraints & Document Provenance
   - Pattern 3: Functional Actions & `perform action` Allocation
   - Pattern 4: Executable Guarded State Machines
   - Pattern 5: Pure Modeled Calculations (`#[sysml::calc]`)
   - Pattern 6: ISO 26262 Safety Goals, FMEA & ASIL Decomposition
   - Pattern 7: Bare-Metal Ferrocene `#![no_std]` Models
2. **Hit a `rustc` or `rsysdsl audit` error/warning?** -> Read [`references/ERRORS.md`](references/ERRORS.md)
   - Direct diagnostic -> 1-line fix table + allowed OMG SI unit cheat sheet.

## Phase Checklists (Crystallizing a System Model)

### Phase 1: Visio Diagram -> Compiling Architecture Draft
- [ ] Copy [`examples/starter_model/`](../../../examples/starter_model/) (or create a new crate in workspace `Cargo.toml`).
- [ ] Every Visio box is a `#[sysml::part] struct`.
- [ ] Every Visio arrow has a `#[sysml::item]` payload, `#[sysml::port]` (`out_item`/`in_item` or `symmetric`), and `#[sysml::interface]`.
- [ ] One top-level assembly `#[sysml::part]` composes all boxes as `#[sysml::part]` fields and wires every arrow with `#[sysml::connect(...)]`.
- [ ] Only import `ScalarValues::*` / `SI::*` / `ISQ::*` / `ModelingMetadata::*` in packages that use them.
- [ ] Passes `cargo test -p <crate>` + `emit` + `rsysdsl audit --files <out.sysml> --stdlib --strict`.

### Phase 2: Spec Corpus -> Requirements & Provenance
- [ ] Each requirement has a stable `id = "REQ-..."`, concise `///` doc, and `#[sysml::rationale("Source: <doc> §<sec>")]`.
- [ ] Quantitative thresholds use `#[sysml::require_constraint(...)]` over `Quantity<f64>` fields.
- [ ] Every requirement is satisfied by at least one part via `#[sysml::satisfy(req = ..., id = "...")]`.
- [ ] `rsysdsl audit --files <out.sysml> --stdlib --strict` reports `orphan_requirements: []`.
- [ ] `rsysdsl trace-matrix --files <out.sysml> --format markdown` generated and reviewed.
- [ ] Every gap in the source is a TBD with a reason (`tbd_value` / `tbd_unit` / `#[sysml::tbd]` / `connect(tbd = ...)`, PATTERNS Pattern 8), never an invented number or unit; `rsysdsl tbd --files <out.sysml> --format markdown` reviewed.

### Phase 3: Behavior, Modes & Parametric Budgets
- [ ] Functions modeled as `#[sysml::action]` and allocated via `#[sysml::perform(...)]`.
- [ ] Operating modes / fault transitions modeled as `#[sysml::state(subject = ...)] pub enum`.
- [ ] Budgets / engineering formulas modeled as `#[sysml::calc] pub fn`.
- [ ] Unit tests execute `evaluate()`, `dispatch_with()` (`Ignored` for unmatched events) / `step_with()` (`Err` for them), and `#[sysml::calc]` directly.

### Phase 4: ISO 26262 Safety & Simulation (When Applicable)
- [ ] Top-level safety goals annotated with `#[sysml::safety_goal(asil = ..., safe_state = "...", ftti_ms = ...)]`.
- [ ] Failure modes documented via `#[sysml::fmea(...)]` (ratings `1..=10`).
- [ ] Dual-channel decompositions verified at compile time with `#[sysml::independent(...)]` + `#[sysml::asil_decompose(...)]`.
- [ ] `iso26262_metadata_package()` included in `Model::new(...)` so `@SafetyGoal`/`@FMEA`/`@AsilDecomposition` validate with 0 warnings.
