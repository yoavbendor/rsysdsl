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
3. **Never invent requirements, thresholds, or limits** that are not in the user's diagram or documents. If a value is unknown, mark it clearly in `///` doc or omit the constraint until the source document specifies it.
4. **Never finish without running the verification loop** (`cargo test` -> emit `.sysml` -> `rsysdsl audit --files ... --stdlib --strict`).

---

## 2. Quickstart: Creating & Verifying a New System Model

Copy the minimal template crate [`examples/starter_model/`](examples/starter_model/) to start any new system:

```bash
# 1. Copy starter template (e.g., to examples/my_system) and add "examples/my_system" to root Cargo.toml
cp -r examples/starter_model examples/my_system
# Rename package name in examples/my_system/Cargo.toml and crate name in src/bin/emit.rs

# 2. Fast compile + unit tests (catches port, connection, specialization, and constraint bugs)
cargo test -p my-system

# 3. Emit OMG-valid .sysml for VS Code SysML v2 / sysml-reactflow
cargo run -q -p my-system --bin emit -- -o target/my_system.sysml

# 4. Strict 0-error / 0-warning OMG Standard Library audit + orphan requirement check
cargo run -q -p sysml-cli -- audit --files target/my_system.sysml --stdlib --strict --pretty

# 5. Export ASPICE bidirectional traceability matrix (Markdown or CSV)
cargo run -q -p sysml-cli -- trace-matrix --files target/my_system.sysml --format markdown

# 6. Generate a standalone interactive offline HTML viewer (opens directly in Chrome/Edge; 0 VS Code extensions or internet)
cargo run -q -p sysml-cli -- view target/my_system.sysml --view interconnection -o target/my_system.html
```

---

## 3. MBSE & Visio -> `rsysdsl` Concept Map

You do **NOT** need to write raw SysML v2 textual syntax. Think in these 9 MBSE concepts:

| System / Visio Concept | `rsysdsl` Rust Construct | Key Rule |
| --- | --- | --- |
| **Diagram / Layer** | `#[sysml::package(name = "MyPkg")] pub mod my_pkg { ... }` | Put `#[sysml::import("ScalarValues::*")]`, `#[sysml::import("SI::*")]`, `#[sysml::import("ModelingMetadata::*")]` ONLY when used in that package. |
| **Subsystem Blueprint** *(Definition)* | `#[sysml::part] pub struct Controller { ... }` | Reusable type (`part def`). |
| **Subsystem Instance** *(Usage — box inside box)* | `#[sysml::part] pub ctrl: Controller` *(field inside assembly struct)* | Owned child instance (`part`). |
| **Type Inheritance** (`is-a`) | `#[sysml::specializes(BasePart)]` | Target must be the same SysML kind (`part` -> `part`). |
| **Arrow Payload** (signal/fluid/power) | `#[sysml::item] pub struct TempReading { ... }` | Derive `Debug, Clone, Copy, Default`. |
| **Arrow Endpoint** (Port) | `#[sysml::port]` with `#[sysml::out_item]` / `#[sysml::in_item]` / `#[sysml::inout_item]` | Directed ports MUST connect `Port` <-> `Conjugated<Port>`. Use `#[sysml::port(symmetric)]` for bidirectional buses (CAN/Ethernet/mechanical). |
| **Arrow Between Boxes** | `#[sysml::interface]` + `#[sysml::connect(interface = Bus, name = "...", from = a.p_out, to = b.p_in)]` | Placed on the parent assembly struct; `rustc` checks field names and port compatibility! |
| **Parameter / Property** | `#[sysml::attr(default = 50.0, unit = "W")] pub power: Quantity<f64>` | Use base SI units (`s`, `m`, `kg`, `N`, `W`, `V`, `A`, `K`, `Hz`, `Pa`, `m/s`). |
| **Requirement + Provenance** | `#[sysml::requirement(id = "REQ-001")]` + `#[sysml::rationale("Source: Doc §3.2")]` + `#[sysml::require_constraint(...)]` | Link every requirement via `#[sysml::satisfy(req = MyReq, id = "REQ-001")]` on a part (0 orphans!). |
| **Function / Behavior** | `#[sysml::action]` + `#[sysml::perform(name: ActionTy)]` on a part | Allocates functions to logical/physical parts. |
| **Operating Modes / Faults** | `#[sysml::state(subject = Assembly)] pub enum Mode { ... }` | Variants = states; `#[sysml::initial]` marks start; `#[sysml::transition(on = Evt, guard = ..., to = Target)]`. |
| **Formula / Budget** | `#[sysml::calc] pub fn margin(limit: f64, actual: f64) -> f64 { limit - actual }` | Only pure arithmetic/comparisons & calls to other `#[sysml::calc]` fns. |
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

