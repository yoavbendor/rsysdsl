//! Gate 1 — Rust Compile-Time Safety & Anti-Semantic-Leakage (`trybuild`).
//!
//! Proves that:
//! - Broken port polarity / mismatched port types fail `rustc`.
//! - Invalid cross-kind specialization (`part :> port`) fails `rustc`.
//! - Mismatched requirement IDs fail `rustc`.
//! - Non-action `perform` targets fail `rustc`.
//! - Non-existent state transition targets fail `rustc`.
//! - Hidden imperative `fn`, raw `impl`, or unannotated `struct` inside `#[sysml::package]` fail `rustc`.
//! - Unknown fields in `#[sysml::require_constraint(...)]` or `#[sysml::transition(..., guard = ...)]` fail `rustc`.
//! - Calling an opaque unmodeled Rust function inside `#[sysml::calc]` fails `rustc`.

#[test]
fn gate1_compile_time_safety() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/valid_system.rs");
    // Structural & traceability safety
    t.compile_fail("tests/ui/bad_port_polarity.rs");
    t.compile_fail("tests/ui/bad_port_mismatch.rs");
    t.compile_fail("tests/ui/bad_specialization_kind.rs");
    t.compile_fail("tests/ui/bad_requirement_id.rs");
    t.compile_fail("tests/ui/bad_perform_target.rs");
    t.compile_fail("tests/ui/bad_state_transition.rs");
    // Anti-semantic-leakage locks (Track A)
    t.compile_fail("tests/ui/hidden_fn_in_package.rs");
    t.compile_fail("tests/ui/hidden_impl_in_package.rs");
    t.compile_fail("tests/ui/unannotated_struct_in_package.rs");
    t.compile_fail("tests/ui/bad_constraint_field.rs");
    t.compile_fail("tests/ui/opaque_call_in_calc.rs");
    t.compile_fail("tests/ui/bad_guard_field.rs");
    // ISO 26262 safety contracts (Track C)
    t.compile_fail("tests/ui/bad_asil_decomposition.rs");
    t.compile_fail("tests/ui/missing_asil_independence.rs");
}
