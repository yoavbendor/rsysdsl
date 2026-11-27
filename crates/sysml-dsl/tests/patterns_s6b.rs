//! S6b.6: every snippet added to PATTERNS.md for trial-2 findings N7 / N11 compiles here and
//! passes the strict OMG stdlib audit:
//! (a) cross-package `#[sysml::satisfy]`: no SysML import, but a Rust `use` of the module;
//! (b) bool item fields and `!` / `&&` / `||` / `==` in guards;
//! (c) a field-less `#[sysml::item]` used as a trigger event;
//! (d) connecting to an inherited port through a `#[sysml::port(redefines)]` field.

use sysml_dsl::prelude::*;

#[sysml::package(name = "PatTypes")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod pat_types {
    use super::*;

    // (c) Field-less event: a pure trigger with no payload.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ResetPressed {}

    // (b) Event with bool and numeric fields.
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct DoorStatus {
        pub door_open: bool,
        pub child_lock: bool,
        #[sysml::attr(unit = "K")]
        pub cavity_temp_k: f64,
    }

    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct PowerFlow {
        #[sysml::attr(default = 0.0, unit = "W")]
        pub power_w: Quantity<f64>,
    }

    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct PowerPort {
        #[sysml::out_item]
        pub power: PowerFlow,
    }

    #[sysml::interface]
    pub struct PowerLink {
        #[sysml::end]
        pub source: PowerPort,
        #[sysml::end]
        pub sink: Conjugated<PowerPort>,
    }
}

#[sysml::package(name = "PatRequirements")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod pat_requirements {
    use super::*;

    /// The heater shall stop when the door opens.
    #[sysml::requirement(id = "PAT-001")]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct StopOnDoorOpen {}
}

// (a) No `#[sysml::import("PatRequirements::*")]` is needed on the SysML side, but the Rust
// `use super::pat_requirements::*;` IS needed so `req = StopOnDoorOpen` resolves.
#[sysml::package(name = "PatSubsystems")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("PatTypes::*")]
pub mod pat_subsystems {
    use super::pat_requirements::*;
    use super::pat_types::*;
    use super::*;

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Heater {
        #[sysml::attr(default = 400.0, unit = "K")]
        pub max_cavity_temp_k: Quantity<f64>,
        #[sysml::attr(default = true)]
        pub enabled: bool,
    }

    // (b) Guards on bool fields: `!`, `&&`, `||`, `== true` all compile to SysML
    // `not` / `and` / `or` / `==` and run in `dispatch_with`.
    #[sysml::state(subject = Heater)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HeaterMode {
        #[sysml::initial]
        #[sysml::transition(
            on = DoorStatus,
            guard = event.door_open || event.cavity_temp_k > subject.max_cavity_temp_k,
            to = Stopped
        )]
        Heating,
        #[sysml::transition(
            on = DoorStatus,
            guard = !event.door_open && !event.child_lock && subject.enabled == true,
            to = Heating
        )]
        // (c) A field-less event as an unguarded trigger.
        #[sysml::transition(on = ResetPressed, to = Heating)]
        Stopped,
    }

    // (d) Abstract parent owns the port.
    #[sysml::part(abstract)]
    #[derive(Debug, Clone, Copy)]
    pub struct PowerSourceBase {
        #[sysml::port]
        pub power_out: PowerPort,
    }

    // (d) To CONNECT to the inherited port, redeclare it as `#[sysml::port(redefines)]` with the
    // same field name: `#[sysml::connect]` checks fields on this Rust struct, and the redefinition
    // keeps the SysML free of duplicate-member warnings.
    #[sysml::part]
    #[sysml::specializes(PowerSourceBase)]
    #[sysml::satisfy(req = StopOnDoorOpen, id = "PAT-001")]
    #[derive(Debug, Clone, Copy)]
    pub struct Mains {
        #[sysml::port(redefines)]
        pub power_out: PowerPort,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Load {
        #[sysml::port]
        pub power_in: Conjugated<PowerPort>,
    }

    #[sysml::part]
    #[sysml::connect(interface = PowerLink, name = "mainsToLoad", from = mains.power_out, to = load.power_in)]
    #[derive(Debug, Clone, Copy)]
    pub struct Appliance {
        #[sysml::part]
        pub mains: Mains,
        #[sysml::part]
        pub load: Load,
    }
}

use pat_subsystems::{Heater, HeaterMode};
use pat_types::{DoorStatus, ResetPressed};

fn model() -> Model {
    Model::new(vec![
        pat_types::sysml_package(),
        pat_requirements::sysml_package(),
        pat_subsystems::sysml_package(),
    ])
}

#[test]
fn s6b_pattern_snippets_emit_and_pass_strict_audit() {
    let model = model();
    let sysml = sysml_emit::emit_model(&model);
    assert!(sysml.contains("satisfy requirement : PatRequirements::StopOnDoorOpen;"), "{sysml}");
    assert!(sysml.contains("not payload.doorOpen and not payload.childLock"), "{sysml}");
    assert!(sysml.contains(" or "), "{sysml}");
    let audit = sysml_emit::audit_model_with_stdlib("patterns_s6b", &model, None, None, true);
    assert!(audit.stdlib_loaded);
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "strict audit failed: {audit:#?}\n--- SYSML ---\n{sysml}"
    );
}

#[test]
fn bool_guards_and_fieldless_events_execute() {
    let heater = Heater::default();
    let mut mode = HeaterMode::initial_state();
    let closed_cool = DoorStatus { door_open: false, child_lock: false, cavity_temp_k: 300.0 };
    assert_eq!(mode.dispatch_with(&closed_cool, &heater), StepOutcome::Ignored);

    let open = DoorStatus { door_open: true, ..closed_cool };
    assert_eq!(mode.dispatch_with(&open, &heater), StepOutcome::Moved(HeaterMode::Stopped));

    // Door closed but child lock on: `!event.child_lock` is false -> ignored.
    let locked = DoorStatus { child_lock: true, ..closed_cool };
    assert_eq!(mode.dispatch_with(&locked, &heater), StepOutcome::Ignored);
    assert_eq!(mode.dispatch_with(&closed_cool, &heater), StepOutcome::Moved(HeaterMode::Heating));

    let too_hot = DoorStatus { cavity_temp_k: 450.0, ..closed_cool };
    assert_eq!(mode.dispatch_with(&too_hot, &heater), StepOutcome::Moved(HeaterMode::Stopped));
    assert_eq!(
        mode.dispatch_with(&ResetPressed {}, &heater),
        StepOutcome::Moved(HeaterMode::Heating)
    );
}
