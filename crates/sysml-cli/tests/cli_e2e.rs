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
fn cli_verify_scenario_registry_list_all_and_trace() {
    let list = Command::new(bin_path())
        .args(["verify", "--list"])
        .output()
        .expect("run rsysdsl verify --list");
    assert_eq!(list.status.code(), Some(0));
    let list_json: serde_json::Value = serde_json::from_slice(&list.stdout).unwrap();
    let names: Vec<_> = list_json
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["name"].as_str().unwrap().to_string())
        .collect();
    assert!(names.contains(&"nominal_tranquility_base_landing".to_string()));
    assert!(names.contains(&"fault_dps_throttle_stuck_65pct".to_string()));

    let all = Command::new(bin_path())
        .args(["verify", "--scenario", "all"])
        .output()
        .expect("run rsysdsl verify all");
    assert_eq!(all.status.code(), Some(0));
    let all_json: serde_json::Value = serde_json::from_slice(&all.stdout).unwrap();
    assert_eq!(all_json["ok"], true);
    assert_eq!(all_json["nominal"]["all_passed"], true);
    assert_eq!(all_json["fault_injection"]["all_passed"], false);

    let unknown = Command::new(bin_path())
        .args(["verify", "--scenario", "no_such_scenario"])
        .output()
        .expect("run rsysdsl verify unknown");
    assert_eq!(unknown.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("unknown scenario"));

    let dir = tempfile::tempdir().unwrap();
    let trace_path = dir.path().join("trace.json");
    let traced = Command::new(bin_path())
        .args(["verify", "--scenario", "fault_dps_throttle_stuck_65pct", "--trace"])
        .arg(&trace_path)
        .output()
        .expect("run rsysdsl verify --trace");
    assert_eq!(traced.status.code(), Some(1));
    let run: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&trace_path).unwrap()).unwrap();
    assert_eq!(run["scenario"], "fault_dps_throttle_stuck_65pct");
    assert!(run["trace"]["time_s"].as_array().unwrap().len() > 100);
    let dps = run["verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["requirement_id"] == "CLR-R057")
        .unwrap();
    assert_eq!(dps["pass"], false);
    assert_eq!(dps["first_violation_s"], 0.0);
}

