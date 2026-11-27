#!/usr/bin/env python3
"""Headless Chromium smoke test for the generated rsysdsl static Pages site."""

from __future__ import annotations

import os
import sys
from pathlib import Path
from playwright.sync_api import sync_playwright

PORTAL_PAGES = [
    "index.html",
    "trace-matrix.html",
    "safety-report.html",
]

VIEWER_PAGES = [
    "starter-ibd.html",
    "starter-general.html",
    "fc-state.html",
    "fc-ibd.html",
    "apollo11-rust.html",
    "apollo11-corpus.html",
    "omg-vehicle.html",
    "omg-vehicle-ibd.html",
    "omg-camera.html",
]

SIM_VIEWER_PAGES = [
    "microwave-sim.html",
    "dishwasher-sim.html",
    "power-tools-sim.html",
]


def detect_chromium_executable() -> str | None:
    for candidate in (
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
    ):
        if os.path.isfile(candidate) and os.access(candidate, os.X_OK):
            return candidate
    return None


def main() -> int:
    site_dir = Path(sys.argv[1] if len(sys.argv) > 1 else "_site").resolve()
    if not site_dir.is_dir():
        print(f"FAIL: site directory does not exist: {site_dir}", file=sys.stderr)
        return 2

    launch_kwargs: dict[str, object] = {"headless": True}
    exe = detect_chromium_executable()
    if exe:
        launch_kwargs["executable_path"] = exe

    failures = 0
    all_pages = (
        [(p, False, False) for p in PORTAL_PAGES]
        + [(p, True, False) for p in VIEWER_PAGES]
        + [(p, True, True) for p in SIM_VIEWER_PAGES]
    )

    with sync_playwright() as pw:
        browser = pw.chromium.launch(**launch_kwargs)
        try:
            context = browser.new_context(viewport={"width": 1440, "height": 900})
            for rel, is_viewer, is_sim in all_pages:
                path = site_dir / rel
                if not path.is_file():
                    print(f"FAIL  {rel:24s} missing file: {path}")
                    failures += 1
                    continue

                page = context.new_page()
                page_errors: list[str] = []
                console_errors: list[str] = []
                page.on("pageerror", lambda err, out=page_errors: out.append(str(err)))
                page.on(
                    "console",
                    lambda msg, out=console_errors: (
                        out.append(msg.text) if msg.type == "error" else None
                    ),
                )

                page.goto(path.as_uri(), wait_until="load")
                if is_viewer:
                    page.wait_for_timeout(1500)
                    metrics = page.evaluate(
                        """() => {
                            const svgs = Array.from(document.querySelectorAll('svg'));
                            let shapeCount = 0;
                            let textCount = 0;
                            for (const svg of svgs) {
                                shapeCount += svg.querySelectorAll('rect, path, circle, polygon, line, ellipse').length;
                                textCount += svg.querySelectorAll('text, tspan').length;
                            }
                            const drawer = document.getElementById('rsysdsl-trace-drawer');
                            const cards = document.querySelectorAll('#rsysdsl-trace-grid .rst-card').length;
                            const scrubber = document.getElementById('rsysdsl-trace-scrubber');
                            let scrubbedTime = '';
                            if (scrubber) {
                                scrubber.value = scrubber.max;
                                scrubber.dispatchEvent(new Event('input', { bubbles: true }));
                                const lbl = document.getElementById('rst-time-lbl');
                                scrubbedTime = lbl ? lbl.textContent.trim() : '';
                            }
                            return {
                                svgCount: svgs.length,
                                shapeCount,
                                textCount,
                                hasDrawer: Boolean(drawer),
                                traceCards: cards,
                                scrubbedTime,
                            };
                        }"""
                    )
                else:
                    metrics = {
                        "svgCount": 0,
                        "shapeCount": 0,
                        "textCount": 0,
                        "hasDrawer": False,
                        "traceCards": 0,
                        "scrubbedTime": "",
                    }

                page.close()

                ok = len(page_errors) == 0 and len(console_errors) == 0
                if is_viewer:
                    ok = (
                        ok
                        and metrics["svgCount"] > 0
                        and metrics["shapeCount"] > 0
                        and metrics["textCount"] > 0
                    )
                if is_sim:
                    ok = (
                        ok
                        and metrics["hasDrawer"]
                        and metrics["traceCards"] > 0
                        and metrics["scrubbedTime"] != ""
                    )

                status = "PASS" if ok else "FAIL"
                if is_sim:
                    detail = (
                        f"svgs={metrics['svgCount']} trace_cards={metrics['traceCards']} "
                        f"end={metrics['scrubbedTime']} pageerrors={len(page_errors)} "
                        f"console_errors={len(console_errors)}"
                    )
                elif is_viewer:
                    detail = (
                        f"svgs={metrics['svgCount']} shapes={metrics['shapeCount']} "
                        f"texts={metrics['textCount']} pageerrors={len(page_errors)} "
                        f"console_errors={len(console_errors)}"
                    )
                else:
                    detail = (
                        f"pageerrors={len(page_errors)} console_errors={len(console_errors)}"
                    )
                print(f"{status:4s}  {rel:24s} {detail}")
                if not ok:
                    failures += 1
                    for err in page_errors:
                        print(f"      pageerror: {err}")
                    for err in console_errors:
                        print(f"      console.error: {err}")
        finally:
            browser.close()

    if failures:
        print(f"\nSmoke test FAILED: {failures} page(s) failed", file=sys.stderr)
        return 1
    print(f"\nSmoke test PASSED: all {len(all_pages)} pages rendered cleanly")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
