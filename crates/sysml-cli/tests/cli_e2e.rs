//! End-to-end CLI tests for `rsysdsl` verifying JSON stdout, stderr diagnostics,
//! traceability audit, and clean exit codes (`0` = ok, `1` = failure, `2` = IO/parse error).

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
        .args(["emit", "--model", "apollo11-model"])
        .output()
        .expect("run rsysdsl emit");
    assert_eq!(out.status.code(), Some(0));
    let val: serde_json::Value =
        serde_json::from_slice(&out.stdout).expect("stdout must be valid compact JSON");
    assert_eq!(val["validation_ok"], true);
    assert_eq!(val["validation_errors"], 0);
    assert_eq!(val["packages"], 10);
}

#[test]
fn cli_audit_with_verify_passes_with_zero_findings() {
    let out = Command::new(bin_path())
        .args(["audit", "--model", "apollo11-model", "--verify"])
        .output()
        .expect("run rsysdsl audit");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["ok"], true);
    assert_eq!(val["validation_ok"], true);
    assert_eq!(val["orphan_requirements"].as_array().unwrap().len(), 0);
    assert_eq!(
        val["unverified_constrained_requirements"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(val["total_calcs"], 3);
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

#[test]
fn cli_audit_with_stdlib_and_strict_passes_with_zero_warnings() {
    let out = Command::new(bin_path())
        .args([
            "audit",
            "--model",
            "apollo11-model",
            "--verify",
            "--stdlib",
            "--strict",
        ])
        .output()
        .expect("run rsysdsl audit --stdlib --strict");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["ok"], true);
    assert_eq!(val["stdlib_loaded"], true);
    assert_eq!(val["validation_errors"], 0);
    assert_eq!(val["validation_warnings"], 0);
}

#[test]
fn cli_trace_matrix_json_markdown_csv_and_sysml_files() {
    // 1. JSON with verification verdicts on apollo11-model
    let json_out = Command::new(bin_path())
        .args([
            "trace-matrix",
            "--model",
            "apollo11-model",
            "--verify",
            "--format",
            "json",
        ])
        .output()
        .expect("run rsysdsl trace-matrix json");
    assert_eq!(
        json_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&json_out.stderr)
    );
    let val: serde_json::Value = serde_json::from_slice(&json_out.stdout).unwrap();
    assert_eq!(val["summary"]["total_requirements"], 5);
    assert_eq!(val["summary"]["satisfied_requirements"], 5);
    assert_eq!(val["summary"]["satisfaction_coverage_pct"], 100.0);
    assert_eq!(val["summary"]["verified_pass"], 4);
    assert_eq!(val["summary"]["verified_fail"], 0);
    assert_eq!(val["summary"]["total_calcs"], 3);

    // 2. Markdown table
    let md_out = Command::new(bin_path())
        .args([
            "trace-matrix",
            "--model",
            "apollo11-model",
            "--verify",
            "--format",
            "markdown",
        ])
        .output()
        .expect("run rsysdsl trace-matrix markdown");
    assert_eq!(md_out.status.code(), Some(0));
    let md = String::from_utf8(md_out.stdout).unwrap();
    assert!(md.contains("# ASPICE Bidirectional Traceability Matrix"));
    assert!(md.contains("| Req ID | Requirement | Package | Satisfied By | Allocated Functions | Constraints | Verification |"));
    assert!(md.contains("HLR-R002"));
    assert!(md.contains("CLR-R050"));
    assert!(md.contains("CLR-R057"));
    assert!(md.contains("CLR-R058"));

    // 3. RFC-4180 CSV
    let csv_out = Command::new(bin_path())
        .args([
            "trace-matrix",
            "--model",
            "apollo11-model",
            "--verify",
            "--format",
            "csv",
        ])
        .output()
        .expect("run rsysdsl trace-matrix csv");
    assert_eq!(csv_out.status.code(), Some(0));
    let csv = String::from_utf8(csv_out.stdout).unwrap();
    assert!(csv.starts_with(
        "req_id,req_name,package,satisfied_by,allocated_functions,constraints,refines,verification_status,verification_scenario\n"
    ));
    assert!(csv.contains("CLR-R050"));

    // 4. Directly on real Airbus Apollo 11 .sysml fixture files
    let root = workspace_root().join("tests/fixtures/apollo-11-sysml-v2");
    let files_out = Command::new(bin_path())
        .arg("trace-matrix")
        .arg("--files")
        .arg(root.join("Requirements/TechnicalRequirementsPackage.sysml"))
        .arg(root.join("Technical/SystemSpecificationPackage.sysml"))
        .args(["--format", "json"])
        .output()
        .expect("run rsysdsl trace-matrix --files");
    assert_eq!(
        files_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&files_out.stderr)
    );
    let fval: serde_json::Value = serde_json::from_slice(&files_out.stdout).unwrap();
    assert!(fval["summary"]["total_requirements"].as_u64().unwrap() >= 50);
    assert!(fval["summary"]["satisfied_requirements"].as_u64().unwrap() >= 40);
}

#[test]
fn cli_audit_with_files_stdlib_strict_passes() {
    let dir = tempfile::tempdir().unwrap();
    let emitted_path = dir.path().join("apollo11.sysml");

    let emit_out = Command::new(bin_path())
        .args(["emit", "--model", "apollo11-model", "--raw", "-o"])
        .arg(&emitted_path)
        .output()
        .expect("emit apollo11-model to temp file");
    assert_eq!(emit_out.status.code(), Some(0));

    let audit_out = Command::new(bin_path())
        .arg("audit")
        .arg("--files")
        .arg(&emitted_path)
        .args(["--stdlib", "--strict"])
        .output()
        .expect("run rsysdsl audit --files --stdlib --strict");
    assert_eq!(
        audit_out.status.code(),
        Some(0),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&audit_out.stderr),
        String::from_utf8_lossy(&audit_out.stdout)
    );
    let val: serde_json::Value = serde_json::from_slice(&audit_out.stdout).unwrap();
    assert_eq!(val["ok"], true);
    assert_eq!(val["stdlib_loaded"], true);
    assert_eq!(val["validation_errors"], 0);
    assert_eq!(val["validation_warnings"], 0);
    assert_eq!(val["orphan_requirements"].as_array().unwrap().len(), 0);
}
