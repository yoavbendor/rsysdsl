//! S6.5: `#[sysml::attr(..., evidence)]` separates a requirement's measured value from its
//! spec limit. Without a value the evidence is NaN (`evaluate()` is false until a verification
//! case supplies it); the trace matrix only counts verdicts as verification, and the audit flags
//! a "measured" value copied from the limit as `hollow-requirement`.

use sysml_dsl::prelude::*;
use sysml_dsl::ir::{VerificationReport, VerificationStatus, VerificationVerdict};

#[sysml::package(name = "EvidenceDemo")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod evidence_demo {
    use super::*;

    /// EV-001: The heater shall draw at most 1500 W.
    #[sysml::requirement(id = "EV-001")]
    #[sysml::require_constraint(measured_power_w <= max_power_w)]
    #[derive(Debug, Clone, Copy)]
    pub struct HeaterPowerLimit {
        #[sysml::attr(default = 1500.0, unit = "W")]
        pub max_power_w: Quantity<f64>,
        /// Not in the spec: supplied by verification.
        #[sysml::attr(unit = "W", evidence)]
        pub measured_power_w: Quantity<f64>,
    }

    /// EV-002: hollow on purpose (measured value copied from the limit).
    #[sysml::requirement(id = "EV-002")]
    #[sysml::require_constraint(actual_temp_k <= max_temp_k)]
    #[derive(Debug, Clone, Copy)]
    pub struct HollowTempLimit {
        #[sysml::attr(default = 350.0, unit = "K")]
        pub max_temp_k: Quantity<f64>,
        #[sysml::attr(default = 350.0, unit = "K")]
        pub actual_temp_k: Quantity<f64>,
    }

    /// EV-003: evidence attribute WITH a model default (a placeholder, not evidence).
    #[sysml::requirement(id = "EV-003")]
    #[sysml::require_constraint(measured_flow <= max_flow)]
    #[derive(Debug, Clone, Copy)]
    pub struct FlowLimit {
        #[sysml::attr(default = 0.002, unit = "m^3/s")]
        pub max_flow: Quantity<f64>,
        #[sysml::attr(default = 0.001, unit = "m^3/s", evidence)]
        pub measured_flow: Quantity<f64>,
    }

    #[sysml::part]
    #[sysml::satisfy(req = HeaterPowerLimit, id = "EV-001")]
    #[sysml::satisfy(req = HollowTempLimit, id = "EV-002")]
    #[sysml::satisfy(req = FlowLimit, id = "EV-003")]
    #[derive(Debug, Clone, Copy)]
    pub struct Heater {
        #[sysml::attr(default = 1200.0, unit = "W")]
        pub rated_power_w: Quantity<f64>,
    }
}

use evidence_demo::*;

#[test]
fn unmeasured_evidence_never_evaluates_true() {
    let r = HeaterPowerLimit::default();
    assert!(r.measured_power_w.value().is_nan());
    assert!(!r.evaluate(), "no evidence -> requirement cannot pass");
    let measured = HeaterPowerLimit {
        measured_power_w: Quantity::new(1210.0, "W"),
        ..Default::default()
    };
    assert!(measured.evaluate());
}

#[test]
fn evidence_is_emitted_audited_and_traced_as_placeholder_until_verified() {
    let model = Model::new(vec![evidence_demo::sysml_package()]);
    let sysml = sysml_emit::emit_model(&model);
    assert!(sysml.contains("[evidence]"), "{sysml}");

    let audit = sysml_emit::audit_model_with_stdlib("evidence", &model, None, None, true);
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "hollow requirements warn but do not fail: {audit:#?}\n{sysml}"
    );
    assert_eq!(audit.hollow_requirements.len(), 1, "{audit:#?}");
    assert!(audit.hollow_requirements[0].starts_with("EV-002"));

    let tm = sysml_emit::build_trace_matrix("evidence", &model, None);
    let row = |id: &str| tm.rows.iter().find(|r| r.req_id == id).unwrap().clone();
    // Evidence attribute without a value: nothing is claimed yet.
    assert_eq!(row("EV-001").evidence_source, "no evidence supplied");
    assert_eq!(row("EV-001").evidence_attributes, vec!["measuredPowerW".to_string()]);
    assert_eq!(row("EV-002").evidence_source, "unmarked");
    // Evidence attribute with a default: a model placeholder.
    assert_eq!(row("EV-003").evidence_source, "model default (placeholder)");
    assert_eq!(tm.summary.hollow_requirements, 1);
    assert_eq!(tm.summary.placeholder_evidence, 2, "EV-002 + EV-003, not EV-001");
    let md = sysml_emit::render_trace_matrix_markdown(&tm);
    let line = |id: &str| md.lines().find(|l| l.contains(id)).unwrap().to_string();
    assert!(line("EV-001").contains("no evidence supplied yet"), "{md}");
    assert!(line("EV-003").contains("evidence is a model default (placeholder)"), "{md}");
    assert!(!line("EV-001").contains("placeholder"), "{md}");

    let report = VerificationReport::from_verdicts(
        "evidence",
        vec![VerificationVerdict {
            req_id: "EV-001".into(),
            req_name: "HeaterPowerLimit".into(),
            status: VerificationStatus::Pass,
            scenario: "bench".into(),
            satisfied_by: "Heater".into(),
            evidence: serde_json::json!({ "measured_power_w": 1210.0 }),
            message: "ok".into(),
        }],
    );
    let tm = sysml_emit::build_trace_matrix("evidence", &model, Some(&report));
    let r1 = tm.rows.iter().find(|r| r.req_id == "EV-001").unwrap();
    assert_eq!(r1.verification_status, "pass");
    assert_eq!(r1.evidence_source, "verification");
}
