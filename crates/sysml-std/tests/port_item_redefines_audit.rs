//! S6.3: a port specializing a sysml-std port that re-declares an inherited `out item` must
//! use `#[sysml::out_item(redefines)]` (emits `out item :>> electrical : ...;`). Without it the
//! strict audit reports `validateNamespaceDistinguishibility` (duplicate inherited member).

use sysml_dsl::prelude::*;
use sysml_emit::{audit_model_with_stdlib, emit_model};
use sysml_std::prelude::*;

#[sysml::package(name = "PortItemRedefines")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdFlows::*")]
pub mod port_item_redefines {
    use super::*;

    /// Mains electrical energy (narrows the std `ElectricalEnergy` payload).
    #[sysml::item]
    #[sysml::specializes(ElectricalEnergy)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MainsEnergy {
        #[sysml::attr(redefines = "voltage", default = 230.0, unit = "V")]
        pub voltage: Quantity<f64>,
    }

    /// Mains port: specializes the std power port and narrows its inherited `electrical` item.
    #[sysml::port]
    #[sysml::specializes(ElectricalPowerPort)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct MainsPort {
        #[sysml::out_item(redefines)]
        pub electrical: MainsEnergy,
    }

    /// Same, redefining under a new local name.
    #[sysml::port]
    #[sysml::specializes(ElectricalPowerPort)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct RenamedMainsPort {
        #[sysml::out_item(redefines = "electrical")]
        pub mains: MainsEnergy,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Socket {
        #[sysml::port]
        pub out_a: MainsPort,
        #[sysml::port]
        pub out_b: RenamedMainsPort,
    }
}

fn model() -> Model {
    let mut pkgs = sysml_std::std_packages();
    pkgs.push(port_item_redefines::sysml_package());
    Model::new(pkgs)
}

#[test]
fn out_item_redefines_passes_strict_audit() {
    let model = model();
    let sysml = emit_model(&model);
    assert!(sysml.contains("out item :>> electrical : MainsEnergy;"), "{sysml}");
    assert!(sysml.contains("out item mains : MainsEnergy :>> electrical;"), "{sysml}");
    let audit = audit_model_with_stdlib("port_item_redefines", &model, None, None, true);
    assert!(audit.stdlib_loaded);
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "strict audit failed: {audit:#?}"
    );
}

#[test]
fn redeclaring_inherited_item_without_redefines_is_flagged() {
    // The same text without `:>>` duplicates the inherited member.
    let model = model();
    let sysml = emit_model(&model).replace(
        "out item :>> electrical : MainsEnergy;",
        "out item electrical : MainsEnergy;",
    );
    let report = sysml_emit::validate_sysml_units_with_lib(
        &[("dup.sysml".to_string(), sysml)],
        sysml_emit::discover_omg_stdlib().as_deref(),
    );
    assert!(
        report
            .warnings
            .iter()
            .chain(report.model_errors.iter())
            .any(|d| d.message.contains("Distinguishibility") || d.message.contains("distinguish")),
        "expected a duplicate-member diagnostic: {report:#?}"
    );
}
