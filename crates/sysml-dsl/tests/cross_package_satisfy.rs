//! S6.1: a `#[sysml::satisfy]` that points at a requirement in another package must pass the
//! strict OMG audit WITHOUT a manual `#[sysml::import("OtherPkg::*")]` on the satisfying package.

use sysml_dsl::prelude::*;

#[sysml::package(name = "CrossReqs")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod cross_reqs {
    use super::*;

    /// The supply shall deliver at least 100 W.
    #[sysml::requirement(id = "XP-001")]
    #[sysml::require_constraint(delivered_w >= min_w)]
    #[derive(Debug, Clone, Copy)]
    pub struct MinPower {
        #[sysml::attr(default = 100.0, unit = "W")]
        pub min_w: Quantity<f64>,
        #[sysml::attr(default = 120.0, unit = "W")]
        pub delivered_w: Quantity<f64>,
    }
}

// Note: no `#[sysml::import("CrossReqs::*")]` here.
#[sysml::package(name = "CrossArch")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod cross_arch {
    use super::*;

    #[sysml::part]
    #[sysml::satisfy(req = crate::cross_reqs::MinPower, id = "XP-001")]
    #[derive(Debug, Clone, Copy)]
    pub struct Supply {
        #[sysml::attr(default = 120.0, unit = "W")]
        pub rated_w: Quantity<f64>,
    }
}

#[test]
fn cross_package_satisfy_passes_strict_audit_without_manual_import() {
    let model = Model::new(vec![cross_reqs::sysml_package(), cross_arch::sysml_package()]);
    let sysml = sysml_emit::emit_model(&model);
    assert!(
        sysml.contains("satisfy requirement : CrossReqs::MinPower;"),
        "cross-package satisfy must be emitted qualified:\n{sysml}"
    );
    let audit = sysml_emit::audit_model_with_stdlib("cross_pkg", &model, None, None, true);
    assert!(audit.stdlib_loaded, "OMG sysml.library must be loaded");
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "strict audit failed: {audit:#?}\n--- SYSML ---\n{sysml}"
    );
}
