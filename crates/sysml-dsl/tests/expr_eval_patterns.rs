//! C2: IR evaluator agreement on the expression patterns the transpiler accepts (if/else,
//! `!`, `||`, unary minus, `self.field`, integer and boolean fields, several constraints, calcs
//! with locals, conditionals and nested calc calls). The compiled `evaluate()` / calc fns and
//! `sysml_emit::expr_eval` must agree on defaults and on perturbed value sets.

use sysml_dsl::prelude::*;

#[sysml::package(name = "ExprPatterns")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod expr_patterns {
    use super::*;

    /// XP-001: power limit depends on a boolean mode (if/else in a constraint).
    #[sysml::requirement(id = "XP-001")]
    #[sysml::require_constraint(if boost { power_w <= boost_limit_w } else { power_w <= normal_limit_w })]
    #[derive(Debug, Clone, Copy)]
    pub struct ModalPowerLimit {
        #[sysml::attr(default = false)]
        pub boost: bool,
        #[sysml::attr(default = 900.0, unit = "W")]
        pub power_w: Quantity<f64>,
        #[sysml::attr(default = 1200.0, unit = "W")]
        pub boost_limit_w: Quantity<f64>,
        #[sysml::attr(default = 800.0, unit = "W")]
        pub normal_limit_w: Quantity<f64>,
    }

    /// XP-002: negation, disjunction, unary minus and `self.` access.
    #[sysml::requirement(id = "XP-002")]
    #[sysml::require_constraint(!(self.temp_k > max_temp_k) && delta_k >= -tolerance_k || bypass)]
    #[derive(Debug, Clone, Copy)]
    pub struct ThermalWindow {
        #[sysml::attr(default = 330.0, unit = "K")]
        pub temp_k: Quantity<f64>,
        #[sysml::attr(default = 350.0, unit = "K")]
        pub max_temp_k: Quantity<f64>,
        #[sysml::attr(default = -1.0, unit = "K")]
        pub delta_k: Quantity<f64>,
        #[sysml::attr(default = 2.0, unit = "K")]
        pub tolerance_k: Quantity<f64>,
        #[sysml::attr(default = false)]
        pub bypass: bool,
    }

    /// XP-003: arithmetic on both sides, two constraints, an integer count.
    #[sysml::requirement(id = "XP-003")]
    #[sysml::require_constraint(load_w * 2.0 - margin_w <= max_w)]
    #[sysml::require_constraint(cycles <= max_cycles)]
    #[derive(Debug, Clone, Copy)]
    pub struct LoadAndCycles {
        #[sysml::attr(default = 300.0, unit = "W")]
        pub load_w: Quantity<f64>,
        #[sysml::attr(default = 50.0, unit = "W")]
        pub margin_w: Quantity<f64>,
        #[sysml::attr(default = 600.0, unit = "W")]
        pub max_w: Quantity<f64>,
        #[sysml::attr(default = 1000, dimensionless)]
        pub cycles: i64,
        #[sysml::attr(default = 5000, dimensionless)]
        pub max_cycles: i64,
    }

    /// XP-004: evidence operand (unknown on defaults) and an equality.
    #[sysml::requirement(id = "XP-004")]
    #[sysml::require_constraint(measured_v <= max_v && phases == 3.0)]
    #[derive(Debug, Clone, Copy)]
    pub struct SupplyVoltage {
        #[sysml::attr(default = 253.0, unit = "V")]
        pub max_v: Quantity<f64>,
        #[sysml::attr(unit = "V", evidence)]
        pub measured_v: Quantity<f64>,
        #[sysml::attr(default = 3.0, dimensionless)]
        pub phases: f64,
    }

    /// XP-005: division and a ratio bound.
    #[sysml::requirement(id = "XP-005")]
    #[sysml::require_constraint(energy_j / power_w >= min_runtime_s)]
    #[derive(Debug, Clone, Copy)]
    pub struct Runtime {
        #[sysml::attr(default = 180000.0, unit = "J")]
        pub energy_j: Quantity<f64>,
        #[sysml::attr(default = 500.0, unit = "W")]
        pub power_w: Quantity<f64>,
        #[sysml::attr(default = 300.0, unit = "s")]
        pub min_runtime_s: Quantity<f64>,
    }

    #[sysml::part]
    #[sysml::satisfy(req = ModalPowerLimit, id = "XP-001")]
    #[sysml::satisfy(req = ThermalWindow, id = "XP-002")]
    #[sysml::satisfy(req = LoadAndCycles, id = "XP-003")]
    #[sysml::satisfy(req = SupplyVoltage, id = "XP-004")]
    #[sysml::satisfy(req = Runtime, id = "XP-005")]
    #[derive(Debug, Clone, Copy)]
    pub struct Unit {
        #[sysml::attr(default = 900.0, unit = "W")]
        pub rated_power_w: Quantity<f64>,
    }

    /// Clamped positive margin (if/else, unary minus).
    #[sysml::calc]
    pub fn clamped_margin(limit: f64, actual: f64) -> f64 {
        let raw = limit - actual;
        if raw > 0.0 { raw } else { -raw * 0.0 }
    }

    /// Nested calc call with locals.
    #[sysml::calc]
    pub fn doubled_margin_ratio(limit: f64, actual: f64, scale: f64) -> f64 {
        let m = clamped_margin(limit, actual);
        let r = m / scale;
        r * 2.0 + -(limit * 0.0)
    }

    /// Boolean logic folded into a number.
    #[sysml::calc]
    pub fn in_band(x: f64, lo: f64, hi: f64) -> f64 {
        if x >= lo && !(x > hi) || x == lo { 1.0 } else { 0.0 }
    }
}

#[test]
fn ir_evaluator_agrees_with_compiled_patterns() {
    let model = Model::new(vec![expr_patterns::sysml_package()]);
    let probes = expr_patterns::sysml_compiled_probes();
    assert_eq!(probes.requirements.len(), 5);
    assert_eq!(probes.calcs.len(), 3);
    let r = sysml_emit::expr_eval::agreement::check(&model, &probes, 64);
    assert!(r.unmatched_probes.is_empty(), "{:?}", r.unmatched_probes);
    assert!(r.disagreements.is_empty(), "{:#?}", r.disagreements);
    assert_eq!(r.requirements_compared, 5);
    assert_eq!(r.calcs_compared, 3);
    // XP-004 has an unmeasured evidence operand: unknown on defaults, never compared.
    assert_eq!(r.cases_unknown, 1);
    assert!(r.requirement_passes > 20 && r.requirement_fails > 20, "{r:?}");
}

/// The agreement check is not vacuous: an IR whose limit differs from the compiled default
/// disagrees on the defaults set.
#[test]
fn agreement_check_detects_a_diverging_ir() {
    let mut model = Model::new(vec![expr_patterns::sysml_package()]);
    for m in &mut model.packages[0].members {
        if let sysml_dsl::ir::PackageMember::RequirementDef(r) = m {
            if r.name == "ModalPowerLimit" {
                for a in &mut r.attributes {
                    if a.name.as_deref() == Some("normalLimitW") {
                        a.value = Some("1000.0".into());
                    }
                }
            }
        }
    }
    let probes = expr_patterns::sysml_compiled_probes();
    let r = sysml_emit::expr_eval::agreement::check(&model, &probes, 0);
    assert_eq!(r.disagreements.len(), 1, "{:#?}", r.disagreements);
    assert!(r.disagreements[0].contains("ModalPowerLimit"));
}
