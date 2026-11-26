//! Gates 2 & 3 — Normative SysML v2 Validation & Full Apollo 11 Bidirectional Round-Trip.
//!
//! Exercises:
//! 1. Hand-written 5-layer `apollo11-slice` Rust DSL -> `sysml-emit` -> Open-MBEE validation (0 errors).
//! 2. Real Airbus Apollo 11 vertical slice `.sysml` -> `sysml-lift` -> generated Rust DSL
//!    -> `cargo run` (`rustc` compile + execute) -> `sysml-emit` -> re-emitted `.sysml`
//!    -> Open-MBEE validation (0 errors) + semantic AST/graph diff (0 missing, 0 extra).
//! 3. Full 28-file Airbus Apollo 11 corpus audit verifying 0 parse errors and explicit
//!    logging of every unsupported construct.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sysml_emit::{emit_model, validate_sysml_source, validate_sysml_units};
use sysml_lift::{compare_sysml_roundtrip, generate_rust_dsl, lift_sysml_units, CodegenOptions};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn apollo11_fixture(rel: &str) -> (String, String) {
    let path = workspace_root()
        .join("tests/fixtures/apollo-11-sysml-v2")
        .join(rel);
    let src = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    (rel.to_string(), src)
}

/// Gate 2: Validate the pure declarative 5-layer Apollo 11 Rust DSL model (`apollo11-model`)
/// with Open-MBEE `sysml-toolkit` AND run full traceability audit (`audit_model`) with verification.
#[test]
fn gate2_handwritten_apollo11_model_validates_and_audits_cleanly() {
    let model = apollo11_model::build_apollo11_model();
    let emitted = apollo11_model::emit_apollo11_model_sysml();
    let report = validate_sysml_source("apollo11_model.sysml", &emitted);
    assert!(
        report.valid,
        "Expected 0 Open-MBEE validation errors on emitted Apollo 11 model, got:\n{}\n\nEmitted SysML:\n{}",
        serde_json::to_string_pretty(&report).unwrap(),
        emitted
    );
    assert_eq!(report.syntax_errors.len(), 0);
    assert_eq!(report.context_errors.len(), 0);
    assert_eq!(report.model_errors.len(), 0);
    // Verify modeled #[sysml::calc] defs and compiled state guards appear in emitted .sysml!
    assert!(emitted.contains("calc def ascentBurnDuration"));
    assert!(emitted.contains("if (payload.verticalVelocity <= target.maxTouchdownVerticalVelocity)"));

    let verification = apollo11_sim::run_nominal_verification();
    let audit = sysml_emit::audit_model("apollo11_model", &model, Some(&verification));
    assert!(
        audit.ok,
        "Expected 0 traceability audit findings on apollo11-model, got:\n{}",
        serde_json::to_string_pretty(&audit).unwrap()
    );
    assert_eq!(audit.orphan_requirements.len(), 0);
    assert_eq!(audit.unverified_constrained_requirements.len(), 0);
    assert_eq!(audit.total_calcs, 3);
}

/// Gate 2 (Strict OMG Standard Library): Validate and audit `apollo11-model` against
/// the full official OMG SysML v2 Standard Library (`sysml.library`) with 0 errors AND 0 warnings.
#[test]
fn gate2_apollo11_model_zero_errors_zero_warnings_with_omg_stdlib() {
    let model = apollo11_model::build_apollo11_model();
    let emitted = apollo11_model::emit_apollo11_model_sysml();
    let report = sysml_emit::validate_sysml_source_with_stdlib(
        "apollo11_model.sysml",
        &emitted,
        None,
    );
    assert!(
        report.stdlib_loaded,
        "Expected OMG sysml.library to be discovered and loaded"
    );
    assert!(
        report.valid && report.warning_count == 0,
        "Expected 0 errors and 0 warnings with OMG stdlib, got:\n{}",
        serde_json::to_string_pretty(&report).unwrap()
    );

    let verification = apollo11_sim::run_nominal_verification();
    let audit = sysml_emit::audit_model_with_stdlib(
        "apollo11_model",
        &model,
        Some(&verification),
        None,
        true,
    );
    assert!(
        audit.ok && audit.stdlib_loaded && audit.validation_warnings == 0,
        "Expected 0 findings and 0 warnings in strict stdlib audit, got:\n{}",
        serde_json::to_string_pretty(&audit).unwrap()
    );
}

