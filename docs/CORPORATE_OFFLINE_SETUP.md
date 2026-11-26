# Corporate Host, Offline Bundling & VS Code SysML v2 Setup

This guide covers cloning `rsysdsl` onto a corporate workstation, running Cursor-CLI with a cost-aware model (Composer 2.5 / Haiku-class), and visually inspecting emitted `.sysml` files in VS Code.

---

## 1. Online Corporate Host Setup (GitHub / crates.io Accessible)

```bash
git clone https://github.com/yoavbendor/rsysdsl.git
cd rsysdsl
cargo test -p starter-model
cargo run -q -p starter-model --bin emit -- -o target/starter_model.sysml
cargo run -q -p sysml-cli -- audit --files target/starter_model.sysml --stdlib --strict --pretty
```

---

## 2. Air-Gapped / Restricted Corporate Network Setup

On a machine **with** internet access, run the bundling script once:

```bash
./scripts/vendor-offline.sh
```

This creates:
- `vendor/` — all Cargo crates (including pinned `Open-MBEE/sysml-toolkit`)
- `vendor-stdlib/sysml.library/` — official OMG SysML v2 Standard Library
- `.cargo/config.toml` — offline Cargo source replacement configuration

Transfer the repo directory (including `vendor/`, `vendor-stdlib/`, and `.cargo/config.toml`) to your corporate host, then run completely offline:

```bash
export SYSML_LIBRARY_DIR="$PWD/vendor-stdlib/sysml.library"
cargo test --offline -p starter-model
cargo run --offline -q -p sysml-cli -- audit --files target/starter_model.sysml --stdlib --strict --pretty
```

---

## 3. Viewing Emitted `.sysml` in VS Code

1. Open the `rsysdsl` workspace in VS Code.
2. Install either (or both):
   - **[SysML v2 Support (`JamieD.sysml-v2-support`)](https://marketplace.visualstudio.com/items?itemName=JamieD.sysml-v2-support)** — syntax highlighting, outline navigation, and diagram previews.
   - **`sysml-reactflow`** (listed on [awesome-sysml](https://github.com/mycr0ft/awesome-sysml)) — interactive Internal Block Diagram (IBD) / Block Definition Diagram (BDD) graph renderer.
3. Open any emitted file (e.g., `target/starter_model.sysml` or `target/my_system.sysml`). Because every model crate defines a single top-level context `#[sysml::part]` with `#[sysml::connect(...)]` arrows, the IBD view will immediately show your Visio boxes and connections cleanly.

---

## 4. Starter Prompts for Cursor-CLI (Cost-Aware Models)

### Phase 1 Prompt — Visio Diagram to High-Level Model
```text
Read AGENTS.md first (and .agents/skills/rsysdsl-modeling/references/PATTERNS.md if needed).
Copy examples/starter_model to examples/<my_system>, register it in root Cargo.toml,
and model the attached Visio diagram (<path/to/diagram.png or .vsdx>) strictly in rsysdsl Rust.
Do NOT edit .sysml files directly.
Run `cargo test -p <my-system>`, emit `target/<my_system>.sysml`, and pass
`cargo run -q -p sysml-cli -- audit --files target/<my_system>.sysml --stdlib --strict --pretty`
with 0 errors and 0 warnings before finishing.
```

### Phase 2–3 Prompt — Crystallizing a Corpus (Provenance, Requirements, States)
```text
Follow Phase 2 and Phase 3 of AGENTS.md on examples/<my_system>.
Read the documents in <docs_folder/>. For each requirement, add a #[sysml::requirement(id = "...")]
with #[sysml::rationale("Source: <file>, <section>")] provenance, #[sysml::require_constraint(...)]
for every numeric threshold, and #[sysml::satisfy(...)] on the realizing part.
Add #[sysml::state] enums for operating/fault modes and #[sysml::calc] for documented formulas.
Re-emit target/<my_system>.sysml, verify 0 errors / 0 warnings / 0 orphan requirements via
`rsysdsl audit --files target/<my_system>.sysml --stdlib --strict`, and print the Markdown trace matrix.
```
