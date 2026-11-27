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

#[test]
fn cli_safety_report_builtin_and_files_passes() {
    // 1. Built-in embedded-flight-controller JSON report
    let json_out = Command::new(bin_path())
        .args([
            "safety-report",
            "--model",
            "embedded-flight-controller",
            "--format",
            "json",
        ])
        .output()
        .expect("run rsysdsl safety-report json");
    assert_eq!(
        json_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&json_out.stderr)
    );
    let val: serde_json::Value = serde_json::from_slice(&json_out.stdout).unwrap();
    assert_eq!(val["total_safety_goals"], 1);
    assert_eq!(val["total_decompositions"], 1);
    assert_eq!(val["total_fmea_entries"], 2);
    assert_eq!(val["max_rpn"], 60);
    assert_eq!(val["single_point_fault_count"], 0);
    assert_eq!(val["dual_channel_cut_set_count"], 1);

    // 2. Markdown report
    let md_out = Command::new(bin_path())
        .args([
            "safety-report",
            "--model",
            "embedded-flight-controller",
            "--format",
            "markdown",
        ])
        .output()
        .expect("run rsysdsl safety-report markdown");
    assert_eq!(md_out.status.code(), Some(0));
    let md = String::from_utf8(md_out.stdout).unwrap();
    assert!(md.contains("# ISO 26262 Safety, FMEA & Cut-Set Report"));
    assert!(md.contains("SG-001"));
    assert!(md.contains("SpuriousThrottleHigh"));
    assert!(md.contains("dual_channel_decomposed"));

    // 3. Emit to .sysml -> safety-report --files round-trip!
    let dir = tempfile::tempdir().unwrap();
    let emitted = dir.path().join("flight_controller.sysml");
    let emit_out = Command::new(bin_path())
        .args([
            "emit",
            "--model",
            "embedded-flight-controller",
            "--raw",
            "-o",
        ])
        .arg(&emitted)
        .output()
        .expect("emit embedded-flight-controller");
    assert_eq!(emit_out.status.code(), Some(0));

    let files_out = Command::new(bin_path())
        .arg("safety-report")
        .arg("--files")
        .arg(&emitted)
        .args(["--format", "json"])
        .output()
        .expect("run rsysdsl safety-report --files");
    assert_eq!(
        files_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&files_out.stderr)
    );
    let fval: serde_json::Value = serde_json::from_slice(&files_out.stdout).unwrap();
    assert_eq!(fval["total_safety_goals"], 1);
    assert_eq!(fval["total_decompositions"], 1);
    assert_eq!(fval["total_fmea_entries"], 2);
    assert_eq!(fval["max_rpn"], 60);
    assert_eq!(fval["single_point_fault_count"], 0);
    assert_eq!(fval["dual_channel_cut_set_count"], 1);
}

