# Corporate Host, Offline Bundling & VS Code SysML v2 Setup

This guide covers cloning `rsysdsl` onto a corporate workstation, running Cursor-CLI with a cost-aware model (Composer 2.5 / Haiku-class), and visually inspecting emitted `.sysml` files in VS Code.

---

## 1. Corporate Host Setup (crates.io Accessible — Zero Git Submodules!)

`Open-MBEE/sysmlv2-syntax`, `sysmlv2-model`, and the official OMG Standard Library (`third_party/omg/sysml.library`) are **bundled directly in `third_party/`**, so Cargo never clones any external GitHub repositories or 2.1 GB git submodules. Root `rust-toolchain.toml` is intentionally omitted so `rustup` never stalls trying to download a pinned compiler on air-gapped or fenced corporate hosts (CI pins Rust `1.97.0` directly in `.github/workflows/ci-pages.yml` for deterministic `trybuild` UI snapshot checks).

```bash
git clone https://github.com/yoavbendor/rsysdsl.git
cd rsysdsl
cargo test -p starter-model
cargo run -q -p starter-model --bin emit -- -o target/starter_model.sysml
cargo run -q -p sysml-cli -- audit --files target/starter_model.sysml --stdlib --strict --pretty
```

---

## 2. Completely Air-Gapped Host Setup (No crates.io Access)

On a machine **with** crates.io access, run the bundling script once:

```bash
./scripts/vendor-offline.sh
```

This creates `vendor/` (all crates.io crates) and `.cargo/config.toml`. Transfer the repo directory to your air-gapped host, then run completely offline (`third_party/omg/sysml.library` is auto-discovered with no environment variables required):

```bash
cargo test --offline -p starter-model
cargo run --offline -q -p starter-model --bin emit -- -o target/starter_model.sysml
cargo run --offline -q -p sysml-cli -- audit --files target/starter_model.sysml --stdlib --strict --pretty
```

---

## 3. Viewing Emitted `.sysml` in VS Code or Standalone Offline HTML

### Option A — Zero-Install Standalone Offline HTML Viewer (`rsysdsl view`)

If you do not want to install any VS Code extensions, `rsysdsl view` bundles the MIT-licensed `VSCode_SysML_Extension` runtime together with D3, ELK, and Cytoscape into a single self-contained `.html` file that works offline over `file://` (or via `--serve`):

```bash
cargo run -q -p sysml-cli -- view target/starter_model.sysml --view interconnection -o target/starter_model.html
# Or launch a local loopback server on http://127.0.0.1:8080/
cargo run -q -p sysml-cli -- view target/starter_model.sysml --view interconnection --serve --port 8080
```

Features included in the generated HTML:
- **10 interactive diagram views:** General (`general`), Interconnection/IBD (`interconnection`), State (`state`), Action Flow (`activity`), Sequence (`sequence`), Package (`package`), Tree (`tree`), Hierarchy (`hierarchy`), Graph (`graph`), and Case (`usecase`).
- **SVG / PNG export** and **Light / Dark theme toggle** (`--theme light` or `--theme dark`).

### Option B — VS Code Extensions

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
