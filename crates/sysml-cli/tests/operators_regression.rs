//! Regression tests for the budget / refute / trade-study operators against the
//! Haiku trial-3 fixtures in `tests/fixtures/haiku_t3/`.
//!
//! These tests are written against the D2/D3 JSON contract from
//! `docs/SPRINT_CONSOLIDATE_PLAN.md` ("JSON contract for C1 / C1-T (fixed)"):
//! - `budget`: each domain has `status` (`pass | fail | incomplete`) and
//!   `untracked` (`[{element, attribute, unit, value}]`); top-level `ok` is false
//!   unless every limited domain is `pass`; `--strict` exits 1 when `ok` is false.
//! - `refute`: top-level `status` (`feasible | refuted | not_applicable`);
//!   `feasible` is `null` when `not_applicable`.
//! - `trade-study`: top-level `status` (`ok | not_applicable`).
//!
//! Many of these fail on the current code by design (no-false-pass regressions).

use std::path::{Path, PathBuf};
use std::process::Command;

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rsysdsl"))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn fixture_path(name: &str) -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join("haiku_t3")
        .join(name)
}

/// Run `rsysdsl` with `args`, return (exit code, parsed stdout JSON, stderr text).
fn run_json(args: &[&str]) -> (Option<i32>, serde_json::Value, String) {
    let out = Command::new(bin_path())
        .args(args)
        .output()
        .expect("run rsysdsl");
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout must be valid JSON for args {:?}: {e}\nstdout: {}\nstderr: {stderr}",
            args,
            String::from_utf8_lossy(&out.stdout)
        )
    });
    (out.status.code(), val, stderr)
}

fn pretty(v: &serde_json::Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string())
}

fn has_untracked_attribute(domain: &serde_json::Value, attribute: &str) -> bool {
    domain["untracked"]
        .as_array()
        .map(|entries| entries.iter().any(|e| e["attribute"] == attribute))
        .unwrap_or(false)
}

const TRIAL3_FIXTURES: [&str; 12] = [
    "greenhouse_r1.sysml",
    "greenhouse_r2.sysml",
    "greenhouse_r3.sysml",
    "ebike_r1.sysml",
    "ebike_r2.sysml",
    "ebike_r3.sysml",
    "water_heater_r1.sysml",
    "water_heater_r2.sysml",
    "water_heater_r3.sysml",
    "ventilation_r1.sysml",
    "ventilation_r2.sysml",
    "ventilation_r3.sysml",
];

// (a) P2: an unrecognised `ratedPowerW [W]` must not produce a false PASS.
#[test]
fn budget_does_not_pass_unrecognised_watts_water_heater() {
    let fixture = fixture_path("water_heater_r1.sysml");
    let (_code, val, _stderr) = run_json(&[
        "budget",
        "--files",
        fixture.to_str().unwrap(),
        "--max-power-w",
        "100",
    ]);
    let power = &val["power"];
    assert_ne!(
        power["status"], "pass",
        "false PASS: power.status must not be \"pass\" for an untracked 2000 W element\n{}",
        pretty(&val)
    );
    assert_eq!(
        power["status"], "incomplete",
        "power.status must be \"incomplete\"\n{}",
        pretty(&val)
    );
    assert!(
        has_untracked_attribute(power, "ratedPowerW"),
        "power.untracked must contain an entry with attribute \"ratedPowerW\"\n{}",
        pretty(power)
    );
    assert_eq!(
        val["ok"], false,
        "top-level ok must be false\n{}",
        pretty(&val)
    );
}

// (b) Same as (a) with --strict: exit code must be 1.
#[test]
fn budget_strict_exits_nonzero_water_heater() {
    let fixture = fixture_path("water_heater_r1.sysml");
    let (code, val, stderr) = run_json(&[
        "budget",
        "--files",
        fixture.to_str().unwrap(),
        "--max-power-w",
        "100",
        "--strict",
    ]);
    assert_eq!(
        code,
        Some(1),
        "budget --strict must exit 1 when ok is false\nstdout:\n{}\nstderr: {stderr}",
        pretty(&val)
    );
}

