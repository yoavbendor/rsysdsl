# `rsysdsl` Agent Authoring Guide (`AGENTS.md`)

You are authoring a **System as Code** model in **`rsysdsl`** — a strongly typed Rust DSL that compiles into **OMG-valid SysML v2 (`.sysml`)** and validates against the official OMG Standard Library (`sysml.library`).

> **Core idea:** You write **idiomatic Rust** (`struct`, `enum`, `#[sysml::*]`). The `rustc` compiler enforces ports, connections, specializations, requirement IDs, state targets, and ISO 26262 safety rules at compile time. `sysml-emit` deterministically generates valid `.sysml` for VS Code viewers (`JamieD.sysml-v2-support` / `sysml-reactflow`).

---

## 1. Golden Rules (Non-Negotiable)

1. **Author ONLY Rust in `src/` — NEVER hand-edit `.sysml`.** `.sysml` is a compiled build artifact.
2. **No semantic leakage inside `#[sysml::package]`:** Raw `fn`, `impl`, `static`, `trait`, or unannotated `struct`/`enum` inside a `#[sysml::package]` module are **compile errors**.
   - Use `#[sysml::attr(default = ..., unit = "...")]` for parameter values (`impl Default` is auto-generated).
   - Use `#[sysml::require_constraint(expr)]` for requirement predicates.
   - Use `#[sysml::transition(on = Event, guard = expr, to = State)]` for guarded transitions.
   - Use `#[sysml::calc] pub fn ...` for pure calculations.
   - Put physics loops, test harnesses, or fault injectors in a separate `<system>_sim` crate (or `#[cfg(test)]`).
3. **Never invent requirements, thresholds, or limits** that are not in the user's diagram or documents. If a value is unknown, mark it clearly in `///` doc or omit the constraint until the source document specifies it. This includes numbers inherited from `sysml-std` parents (library placeholders such as 50 Hz mains or 48 MHz MCU clock): redefine those your source specifies; `rsysdsl lint` lists the rest as `inherited_std_default`.
4. **Never finish without running the verification loop** (`cargo test` -> emit `.sysml` -> `rsysdsl audit --files ... --stdlib --strict`).

---

## 2. Quickstart: Creating & Verifying a New System Model

Scaffold every new system with `rsysdsl new`. It generates a compiling crate with the layered package structure (`<Name>Types` -> `<Name>Requirements` -> `<Name>Subsystems` -> `<Name>Context`; `#[sysml::satisfy]` goes on the realizing subsystem part), an emit binary and audit/lint tests, and registers it in the root `Cargo.toml`:

```bash
# 0. Install the CLI once as `rsysdsl` (from the repo root; re-run after pulling changes).
#    Without installing, `cargo run -q -p sysml-cli -- <cmd>` from the repo root is equivalent.
cargo install --path crates/sysml-cli

# 1. Scaffold examples/my_system (package `my-system`); refuses to overwrite an existing directory.
#    WARNING: without --dir this writes INTO THIS REPO's examples/ and edits the root Cargo.toml.
#    Options: --dir <path>, --no-workspace (standalone crate, not added to root Cargo.toml members)
rsysdsl new my_system
# Standalone model outside the repo: `rsysdsl new my_system --no-workspace --dir ../my_system`.
#   Its Cargo.toml has its own [workspace] and ABSOLUTE path deps (sysml-dsl, sysml-std, sysml-emit)
#   on this checkout (update them if the checkout moves). It is not a workspace member, so `-p`
#   does not work: run steps 2-3 inside the crate (`cd ../my_system && cargo test`) or with
#   `--manifest-path ../my_system/Cargo.toml`; the JSON printed by `rsysdsl new` lists the exact commands.
# Alternative: copy the single-package starter template (examples/starter_model/), add
# "examples/my_system" to root Cargo.toml members, and rename the package in Cargo.toml + src/bin/emit.rs

# 2. Fast compile + unit tests (catches port, connection, specialization, and constraint bugs)
cargo test -p my-system

# 3. Emit OMG-valid .sysml for VS Code SysML v2 / sysml-reactflow
#    (--omit-std keeps the sysml-std library packages out of the file; audit/lint/view load them)
cargo run -q -p my-system --bin emit -- -o target/my_system.sysml --omit-std

# 4. Strict 0-error / 0-warning OMG Standard Library audit + orphan requirement check
rsysdsl audit --files target/my_system.sysml --stdlib --strict --pretty

# 5. Export ASPICE bidirectional traceability matrix (Markdown or CSV)
rsysdsl trace-matrix --files target/my_system.sysml --format markdown

# 6. Check architecture & hierarchy smells (duplicate parts, unclassified interfaces, flat assemblies)
rsysdsl lint --files target/my_system.sysml --strict --format markdown

# 7. Generate a standalone interactive offline HTML viewer (opens directly in Chrome/Edge; 0 VS Code extensions or internet)
rsysdsl view target/my_system.sysml --view interconnection -o target/my_system.html
```

