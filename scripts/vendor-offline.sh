#!/usr/bin/env bash
# Bundles all crates.io dependencies into ./vendor and generates .cargo/config.toml
# for fully air-gapped corporate hosts. (Open-MBEE sysmlv2-syntax/sysmlv2-model and
# OMG sysml.library are already bundled in ./third_party — zero git submodules needed!)

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "[1/1] Vendoring crates.io dependencies into ./vendor ..."
mkdir -p .cargo
cargo vendor --locked vendor > .cargo/config.toml

echo "Done! Offline bundle ready."
echo "On your air-gapped host, run:"
echo "  cargo test --offline -p starter-model"
echo "  cargo run --offline -q -p starter-model --bin emit -- -o target/starter_model.sysml"
echo "  cargo run --offline -q -p sysml-cli -- audit --files target/starter_model.sysml --stdlib --strict --pretty"
