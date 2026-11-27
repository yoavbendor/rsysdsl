//! S7.2 (finding N4): `#[sysml::attr(redefines = "...", tbd_value = "reason")]` explicitly blanks
//! an inherited sysml-std library default the source does not specify. The redefinition is
//! emitted without a value plus `@TbdMetadata::TBD`, passes the strict audit, counts in the TBD
//! ledger, and `inherited_std_default` no longer lists it.

use sysml_dsl::ir::TbdKind;
use sysml_dsl::prelude::*;
use sysml_emit::{emit_model_split_std, validate_sysml_units_with_lib};
use sysml_std::prelude::*;

#[sysml::package(name = "TbdValueDemo")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdPower::*")]
pub mod tbd_value_demo {
    use super::*;

    #[sysml::part]
    #[sysml::specializes(AcMainsSupply)]
    #[derive(Debug, Clone, Copy)]
    pub struct Mains {
        /// Source: grid spec section 2 (60 Hz).
        #[sysml::attr(redefines = "mainsFrequency", default = 60.0, unit = "Hz")]
        pub mains_frequency: Quantity<f64>,
        /// The source names the supply but not its voltage.
        #[sysml::attr(
            redefines = "nominalVoltage",
            tbd_value = "grid spec gives no nominal voltage",
            unit = "V"
        )]
        pub nominal_voltage: Quantity<f64>,
    }
}

fn model() -> Model {
    let mut pkgs = sysml_std::std_packages();
    pkgs.push(tbd_value_demo::sysml_package());
    Model::new(pkgs)
}

#[test]
fn tbd_value_default_is_nan() {
    let m = tbd_value_demo::Mains::default();
    assert!(m.nominal_voltage.value().is_nan());
    assert_eq!(m.mains_frequency.value(), 60.0);
}

#[test]
fn tbd_value_blanks_inherited_std_default_and_passes_strict_audit() {
    let model = model();
    let (own, std_text) = emit_model_split_std(&model);
    assert!(own.contains("attribute :>> nominalVoltage {"), "{own}");
    assert!(own.contains("category = \"value\";"), "{own}");
    assert!(!own.contains("nominalVoltage default"), "{own}");
    assert!(own.contains("library package TbdMetadata"), "{own}");

    let report = validate_sysml_units_with_lib(
        &[
            ("model.sysml".to_string(), own),
            ("<sysml-std>".to_string(), std_text),
        ],
        sysml_emit::discover_omg_stdlib().as_deref(),
    );
    assert!(report.stdlib_loaded);
    assert!(report.error_count == 0 && report.warning_count == 0, "{report:#?}");

    let ledger = sysml_emit::collect_tbd_ledger(&model);
    assert_eq!(ledger.len(), 1, "{ledger:#?}");
    assert_eq!(ledger[0].kind, TbdKind::Value);
    assert_eq!(ledger[0].element, "TbdValueDemo::Mains::nominalVoltage");

    let lint = sysml_emit::lint_model("tbd_value", &model);
    let inherited: Vec<_> = lint
        .findings
        .iter()
        .filter(|f| f.code == "inherited_std_default" && f.element == "Mains")
        .collect();
    assert_eq!(inherited.len(), 1, "{lint:#?}");
    let msg = &inherited[0].message;
    assert!(!msg.contains("nominalVoltage"), "blanked by tbd_value: {msg}");
    assert!(!msg.contains("mainsFrequency"), "redefined with the source value: {msg}");
    assert!(msg.contains("maxCurrent"), "still inherited: {msg}");
}