#[test]
fn cli_verify_microwave_fault_scenarios_and_trace_matrix() {
    let all = Command::new(bin_path())
        .args(["verify", "--scenario", "all"])
        .output()
        .expect("run rsysdsl verify all");
    assert_eq!(all.status.code(), Some(0));
    let all_json: serde_json::Value = serde_json::from_slice(&all.stdout).unwrap();
    assert_eq!(all_json["microwave_nominal"]["all_passed"], true);
    assert_eq!(all_json["microwave_door_open"]["all_passed"], true);
    assert_eq!(all_json["microwave_primary_interlock_welded"]["all_passed"], true);
    assert_eq!(all_json["microwave_broken_interlock"]["all_passed"], false);

    // Deliberately broken interlock: expected failure, exit 1, MW-004 is the failing requirement.
    let broken = Command::new(bin_path())
        .args(["verify", "--scenario", "mw-broken-interlock"])
        .output()
        .expect("run rsysdsl verify mw-broken-interlock");
    assert_eq!(broken.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&broken.stdout).unwrap();
    let failed: Vec<_> = report["verdicts"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["status"] == "fail")
        .map(|v| v["req_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(failed, vec!["MW-004"]);

    let md = Command::new(bin_path())
        .args(["trace-matrix", "--model", "microwave", "--verify", "--format", "markdown"])
        .output()
        .expect("run rsysdsl trace-matrix microwave");
    assert_eq!(md.status.code(), Some(0));
    let md = String::from_utf8(md.stdout).unwrap();
    assert!(md.contains("2 verified PASS / 0 FAIL / 2 unverified"), "{md}");
    assert!(md.contains("microwave_door_open_mid_cook, microwave_door_open_primary_interlock_welded"));
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
        "req_id,req_name,package,satisfied_by,allocated_functions,constraints,refines,verification_status,verification_scenario,evidence_source,tbds\n"
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

fn run_new(dir: &Path, name: &str) -> std::process::Output {
    Command::new(bin_path())
        .args(["new", name, "--no-workspace", "--dir"])
        .arg(dir)
        .output()
        .expect("run rsysdsl new")
}

#[test]
fn cli_new_scaffolds_layered_crate_and_refuses_overwrite() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("pump_station");
    let out = run_new(&dir, "PumpStation");
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["ok"], true);
    assert_eq!(val["package"], "pump-station");
    assert_eq!(val["crate_name"], "pump_station");
    assert!(val["workspace_member"].is_null());

    let cargo = std::fs::read_to_string(dir.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("name = \"pump-station\""));
    assert!(
        cargo.contains("[workspace]"),
        "standalone crate must declare its own workspace"
    );
    assert!(cargo.contains("sysml-dsl = { path = "));
    assert!(
        cargo.contains("sysml-std = { path = "),
        "standalone crate must depend on sysml-std"
    );

    let lib = std::fs::read_to_string(dir.join("src/lib.rs")).unwrap();
    for needle in [
        "#[sysml::package(name = \"PumpStationTypes\")]",
        "#[sysml::package(name = \"PumpStationSubsystems\")]",
        "#[sysml::package(name = \"PumpStationRequirements\")]",
        "#[sysml::package(name = \"PumpStationContext\")]",
        "#[sysml::import(\"PumpStationSubsystems::*\")]",
        "pub struct PumpStationSystem",
        "#[sysml::connect(",
        "Source: TODO — replace with document reference",
        "audit_model_with_stdlib(\"pump_station\"",
        "sysml_std::std_packages()",
        "cargo test            # run inside this crate's directory",
    ] {
        assert!(lib.contains(needle), "lib.rs missing `{needle}`");
    }
    assert!(
        !lib.contains("{{"),
        "unrendered template placeholder in lib.rs"
    );
    // Layer order: Types -> Requirements -> Subsystems -> Context, so realizing subsystem
    // parts can `#[sysml::satisfy]` requirements from a layer above them.
    let pos = |pkg: &str| {
        lib.find(&format!("#[sysml::package(name = \"PumpStation{pkg}\")]"))
            .unwrap()
    };
    assert!(pos("Types") < pos("Requirements"));
    assert!(pos("Requirements") < pos("Subsystems"));
    assert!(pos("Subsystems") < pos("Context"));
    assert!(
        lib[pos("Subsystems")..pos("Context")].contains("#[sysml::satisfy("),
        "a subsystem part must satisfy the placeholder requirement"
    );

    assert!(!lib.contains("cargo test -p"), "standalone crate cannot use `-p`");
    for step in val["next_steps"].as_array().unwrap() {
        assert!(
            !step.as_str().unwrap().contains(" -p "),
            "standalone next step uses `-p`: {step}"
        );
    }

    let emit = std::fs::read_to_string(dir.join("src/bin/emit.rs")).unwrap();
    assert!(emit.contains("pump_station::build_model()"));

    // A second run must refuse to overwrite the existing directory.
    let again = run_new(&dir, "PumpStation");
    assert_eq!(again.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&again.stderr).contains("refusing to overwrite"));

    // Invalid names are rejected before anything is written.
    let bad = run_new(&tmp.path().join("bad"), "9lives");
    assert_eq!(bad.status.code(), Some(2));
    assert!(!tmp.path().join("bad").exists());
}

/// Full gate on a scaffolded crate: `cargo test`, emit, strict stdlib audit and strict lint.
/// Ignored by default because it compiles the crate and its deps in a fresh target dir;
/// run with `cargo test -p sysml-cli --test cli_e2e -- --ignored`.
#[test]
#[ignore = "slow: compiles the scaffolded crate from scratch"]
fn cli_new_scaffold_builds_audits_and_lints_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("gate_demo");
    let out = run_new(&dir, "gate_demo");
    assert_eq!(out.status.code(), Some(0));
    std::fs::copy(workspace_root().join("Cargo.lock"), dir.join("Cargo.lock")).unwrap();

    let manifest = dir.join("Cargo.toml");
    let test = Command::new(env!("CARGO"))
        .args(["test", "-q", "--offline", "--manifest-path"])
        .arg(&manifest)
        .output()
        .unwrap();
    assert!(
        test.status.success(),
        "cargo test failed: {}",
        String::from_utf8_lossy(&test.stderr)
    );

    let sysml = dir.join("gate_demo.sysml");
    let emit = Command::new(env!("CARGO"))
        .args(["run", "-q", "--offline", "--bin", "emit", "--manifest-path"])
        .arg(&manifest)
        .arg("--")
        .arg("-o")
        .arg(&sysml)
        .output()
        .unwrap();
    assert!(
        emit.status.success(),
        "{}",
        String::from_utf8_lossy(&emit.stderr)
    );

    let audit = Command::new(bin_path())
        .args(["audit", "--stdlib", "--strict", "--files"])
        .arg(&sysml)
        .output()
        .unwrap();
    assert_eq!(
        audit.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&audit.stdout)
    );
    let val: serde_json::Value = serde_json::from_slice(&audit.stdout).unwrap();
    assert_eq!(val["validation_errors"], 0);
    assert_eq!(val["validation_warnings"], 0);
    assert_eq!(val["orphan_requirements"].as_array().unwrap().len(), 0);

    let lint = Command::new(bin_path())
        .args(["lint", "--strict", "--files"])
        .arg(&sysml)
        .output()
        .unwrap();
    assert_eq!(
        lint.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&lint.stdout)
    );
}

