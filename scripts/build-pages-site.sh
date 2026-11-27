#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-_site}"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PROFILE="${RSYSDSL_PROFILE:-release}"
if [[ "$PROFILE" != "release" && "$PROFILE" != "debug" ]]; then
  echo "error: RSYSDSL_PROFILE must be 'release' or 'debug' (got '$PROFILE')" >&2
  exit 2
fi

if [[ -n "${RSYSDSL_BIN:-}" ]]; then
  RSYSDSL="$RSYSDSL_BIN"
  if [[ -n "${RSYSDSL_EMIT_BIN:-}" ]]; then
    STARTER_EMIT="$RSYSDSL_EMIT_BIN"
  else
    BIN_DIR="$(dirname "$RSYSDSL")"
    if [[ -x "$BIN_DIR/emit" ]]; then
      STARTER_EMIT="$BIN_DIR/emit"
    else
      cargo build --profile "$([[ "$PROFILE" == "release" ]] && echo release || echo dev)" -p starter-model >&2
      STARTER_EMIT="$ROOT_DIR/target/$PROFILE/emit"
    fi
  fi
else
  if [[ "$PROFILE" == "release" ]]; then
    cargo build --release -p sysml-cli -p starter-model >&2
    RSYSDSL="$ROOT_DIR/target/release/rsysdsl"
    STARTER_EMIT="$ROOT_DIR/target/release/emit"
  else
    cargo build -p sysml-cli -p starter-model >&2
    RSYSDSL="$ROOT_DIR/target/debug/rsysdsl"
    STARTER_EMIT="$ROOT_DIR/target/debug/emit"
  fi
fi

if [[ ! -x "$RSYSDSL" ]]; then
  echo "error: rsysdsl binary not executable at '$RSYSDSL'" >&2
  exit 2
fi
if [[ ! -x "$STARTER_EMIT" ]]; then
  echo "error: starter-model emit binary not executable at '$STARTER_EMIT'" >&2
  exit 2
fi

mkdir -p "$OUT_DIR/data" "$OUT_DIR/sysml"

echo "[1/6] Starter Thermal Monitor (Rust DSL -> Visio IBD)..." >&2
"$STARTER_EMIT" -o "$OUT_DIR/sysml/starter_model.sysml" >&2
"$RSYSDSL" audit --files "$OUT_DIR/sysml/starter_model.sysml" --stdlib --strict > "$OUT_DIR/data/starter-audit.json"
"$RSYSDSL" view "$OUT_DIR/sysml/starter_model.sysml" --view interconnection --title "Starter Thermal Monitor (IBD)" -o "$OUT_DIR/starter-ibd.html" > /dev/null
"$RSYSDSL" view "$OUT_DIR/sysml/starter_model.sysml" --view general --title "Starter Thermal Monitor (General)" -o "$OUT_DIR/starter-general.html" > /dev/null

echo "[2/6] Embedded Flight Controller (Ferrocene #![no_std] ISO 26262)..." >&2
"$RSYSDSL" emit --model embedded-flight-controller --raw > "$OUT_DIR/sysml/embedded_flight_controller.sysml"
"$RSYSDSL" audit --model embedded-flight-controller --stdlib --strict > "$OUT_DIR/data/fc-audit.json"
"$RSYSDSL" safety-report --model embedded-flight-controller --format markdown -o "$OUT_DIR/data/fc-safety-report.md" > /dev/null
"$RSYSDSL" safety-report --model embedded-flight-controller --format json -o "$OUT_DIR/data/fc-safety-report.json" > /dev/null
"$RSYSDSL" view --model embedded-flight-controller --view state --title "Embedded Flight Controller (State)" -o "$OUT_DIR/fc-state.html" > /dev/null
"$RSYSDSL" view --model embedded-flight-controller --view interconnection --title "Embedded Flight Controller (IBD)" -o "$OUT_DIR/fc-ibd.html" > /dev/null

echo "[3/6] Apollo 11 Rust DSL (Executable Verification + ASPICE Traceability)..." >&2
"$RSYSDSL" emit --model apollo11-model --raw > "$OUT_DIR/sysml/apollo11_rust.sysml"
"$RSYSDSL" audit --model apollo11-model --stdlib --strict --verify > "$OUT_DIR/data/apollo11-rust-audit.json"
"$RSYSDSL" trace-matrix --model apollo11-model --verify --format markdown -o "$OUT_DIR/data/apollo11-trace-matrix.md" > /dev/null
"$RSYSDSL" trace-matrix --model apollo11-model --verify --format json -o "$OUT_DIR/data/apollo11-trace-matrix.json" > /dev/null
"$RSYSDSL" view --model apollo11-model --view general --title "Apollo 11 Rust DSL (General)" -o "$OUT_DIR/apollo11-rust.html" > /dev/null

