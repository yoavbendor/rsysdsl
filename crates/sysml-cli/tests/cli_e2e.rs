//! End-to-end CLI tests for `rsysdsl` verifying JSON stdout, stderr diagnostics,
//! and clean exit codes (`0` = ok, `1` = validation/roundtrip/verify failure, `2` = IO/parse error).

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

#[test]
fn cli_emit_outputs_valid_json_and_zero_exit() {
    let out = Command::new(bin_path())
        .args(["emit", "--model", "apollo11-slice"])
        .output()
        .expect("run rsysdsl emit");
    assert_eq!(out.status.code(), Some(0));
    let val: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid compact JSON");
    assert_eq!(val["validation_ok"], true);
    assert_eq!(val["validation_errors"], 0);
    assert!(val["packages"].as_u64().unwrap() >= 8);
}

#[test]
fn cli_verify_nominal_and_fault_exit_codes() {
    let nom = Command::new(bin_path())
        .args(["verify", "--scenario", "nominal"])
        .output()
        .expect("run rsysdsl verify nominal");
    assert_eq!(nom.status.code(), Some(0));
    let nom_json: serde_json::Value = serde_json::from_slice(&nom.stdout).unwrap();
    assert_eq!(nom_json["all_passed"], true);
    assert_eq!(nom_json["passed"], 4);

    let fault = Command::new(bin_path())
        .args(["verify", "--scenario", "fault"])
        .output()
        .expect("run rsysdsl verify fault");
    assert_eq!(fault.status.code(), Some(1));
    let fault_json: serde_json::Value = serde_json::from_slice(&fault.stdout).unwrap();
    assert_eq!(fault_json["all_passed"], false);
    assert!(fault_json["failed"].as_u64().unwrap() >= 2);
}

#[test]
fn cli_roundtrip_check_on_apollo11_slice_passes() {
    let root = workspace_root().join("tests/fixtures/apollo-11-sysml-v2");
    let out = Command::new(bin_path())
        .arg("roundtrip-check")
        .arg(root.join("CoSMA/CoSMAPackage.sysml"))
        .arg(root.join("Logical/LogicalComponentsPackage.sysml"))
        .arg(root.join("Technical/TechnicalPortsPackage.sysml"))
        .arg(root.join("Technical/TechnicalComponentsPackage.sysml"))
        .output()
        .expect("run rsysdsl roundtrip-check");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["ok"], true);
    assert_eq!(val["validation_ok"], true);
    assert_eq!(val["roundtrip"]["fidelity_ok"], true);
}

#[test]
fn cli_missing_file_returns_exit_code_2() {
    let out = Command::new(bin_path())
        .args(["validate", "/nonexistent/does_not_exist.sysml"])
        .output()
        .expect("run rsysdsl validate missing");
    assert_eq!(out.status.code(), Some(2));
    assert!(!out.stderr.is_empty());
}