---

## 3. MBSE & Visio -> `rsysdsl` Concept Map

You do **NOT** need to write raw SysML v2 textual syntax. Prefer specializing from **`sysml-std`** (`use sysml_std::prelude::*;` + prepend `sysml_std::std_packages()` in `Model::new(...)`) instead of reinventing common flows, comms, power, mechanics, electronics, HMI, or natural bodies!

| System / Visio Concept | `rsysdsl` Rust Construct | Key Rule |
| --- | --- | --- |
| **Diagram / Layer** | `#[sysml::package(name = "MyPkg")] pub mod my_pkg { ... }` | Put `#[sysml::import("ScalarValues::*")]`, `#[sysml::import("SI::*")]`, `#[sysml::import("SysmlStdFlows::*")]`, etc. ONLY when used in that package. |
| **Abstract Base Type** | `#[sysml::part(abstract)]`, `#[sysml::port(abstract)]`, `#[sysml::interface(abstract)]` | Extract shared ports/attributes into an abstract base (`abstract part def`). |
| **Subsystem Blueprint** *(Definition)* | `#[sysml::part] pub struct Controller { ... }` | Reusable type (`part def`). |
| **Subsystem Instance** *(Usage — box inside box)* | `#[sysml::part] pub ctrl: Controller` *(field inside assembly struct)* | Owned child instance (`part`). |
| **Inheritance & Redefinition** (`is-a` / `:>>`) | `#[sysml::specializes(BasePart)]` + `#[sysml::attr(redefines = "field", default = ...)]` + `#[sysml::port(redefines)]` | `rustc` checks ancestor chains at compile time! Use `#[sysml::port(redefines)]` to reuse inherited parent ports with 0 Open-MBEE warnings. |
| **Product-Line Variants** (`variation`/`variant`) | `#[sysml::variation(of = BasePart)] pub enum MyOpt { A(SubA), B(SubB) }` | `rustc` verifies every variant payload specializes `BasePart`. |
| **Logical -> Physical Allocation** | `#[sysml::allocate(name = "fnToHw", from = logical, to = hw)]` | Placed on parent assembly; `rustc` checks both dotted paths. |
| **Arrow Payload** (signal/fluid/power) | `#[sysml::item] pub struct TempReading { ... }` | Derive `Debug, Clone, Copy, Default`. Prefer `SysmlStdFlows::*`. |
| **Arrow Endpoint** (Port) | `#[sysml::port]` with `#[sysml::out_item]` / `#[sysml::in_item]` / `#[sysml::inout_item]` | Directed ports MUST connect `Port` <-> `Conjugated<Port>`. Use `#[sysml::port(symmetric)]` for bidirectional buses (CAN/I2C/Ethernet). |
| **Arrow Between Boxes** | `#[sysml::interface]` + `#[sysml::connect(interface = Bus, name = "...", from = a.p_out, to = b.p_in)]` | Placed on the parent assembly struct; `rustc` checks field names and port compatibility! |
| **Parameter / Property** | `#[sysml::attr(default = 50.0, unit = "W")] pub power: Quantity<f64>` | Use base SI units (`s`, `m`, `kg`, `N`, `W`, `V`, `A`, `K`, `Hz`, `Pa`, `J`); composites in ASCII (`m/s`, `m^3/s`, `N*m`, `J/(kg*K)`). Unknown / non-OMG units (`kN`, `ms`, `%`) and dimension-mismatched constraints/guards are compile errors. |
| **Requirement + Provenance** | `#[sysml::requirement(id = "REQ-001")]` + `#[sysml::rationale("Source: Doc §3.2")]` + `#[sysml::require_constraint(...)]` | Link every requirement via `#[sysml::satisfy(req = MyReq, id = "REQ-001")]` on a part (0 orphans!), also across packages. Limits get `default = <spec value>`; measured values get `#[sysml::attr(unit = "...", evidence)]` with **no default** (never copy the limit). |
| **Function / Behavior** | `#[sysml::action]` + `#[sysml::perform(name: ActionTy)]` on a part | Allocates functions to logical/physical parts. |
| **Operating Modes / Faults** | `#[sysml::state(subject = Assembly)] pub enum Mode { ... }` | Variants = states; `#[sysml::initial]` marks start; `#[sysml::transition(on = Evt, guard = ..., to = Target)]`. |
| **Formula / Budget** | `#[sysml::calc] pub fn margin(limit: f64, actual: f64) -> f64 { limit - actual }` | Only pure arithmetic/comparisons & calls to other `#[sysml::calc]` fns. Reuse `SysmlStdFlows` physics calcs! |
| **ISO 26262 Safety** | `#[sysml::safety_goal(...)]`, `#[sysml::fmea(...)]`, `#[sysml::independent(...)]`, `#[sysml::asil_decompose(...)]` | Include `iso26262_metadata_package()` in `Model::new(...)` and `#[sysml::import("ISO26262Metadata::*")]`. |

