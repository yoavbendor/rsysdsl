#!/usr/bin/env bash
# Bundles Cargo crates (vendor/) and the OMG SysML v2 Standard Library (vendor-stdlib/sysml.library/)
# so rsysdsl can build, emit, and run 0-warning --stdlib --strict audits on air-gapped corporate hosts.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "[1/3] Vendoring Cargo dependencies into ./vendor ..."
mkdir -p .cargo
cargo vendor --locked vendor > .cargo/config.toml

echo "[2/3] Locating OMG SysML v2 Standard Library (sysml.library) ..."
STDLIB_SRC=""
if [[ -n "${SYSML_LIBRARY_DIR:-}" && -d "${SYSML_LIBRARY_DIR}" ]]; then
    STDLIB_SRC="${SYSML_LIBRARY_DIR}"
else
    for base in "${CARGO_HOME:-}" "/opt/cargo" "$HOME/.cargo"; do
        [[ -n "$base" && -d "$base/git/checkouts" ]] || continue
        found="$(find "$base/git/checkouts" -maxdepth 4 -type d -path '*/sysml-toolkit-*/spec-refs/SysML-v2-Release/sysml.library' 2>/dev/null | head -n 1 || true)"
        if [[ -n "$found" ]]; then
            STDLIB_SRC="$found"
            break
        fi
    done
fi

if [[ -z "$STDLIB_SRC" ]]; then
    echo "error: could not locate sysml.library; run 'cargo fetch' first or set SYSML_LIBRARY_DIR" >&2
    exit 1
fi

echo "[3/3] Copying OMG stdlib from $STDLIB_SRC -> ./vendor-stdlib/sysml.library ..."
rm -rf vendor-stdlib/sysml.library
mkdir -p vendor-stdlib
cp -R "$STDLIB_SRC" vendor-stdlib/sysml.library

echo "Done! Offline bundle ready."
echo "On your air-gapped host, run:"
echo "  export SYSML_LIBRARY_DIR=\"\$PWD/vendor-stdlib/sysml.library\""
echo "  cargo test --offline -p starter-model"