/// Gate 2 + Gate 3: Full end-to-end Apollo 11 bidirectional round-trip INCLUDING
/// compiling and executing the lifted Rust DSL source code with `cargo` (`rustc`)!
#[test]
fn gate3_apollo11_vertical_slice_full_compiled_roundtrip() {
    let slice_files = [
        "CoSMA/CoSMAPackage.sysml",
        "Operation/OperationsPackage.sysml",
        "Purpose/CapabilitiesPackage.sysml",
        "Purpose/MissionPhasesPackage.sysml",
        "Function/FunctionsPackage.sysml",
        "Logical/LogicalComponentsPackage.sysml",
        "Technical/TechnicalPortsPackage.sysml",
        "Technical/TechnicalComponentsPackage.sysml",
        "Requirements/MissionRequirementsPackage.sysml",
        "Requirements/TechnicalRequirementsPackage.sysml",
        "Technical/SystemSpecificationPackage.sysml",
        "Analysis/CalculationsPackage.sysml",
        "Analysis/AnalysisPackage.sysml",
    ];
    let original_units: Vec<(String, String)> =
        slice_files.iter().map(|f| apollo11_fixture(f)).collect();

    // Step 1: Lift real Airbus Apollo 11 .sysml files -> IR + Rust DSL source
    let lifted = lift_sysml_units(&original_units);
    assert!(
        lifted.parse_errors.is_empty(),
        "Unexpected parse errors on Apollo 11 fixtures: {:?}",
        lifted.parse_errors
    );
    assert_eq!(lifted.stats.packages, slice_files.len());
    assert_eq!(
        lifted.unsupported.len(),
        0,
        "Expected 0 unsupported constructs in 13-package vertical slice, got: {:?}",
        lifted.unsupported
    );
    assert!(lifted.stats.part_defs >= 50);
    assert!(lifted.stats.perform_actions >= 30);
    assert!(lifted.stats.requirement_defs >= 100);
    assert!(lifted.stats.satisfy_links >= 50);
    assert_eq!(lifted.stats.calc_defs, 14);
    assert_eq!(lifted.stats.analysis_defs, 3);
    assert_eq!(lifted.stats.analysis_usages, 5);
    assert!(lifted.stats.refinements >= 42);

    let generated_rs = generate_rust_dsl(
        &lifted.model,
        &CodegenOptions {
            emit_model_fn: true,
            emit_main_emitter: true,
        },
    );

    // Step 2: Compile & execute the generated Rust DSL crate with `cargo run`!
    let root = workspace_root();
    let tmp = tempfile::tempdir().expect("create tempdir");
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();

    let cargo_toml = format!(
        r#"[package]
name = "apollo11-lifted-roundtrip"
version = "0.1.0"
edition = "2021"

[dependencies]
sysml-dsl = {{ path = "{}/crates/sysml-dsl" }}
sysml-ir = {{ path = "{}/crates/sysml-ir" }}
sysml-emit = {{ path = "{}/crates/sysml-emit", default-features = false }}
"#,
        root.display(),
        root.display(),
        root.display()
    );
    fs::write(tmp.path().join("Cargo.toml"), cargo_toml).unwrap();
    fs::write(src_dir.join("main.rs"), &generated_rs).unwrap();

    // Reuse workspace target dir to make compiling the generated crate fast and offline-friendly
    let target_dir = root.join("target/roundtrip-check");
    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(tmp.path().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .expect("invoke cargo run on lifted Rust DSL");

    assert!(
        output.status.success(),
        "Generated Rust DSL failed to compile/run!\nstderr:\n{}\n\nGenerated Rust (first 120 lines):\n{}",
        String::from_utf8_lossy(&output.stderr),
        generated_rs.lines().take(120).collect::<Vec<_>>().join("\n")
    );

    let reemitted_sysml = String::from_utf8(output.stdout).expect("valid utf-8 re-emitted sysml");
    assert!(!reemitted_sysml.trim().is_empty());

    // Step 3 (Gate 2): Validate re-emitted .sysml with Open-MBEE sysml-toolkit -> 0 errors!
    let val = validate_sysml_units(&[("apollo11_reemitted.sysml".to_string(), reemitted_sysml.clone())]);
    assert!(
        val.valid,
        "Re-emitted Apollo 11 SysML v2 failed Open-MBEE validation:\n{}",
        serde_json::to_string_pretty(&val).unwrap()
    );

    // Step 4 (Gate 3): Semantic AST/graph round-trip comparison -> 0 missing, 0 extra!
    let diff = compare_sysml_roundtrip(
        &original_units,
        &[("apollo11_reemitted.sysml".to_string(), reemitted_sysml)],
    );
    assert!(
        diff.fidelity_ok,
        "Apollo 11 round-trip semantic mismatch!\n{}",
        serde_json::to_string_pretty(&diff).unwrap()
    );
    assert_eq!(diff.missing_count, 0);
    assert_eq!(diff.extra_count, 0);
    assert!(diff.total_original_facts > 1400);
}

/// Full 28-file Airbus Apollo 11 corpus lift + re-emit validation check.
#[test]
fn gate2_and_gate3_full_apollo11_corpus_audit() {
    let fixture_root = workspace_root().join("tests/fixtures/apollo-11-sysml-v2");
    let mut files = Vec::new();
    collect_sysml_files(&fixture_root, &fixture_root, &mut files);
    files.sort();
    assert!(files.len() >= 25, "Expected full Apollo 11 corpus, found {}", files.len());

    let units: Vec<(String, String)> = files
        .iter()
        .map(|rel| {
            let content = fs::read_to_string(fixture_root.join(rel)).unwrap();
            (rel.clone(), content)
        })
        .collect();

    let lifted = lift_sysml_units(&units);
    assert!(
        lifted.parse_errors.is_empty(),
        "Parse errors on Apollo 11 corpus: {:?}",
        lifted.parse_errors
    );
    assert_eq!(
        lifted.unsupported.len(),
        0,
        "Expected 0 unsupported constructs across full 28-file Apollo 11 corpus (100.0%), got: {:#?}",
        lifted.unsupported
    );
    assert_eq!(lifted.stats.calc_defs, 15);
    assert_eq!(lifted.stats.analysis_defs, 3);
    assert_eq!(lifted.stats.analysis_usages, 5);
    assert_eq!(lifted.stats.comments, 3);
    assert!(lifted.stats.portions >= 60);
    assert_eq!(lifted.stats.views, 1);
    assert!(lifted.stats.refinements >= 42);

    let reemitted = emit_model(&lifted.model);
    let val = validate_sysml_source("apollo11_full_reemitted.sysml", &reemitted);
    assert!(
        val.valid,
        "Re-emitted full Apollo 11 corpus failed validation:\n{}",
        serde_json::to_string_pretty(&val).unwrap()
    );

    let diff = compare_sysml_roundtrip(&units, &[("reemitted.sysml".to_string(), reemitted)]);
    assert!(
        diff.fidelity_ok,
        "Full Apollo 11 corpus IR round-trip mismatch:\n{}",
        serde_json::to_string_pretty(&diff).unwrap()
    );
    assert!(diff.total_original_facts >= 2800);
    eprintln!(
        "FULL 28-FILE APOLLO 11 CORPUS: 0 unsupported (100.0%), {} / {} semantic facts matched, stats={:?}",
        diff.total_original_facts, diff.total_reemitted_facts, lifted.stats
    );
}

fn collect_sysml_files(base: &Path, dir: &Path, out: &mut Vec<String>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_sysml_files(base, &path, out);
        } else if path.extension().and_then(|s| s.to_str()) == Some("sysml") {
            let rel = path.strip_prefix(base).unwrap().to_string_lossy().to_string();
            out.push(rel);
        }
    }
}

/// Gate 5: Ensure README.md, COVERAGE.md, and Apollo 11 fixture provenance are present and complete.
#[test]
fn gate5_reproducible_docs_and_coverage_present() {
    let root = workspace_root();
    let readme = fs::read_to_string(root.join("README.md")).expect("README.md must exist");
    let coverage = fs::read_to_string(root.join("COVERAGE.md")).expect("COVERAGE.md must exist");
    let provenance = fs::read_to_string(
        root.join("tests/fixtures/apollo-11-sysml-v2/PROVENANCE.md"),
    )
    .expect("PROVENANCE.md must exist");

    assert!(readme.contains("cargo test --workspace"));
    assert!(readme.contains("rsysdsl"));
    assert!(coverage.contains("CoSMAPackage.sysml"));
    assert!(coverage.contains("ISO 26262"));
    assert!(coverage.contains("Ferrocene"));
    assert!(coverage.contains("no_std"));
    assert!(provenance.contains("0e8a7d6790cfa7e15b991eb745e9a38f9bed0474"));
}
