//! Gate 1 — Rust Compile-Time Safety (`trybuild`).
//!
//! Proves that broken port connections, invalid specializations, mismatched
//! requirement IDs, non-action `perform` targets, and illegal state transition
//! targets all fail `rustc` compilation.

#[test]
fn gate1_compile_time_safety() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/valid_system.rs");
    t.compile_fail("tests/ui/bad_port_polarity.rs");
    t.compile_fail("tests/ui/bad_port_mismatch.rs");
    t.compile_fail("tests/ui/bad_specialization_kind.rs");
    t.compile_fail("tests/ui/bad_requirement_id.rs");
    t.compile_fail("tests/ui/bad_perform_target.rs");
    t.compile_fail("tests/ui/bad_state_transition.rs");
}
