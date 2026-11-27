//! S7.7: the "Gradual design with TBDs" snippet of PATTERNS.md (Pattern 8) compiles, emits,
//! passes the strict audit at the default maturity, fails at `release`, and its ledger lists
//! every hatch the pattern table names.

use sysml_dsl::ir::{TbdKind, TbdPhase};
use sysml_dsl::prelude::*;

#[sysml::package(name = "GradualDesign")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdPower::*")]
pub mod gradual_design {
    use super::*;
    use sysml_std::prelude::*;

    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Telemetry {
        #[sysml::attr(unit = "K")]
        pub temp_k: f64,
    }

    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct TelemetryPort {
        #[sysml::out_item]
        pub telemetry: Telemetry,
    }

    /// GD-001: The pump shall deliver at least <flow> (value not in the spec yet).
    #[sysml::requirement(id = "GD-001")]
    #[sysml::require_constraint(measured_flow >= min_flow)]
    #[derive(Debug, Clone, Copy)]
    pub struct PumpFlow {
        #[sysml::attr(unit = "m^3/s", tbd_value = "spec section 4 gives no minimum flow", tbd_owner = "hydraulics")]
        pub min_flow: Quantity<f64>,
        #[sysml::attr(unit = "m^3/s", evidence)]
        pub measured_flow: Quantity<f64>,
    }

    /// Mains supply named in the spec; its voltage is not.
    #[sysml::part]
    #[sysml::specializes(AcMainsSupply)]
    #[derive(Debug, Clone, Copy)]
    pub struct Mains {
        #[sysml::attr(redefines = "nominalVoltage", tbd_value = "spec names the supply, not its voltage", unit = "V")]
        pub nominal_voltage: Quantity<f64>,
    }

    #[sysml::part]
    #[sysml::tbd(reason = "pump supplier not selected", owner = "procurement", resolve_by = "preliminary")]
    #[sysml::satisfy(req = PumpFlow, id = "GD-001")]
    #[derive(Debug, Clone, Copy)]
    pub struct Pump {
        #[sysml::attr(default = 4, dimensionless)]
        pub impeller_blades: i64,
        #[sysml::attr(default = 0.8, tbd_unit = "datasheet table 2 gives no unit")]
        pub loss_coefficient: f64,
        #[sysml::port]
        pub telemetry_out: TelemetryPort,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Controller {
        #[sysml::port]
        pub telemetry_in: TelemetryPort,
    }

    /// Context: the telemetry link exists, its protocol does not yet (port polarity unchecked).
    #[sysml::part]
    #[sysml::connect(tbd = "fieldbus not chosen", name = "telemetryLink", from = pump.telemetry_out, to = controller.telemetry_in)]
    #[derive(Debug, Clone, Copy)]
    pub struct Plant {
        #[sysml::part]
        pub mains: Mains,
        #[sysml::part]
        pub pump: Pump,
        #[sysml::part]
        pub controller: Controller,
    }
}

fn model() -> Model {
    let mut pkgs = sysml_std::std_packages();
    pkgs.push(gradual_design::sysml_package());
    Model::new(pkgs)
}

#[test]
fn pattern8_snippet_is_in_patterns_md() {
    let md = include_str!("../../../.agents/skills/rsysdsl-modeling/references/PATTERNS.md");
    let src = include_str!("patterns_s7.rs");
    let start = src.find("#[sysml::package(name = \"GradualDesign\")]").unwrap();
    let end = src.find("\nfn model()").unwrap();
    let snippet = src[start..end].trim();
    assert!(md.contains(snippet), "PATTERNS.md Pattern 8 drifted from patterns_s7.rs");
}

#[test]
fn pattern8_audits_lints_and_gates_by_maturity() {
    let model = model();
    let mut audit = sysml_emit::audit_model_with_stdlib("patterns_s7", &model, None, None, true);
    let sysml = sysml_emit::emit_model(&model);
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "{audit:#?}\n{sysml}"
    );
    assert_eq!(audit.tbd_count, 5, "{:#?}", audit.tbd_ledger);
    let kinds: Vec<TbdKind> = audit.tbd_ledger.iter().map(|e| e.kind).collect();
    for k in [TbdKind::Value, TbdKind::Element, TbdKind::Unit, TbdKind::Channel] {
        assert!(kinds.contains(&k), "{kinds:?}");
    }

    sysml_emit::apply_maturity(&mut audit, TbdPhase::Preliminary, true);
    assert!(!audit.ok);
    assert_eq!(audit.overdue_tbds.len(), 1, "only the supplier TBD is due at preliminary");
    sysml_emit::apply_maturity(&mut audit, TbdPhase::Release, true);
    assert_eq!(audit.overdue_tbds.len(), 5);

    let lint = sysml_emit::lint_model("patterns_s7", &model);
    assert!(lint.clean, "{:#?}", lint.findings);
    assert_eq!(lint.summary.tbd_count, 5);
    assert!(!lint
        .findings
        .iter()
        .any(|f| f.code == "inherited_std_default" && f.message.contains("nominalVoltage")));
}
