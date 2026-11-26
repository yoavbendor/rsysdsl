//! Canonical OMG SysML v2 Car & Camera Standard Library Validation & Round-Trip Tests.
//!
//! Exercises official OMG SysML v2 specification examples (`tests/fixtures/omg-camera`
//! and `tests/fixtures/omg-vehicle`):
//! 1. Validates original `.sysml` units against the full OMG Standard Library (`sysml.library`)
//!    with 0 errors and 0 warnings.
//! 2. Lifts into `sysml_ir::Model`, re-emits `.sysml`, and verifies lossless semantic AST/graph
//!    round-trip fidelity (`fidelity_ok`, 0 missing, 0 extra, 0 re-emitted parse errors).

use std::fs;
use std::path::{Path, PathBuf};

use sysml_emit::{discover_omg_stdlib, emit_model, validate_sysml_units_with_lib};
use sysml_lift::{compare_sysml_roundtrip, lift_sysml_units};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn load_sorted_sysml_units(rel_dir: &str) -> Vec<(String, String)> {
    let dir = workspace_root().join(rel_dir);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("failed to read fixture dir {}: {e}", dir.display()))
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("sysml") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    paths.sort();
    assert!(
        !paths.is_empty(),
        "expected .sysml fixtures under {}",
        dir.display()
    );
    paths
        .into_iter()
        .map(|path| {
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("fixture.sysml")
                .to_string();
            let src = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
            (name, src)
        })
        .collect()
}

#[test]
fn omg_camera_stdlib_validation_and_roundtrip() {
    let units = load_sorted_sysml_units("tests/fixtures/omg-camera");
    let lib_dir = discover_omg_stdlib().expect("OMG sysml.library must be discoverable");

    let val = validate_sysml_units_with_lib(&units, Some(&lib_dir));
    assert!(
        val.valid,
        "Expected valid OMG Camera fixtures with stdlib, got:\n{}",
        serde_json::to_string_pretty(&val).unwrap()
    );
    assert_eq!(val.error_count, 0);
    assert_eq!(val.warning_count, 0);

    let lifted = lift_sysml_units(&units);
    assert!(
        lifted.parse_errors.is_empty(),
        "Unexpected parse errors lifting OMG Camera: {:?}",
        lifted.parse_errors
    );
    let reemitted = emit_model(&lifted.model);

    let diff = compare_sysml_roundtrip(
        &units,
        &[("omg_camera_reemitted.sysml".to_string(), reemitted)],
    );
    assert!(
        diff.reemitted_parse_errors.is_empty(),
        "Re-emitted OMG Camera had parse errors: {:?}",
        diff.reemitted_parse_errors
    );
    assert!(
        diff.fidelity_ok,
        "OMG Camera round-trip mismatch:\n{}",
        serde_json::to_string_pretty(&diff).unwrap()
    );
    assert_eq!(diff.missing_count, 0);
    assert_eq!(diff.extra_count, 0);
    assert!(
        diff.total_original_facts >= 4,
        "Expected >= 4 original facts for OMG Camera, got {}",
        diff.total_original_facts
    );
}

#[test]
fn omg_vehicle_stdlib_validation_and_roundtrip() {
    let units = load_sorted_sysml_units("tests/fixtures/omg-vehicle");
    let lib_dir = discover_omg_stdlib().expect("OMG sysml.library must be discoverable");

    let val = validate_sysml_units_with_lib(&units, Some(&lib_dir));
    assert!(
        val.valid,
        "Expected valid OMG Vehicle fixtures with stdlib, got:\n{}",
        serde_json::to_string_pretty(&val).unwrap()
    );
    assert_eq!(val.error_count, 0);
    assert_eq!(val.warning_count, 0);

    let lifted = lift_sysml_units(&units);
    assert!(
        lifted.parse_errors.is_empty(),
        "Unexpected parse errors lifting OMG Vehicle: {:?}",
        lifted.parse_errors
    );
    let reemitted = emit_model(&lifted.model);

    let diff = compare_sysml_roundtrip(
        &units,
        &[("omg_vehicle_reemitted.sysml".to_string(), reemitted)],
    );
    assert!(
        diff.reemitted_parse_errors.is_empty(),
        "Re-emitted OMG Vehicle had parse errors: {:?}",
        diff.reemitted_parse_errors
    );
    assert!(
        diff.fidelity_ok,
        "OMG Vehicle round-trip mismatch:\n{}",
        serde_json::to_string_pretty(&diff).unwrap()
    );
    assert_eq!(diff.missing_count, 0);
    assert_eq!(diff.extra_count, 0);
    assert!(
        diff.total_original_facts >= 64,
        "Expected >= 64 original facts for OMG Vehicle, got {}",
        diff.total_original_facts
    );
}