#[test]
fn cli_view_generates_standalone_html_for_files_and_builtin_models() {
    let dir = tempfile::tempdir().unwrap();

    // 1. Built-in embedded-flight-controller with --view state (dark default theme)
    let fc_html_path = dir.path().join("fc.html");
    let fc_out = Command::new(bin_path())
        .args([
            "view",
            "--model",
            "embedded-flight-controller",
            "--view",
            "state",
            "-o",
        ])
        .arg(&fc_html_path)
        .output()
        .expect("run rsysdsl view built-in model");
    assert_eq!(
        fc_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&fc_out.stderr)
    );
    let fc_json: serde_json::Value =
        serde_json::from_slice(&fc_out.stdout).expect("stdout must be valid JSON");
    assert_eq!(fc_json["ok"], true);
    assert_eq!(fc_json["view"], "state");
    assert_eq!(fc_json["theme"], "dark");
    assert!(fc_html_path.exists());
    let fc_html = std::fs::read_to_string(&fc_html_path).unwrap();
    assert!(fc_html.contains("<!DOCTYPE html>"));
    assert!(fc_html.contains("SysMLDiagramRuntime.mount"));
    assert!(fc_html.contains("\"currentView\":\"state\""));
    assert!(fc_html.contains("ControllerMode"));
    assert!(fc_html.contains("data-theme=\"dark\""));

    // 2. Lift a .sysml file and render interconnection (ibd) view in light theme
    let demo_sysml_path = dir.path().join("demo.sysml");
    std::fs::write(
        &demo_sysml_path,
        r#"package ThermalDemo {
    port def TempPort {
        out item t : Real;
    }
    part def Sensor {
        port out_p : TempPort;
    }
    part def Controller {
        port in_p : ~TempPort;
    }
    part def Monitor {
        part s : Sensor;
        part c : Controller;
        connection link connect s.out_p to c.in_p;
    }
}
"#,
    )
    .unwrap();

    let demo_html_path = dir.path().join("demo.html");
    let demo_out = Command::new(bin_path())
        .arg("view")
        .arg(&demo_sysml_path)
        .args(["--view", "interconnection", "--theme", "light", "-o"])
        .arg(&demo_html_path)
        .output()
        .expect("run rsysdsl view on .sysml file");
    assert_eq!(
        demo_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&demo_out.stderr)
    );
    let demo_json: serde_json::Value =
        serde_json::from_slice(&demo_out.stdout).expect("stdout must be valid JSON");
    assert_eq!(demo_json["ok"], true);
    assert_eq!(demo_json["view"], "ibd");
    assert_eq!(demo_json["theme"], "light");
    let demo_html = std::fs::read_to_string(&demo_html_path).unwrap();
    assert!(demo_html.contains("data-theme=\"light\""));
    assert!(demo_html.contains("\"currentView\":\"ibd\""));
    assert!(demo_html.contains("ThermalDemo"));
    assert!(demo_html.contains("s.out_p"));

    // 3. --json-snapshot prints valid ViewerSnapshot JSON with "command": "update"
    let snap_out = Command::new(bin_path())
        .arg("view")
        .arg(&demo_sysml_path)
        .args(["--view", "interconnection", "--json-snapshot"])
        .output()
        .expect("run rsysdsl view --json-snapshot");
    assert_eq!(
        snap_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&snap_out.stderr)
    );
    let snap_json: serde_json::Value =
        serde_json::from_slice(&snap_out.stdout).expect("snapshot must be valid JSON");
    assert_eq!(snap_json["command"], "update");
    assert_eq!(snap_json["currentView"], "ibd");
}

#[test]
fn cli_lint_reports_metrics_and_smells() {
    // 1. Built-in embedded-flight-controller exits 0 and emits valid JSON with summary.total_parts > 0
    let fc_out = Command::new(bin_path())
        .args(["lint", "--model", "embedded-flight-controller"])
        .output()
        .expect("run rsysdsl lint on embedded-flight-controller");
    assert_eq!(
        fc_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&fc_out.stderr)
    );
    let fc_json: serde_json::Value =
        serde_json::from_slice(&fc_out.stdout).expect("stdout must be valid JSON");
    assert!(
        fc_json["summary"]["total_parts"].as_u64().unwrap_or(0) > 0,
        "expected summary.total_parts > 0, got {}",
        fc_json["summary"]["total_parts"]
    );

    // 2. Duplicate unspecialized parts sharing 3 features trigger duplicate_part_shape and exit 1 under --strict
    let dir = tempfile::tempdir().unwrap();
    let smell_path = dir.path().join("smells.sysml");
    std::fs::write(
        &smell_path,
        r#"package LintSmellDemo {
    port def PowerPort;
    part def TempSensor {
        attribute sampleRate : Real = 100.0;
        attribute mass : Real = 0.05;
        port powerIn : PowerPort;
    }
    part def PressSensor {
        attribute sampleRate : Real = 50.0;
        attribute mass : Real = 0.06;
        port powerIn : PowerPort;
    }
}
"#,
    )
    .unwrap();

    let strict_out = Command::new(bin_path())
        .arg("lint")
        .arg("--files")
        .arg(&smell_path)
        .arg("--strict")
        .output()
        .expect("run rsysdsl lint --files --strict");
    assert_eq!(
        strict_out.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&strict_out.stdout),
        String::from_utf8_lossy(&strict_out.stderr)
    );
    let stdout_str = String::from_utf8(strict_out.stdout).unwrap();
    assert!(
        stdout_str.contains("duplicate_part_shape"),
        "expected duplicate_part_shape in stdout, got: {stdout_str}"
    );
    let smell_json: serde_json::Value =
        serde_json::from_str(&stdout_str).expect("stdout must be valid JSON");
    assert_eq!(smell_json["clean"], false);
}
