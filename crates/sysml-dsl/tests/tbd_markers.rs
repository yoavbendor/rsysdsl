//! S7.3: the generic `#[sysml::tbd(reason = "...", owner = "...", resolve_by = "...")]` marker on
//! parts, ports, items, interfaces, requirements, states and part / port / attribute fields, plus
//! `tbd = "reason"` on transitions. Each is emitted as `@TbdMetadata::TBD`, passes the strict
//! audit, appears in the TBD ledger and survives a lift of the emitted SysML.

use sysml_dsl::ir::{TbdKind, TbdPhase};
use sysml_dsl::prelude::*;

#[sysml::package(name = "TbdMarkerDemo")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod tbd_marker_demo {
    use super::*;

    #[sysml::tbd(reason = "payload fields not in the ICD yet", owner = "comms")]
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct StatusFrame {
        #[sysml::attr(dimensionless)]
        pub code: i64,
    }

    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Wake {}

    #[sysml::port]
    #[sysml::tbd(reason = "connector type open", resolve_by = "preliminary")]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct StatusPort {
        #[sysml::out_item]
        pub frame: StatusFrame,
    }

    #[sysml::interface]
    #[sysml::tbd(reason = "bus protocol open", resolve_by = "detailed")]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct StatusLink {
        #[sysml::end]
        pub tx: StatusPort,
        #[sysml::end]
        pub rx: Conjugated<StatusPort>,
    }

    /// TM-001: The sensor shall report its status.
    #[sysml::requirement(id = "TM-001")]
    #[sysml::tbd(reason = "reporting period not stated in the spec", owner = "systems", resolve_by = "release")]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ReportStatus {}

    #[sysml::part]
    #[sysml::tbd(reason = "sensor supplier not selected")]
    #[sysml::satisfy(req = ReportStatus, id = "TM-001")]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Sensor {
        #[sysml::port]
        #[sysml::tbd(reason = "pinout open")]
        pub status_out: StatusPort,
        #[sysml::tbd(reason = "sample rate pending")]
        #[sysml::attr(default = 10.0, unit = "Hz")]
        pub sample_rate: Quantity<f64>,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct Logger {
        #[sysml::port]
        pub status_in: Conjugated<StatusPort>,
    }

    #[sysml::part]
    #[sysml::connect(interface = StatusLink, name = "statusLink", from = sensor.status_out, to = logger.status_in)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct System {
        #[sysml::part]
        #[sysml::tbd(reason = "redundancy not decided", resolve_by = "concept")]
        pub sensor: Sensor,
        #[sysml::part]
        pub logger: Logger,
    }

    #[sysml::state]
    #[sysml::tbd(reason = "low-power modes not specified")]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum SensorMode {
        #[sysml::initial]
        #[sysml::transition(on = Wake, to = Active, tbd = "wake trigger not specified")]
        Sleep,
        #[sysml::tbd(reason = "active sub-modes open")]
        Active,
    }
}

fn model() -> Model {
    Model::new(vec![tbd_marker_demo::sysml_package()])
}

#[test]
fn tbd_markers_emit_audit_ledger_and_lift() {
    let model = model();
    let sysml = sysml_emit::emit_model(&model);
    assert_eq!(sysml.matches("@TbdMetadata::TBD {").count(), 11, "{sysml}");

    let audit = sysml_emit::audit_model_with_stdlib("tbd_markers", &model, None, None, true);
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "{audit:#?}\n{sysml}"
    );

    let ledger = sysml_emit::collect_tbd_ledger(&model);
    let elements: Vec<&str> = ledger.iter().map(|e| e.element.as_str()).collect();
    for expected in [
        "TbdMarkerDemo::StatusFrame",
        "TbdMarkerDemo::StatusPort",
        "TbdMarkerDemo::StatusLink",
        "TbdMarkerDemo::ReportStatus",
        "TbdMarkerDemo::Sensor",
        "TbdMarkerDemo::Sensor::statusOut",
        "TbdMarkerDemo::Sensor::sampleRate",
        "TbdMarkerDemo::System::sensor",
        "TbdMarkerDemo::SensorMode",
        "TbdMarkerDemo::SensorMode::active",
        "TbdMarkerDemo::SensorMode::sleep -> active",
    ] {
        assert!(elements.contains(&expected), "{expected} missing from {elements:#?}");
    }
    assert!(ledger.iter().all(|e| e.kind == TbdKind::Element));
    let req = ledger.iter().find(|e| e.element.ends_with("ReportStatus")).unwrap();
    assert_eq!(req.owner.as_deref(), Some("systems"));
    assert_eq!(req.resolve_by, TbdPhase::Release);

    let lifted = sysml_lift::lift_sysml_units(&[("tbd.sysml".to_string(), sysml)]);
    assert!(lifted.unsupported.is_empty(), "{:#?}", lifted.unsupported);
    assert_eq!(sysml_emit::collect_tbd_ledger(&lifted.model), ledger);
}

#[test]
fn tbd_markers_do_not_change_behaviour() {
    let mut mode = tbd_marker_demo::SensorMode::initial_state();
    assert_eq!(
        mode.dispatch_event(&tbd_marker_demo::Wake {}),
        StepOutcome::Moved(tbd_marker_demo::SensorMode::Active)
    );
}
