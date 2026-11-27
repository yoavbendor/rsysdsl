//! S7.1: every numeric attribute declares `unit`, `dimensionless`, `tbd_unit = "reason"` or
//! `custom_unit = "reason"`. `dimensionless` takes part in dimension checks as dimension 1;
//! `tbd_unit` and `custom_unit` skip them and are emitted as `@TbdMetadata::TBD` metadata,
//! counted in the TBD ledger and lifted back from the emitted SysML.

use sysml_dsl::ir::{TbdKind, TbdPhase};
use sysml_dsl::prelude::*;

#[sysml::package(name = "TbdUnitDemo")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod tbd_unit_demo {
    use super::*;

    /// TU-001: The pump efficiency shall be at least 0.6 (ratio).
    #[sysml::requirement(id = "TU-001")]
    #[sysml::require_constraint(efficiency >= min_efficiency)]
    #[derive(Debug, Clone, Copy)]
    pub struct PumpEfficiency {
        #[sysml::attr(default = 0.6, dimensionless)]
        pub min_efficiency: f64,
        #[sysml::attr(dimensionless, evidence)]
        pub efficiency: f64,
    }

    #[sysml::part]
    #[sysml::satisfy(req = PumpEfficiency, id = "TU-001")]
    #[derive(Debug, Clone, Copy)]
    pub struct Pump {
        #[sysml::attr(default = 2, dimensionless)]
        pub impeller_count: i64,
        /// Head loss coefficient: the source table gives no unit.
        #[sysml::attr(
            default = 0.8,
            tbd_unit = "datasheet table 4 gives no unit",
            tbd_owner = "hydraulics",
            tbd_resolve_by = "preliminary"
        )]
        pub loss_coefficient: f64,
    }
}

#[sysml::package(name = "CustomUnitDemo")]
pub mod custom_unit_demo {
    use super::*;

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Fan {
        #[sysml::attr(default = 3000.0, unit = "rpm", custom_unit = "supplier datasheet uses rpm")]
        pub speed: f64,
    }
}

#[test]
fn dimensionless_and_tbd_unit_emit_audit_and_round_trip() {
    let model = Model::new(vec![tbd_unit_demo::sysml_package()]);
    let sysml = sysml_emit::emit_model(&model);
    assert!(sysml.contains("@TbdMetadata::TBD {"), "{sysml}");
    assert!(sysml.contains("category = \"unit\";"), "{sysml}");
    assert!(sysml.contains("resolveBy = \"preliminary\";"), "{sysml}");
    assert!(sysml.contains("library package TbdMetadata"), "auto-included: {sysml}");
    // Dimensionless values carry no unit in SysML.
    assert!(sysml.contains("attribute impellerCount : Integer default 2;"), "{sysml}");

    let audit = sysml_emit::audit_model_with_stdlib("tbd_units", &model, None, None, true);
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "{audit:#?}\n{sysml}"
    );

    let ledger = sysml_emit::collect_tbd_ledger(&model);
    assert_eq!(ledger.len(), 1, "{ledger:#?}");
    assert_eq!(ledger[0].element, "TbdUnitDemo::Pump::lossCoefficient");
    assert_eq!(ledger[0].kind, TbdKind::Unit);
    assert_eq!(ledger[0].owner.as_deref(), Some("hydraulics"));
    assert_eq!(ledger[0].resolve_by, TbdPhase::Preliminary);

    let lifted = sysml_lift::lift_sysml_units(&[("tbd.sysml".to_string(), sysml)]);
    assert!(lifted.unsupported.is_empty(), "{:#?}", lifted.unsupported);
    assert_eq!(sysml_emit::collect_tbd_ledger(&lifted.model), ledger);
}

#[test]
fn custom_unit_is_a_counted_tbd() {
    let model = Model::new(vec![custom_unit_demo::sysml_package()]);
    let sysml = sysml_emit::emit_model(&model);
    assert!(sysml.contains("category = \"custom_unit\";"), "{sysml}");
    assert!(
        sysml.contains("private import ScalarValues::*;"),
        "ScalarValues should be auto-imported when omitted: {sysml}"
    );
    assert!(
        sysml.contains("[CustomUnitMetadata::rpm]"),
        "custom unit should resolve via CustomUnitMetadata: {sysml}"
    );
    assert!(
        sysml.contains("library package CustomUnitMetadata"),
        "auto-included CustomUnitMetadata: {sysml}"
    );

    let audit = sysml_emit::audit_model_with_stdlib("custom_unit", &model, None, None, true);
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "custom_unit + omitted ScalarValues import must pass strict OMG stdlib audit:\n{audit:#?}\n{sysml}"
    );

    let ledger = sysml_emit::collect_tbd_ledger(&model);
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].kind, TbdKind::CustomUnit);
    assert_eq!(ledger[0].reason, "supplier datasheet uses rpm");
    assert_eq!(ledger[0].resolve_by, TbdPhase::Detailed, "default phase");

    let lifted = sysml_lift::lift_sysml_units(&[("custom.sysml".to_string(), sysml)]);
    assert!(lifted.unsupported.is_empty(), "{:#?}", lifted.unsupported);
    assert_eq!(sysml_emit::collect_tbd_ledger(&lifted.model), ledger);
    let lifted_unit = match &lifted.model.packages[0].members[0] {
        sysml_ir::PackageMember::PartDef(p) => p.attributes[0].unit.as_deref(),
        _ => None,
    };
    assert_eq!(lifted_unit, Some("rpm"));
}

#[test]
fn dimensionless_requirement_evaluates() {
    let r = tbd_unit_demo::PumpEfficiency {
        efficiency: 0.7,
        ..Default::default()
    };
    assert!(r.evaluate());
    assert!(!tbd_unit_demo::PumpEfficiency::default().evaluate());
}
