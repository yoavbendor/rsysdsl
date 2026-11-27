#!/usr/bin/env bash
# S7.6 TBD ratchet: for every example with an `emit` binary, emit its SysML and compare the
# TBD ledger with the committed `examples/<name>/tbd-baseline.json`. Fails (exit 1) if any
# example's TBD count rose (total or per kind). Raising a baseline must be a deliberate change
# committed together with the new TBD: `scripts/check-tbd-baselines.sh --write`.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MODE="--check"
if [[ "${1:-}" == "--write" ]]; then
  MODE="--write-baseline"
fi

cargo build -q -p sysml-cli >&2
RSYSDSL="${RSYSDSL_BIN:-$ROOT_DIR/target/debug/rsysdsl}"
OUT_DIR="$ROOT_DIR/target/tbd-check"
mkdir -p "$OUT_DIR"

status=0
for dir in examples/*/; do
  dir="${dir%/}"
  name="$(basename "$dir")"
  [[ -f "$dir/src/bin/emit.rs" ]] || continue
  pkg="$(sed -n 's/^name *= *"\(.*\)"/\1/p' "$dir/Cargo.toml" | head -n1)"
  sysml="target/tbd-check/$name.sysml"
  cargo run -q -p "$pkg" --bin emit -- -o "$sysml" --omit-std 2>/dev/null
  if "$RSYSDSL" tbd --files "$sysml" --baseline "$dir/tbd-baseline.json" "$MODE" > "$OUT_DIR/$name.tbd.json"; then
    echo "ok   $name ($(sed -n 's/.*"current_count": *\([0-9]*\).*/\1/p;s/.*"tbd_count": *\([0-9]*\).*/\1/p' "$OUT_DIR/$name.tbd.json" | head -n1) TBDs)"
  else
    echo "FAIL $name: TBD count rose above $dir/tbd-baseline.json" >&2
    status=1
  fi
done
exit $status
