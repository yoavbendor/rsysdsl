//! S7.4: `#[sysml::connect(tbd = "reason", ...)]` without an interface is a TBD channel ("some
//! comms link between A and B, protocol TBD"): field paths are checked, port compatibility and
//! interface typing are not. It is emitted as a plain connection with `@TbdMetadata::TBD`
//! (category channel), counted in the ledger, and `untyped_connections` does not fire for it.

use sysml_dsl::ir::TbdKind;
use sysml_dsl::prelude::*;

#[sysml::package(name = "TbdChannelDemo")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod tbd_channel_demo {
    use super::*;

    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Telemetry {
        #[sysml::attr(unit = "K")]
        pub temp_k: f64,
    }

    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Command {
        #[sysml::attr(dimensionless)]
        pub code: i64,
    }

    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct TelemetryPort {
        #[sysml::out_item]
        pub telemetry: Telemetry,
    }

    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct CommandPort {
        #[sysml::in_item]
        pub command: Command,
    }

    #[sysml::interface]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct TelemetryLink {
        #[sysml::end]
        pub tx: TelemetryPort,
        #[sysml::end]
        pub rx: Conjugated<TelemetryPort>,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Probe {
        #[sysml::port]
        pub tm_out: TelemetryPort,
        #[sysml::port]
        pub tm_out2: TelemetryPort,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Station {
        #[sysml::port]
        pub tm_in: Conjugated<TelemetryPort>,
        /// Deliberately incompatible with `TelemetryPort`: only a TBD channel may join them.
        #[sysml::port]
        pub cmd_in: CommandPort,
    }

    #[sysml::part]
    #[sysml::connect(interface = TelemetryLink, name = "tmLink", from = probe.tm_out, to = station.tm_in)]
    #[sysml::connect(tbd = "uplink protocol not in the ICD", name = "auxLink", from = probe.tm_out2, to = station.cmd_in)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Mission {
        #[sysml::part]
        pub probe: Probe,
        #[sysml::part]
        pub station: Station,
    }
}

#[test]
fn tbd_channel_is_emitted_counted_and_not_an_untyped_connection() {
    let model = Model::new(vec![tbd_channel_demo::sysml_package()]);
    let sysml = sysml_emit::emit_model(&model);
    assert!(sysml.contains("connection auxLink connect probe.tmOut2 to station.cmdIn {"), "{sysml}");
    assert!(sysml.contains("category = \"channel\";"), "{sysml}");

    let audit = sysml_emit::audit_model_with_stdlib("tbd_channel", &model, None, None, true);
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "{audit:#?}\n{sysml}"
    );

    let ledger = sysml_emit::collect_tbd_ledger(&model);
    assert_eq!(ledger.len(), 1, "{ledger:#?}");
    assert_eq!(ledger[0].kind, TbdKind::Channel);
    assert_eq!(ledger[0].element, "TbdChannelDemo::Mission::auxLink");

    let lint = sysml_emit::lint_model("tbd_channel", &model);
    assert!(
        !lint.findings.iter().any(|f| f.code == "untyped_connections"),
        "{lint:#?}"
    );

    let lifted = sysml_lift::lift_sysml_units(&[("ch.sysml".to_string(), sysml)]);
    assert!(lifted.unsupported.is_empty(), "{:#?}", lifted.unsupported);
    assert_eq!(sysml_emit::collect_tbd_ledger(&lifted.model), ledger);
}