#[test]
fn cli_audit_and_lint_load_sysml_std_for_files_emitted_with_omit_std() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("microwave_no_std.sysml");
    let (own, _std) = sysml_emit::emit_model_split_std(&microwave::build_model());
    assert!(!own.contains("package SysmlStd"));
    std::fs::write(&path, own).unwrap();

    let audit_out = Command::new(bin_path())
        .arg("audit")
        .arg("--files")
        .arg(&path)
        .args(["--stdlib", "--strict"])
        .output()
        .expect("run rsysdsl audit");
    assert_eq!(
        audit_out.status.code(),
        Some(0),
        "stdout: {}",
        String::from_utf8_lossy(&audit_out.stdout)
    );
    let val: serde_json::Value = serde_json::from_slice(&audit_out.stdout).unwrap();
    assert_eq!(val["validation_errors"], 0);
    assert_eq!(val["validation_warnings"], 0);

    let lint_out = Command::new(bin_path())
        .arg("lint")
        .arg("--files")
        .arg(&path)
        .args(["--strict", "--format", "json"])
        .output()
        .expect("run rsysdsl lint");
    assert_eq!(lint_out.status.code(), Some(0));
    let lint: serde_json::Value = serde_json::from_slice(&lint_out.stdout).unwrap();
    assert!(
        lint["summary"]["total_packages"].as_u64().unwrap_or(0) >= 9,
        "bundled sysml-std packages must be auto-loaded: {lint}"
    );
    assert_eq!(
        lint["summary"]["inherited_std_default_count"], 0,
        "microwave redefines every concrete std default it inherits: {lint}"
    );

    // Concrete sysml-std defaults (e.g. AcMainsSupply 230 V / 16 A / 50 Hz) must still be
    // reported as `inherited_std_default` once sysml-std is auto-loaded.
    let mains_path = dir.path().join("mains_no_std.sysml");
    std::fs::write(
        &mains_path,
        "package MainsDemo {\n    private import SysmlStdPower::*;\n    part def WallInput :> AcMainsSupply;\n}\n",
    )
    .unwrap();
    let mains_lint_out = Command::new(bin_path())
        .arg("lint")
        .arg("--files")
        .arg(&mains_path)
        .args(["--strict", "--format", "json"])
        .output()
        .expect("run rsysdsl lint on mains");
    assert_eq!(mains_lint_out.status.code(), Some(0));
    let mains_lint: serde_json::Value = serde_json::from_slice(&mains_lint_out.stdout).unwrap();
    let findings = mains_lint["findings"].as_array().unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f["code"] == "inherited_std_default"),
        "concrete std defaults must be reported once std is auto-loaded: {mains_lint}"
    );
}

/// S7.5 fixture: three TBDs due at `preliminary`, `detailed` (default) and `release`.
const TBD_FIXTURE: &str = r#"package TbdCli {
    private import ScalarValues::*;
    private import SI::*;
    part def Pump {
        @TbdMetadata::TBD {
            category = "element";
            reason = "supplier not selected";
            owner = "systems";
            resolveBy = "preliminary";
        }
        attribute ratedPower : Real {
            @TbdMetadata::TBD {
                category = "value";
                reason = "datasheet pending";
                resolveBy = "detailed";
            }
        }
        attribute lossCoefficient : Real = 0.8 {
            @TbdMetadata::TBD {
                category = "unit";
                reason = "table gives no unit";
                resolveBy = "release";
            }
        }
    }
}

library package TbdMetadata {
    private import ScalarValues::*;
    metadata def TBD {
        attribute category : String;
        attribute reason : String;
        attribute owner : String;
        attribute resolveBy : String;
    }
}
"#;

fn audit_exit(file: &Path, maturity: &str) -> (Option<i32>, serde_json::Value) {
    let out = Command::new(bin_path())
        .args(["audit", "--files"])
        .arg(file)
        .args(["--stdlib", "--strict", "--maturity", maturity])
        .output()
        .expect("run rsysdsl audit");
    (out.status.code(), serde_json::from_slice(&out.stdout).unwrap())
}

