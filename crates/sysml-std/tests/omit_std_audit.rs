//! S6.4b: `emit --omit-std` splits the sysml-std packages out of the model's file; the model
//! file plus the std unit still passes the strict audit (what `rsysdsl audit --files` does).

use sysml_dsl::prelude::*;
use sysml_emit::{emit_model_split_std, validate_sysml_units_with_lib};
use sysml_std::prelude::*;

#[sysml::package(name = "OmitStdDemo")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("SysmlStdPower::*")]
pub mod omit_std_demo {
    use super::*;

    #[sysml::part]
    #[sysml::specializes(AcMainsSupply)]
    #[derive(Debug, Clone, Copy)]
    pub struct Mains {
        #[sysml::attr(redefines = "mainsFrequency", default = 60.0, unit = "Hz")]
        pub mains_frequency: Quantity<f64>,
    }
}

#[test]
fn omit_std_output_passes_strict_audit_with_std_as_library_unit() {
    let mut pkgs = sysml_std::std_packages();
    pkgs.push(omit_std_demo::sysml_package());
    let (own, std_text) = emit_model_split_std(&Model::new(pkgs));
    assert!(!own.contains("package SysmlStd"), "{own}");
    assert!(own.contains("package OmitStdDemo"));
    assert!(std_text.contains("library package SysmlStdPower"));

    let report = validate_sysml_units_with_lib(
        &[
            ("model.sysml".to_string(), own),
            ("<sysml-std>".to_string(), std_text),
        ],
        sysml_emit::discover_omg_stdlib().as_deref(),
    );
    assert!(report.stdlib_loaded);
    assert!(
        report.error_count == 0 && report.warning_count == 0,
        "{report:#?}"
    );
}
