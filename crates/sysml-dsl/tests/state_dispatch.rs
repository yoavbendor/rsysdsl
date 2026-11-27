//! S6b.5: `dispatch_with` / `dispatch_event` return `StepOutcome::Ignored` for an event that
//! matches no transition (the normal SysML no-op), while `step_with` / `step_event` keep
//! returning `Err` for existing callers.

use sysml_dsl::prelude::*;

#[sysml::package(name = "DispatchDemo")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod dispatch_demo {
    use super::*;

    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct TempReading {
        #[sysml::attr(unit = "K")]
        pub temp_k: f64,
    }

    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct ResetCommand {
        #[sysml::attr(default = 0.0)]
        pub code: f64,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Oven {
        #[sysml::attr(default = 500.0, unit = "K")]
        pub trip_temp_k: Quantity<f64>,
    }

    /// Guarded machine with a subject (`step_with` / `dispatch_with`).
    #[sysml::state(subject = Oven)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum OvenMode {
        #[sysml::initial]
        #[sysml::transition(on = TempReading, guard = event.temp_k > subject.trip_temp_k, to = Tripped)]
        Running,
        #[sysml::transition(on = ResetCommand, to = Running)]
        Tripped,
    }

    /// Machine without a subject (`step_event` / `dispatch_event`).
    #[sysml::state]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LampMode {
        #[sysml::initial]
        #[sysml::transition(on = ResetCommand, to = On)]
        Off,
        On,
    }
}

use dispatch_demo::*;

#[test]
fn dispatch_with_reports_ignored_events_without_error() {
    let oven = Oven::default();
    let mut mode = OvenMode::initial_state();

    // Guard false: ignored, state unchanged. `step_with` still says Err.
    let cool = TempReading { temp_k: 300.0 };
    assert_eq!(mode.dispatch_with(&cool, &oven), StepOutcome::Ignored);
    assert!(mode.step_with(&cool, &oven).is_err());
    assert_eq!(mode, OvenMode::Running);

    // Wrong trigger for the current state: ignored.
    assert!(mode.dispatch_with(&ResetCommand::default(), &oven).is_ignored());

    // Guard true: moves.
    let hot = TempReading { temp_k: 600.0 };
    let out = mode.dispatch_with(&hot, &oven);
    assert_eq!(out, StepOutcome::Moved(OvenMode::Tripped));
    assert!(out.moved());
    assert_eq!(mode, OvenMode::Tripped);
}

#[test]
fn dispatch_event_without_subject() {
    let mut lamp = LampMode::initial_state();
    assert_eq!(lamp.dispatch_event(&TempReading::default()), StepOutcome::Ignored);
    assert_eq!(
        lamp.dispatch_event(&ResetCommand::default()),
        StepOutcome::Moved(LampMode::On)
    );
    // `On` has no outgoing transition: every event is ignored.
    assert_eq!(lamp.dispatch_event(&ResetCommand::default()), StepOutcome::Ignored);
}