#[test]
fn cli_tbd_ledger_and_audit_maturity_gate() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("tbd.sysml");
    std::fs::write(&file, TBD_FIXTURE).unwrap();

    // Ledger: JSON and Markdown.
    let out = Command::new(bin_path())
        .args(["tbd", "--files"])
        .arg(&file)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stderr));
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["tbd_count"], 3);
    assert_eq!(val["by_kind"]["value"], 1);
    assert_eq!(val["by_kind"]["channel"], 0);
    let e0 = &val["entries"][0];
    assert_eq!(e0["element"], "TbdCli::Pump");
    assert_eq!(e0["kind"], "element");
    assert_eq!(e0["owner"], "systems");
    assert_eq!(e0["resolve_by"], "preliminary");
    let md = Command::new(bin_path())
        .args(["tbd", "--format", "markdown", "--files"])
        .arg(&file)
        .output()
        .unwrap();
    let md = String::from_utf8_lossy(&md.stdout);
    assert!(md.contains("| `TbdCli::Pump::ratedPower` | value | detailed | — | datasheet pending |"), "{md}");

    // Maturity: TBDs due at or before the phase fail the strict audit.
    let (code, val) = audit_exit(&file, "concept");
    assert_eq!(code, Some(0), "{val:#}");
    assert_eq!(val["tbd_count"], 3);
    assert_eq!(val["tbd_ledger"].as_array().unwrap().len(), 3);
    assert_eq!(val["overdue_tbds"].as_array().unwrap().len(), 0);
    let (code, val) = audit_exit(&file, "preliminary");
    assert_eq!(code, Some(1));
    assert_eq!(val["overdue_tbds"].as_array().unwrap().len(), 1);
    let (code, val) = audit_exit(&file, "detailed");
    assert_eq!(code, Some(1));
    assert_eq!(val["overdue_tbds"].as_array().unwrap().len(), 2);
    let (code, val) = audit_exit(&file, "release");
    assert_eq!(code, Some(1));
    assert_eq!(val["overdue_tbds"].as_array().unwrap().len(), 3, "release allows no TBDs");
    assert!(val["findings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["rule"] == "tbd-overdue" && f["severity"] == "error"));

    // Lint reports the count as info only; trace-matrix carries the TBD summary.
    let out = Command::new(bin_path())
        .args(["lint", "--strict", "--files"])
        .arg(&file)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", String::from_utf8_lossy(&out.stdout));
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["summary"]["tbd_count"], 3);
    let out = Command::new(bin_path())
        .args(["trace-matrix", "--files"])
        .arg(&file)
        .output()
        .unwrap();
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["summary"]["tbd_count"], 3);
}

#[test]
fn cli_tbd_ratchet_catches_added_tbds() {
    let tmp = tempfile::tempdir().unwrap();
    let file = tmp.path().join("tbd.sysml");
    let baseline = tmp.path().join("tbd-baseline.json");
    std::fs::write(&file, TBD_FIXTURE).unwrap();
    let run = |args: &[&str]| {
        Command::new(bin_path())
            .args(["tbd", "--files"])
            .arg(&file)
            .arg("--baseline")
            .arg(&baseline)
            .args(args)
            .output()
            .unwrap()
    };
    // Missing baseline: IO error.
    assert_eq!(run(&["--check"]).status.code(), Some(2));
    assert_eq!(run(&["--write-baseline"]).status.code(), Some(0));
    assert_eq!(run(&["--check"]).status.code(), Some(0));

    // One more TBD (a value TBD): the ratchet fails and names the new entry.
    let more = TBD_FIXTURE.replace(
        "    part def Pump {",
        "    part def Valve {\n        attribute cv : Real {\n            @TbdMetadata::TBD {\n                category = \"value\";\n                reason = \"valve sizing open\";\n                resolveBy = \"detailed\";\n            }\n        }\n    }\n    part def Pump {",
    );
    std::fs::write(&file, &more).unwrap();
    let out = run(&["--check"]);
    assert_eq!(out.status.code(), Some(1));
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["ok"], false);
    assert_eq!(val["current_count"], 4);
    assert_eq!(val["new_entries"][0]["element"], "TbdCli::Valve::cv");
    let rose: Vec<String> = serde_json::from_value(val["rose"].clone()).unwrap();
    assert_eq!(rose, vec!["total: 3 -> 4".to_string(), "value: 1 -> 2".to_string()]);
    assert!(String::from_utf8_lossy(&out.stderr).contains("+ TbdCli::Valve::cv [value"));

    // Fewer TBDs than the baseline: passes (resolved entries are reported).
    let fewer = TBD_FIXTURE.replace(
        "        @TbdMetadata::TBD {\n            category = \"element\";\n            reason = \"supplier not selected\";\n            owner = \"systems\";\n            resolveBy = \"preliminary\";\n        }\n",
        "",
    );
    assert_ne!(fewer, TBD_FIXTURE);
    std::fs::write(&file, &fewer).unwrap();
    let out = run(&["--check"]);
    assert_eq!(out.status.code(), Some(0));
    let val: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(val["resolved_entries"].as_array().unwrap().len(), 1);
}