---

## 4. Reading Visio Inputs

- **PNG / SVG image:** List every box (subsystem), nested container (parent assembly), and directed/bidirectional arrow (payload + source -> target) before writing Rust.
- **`.vsdx` file:** A `.vsdx` is a ZIP archive of XML pages. Extract shape labels and connectors quickly with:
  ```bash
  python3 -c "import zipfile, xml.etree.ElementTree as ET, sys
  with zipfile.ZipFile(sys.argv[1]) as z:
    for n in sorted(z.namelist()):
      if n.startswith('visio/pages/page') and n.endswith('.xml'):
        root = ET.fromstring(z.read(n))
        texts = [' '.join(t.itertext()).strip() for t in root.iter() if t.tag.endswith('}Text')]
        print(n, [t for t in texts if t])" diagram.vsdx
  ```
- **Text / Mermaid / Spec docs:** Map each entity/arrow/requirement directly using Section 3.

---

## 5. Incremental Crystallization Protocol (5 Phases)

Always build the system in **compiling, auditable increments** — never try to model an entire corpus in one giant pass:

1. **Phase 1 — High-Level Skeleton (Visio -> IBD):**
   - Create 1–2 packages (`Architecture` or `TypesAndPorts` + `Architecture`) importing `sysml-std` packages (`SysmlStdFlows`, `SysmlStdComms`, `SysmlStdPower`, `SysmlStdMechanics`, `SysmlStdElectronics`, `SysmlStdHmi`, `SysmlStdNatural`).
   - Specialize `sysml-std` bases (`PowerSource`, `ElectricMotor`, `Microcontroller`, `SensorBase`, `HmiPanel`, `ThermalEnclosure`, `AmbientEnvironment`, etc.) and wire arrows via `#[sysml::connect]`.
   - Always include **one top-level context `#[sysml::part]`** composing all top-level boxes so `rsysdsl view` renders a clean connected Internal Block Diagram (IBD).
   - **Gate:** `cargo test` + `emit` + `rsysdsl audit --files ... --stdlib --strict` -> **0 errors, 0 warnings**.
