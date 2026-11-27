//! S6.2: the smallest working interface taxonomy is an end-less abstract base
//! (`#[sysml::interface(abstract)] pub struct EnergyChannel {}`) specialized by concrete
//! interfaces that declare their own two `#[sysml::end]` ports. It passes the strict OMG audit
//! and clears the `unclassified_interfaces` lint.

use sysml_dsl::prelude::*;

#[sysml::package(name = "IfaceTaxonomy")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod iface_taxonomy {
    use super::*;

    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Heat {
        #[sysml::attr(default = 0.0, unit = "W")]
        pub rate: Quantity<f64>,
    }

    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Charge {
        #[sysml::attr(default = 0.0, unit = "A")]
        pub current: Quantity<f64>,
    }

    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Torque {
        #[sysml::attr(default = 0.0, unit = "N*m")]
        pub torque: Quantity<f64>,
    }

    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct HeatPort {
        #[sysml::out_item]
        pub heat: Heat,
    }

    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ChargePort {
        #[sysml::out_item]
        pub charge: Charge,
    }

    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ShaftPort {
        #[sysml::out_item]
        pub torque: Torque,
    }

    /// Abstract classification base: no ends of its own.
    #[sysml::interface(abstract)]
    pub struct EnergyChannel {}

    #[sysml::interface]
    #[sysml::specializes(EnergyChannel)]
    pub struct HeatLink {
        #[sysml::end]
        pub src: HeatPort,
        #[sysml::end]
        pub dst: Conjugated<HeatPort>,
    }

    #[sysml::interface]
    #[sysml::specializes(EnergyChannel)]
    pub struct ChargeLink {
        #[sysml::end]
        pub src: ChargePort,
        #[sysml::end]
        pub dst: Conjugated<ChargePort>,
    }

    #[sysml::interface]
    #[sysml::specializes(EnergyChannel)]
    pub struct ShaftLink {
        #[sysml::end]
        pub src: ShaftPort,
        #[sysml::end]
        pub dst: Conjugated<ShaftPort>,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Source {
        #[sysml::port]
        pub heat_out: HeatPort,
        #[sysml::port]
        pub charge_out: ChargePort,
        #[sysml::port]
        pub shaft_out: ShaftPort,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Sink {
        #[sysml::port]
        pub heat_in: Conjugated<HeatPort>,
        #[sysml::port]
        pub charge_in: Conjugated<ChargePort>,
        #[sysml::port]
        pub shaft_in: Conjugated<ShaftPort>,
    }

    #[sysml::part]
    #[sysml::connect(interface = HeatLink, name = "heat", from = src.heat_out, to = dst.heat_in)]
    #[sysml::connect(interface = ChargeLink, name = "charge", from = src.charge_out, to = dst.charge_in)]
    #[sysml::connect(interface = ShaftLink, name = "shaft", from = src.shaft_out, to = dst.shaft_in)]
    #[derive(Debug, Clone, Copy)]
    pub struct Context {
        #[sysml::part]
        pub src: Source,
        #[sysml::part]
        pub dst: Sink,
    }
}

#[test]
fn endless_abstract_interface_base_audits_and_lints_clean() {
    let model = Model::new(vec![iface_taxonomy::sysml_package()]);
    let sysml = sysml_emit::emit_model(&model);
    assert!(sysml.contains("abstract interface def EnergyChannel {"), "{sysml}");
    assert!(sysml.contains("interface def HeatLink :> EnergyChannel {"), "{sysml}");

    let audit = sysml_emit::audit_model_with_stdlib("iface", &model, None, None, true);
    assert!(audit.stdlib_loaded);
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "strict audit failed: {audit:#?}\n--- SYSML ---\n{sysml}"
    );

    let lint = sysml_emit::lint_model("iface", &model);
    assert!(lint.clean, "lint must be clean: {:#?}", lint.findings);
    assert_eq!(lint.summary.classified_interfaces, 3);
}