// (c) Same as (a) on ventilation: two 170 W fans with `ratedPowerW`.
#[test]
fn budget_does_not_pass_unrecognised_watts_ventilation() {
    let fixture = fixture_path("ventilation_r1.sysml");
    let (_code, val, _stderr) = run_json(&[
        "budget",
        "--files",
        fixture.to_str().unwrap(),
        "--max-power-w",
        "100",
    ]);
    let power = &val["power"];
    assert_ne!(
        power["status"], "pass",
        "false PASS: power.status must not be \"pass\" for untracked fan watts\n{}",
        pretty(&val)
    );
    assert_eq!(
        power["status"], "incomplete",
        "power.status must be \"incomplete\"\n{}",
        pretty(&val)
    );
    assert!(
        has_untracked_attribute(power, "ratedPowerW"),
        "power.untracked must contain an entry with attribute \"ratedPowerW\"\n{}",
        pretty(power)
    );
    assert_eq!(
        val["ok"], false,
        "top-level ok must be false\n{}",
        pretty(&val)
    );
}

// (d) Over all 12 trial-3 models: a model with `[W]` quantities must never
// silently pass a power budget with zero counted power.
#[test]
fn budget_never_silently_passes_on_any_trial_model() {
    let mut violating: Vec<&str> = Vec::new();
    let mut details: Vec<String> = Vec::new();
    for name in TRIAL3_FIXTURES {
        let path = fixture_path(name);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
        if !text.contains("[W]") {
            continue;
        }
        let (_code, val, _stderr) = run_json(&[
            "budget",
            "--files",
            path.to_str().unwrap(),
            "--max-power-w",
            "1000000000",
        ]);
        let power = &val["power"];
        let total = power["total"].as_f64();
        // The contract requires a string `status` on every domain. A missing
        // status, or "pass", on a zero-total [W] model is a silent pass.
        let status_is_not_pass = power["status"].as_str().is_some_and(|s| s != "pass");
        if total == Some(0.0) && !status_is_not_pass {
            violating.push(name);
            details.push(format!(
                "{name}: power.total = 0 with [W] present, but power.status = {} (expected a string other than \"pass\")\n{}",
                power["status"],
                pretty(power)
            ));
        } else if total.is_none() {
            violating.push(name);
            details.push(format!(
                "{name}: power.total missing or not a number\n{}",
                pretty(power)
            ));
        }
    }
    assert!(
        violating.is_empty(),
        "silent false PASS on {} trial-3 fixture(s): {}\n\n{}",
        violating.len(),
        violating.join(", "),
        details.join("\n---\n")
    );
}

// (e) P3: a model with no variations and no recognised requirements is
// not_applicable, never "feasible: false".
#[test]
fn refute_empty_problem_is_not_applicable() {
    let fixture = fixture_path("water_heater_r1.sysml");
    let (_code, val, _stderr) = run_json(&["refute", "--files", fixture.to_str().unwrap()]);
    assert_eq!(
        val["status"], "not_applicable",
        "refute status must be \"not_applicable\" for an empty problem\n{}",
        pretty(&val)
    );
    assert!(
        val["feasible"].is_null(),
        "refute.feasible must be JSON null when not_applicable (got {})\n{}",
        val["feasible"],
        pretty(&val)
    );
}

// (f) trade-study with no #[sysml::variation] is not_applicable.
#[test]
fn trade_study_without_variations_is_not_applicable() {
    let fixture = fixture_path("water_heater_r1.sysml");
    let (_code, val, _stderr) = run_json(&["trade-study", "--files", fixture.to_str().unwrap()]);
    assert_eq!(
        val["status"], "not_applicable",
        "trade-study status must be \"not_applicable\" when there are no variations\n{}",
        pretty(&val)
    );
}

// (g) Guard against over-correction: the built-in dishwasher is within a
// very large power budget and must still pass.
#[test]
fn budget_on_builtin_dishwasher_still_passes() {
    let (_code, val, _stderr) = run_json(&[
        "budget",
        "--model",
        "dishwasher",
        "--max-power-w",
        "1000000",
    ]);
    assert_eq!(
        val["ok"], true,
        "built-in dishwasher must still pass a 1 MW budget (ok = true)\n{}",
        pretty(&val)
    );
}