echo "[4/6] Full Airbus Apollo 11 Corpus (28 .sysml Files Round-Trip)..." >&2
mapfile -d '' APOLLO11_FILES < <(find tests/fixtures/apollo-11-sysml-v2 -type f -name '*.sysml' -print0 | LC_ALL=C sort -z)
if [[ ${#APOLLO11_FILES[@]} -eq 0 ]]; then
  echo "error: no Apollo 11 fixture files found" >&2
  exit 2
fi
"$RSYSDSL" roundtrip-check "${APOLLO11_FILES[@]}" --reemitted-out "$OUT_DIR/sysml/apollo11_reemitted.sysml" > "$OUT_DIR/data/apollo11-roundtrip.json"
"$RSYSDSL" view "${APOLLO11_FILES[@]}" --view package --title "Full Airbus Apollo 11 Corpus (28 files)" -o "$OUT_DIR/apollo11-corpus.html" > /dev/null

echo "[5/6] OMG Canonical Vehicle / Car..." >&2
mapfile -d '' OMG_VEHICLE_FILES < <(find tests/fixtures/omg-vehicle -maxdepth 1 -type f -name '*.sysml' -print0 | LC_ALL=C sort -z)
if [[ ${#OMG_VEHICLE_FILES[@]} -eq 0 ]]; then
  echo "error: no OMG Vehicle fixture files found" >&2
  exit 2
fi
"$RSYSDSL" validate --stdlib --strict "${OMG_VEHICLE_FILES[@]}" > "$OUT_DIR/data/omg-vehicle-validate.json"
"$RSYSDSL" roundtrip-check "${OMG_VEHICLE_FILES[@]}" --reemitted-out "$OUT_DIR/sysml/omg_vehicle_reemitted.sysml" > "$OUT_DIR/data/omg-vehicle-roundtrip.json"
"$RSYSDSL" view "${OMG_VEHICLE_FILES[@]}" --view general --title "OMG Canonical Vehicle / Car (General)" -o "$OUT_DIR/omg-vehicle.html" > /dev/null
"$RSYSDSL" view "${OMG_VEHICLE_FILES[@]}" --view interconnection --title "OMG Canonical Vehicle / Car (IBD)" -o "$OUT_DIR/omg-vehicle-ibd.html" > /dev/null

echo "[6/9] OMG Canonical Camera..." >&2
mapfile -d '' OMG_CAMERA_FILES < <(find tests/fixtures/omg-camera -maxdepth 1 -type f -name '*.sysml' -print0 | LC_ALL=C sort -z)
if [[ ${#OMG_CAMERA_FILES[@]} -eq 0 ]]; then
  echo "error: no OMG Camera fixture files found" >&2
  exit 2
fi
"$RSYSDSL" validate --stdlib --strict "${OMG_CAMERA_FILES[@]}" > "$OUT_DIR/data/omg-camera-validate.json"
"$RSYSDSL" roundtrip-check "${OMG_CAMERA_FILES[@]}" --reemitted-out "$OUT_DIR/sysml/omg_camera_reemitted.sysml" > "$OUT_DIR/data/omg-camera-roundtrip.json"
"$RSYSDSL" view "${OMG_CAMERA_FILES[@]}" --view general --title "OMG Canonical Camera (General)" -o "$OUT_DIR/omg-camera.html" > /dev/null

echo "[7/9] Microwave Oven (FMEA Fault Injection + Live Sim Trace)..." >&2
"$RSYSDSL" emit --model microwave --raw > "$OUT_DIR/sysml/microwave.sysml"
"$RSYSDSL" audit --model microwave --stdlib --strict > "$OUT_DIR/data/microwave-audit.json"
"$RSYSDSL" verify --scenario mw-door-open --trace "$OUT_DIR/data/microwave-trace.json" > /dev/null
"$RSYSDSL" view --model microwave --view interconnection --trace "$OUT_DIR/data/microwave-trace.json" --title "Microwave Oven (IBD + Live Sim Trace)" -o "$OUT_DIR/microwave-sim.html" > /dev/null

echo "[8/9] Dishwasher (Hybrid Thermal/Fluidic ODE + Live Sim Trace)..." >&2
"$RSYSDSL" emit --model dishwasher --raw > "$OUT_DIR/sysml/dishwasher.sysml"
"$RSYSDSL" audit --model dishwasher --stdlib --strict --verify > "$OUT_DIR/data/dishwasher-audit.json"
"$RSYSDSL" verify --scenario dw-door-open --trace "$OUT_DIR/data/dishwasher-trace.json" > /dev/null
"$RSYSDSL" view --model dishwasher --view interconnection --trace "$OUT_DIR/data/dishwasher-trace.json" --title "Dishwasher (IBD + Live Sim Trace)" -o "$OUT_DIR/dishwasher-sim.html" > /dev/null

echo "[9/9] Cordless Power Tools Family (5Ah Battery Discharge + Live Sim Trace)..." >&2
"$RSYSDSL" emit --model power-tools-family --raw > "$OUT_DIR/sysml/power_tools_family.sysml"
"$RSYSDSL" audit --model power-tools-family --stdlib --strict --verify > "$OUT_DIR/data/power-tools-audit.json"
"$RSYSDSL" verify --scenario tool-5ah --trace "$OUT_DIR/data/power-tools-trace.json" > /dev/null
"$RSYSDSL" view --model power-tools-family --view interconnection --trace "$OUT_DIR/data/power-tools-trace.json" --title "Power Tools Family (IBD + Live Sim Trace)" -o "$OUT_DIR/power-tools-sim.html" > /dev/null

GIT_SHA="$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
BUILD_UTC="$(date -u '+%Y-%m-%d %H:%M:%S UTC')"

python3 - "$OUT_DIR" "$GIT_SHA" "$BUILD_UTC" << 'PYEOF'
import html
import json
import os
import sys
from pathlib import Path

out_dir = Path(sys.argv[1])
git_sha = sys.argv[2]
build_utc = sys.argv[3]
data_dir = out_dir / "data"


def load_json(name: str) -> dict:
    with open(data_dir / name, "r", encoding="utf-8") as f:
        return json.load(f)


def esc(val: object) -> str:
    return html.escape(str(val), quote=True)


starter_audit = load_json("starter-audit.json")
fc_audit = load_json("fc-audit.json")
fc_safety = load_json("fc-safety-report.json")
a11_rust_audit = load_json("apollo11-rust-audit.json")
a11_trace = load_json("apollo11-trace-matrix.json")
a11_rt = load_json("apollo11-roundtrip.json")
veh_val = load_json("omg-vehicle-validate.json")
veh_rt = load_json("omg-vehicle-roundtrip.json")
cam_val = load_json("omg-camera-validate.json")
cam_rt = load_json("omg-camera-roundtrip.json")
mw_audit = load_json("microwave-audit.json")
dw_audit = load_json("dishwasher-audit.json")
pt_audit = load_json("power-tools-audit.json")

total_rt_facts = (
    a11_rt["roundtrip"]["total_original_facts"]
    + veh_rt["roundtrip"]["total_original_facts"]
    + cam_rt["roundtrip"]["total_original_facts"]
)
total_stdlib_errors = (
    starter_audit["validation_errors"]
    + fc_audit["validation_errors"]
    + a11_rust_audit["validation_errors"]
    + veh_val["error_count"]
    + cam_val["error_count"]
    + mw_audit["validation_errors"]
    + dw_audit["validation_errors"]
    + pt_audit["validation_errors"]
)
total_stdlib_warnings = (
    starter_audit["validation_warnings"]
    + fc_audit["validation_warnings"]
    + a11_rust_audit["validation_warnings"]
    + veh_val["warning_count"]
    + cam_val["warning_count"]
    + mw_audit["validation_warnings"]
    + dw_audit["validation_warnings"]
    + pt_audit["validation_warnings"]
)

BASE_CSS = """
:root {
  --bg: #0b1020;
  --surface: #121a2f;
  --surface-2: #18233f;
  --border: #253459;
  --text: #e6edf7;
  --muted: #93a4c7;
  --accent: #60a5fa;
  --accent-2: #a78bfa;
  --ok: #34d399;
  --warn: #fbbf24;
  --bad: #f87171;
  --mono: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;
  --sans: Inter, ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  background: radial-gradient(1200px 600px at 15% -10%, #162347 0%, var(--bg) 55%);
  color: var(--text);
  font-family: var(--sans);
  line-height: 1.5;
}
a { color: var(--accent); text-decoration: none; }
a:hover { text-decoration: underline; }
.wrap { max-width: 1220px; margin: 0 auto; padding: 32px 24px 64px; }
header.hero {
  background: linear-gradient(135deg, rgba(24,35,63,0.92), rgba(14,21,40,0.95));
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 28px 30px;
  margin-bottom: 28px;
  box-shadow: 0 18px 40px rgba(0, 0, 0, 0.35);
}
.eyebrow {
  font-family: var(--mono);
  font-size: 12px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--accent);
  margin-bottom: 8px;
}
h1 { margin: 0 0 8px; font-size: 28px; letter-spacing: -0.02em; }
.sub { color: var(--muted); margin: 0 0 18px; max-width: 78ch; }
.badges { display: flex; flex-wrap: wrap; gap: 10px; align-items: center; }
.badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 11px;
  border-radius: 999px;
  font-family: var(--mono);
  font-size: 12px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  color: var(--text);
}
.badge.ok { border-color: rgba(52, 211, 153, 0.4); color: var(--ok); background: rgba(16, 185, 129, 0.12); }
.badge.warn { border-color: rgba(251, 191, 36, 0.4); color: var(--warn); background: rgba(245, 158, 11, 0.12); }
.badge.bad { border-color: rgba(248, 113, 113, 0.4); color: var(--bad); background: rgba(239, 68, 68, 0.12); }
.grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(340px, 1fr));
  gap: 20px;
}
.card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 22px;
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  gap: 16px;
}
.card-head { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; }
.card-tag {
  font-family: var(--mono);
  font-size: 11px;
  color: var(--accent-2);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  margin-bottom: 4px;
}
.card h2 { margin: 0; font-size: 18px; line-height: 1.3; }
.card p { margin: 8px 0 0; color: var(--muted); font-size: 14px; }
.metrics {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
  background: rgba(11, 16, 32, 0.55);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px;
}
.metric-label { font-size: 11px; color: var(--muted); text-transform: uppercase; letter-spacing: 0.04em; }
.metric-val { font-family: var(--mono); font-size: 14px; font-weight: 600; margin-top: 2px; }
.actions { display: flex; flex-wrap: wrap; gap: 8px; }
.btn {
  display: inline-flex;
  align-items: center;
  padding: 7px 12px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 500;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--text);
  text-decoration: none;
}
.btn.primary { background: #1d4ed8; border-color: #3b82f6; color: #fff; }
.btn:hover { filter: brightness(1.12); text-decoration: none; }
.panel {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 20px;
  margin-bottom: 24px;
  overflow-x: auto;
}
table { width: 100%; border-collapse: collapse; font-size: 13px; }
th, td { text-align: left; padding: 10px 12px; border-bottom: 1px solid var(--border); vertical-align: top; }
th { font-family: var(--mono); font-size: 11px; text-transform: uppercase; letter-spacing: 0.05em; color: var(--muted); }
code { font-family: var(--mono); font-size: 12px; background: rgba(11, 16, 32, 0.7); padding: 2px 5px; border-radius: 4px; }
footer { margin-top: 32px; color: var(--muted); font-size: 12px; font-family: var(--mono); display: flex; justify-content: space-between; flex-wrap: wrap; gap: 12px; }
"""


def status_badge(ok: bool, label_ok: str, label_fail: str) -> str:
    cls = "ok" if ok else "bad"
    txt = label_ok if ok else label_fail
    return f'<span class="badge {cls}">{esc(txt)}</span>'


cards = [
    {
        "tag": "01 · Rust DSL -> Visio IBD",
        "title": "Starter Thermal Monitor",
        "desc": "Copy-pasteable Rust DSL template demonstrating parts, conjugated ports, interface flows, and requirement satisfaction.",
        "ok": starter_audit["ok"] and starter_audit["validation_warnings"] == 0,
        "status": "STDLIB STRICT PASS" if (starter_audit["ok"] and starter_audit["validation_warnings"] == 0) else "CHECK",
        "metrics": [
            ("OMG Stdlib", f"{starter_audit['validation_errors']} err / {starter_audit['validation_warnings']} warn"),
            ("Requirements", f"{starter_audit['satisfied_requirements']}/{starter_audit['total_requirements']} satisfied"),
            ("Orphans", str(len(starter_audit["orphan_requirements"]))),
            ("Calcs", str(starter_audit["total_calcs"])),
        ],
        "links": [
            ("Open IBD Viewer", "starter-ibd.html", True),
            ("General View", "starter-general.html", False),
            ("Raw .sysml", "sysml/starter_model.sysml", False),
            ("Audit JSON", "data/starter-audit.json", False),
        ],
    },
    {
        "tag": "02 · Ferrocene #![no_std] ISO 26262",
        "title": "Embedded Flight Controller",
        "desc": "Zero-std dual-channel flight controller with ASIL-D decomposition, guarded fault state machine, FMEA RPN ranking, and minimal cut sets.",
        "ok": fc_audit["ok"] and fc_audit["validation_warnings"] == 0,
        "status": "STDLIB + ISO 26262 PASS" if (fc_audit["ok"] and fc_audit["validation_warnings"] == 0) else "CHECK",
        "metrics": [
            ("OMG Stdlib", f"{fc_audit['validation_errors']} err / {fc_audit['validation_warnings']} warn"),
            ("Safety Goals", str(fc_safety["total_safety_goals"])),
            ("ASIL Decomps / Max RPN", f"{fc_safety['total_decompositions']} / RPN {fc_safety['max_rpn']}"),
            ("Cut Sets (SPF / Dual)", f"{fc_safety['single_point_fault_count']} / {fc_safety['dual_channel_cut_set_count']}"),
        ],
        "links": [
            ("State Machine Viewer", "fc-state.html", True),
            ("IBD Viewer", "fc-ibd.html", False),
            ("Safety Report HTML", "safety-report.html", False),
            ("Raw .sysml", "sysml/embedded_flight_controller.sysml", False),
        ],
    },
    {
        "tag": "03 · Executable Verification + ASPICE",
        "title": "Apollo 11 Rust DSL",
        "desc": "5-layer lunar lander architecture authored in Rust DSL with closed-loop physics verification and ASPICE bidirectional traceability.",
        "ok": a11_rust_audit["ok"] and a11_rust_audit["validation_warnings"] == 0,
        "status": "VERIFIED + STRICT PASS" if (a11_rust_audit["ok"] and a11_rust_audit["validation_warnings"] == 0) else "CHECK",
        "metrics": [
            ("OMG Stdlib", f"{a11_rust_audit['validation_errors']} err / {a11_rust_audit['validation_warnings']} warn"),
            ("Satisfied Reqs", f"{a11_trace['summary']['satisfied_requirements']}/{a11_trace['summary']['total_requirements']} ({a11_trace['summary']['satisfaction_coverage_pct']}%)"),
            ("Verified PASS", f"{a11_trace['summary']['verified_pass']}/{a11_trace['summary']['constrained_requirements']} ({a11_trace['summary']['verification_coverage_pct']}%)"),
            ("Calcs / States", f"{a11_trace['summary']['total_calcs']} / {a11_trace['summary']['total_state_defs']}"),
        ],
        "links": [
            ("General Viewer", "apollo11-rust.html", True),
            ("Traceability Matrix HTML", "trace-matrix.html", False),
            ("Raw .sysml", "sysml/apollo11_rust.sysml", False),
            ("Audit JSON", "data/apollo11-rust-audit.json", False),
        ],
    },
    {
        "tag": "04 · 28-File Corpus Round-Trip",
        "title": "Full Airbus Apollo 11 Corpus",
        "desc": "Complete 28-file Airbus Apollo 11 SysML v2 benchmark lifted to Rust IR, re-emitted, and checked for lossless AST/graph parity.",
        "ok": a11_rt["ok"],
        "status": "100% ROUND-TRIP PASS" if a11_rt["ok"] else "MISMATCH",
        "metrics": [
            ("Facts Matched", f"{a11_rt['roundtrip']['total_reemitted_facts']}/{a11_rt['roundtrip']['total_original_facts']}"),
            ("Missing / Extra", f"{a11_rt['roundtrip']['missing_count']} / {a11_rt['roundtrip']['extra_count']}"),
            ("Unsupported Constructs", str(a11_rt["unsupported_logged"])),
            ("Re-emitted Errors", str(a11_rt["validation_errors"])),
        ],
        "links": [
            ("Package Viewer", "apollo11-corpus.html", True),
            ("Re-emitted .sysml", "sysml/apollo11_reemitted.sysml", False),
            ("Round-Trip JSON", "data/apollo11-roundtrip.json", False),
        ],
    },
    {
        "tag": "05 · OMG Official Specification",
        "title": "OMG Canonical Vehicle / Car",
        "desc": "VehicleDefinitions, VehicleUsages, and VehicleIndividuals validated strictly against OMG stdlib and verified via bidirectional round-trip.",
        "ok": veh_val["valid"] and veh_val["warning_count"] == 0 and veh_rt["ok"],
        "status": "STDLIB + ROUND-TRIP PASS" if (veh_val["valid"] and veh_val["warning_count"] == 0 and veh_rt["ok"]) else "CHECK",
        "metrics": [
            ("OMG Stdlib", f"{veh_val['error_count']} err / {veh_val['warning_count']} warn"),
            ("Facts Matched", f"{veh_rt['roundtrip']['total_reemitted_facts']}/{veh_rt['roundtrip']['total_original_facts']}"),
            ("Missing / Extra", f"{veh_rt['roundtrip']['missing_count']} / {veh_rt['roundtrip']['extra_count']}"),
            ("Unsupported Constructs", str(veh_rt["unsupported_logged"])),
        ],
        "links": [
            ("General Viewer", "omg-vehicle.html", True),
            ("IBD Viewer", "omg-vehicle-ibd.html", False),
            ("Re-emitted .sysml", "sysml/omg_vehicle_reemitted.sysml", False),
            ("Round-Trip JSON", "data/omg-vehicle-roundtrip.json", False),
        ],
    },
    {
        "tag": "06 · OMG Official Specification",
        "title": "OMG Canonical Camera",
        "desc": "Camera and PictureTaking action-flow composition validated strictly against OMG stdlib and verified via bidirectional round-trip.",
        "ok": cam_val["valid"] and cam_val["warning_count"] == 0 and cam_rt["ok"],
        "status": "STDLIB + ROUND-TRIP PASS" if (cam_val["valid"] and cam_val["warning_count"] == 0 and cam_rt["ok"]) else "CHECK",
        "metrics": [
            ("OMG Stdlib", f"{cam_val['error_count']} err / {cam_val['warning_count']} warn"),
            ("Facts Matched", f"{cam_rt['roundtrip']['total_reemitted_facts']}/{cam_rt['roundtrip']['total_original_facts']}"),
            ("Missing / Extra", f"{cam_rt['roundtrip']['missing_count']} / {cam_rt['roundtrip']['extra_count']}"),
            ("Unsupported Constructs", str(cam_rt["unsupported_logged"])),
        ],
        "links": [
            ("General Viewer", "omg-camera.html", True),
            ("Re-emitted .sysml", "sysml/omg_camera_reemitted.sysml", False),
            ("Round-Trip JSON", "data/omg-camera-roundtrip.json", False),
        ],
    },
    {
        "tag": "07 · FMEA Fault Injection + Live Sim Trace",
        "title": "Microwave Oven",
        "desc": "Dual-interlock safety cutoff (< 20 ms FTTI) with mid-cook door-open fault injection, interactive SVG trace scrubber, and strict OMG stdlib audit.",
        "ok": mw_audit["ok"] and mw_audit["validation_warnings"] == 0,
        "status": "VERIFIED + SIM TRACE" if (mw_audit["ok"] and mw_audit["validation_warnings"] == 0) else "CHECK",
        "metrics": [
            ("OMG Stdlib", f"{mw_audit['validation_errors']} err / {mw_audit['validation_warnings']} warn"),
            ("Requirements", f"{mw_audit['satisfied_requirements']}/{mw_audit['total_requirements']} satisfied"),
            ("Scenario", "mw-door-open (t=45 s)"),
            ("Sim Playback", "Interactive SVG Trace"),
        ],
        "links": [
            ("Open IBD + Sim Trace", "microwave-sim.html", True),
            ("Raw .sysml", "sysml/microwave.sysml", False),
            ("Trace JSON", "data/microwave-trace.json", False),
            ("Audit JSON", "data/microwave-audit.json", False),
        ],
    },
    {
        "tag": "08 · Hybrid Thermal/Fluidic ODE + Live Sim Trace",
        "title": "Dishwasher",
        "desc": "RK4 tub water mass & thermal balance across Fill -> WashHeat -> Rinse -> Dry -> Drain with mid-wash door-open pause and interactive trace playback.",
        "ok": dw_audit["ok"] and dw_audit["validation_warnings"] == 0,
        "status": "VERIFIED + SIM TRACE" if (dw_audit["ok"] and dw_audit["validation_warnings"] == 0) else "CHECK",
        "metrics": [
            ("OMG Stdlib", f"{dw_audit['validation_errors']} err / {dw_audit['validation_warnings']} warn"),
            ("Requirements", f"{dw_audit['satisfied_requirements']}/{dw_audit['total_requirements']} satisfied"),
            ("Scenario", "dw-door-open (t=400 s)"),
            ("Sim Playback", "Interactive SVG Trace"),
        ],
        "links": [
            ("Open IBD + Sim Trace", "dishwasher-sim.html", True),
            ("Raw .sysml", "sysml/dishwasher.sysml", False),
            ("Trace JSON", "data/dishwasher-trace.json", False),
            ("Audit JSON", "data/dishwasher-audit.json", False),
        ],
    },
    {
        "tag": "09 · Product-Line Variants + Live Sim Trace",
        "title": "Cordless Power Tools Family",
        "desc": "18V 5Ah vs 2Ah battery pack discharge & motor thermal ODE with sub-step ZeroCrossing cutoff detection and interactive trace playback.",
        "ok": pt_audit["ok"] and pt_audit["validation_warnings"] == 0,
        "status": "VERIFIED + SIM TRACE" if (pt_audit["ok"] and pt_audit["validation_warnings"] == 0) else "CHECK",
        "metrics": [
            ("OMG Stdlib", f"{pt_audit['validation_errors']} err / {pt_audit['validation_warnings']} warn"),
            ("Requirements", f"{pt_audit['satisfied_requirements']}/{pt_audit['total_requirements']} satisfied"),
            ("Scenario", "tool-5ah (550 W load)"),
            ("Sim Playback", "Interactive SVG Trace"),
        ],
        "links": [
            ("Open IBD + Sim Trace", "power-tools-sim.html", True),
            ("Raw .sysml", "sysml/power_tools_family.sysml", False),
            ("Trace JSON", "data/power-tools-trace.json", False),
            ("Audit JSON", "data/power-tools-audit.json", False),
        ],
    },
]

card_html = []
for c in cards:
    metrics_html = "".join(
        f'<div><div class="metric-label">{esc(k)}</div><div class="metric-val">{esc(v)}</div></div>'
        for k, v in c["metrics"]
    )
    links_html = "".join(
        f'<a class="btn{" primary" if primary else ""}" href="{esc(href)}">{esc(lbl)}</a>'
        for lbl, href, primary in c["links"]
    )
    badge = status_badge(c["ok"], c["status"], c["status"])
    card_html.append(
        f"""<article class="card">
  <div>
    <div class="card-head">
      <div>
        <div class="card-tag">{esc(c["tag"])}</div>
        <h2>{esc(c["title"])}</h2>
      </div>
      {badge}
    </div>
    <p>{esc(c["desc"])}</p>
  </div>
  <div class="metrics">{metrics_html}</div>
  <div class="actions">{links_html}</div>
</article>"""
    )

index_html = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>rsysdsl — System as Code (Rust DSL &lt;-&gt; SysML v2)</title>
  <style>{BASE_CSS}</style>
</head>
<body>
  <div class="wrap">
    <header class="hero">
      <div class="eyebrow">System as Code · Offline Static Showcase</div>
      <h1>rsysdsl — System as Code (Rust DSL &lt;-&gt; SysML v2)</h1>
      <p class="sub">
        Deterministic bidirectional compilation between strongly typed Rust DSL (<code>#[sysml::*]</code>) and
        OMG-valid SysML v2, verified against the official OMG Standard Library (<code>sysml.library</code>) with
        self-contained interactive HTML diagram viewers.
      </p>
      <div class="badges">
        <span class="badge ok">{total_rt_facts:,} round-trip facts verified</span>
        <span class="badge {'ok' if total_stdlib_errors == 0 and total_stdlib_warnings == 0 else 'bad'}">{total_stdlib_errors} stdlib errors / {total_stdlib_warnings} warnings</span>
        <span class="badge">commit <code>{esc(git_sha)}</code></span>
        <span class="badge">built {esc(build_utc)}</span>
        <a class="badge" href="trace-matrix.html">ASPICE Traceability Matrix</a>
        <a class="badge" href="safety-report.html">ISO 26262 Safety Report</a>
      </div>
    </header>
    <main class="grid">
      {''.join(card_html)}
    </main>
    <footer>
      <span>Generated by scripts/build-pages-site.sh · Zero external CDN dependencies</span>
      <span>9 verified models · 12 interactive diagram views (3 with live simulation traces)</span>
    </footer>
  </div>
</body>
</html>
"""
(out_dir / "index.html").write_text(index_html, encoding="utf-8")

# Render trace-matrix.html
tsum = a11_trace["summary"]
trace_rows_html = []
for row in a11_trace["rows"]:
    st = row["verification_status"]
    st_cls = "ok" if st == "pass" else ("bad" if st == "fail" else "warn")
    constraints = "<br>".join(f"<code>{esc(x)}</code>" for x in row["constraints"]) or "—"
    refines = ", ".join(f"<code>{esc(x)}</code>" for x in row["refines"]) or "—"
    sats = ", ".join(f"<code>{esc(x)}</code>" for x in row["satisfied_by"]) or "—"
    fns = ", ".join(f"<code>{esc(x)}</code>" for x in row["allocated_functions"]) or "—"
    ev = esc(json.dumps(row["evidence"])) if row["evidence"] is not None else "—"
    trace_rows_html.append(
        f"""<tr>
  <td><code>{esc(row['req_id'])}</code></td>
  <td><strong>{esc(row['req_name'])}</strong><br><span style="color:var(--muted);font-size:12px">{esc(row['package'])}</span></td>
  <td>{refines}</td>
  <td>{constraints}</td>
  <td>{sats}</td>
  <td>{fns}</td>
  <td><span class="badge {st_cls}">{esc(st.upper())}</span></td>
  <td><code>{ev}</code></td>
</tr>"""
    )

trace_html = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>ASPICE Bidirectional Traceability Matrix — {esc(a11_trace['model'])}</title>
  <style>{BASE_CSS}</style>
</head>
<body>
  <div class="wrap">
    <p><a class="btn" href="index.html">&larr; Back to Showcase</a></p>
    <header class="hero">
      <div class="eyebrow">ASPICE Bidirectional Traceability</div>
      <h1>{esc(a11_trace['model'])} — Requirements to Verification Matrix</h1>
      <div class="badges">
        <span class="badge ok">{tsum['satisfied_requirements']}/{tsum['total_requirements']} satisfied ({tsum['satisfaction_coverage_pct']}%)</span>
        <span class="badge ok">{tsum['verified_pass']}/{tsum['constrained_requirements']} constrained verified PASS ({tsum['verification_coverage_pct']}%)</span>
        <span class="badge">{tsum['total_parts']} parts · {tsum['total_actions']} actions · {tsum['total_calcs']} calcs</span>
        <span class="badge">commit <code>{esc(git_sha)}</code></span>
      </div>
    </header>
    <section class="panel">
      <table>
        <thead>
          <tr>
            <th>ID</th>
            <th>Requirement</th>
            <th>Refines</th>
            <th>Constraints</th>
            <th>Satisfied By</th>
            <th>Allocated Functions</th>
            <th>Status</th>
            <th>Evidence</th>
          </tr>
        </thead>
        <tbody>
          {''.join(trace_rows_html)}
        </tbody>
      </table>
    </section>
  </div>
</body>
</html>
"""
(out_dir / "trace-matrix.html").write_text(trace_html, encoding="utf-8")

# Render safety-report.html
goal_rows_html = []
for g in fc_safety["safety_goals"]:
    sats = ", ".join(f"<code>{esc(x)}</code>" for x in g["satisfied_by"]) or "—"
    decomps = "<br>".join(
        f"<code>{esc(d['left_part'])} [{esc(d['left_asil'])}] + {esc(d['right_part'])} [{esc(d['right_asil'])}]</code>"
        for d in g["decompositions"]
    ) or "—"
    goal_rows_html.append(
        f"""<tr>
  <td><code>{esc(g['req_id'])}</code></td>
  <td><strong>{esc(g['req_name'])}</strong></td>
  <td><span class="badge ok">{esc(g['asil'])}</span></td>
  <td><code>{esc(g['safe_state'])}</code></td>
  <td>{esc(g['ftti_ms'])} ms</td>
  <td>{sats}</td>
  <td>{decomps}</td>
</tr>"""
    )

fmea_rows_html = []
for f in fc_safety["fmea_entries"]:
    asil = f["asil"] or "—"
    fmea_rows_html.append(
        f"""<tr>
  <td><code>{esc(f['part_name'])}</code></td>
  <td>{esc(f['failure_mode'])}</td>
  <td>{esc(f['effect'])}</td>
  <td>{esc(f['severity'])}</td>
  <td>{esc(f['occurrence'])}</td>
  <td>{esc(f['detection'])}</td>
  <td><strong>{esc(f['rpn'])}</strong></td>
  <td><code>{esc(asil)}</code></td>
</tr>"""
    )

cut_rows_html = []
for cs in fc_safety["cut_sets"]:
    elems = ", ".join(f"<code>{esc(e)}</code>" for e in cs["elements"])
    kind_cls = "ok" if cs["order"] >= 2 else "warn"
    cut_rows_html.append(
        f"""<tr>
  <td>{esc(cs['order'])}</td>
  <td><span class="badge {kind_cls}">{esc(cs['kind'])}</span></td>
  <td><code>{esc(cs['target'])}</code></td>
  <td>{elems}</td>
  <td>{esc(cs['note'])}</td>
</tr>"""
    )

safety_html = f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>ISO 26262 Safety, FMEA &amp; Cut-Set Report — {esc(fc_safety['model_name'])}</title>
  <style>{BASE_CSS}</style>
</head>
<body>
  <div class="wrap">
    <p><a class="btn" href="index.html">&larr; Back to Showcase</a></p>
    <header class="hero">
      <div class="eyebrow">ISO 26262 Safety Analysis</div>
      <h1>{esc(fc_safety['model_name'])} — Safety Goals, FMEA &amp; Minimal Cut Sets</h1>
      <div class="badges">
        <span class="badge ok">{fc_safety['total_safety_goals']} safety goals</span>
        <span class="badge ok">{fc_safety['total_decompositions']} ASIL decompositions</span>
        <span class="badge">{fc_safety['total_fmea_entries']} FMEA entries (max RPN {fc_safety['max_rpn']})</span>
        <span class="badge">{fc_safety['single_point_fault_count']} single-point / {fc_safety['dual_channel_cut_set_count']} dual-channel cut sets</span>
        <span class="badge">commit <code>{esc(git_sha)}</code></span>
      </div>
    </header>
    <section class="panel">
      <h2>Safety Goals (<code>@SafetyGoal</code>)</h2>
      <table>
        <thead>
          <tr><th>Goal ID</th><th>Name</th><th>ASIL</th><th>Safe State</th><th>FTTI</th><th>Satisfied By</th><th>ASIL Decomposition</th></tr>
        </thead>
        <tbody>{''.join(goal_rows_html)}</tbody>
      </table>
    </section>
    <section class="panel">
      <h2>FMEA Ranking (<code>@FMEA</code>, Descending by RPN)</h2>
      <table>
        <thead>
          <tr><th>Part</th><th>Failure Mode</th><th>Effect</th><th>S</th><th>O</th><th>D</th><th>RPN</th><th>ASIL</th></tr>
        </thead>
        <tbody>{''.join(fmea_rows_html)}</tbody>
      </table>
    </section>
    <section class="panel">
      <h2>Minimal Fault Cut Sets</h2>
      <table>
        <thead>
          <tr><th>Order</th><th>Kind</th><th>Target</th><th>Elements</th><th>Notes</th></tr>
        </thead>
        <tbody>{''.join(cut_rows_html)}</tbody>
      </table>
    </section>
  </div>
</body>
</html>
"""
(out_dir / "safety-report.html").write_text(safety_html, encoding="utf-8")
PYEOF

echo "Pages showcase generated cleanly under $OUT_DIR" >&2