## 5. Incremental Crystallization Protocol (4 Phases)

Always build the system in **compiling, auditable increments** — never try to model an entire corpus in one giant pass:

1. **Phase 1 — High-Level Skeleton (Visio -> IBD):**
   - Create 1–2 packages (`Architecture` or `TypesAndPorts` + `Architecture`).
   - Model every Visio box as `#[sysml::part]`, every arrow as `#[sysml::item]` + `#[sysml::port]` + `#[sysml::interface]` + `#[sysml::connect]`.
   - Always include **one top-level context `#[sysml::part]`** composing all top-level boxes so `sysml-reactflow` renders a clean connected Internal Block Diagram (IBD).
   - **Gate:** `cargo test` + `emit` + `rsysdsl audit --files ... --stdlib --strict` -> **0 errors, 0 warnings**.
2. **Phase 2 — Requirements & Document Provenance:**
   - Add a `Requirements` package with `#[sysml::requirement(id = "...")]`.
   - Record source traceability on every requirement using `#[sysml::rationale("Source: <doc>, <section/page>")]` and `///` doc comments.
   - Add `#[sysml::require_constraint(...)]` for quantitative limits and `#[sysml::satisfy(req = ..., id = "...")]` on realizing parts.
   - **Gate:** `audit --stdlib --strict` -> **0 orphan requirements** + `trace-matrix --format markdown` shows 100% linkage.
3. **Phase 3 — Functions, Modes & Calculations:**
   - Add `#[sysml::action]` + `#[sysml::perform(...)]`, executable guarded `#[sysml::state]` enums, and `#[sysml::calc]` formulas.
   - **Gate:** Unit tests exercise `evaluate()`, `step_with()`, and `#[sysml::calc]` functions; `audit --stdlib --strict` stays at 0 errors / 0 warnings.
4. **Phase 4 — ISO 26262 Safety & Closed-Loop Simulation (Optional):**
   - Add `#[sysml::safety_goal]`, `#[sysml::fmea]`, `#[sysml::independent]`, `#[sysml::asil_decompose]`, and (if needed) a separate `<system>_sim` crate.

---

## 6. On-Demand Reference Skill (Load Only When Needed)

Keep your context small — only read these files when you need copy-pasteable templates or hit a compiler/validator error:

- **Skill overview & phase checklists:** [`.agents/skills/rsysdsl-modeling/SKILL.md`](.agents/skills/rsysdsl-modeling/SKILL.md)
- **Copy-pasteable Rust DSL patterns (IBD, Provenance, States, Calcs, ISO 26262, `no_std`):** [`.agents/skills/rsysdsl-modeling/references/PATTERNS.md`](.agents/skills/rsysdsl-modeling/references/PATTERNS.md)
- **Compiler & Open-MBEE error -> 1-line fix + SI unit table:** [`.agents/skills/rsysdsl-modeling/references/ERRORS.md`](.agents/skills/rsysdsl-modeling/references/ERRORS.md)
- **Working reference examples:** [`examples/starter_model/`](examples/starter_model/), [`examples/apollo11_model/`](examples/apollo11_model/), [`examples/embedded_flight_controller/`](examples/embedded_flight_controller/)
