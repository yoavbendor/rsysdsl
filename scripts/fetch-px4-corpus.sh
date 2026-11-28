#!/usr/bin/env bash
# Fetches the pinned 16-document PX4 + Pixhawk 6X (FMUv6X) brownfield corpus
# across 4 same-type batches into target/px4_corpus/ with a SHA-256 manifest.
#
# Default route: curl over raw.githubusercontent.com at the pinned commits below
#   (no GitHub API, no auth). Override a pin with PX4_USER_GUIDE_SHA or
#   PIXHAWK_STANDARDS_SHA.
# Opt-in route: FETCH_VIA_GH_API=1 resolves each download URL with `gh api`
#   (default branch, authenticated gh required).
#
# Requires: curl, python3 (with pypdf for PDF text extraction).
# gh is needed only for the FETCH_VIA_GH_API=1 route.
set -euo pipefail

PX4_USER_GUIDE_SHA="${PX4_USER_GUIDE_SHA:-2fa62338099f18fc3092197e856615eca94d2975}"
PIXHAWK_STANDARDS_SHA="${PIXHAWK_STANDARDS_SHA:-a97a915bfb6e20d6c8a41a86c5673350a3e51ceb}"
FETCH_VIA_GH_API="${FETCH_VIA_GH_API:-0}"

OUT_DIR="${1:-target/px4_corpus}"
mkdir -p "$OUT_DIR/batch_1_hardware" "$OUT_DIR/batch_2_sw_arch" "$OUT_DIR/batch_3_comms" "$OUT_DIR/batch_4_ai_safety"

sha_for_repo() {
  case "$1" in
    PX4/PX4-user_guide) echo "$PX4_USER_GUIDE_SHA" ;;
    pixhawk/Pixhawk-Standards) echo "$PIXHAWK_STANDARDS_SHA" ;;
    *) echo "unknown repo: $1" >&2; return 1 ;;
  esac
}

fetch_raw() {
  local repo="$1"
  local path="$2"
  local dest="$3"
  local enc_path url
  enc_path="$(python3 -c "import urllib.parse, sys; print(urllib.parse.quote(sys.argv[1]))" "$path")"
  if [ "$FETCH_VIA_GH_API" = "1" ]; then
    url="$(gh api "repos/${repo}/contents/${enc_path}" --jq '.download_url')"
  else
    url="https://raw.githubusercontent.com/${repo}/$(sha_for_repo "$repo")/${enc_path}"
  fi
  curl -fsSL "$url" -o "$dest"
}

echo "[Batch 1/4] Hardware & Board Standards..." >&2
fetch_raw "pixhawk/Pixhawk-Standards" "DS-012 Pixhawk Autopilot v6X Standard.pdf" "$OUT_DIR/batch_1_hardware/DS-012_FMUv6X_Standard.pdf"
fetch_raw "pixhawk/Pixhawk-Standards" "DS-010 Pixhawk Autopilot Bus Standard.pdf" "$OUT_DIR/batch_1_hardware/DS-010_Autopilot_Bus_Standard.pdf"
fetch_raw "pixhawk/Pixhawk-Standards" "DS-019 Pixhawk versions and revisions.md" "$OUT_DIR/batch_1_hardware/DS-019_Pixhawk_versions.md"
fetch_raw "PX4/PX4-user_guide" "en/flight_controller/autopilot_pixhawk_standard.md" "$OUT_DIR/batch_1_hardware/autopilot_pixhawk_standard.md"

echo "[Batch 2/4] SW Architecture & Control Loops..." >&2
fetch_raw "PX4/PX4-user_guide" "en/concept/architecture.md" "$OUT_DIR/batch_2_sw_arch/architecture.md"
fetch_raw "PX4/PX4-user_guide" "en/concept/px4_systems_architecture.md" "$OUT_DIR/batch_2_sw_arch/px4_systems_architecture.md"
fetch_raw "PX4/PX4-user_guide" "en/concept/control_allocation.md" "$OUT_DIR/batch_2_sw_arch/control_allocation.md"
fetch_raw "PX4/PX4-user_guide" "en/concept/flight_modes.md" "$OUT_DIR/batch_2_sw_arch/flight_modes.md"

