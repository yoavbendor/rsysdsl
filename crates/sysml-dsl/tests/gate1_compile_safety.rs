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
//! - Unknown or OMG-unresolvable `#[sysml::attr(unit = ...)]` symbols fail `rustc`.
//! - Numeric attributes without a unit declaration, empty TBD reasons and a bare `custom_unit` fail `rustc` (S7.1).
//! - Dimensionally inconsistent constraints and transition guards (e.g. `K` vs `W`) fail `rustc`.

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
    // Phase 1 hierarchy & reuse contracts (abstract, redefines, variation, allocate)
    t.pass("tests/ui/valid_hierarchy.rs");
    t.compile_fail("tests/ui/bad_redefines_field.rs");
    t.compile_fail("tests/ui/bad_interface_no_ends.rs");
    t.compile_fail("tests/ui/bad_item_redefines.rs");
    t.compile_fail("tests/ui/bad_variation_member.rs");
    // S1: compile-time unit & dimension safety
    t.compile_fail("tests/ui/bad_unit_symbol.rs");
    // S7.1: every numeric attribute declares its unit; every escape hatch has a reason
    t.compile_fail("tests/ui/bad_missing_unit.rs");
    t.compile_fail("tests/ui/bad_empty_tbd_reason.rs");
    t.compile_fail("tests/ui/bad_bare_custom_unit.rs");
    // S7.2: `tbd_value` and a default are mutually exclusive
    t.compile_fail("tests/ui/bad_tbd_value_with_default.rs");
    // S7.3: #[sysml::tbd] needs a reason and a known phase, and is never dropped silently
    t.compile_fail("tests/ui/bad_tbd_marker.rs");
    t.compile_fail("tests/ui/bad_tbd_no_reason.rs");
    t.compile_fail("tests/ui/bad_tbd_on_action.rs");
    // S7.4: a TBD channel still checks its field paths
    t.compile_fail("tests/ui/bad_tbd_channel_path.rs");
    t.compile_fail("tests/ui/bad_unit_not_in_omg.rs");
    t.compile_fail("tests/ui/bad_constraint_dimension.rs");
    t.compile_fail("tests/ui/bad_guard_dimension.rs");
}