2. **Phase 1.5 — Refactor & Generalize (`rsysdsl lint`):**
   - Run `rsysdsl lint --files target/my_system.sysml --strict --format markdown`.
   - Refactor copy-paste parts into `#[sysml::part(abstract)]` parents + `#[sysml::specializes]` + `#[sysml::variation]`, classify custom interfaces under an end-less abstract base (`#[sysml::interface(abstract)] pub struct EnergyChannel {}` + `#[sysml::specializes(EnergyChannel)]`; never strip interfaces from connects), and break flat >10-part God-assemblies into domain subsystems (`Power`, `Electronics`, `Mechanics`, `HMI`).
   - **Gate:** `rsysdsl lint --strict` -> **CLEAN (0 warnings)**.
3. **Phase 2 — Requirements & Document Provenance:**
   - Add a `Requirements` package with `#[sysml::requirement(id = "...")]`.
   - Record source traceability on every requirement using `#[sysml::rationale("Source: <doc>, <section/page>")]` and `///` doc comments.
   - Add `#[sysml::require_constraint(...)]` for quantitative limits and `#[sysml::satisfy(req = ..., id = "...")]` on realizing parts.
   - **Gate:** `audit --stdlib --strict` -> **0 orphan requirements** + `trace-matrix --format markdown` shows 100% linkage.
4. **Phase 3 — Functions, Modes & Calculations:**
   - Add `#[sysml::action]` + `#[sysml::perform(...)]`, executable guarded `#[sysml::state]` enums, and `#[sysml::calc]` formulas (reusing `SysmlStdFlows` physical laws).
   - **Gate:** Unit tests exercise `evaluate()`, `step_with()`, and `#[sysml::calc]` functions; `audit --stdlib --strict` stays at 0 errors / 0 warnings.
5. **Phase 4 — ISO 26262 Safety & Closed-Loop Simulation (Optional):**
   - Add `#[sysml::safety_goal]`, `#[sysml::fmea]`, `#[sysml::independent]`, `#[sysml::asil_decompose]`, and (if needed) a separate `<system>_sim` crate.

---

## 6. On-Demand Reference Skill & Examples (Load Only When Needed)

Keep your context small — only read these files when you need copy-pasteable templates or hit a compiler/validator error:

- **Skill overview & phase checklists:** [`.agents/skills/rsysdsl-modeling/SKILL.md`](.agents/skills/rsysdsl-modeling/SKILL.md)
- **Copy-pasteable Rust DSL patterns (IBD, Provenance, States, Calcs, ISO 26262, `no_std`):** [`.agents/skills/rsysdsl-modeling/references/PATTERNS.md`](.agents/skills/rsysdsl-modeling/references/PATTERNS.md)
- **Compiler & Open-MBEE error -> 1-line fix + SI unit table:** [`.agents/skills/rsysdsl-modeling/references/ERRORS.md`](.agents/skills/rsysdsl-modeling/references/ERRORS.md)
- **Universal Standard Library:** [`crates/sysml-std/`](crates/sysml-std/)
- **Working reference examples:** [`examples/starter_model/`](examples/starter_model/), [`examples/solar_system/`](examples/solar_system/), [`examples/earth_water_cycle/`](examples/earth_water_cycle/), [`examples/bicycle/`](examples/bicycle/), [`examples/power_tools_family/`](examples/power_tools_family/), [`examples/dishwasher/`](examples/dishwasher/), [`examples/microwave/`](examples/microwave/), [`examples/apollo11_model/`](examples/apollo11_model/), [`examples/embedded_flight_controller/`](examples/embedded_flight_controller/)