echo "[Batch 3/4] Comms & Middleware ICDs..." >&2
fetch_raw "PX4/PX4-user_guide" "en/middleware/uorb.md" "$OUT_DIR/batch_3_comms/uorb.md"
fetch_raw "PX4/PX4-user_guide" "en/middleware/mavlink.md" "$OUT_DIR/batch_3_comms/mavlink.md"
fetch_raw "PX4/PX4-user_guide" "en/middleware/uxrce_dds.md" "$OUT_DIR/batch_3_comms/uxrce_dds.md"
fetch_raw "PX4/PX4-user_guide" "en/can/index.md" "$OUT_DIR/batch_3_comms/can_index.md"

echo "[Batch 4/4] AI Vision & Safety / Failsafe..." >&2
fetch_raw "PX4/PX4-user_guide" "en/computer_vision/obstacle_avoidance.md" "$OUT_DIR/batch_4_ai_safety/obstacle_avoidance.md"
fetch_raw "PX4/PX4-user_guide" "en/computer_vision/collision_prevention.md" "$OUT_DIR/batch_4_ai_safety/collision_prevention.md"
fetch_raw "PX4/PX4-user_guide" "en/computer_vision/path_planning_interface.md" "$OUT_DIR/batch_4_ai_safety/path_planning_interface.md"
fetch_raw "PX4/PX4-user_guide" "en/concept/pwm_limit.md" "$OUT_DIR/batch_4_ai_safety/pwm_limit.md"

echo "[Extract] Converting PDF standards to plain text..." >&2
PX4_USER_GUIDE_SHA="$PX4_USER_GUIDE_SHA" \
PIXHAWK_STANDARDS_SHA="$PIXHAWK_STANDARDS_SHA" \
FETCH_ROUTE="$([ "$FETCH_VIA_GH_API" = "1" ] && echo gh-api || echo raw-pinned)" \
python3 - "$OUT_DIR" << 'PY'
import hashlib, json, os, pathlib, sys
from pypdf import PdfReader

out_dir = pathlib.Path(sys.argv[1])
for pdf_path in sorted(out_dir.rglob("*.pdf")):
    reader = PdfReader(str(pdf_path))
    pages = []
    for i, page in enumerate(reader.pages, 1):
        # layout mode keeps table rows on one line (plain mode drops cells, e.g. DS-010 p.6 "CAN 2 2 1 3" -> "CAN")
        txt = page.extract_text(extraction_mode="layout") or ""
        pages.append(f"--- PAGE {i} ---\n{txt}")
    txt_path = pdf_path.with_suffix(".txt")
    txt_path.write_text("\n\n".join(pages), encoding="utf-8")

manifest = []
total_bytes = 0
for p in sorted(out_dir.rglob("*")):
    if p.is_file() and p.suffix in (".md", ".txt"):
        data = p.read_bytes()
        total_bytes += len(data)
        manifest.append({
            "path": str(p.relative_to(out_dir)),
            "bytes": len(data),
            "approx_tokens": len(data) // 4,
            "sha256": hashlib.sha256(data).hexdigest(),
        })

(out_dir / "manifest.json").write_text(
    json.dumps({
        "docs": len(manifest),
        "total_bytes": total_bytes,
        "approx_total_tokens": total_bytes // 4,
        "sources": {
            "PX4/PX4-user_guide": os.environ["PX4_USER_GUIDE_SHA"],
            "pixhawk/Pixhawk-Standards": os.environ["PIXHAWK_STANDARDS_SHA"],
        },
        "fetch_route": os.environ["FETCH_ROUTE"],
        "files": manifest,
    }, indent=2),
    encoding="utf-8",
)
print(f"Frozen {len(manifest)} text/md docs: {total_bytes:,} bytes (~{total_bytes // 4:,} tokens)")
PY
