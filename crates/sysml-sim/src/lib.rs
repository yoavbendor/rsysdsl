//! `sysml-sim` — a small, deterministic, fixed-step scenario runner.
//!
//! Runner tier only: execution crates (e.g. `apollo11-sim`) depend on this crate and on a
//! model crate; model crates must never depend on `sysml-sim`.
//!
//! - [`Scenario`]: the system under test plus its environment, stepped at a fixed `dt`.
//! - [`rk4_step`] / [`ZeroCrossing`]: continuous dynamics and event detection so the
//!   model's generated `step_with(...)` state machines can fire mid-run.
//! - [`Monitor`]: requirement checks (closures or `EvaluableRequirement::evaluate()`),
//!   sampled every step or once at the end, producing a [`Verdict`] each.
//! - [`run`]: executes a scenario and returns a [`RunResult`] (JSON [`Trace`] + verdicts).
//! - [`Registry`]: named scenarios for `rsysdsl verify --scenario <name>`.

mod integrate;
mod monitor;
mod registry;

pub use integrate::{rk4_step, Crossing, ZeroCrossing};
pub use monitor::{Monitor, Sampling, Verdict};
pub use registry::{RegisteredScenario, Registry, ScenarioOutcome};

use monitor::MonitorState;
use serde::{Deserialize, Serialize};
use sysml_ir::{VerificationReport, VerificationStatus, VerificationVerdict};

/// A time-triggered fault: `apply` mutates the scenario once, at the first sample with
/// `t >= at_s` (before that sample is observed and before the next step).
pub struct Fault<S> {
    pub at_s: f64,
    pub name: String,
    pub apply: Box<dyn FnMut(&mut S)>,
}

impl<S> Fault<S> {
    pub fn at(at_s: f64, name: impl Into<String>, apply: impl FnMut(&mut S) + 'static) -> Self {
        Self {
            at_s,
            name: name.into(),
            apply: Box::new(apply),
        }
    }
}

/// A runnable verification scenario. The implementing struct *is* the simulation state:
/// construct it in its initial configuration, then [`run`] it.
///
/// Execution order (with `t_k = k * dt`, `k` an integer so time never drifts):
/// 1. `initialize()` once;
/// 2. at each `t_k`: apply due faults, record signals / mode, sample `EveryStep` monitors;
///    stop if `k * dt >= duration()` or `is_done(t_k)`; otherwise `step(t_k, dt)`;
/// 3. `finalize(t_end)`, then sample `AtEnd` monitors and capture evidence.
pub trait Scenario: Sized + 'static {
    /// Scenario name used in traces and verdicts.
    fn name(&self) -> &str;
    /// Fixed step size in seconds.
    fn dt(&self) -> f64;
    /// Maximum simulated duration in seconds.
    fn duration(&self) -> f64;
    /// Requirement IDs this scenario verifies. Defaults to the monitors' IDs.
    fn requirement_ids(&self) -> Vec<String> {
        self.monitors()
            .iter()
            .map(|m| m.requirement_id().to_string())
            .collect()
    }
    /// Set up the initial state (e.g. drive the mode machine to its start phase).
    fn initialize(&mut self) {}
    /// Advance the state from `t` to `t + dt`.
    fn step(&mut self, t: f64, dt: f64);
    /// Early termination condition (e.g. touchdown), checked before every step.
    fn is_done(&self, _t: f64) -> bool {
        false
    }
    /// End-of-run hook (e.g. fire a terminal event into the state machine).
    fn finalize(&mut self, _t: f64) {}
    /// Named scalar signals recorded into the trace at every sample.
    fn signals(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
    /// Current discrete mode (e.g. `sysml_state_name()`); changes are logged as trace events.
    fn mode(&self) -> Option<&str> {
        None
    }
    /// Requirement monitors.
    fn monitors(&self) -> Vec<Monitor<Self>>;
    /// Time-triggered fault schedule.
    fn faults(&self) -> Vec<Fault<Self>> {
        Vec::new()
    }
}

/// Kind of a discrete trace event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Fault,
    Mode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraceEvent {
    pub t_s: f64,
    pub kind: EventKind,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    pub name: String,
    pub values: Vec<f64>,
}

/// Deterministic time series of named signals plus discrete events.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Trace {
    pub time_s: Vec<f64>,
    pub signals: Vec<Signal>,
    pub events: Vec<TraceEvent>,
}

impl Trace {
    fn record(&mut self, t: f64, sample: Vec<(&'static str, f64)>) {
        if self.signals.is_empty() {
            self.signals = sample
                .iter()
                .map(|(n, _)| Signal {
                    name: n.to_string(),
                    values: Vec::new(),
                })
                .collect();
        }
        assert_eq!(
            self.signals.len(),
            sample.len(),
            "Scenario::signals() must return the same signals at every sample"
        );
        self.time_s.push(t);
        for (sig, (_, v)) in self.signals.iter_mut().zip(sample) {
            sig.values.push(v);
        }
    }

    /// Values of a named signal.
    pub fn signal(&self, name: &str) -> Option<&[f64]> {
        self.signals
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.values.as_slice())
    }
}

/// Everything a run produced. Serializes to deterministic JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunResult {
    pub scenario: String,
    pub requirement_ids: Vec<String>,
    pub dt_s: f64,
    pub steps: usize,
    pub end_time_s: f64,
    /// `true` when `is_done` ended the run before `duration()`.
    pub stopped_early: bool,
    pub pass: bool,
    pub verdicts: Vec<Verdict>,
    pub trace: Trace,
}

impl RunResult {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("RunResult serializes")
    }

    pub fn verdict(&self, requirement_id: &str) -> Option<&Verdict> {
        self.verdicts
            .iter()
            .find(|v| v.requirement_id == requirement_id)
    }

    /// Convert to the CLI / trace-matrix `VerificationReport` format.
    pub fn to_report(&self, model: &str) -> VerificationReport {
        let verdicts = self
            .verdicts
            .iter()
            .map(|v| VerificationVerdict {
                req_id: v.requirement_id.clone(),
                req_name: v.requirement_name.clone(),
                status: if v.pass {
                    VerificationStatus::Pass
                } else {
                    VerificationStatus::Fail
                },
                scenario: v.scenario.clone(),
                satisfied_by: v.satisfied_by.clone(),
                evidence: v.evidence.clone(),
                message: v.message.clone(),
            })
            .collect();
        VerificationReport::from_verdicts(model, verdicts)
    }
}

/// Run a scenario to completion.
pub fn run<S: Scenario>(scenario: &mut S) -> RunResult {
    let dt = scenario.dt();
    assert!(dt > 0.0 && dt.is_finite(), "Scenario::dt() must be positive");
    let max_steps = (scenario.duration() / dt).round() as usize;

    scenario.initialize();
    let monitors = scenario.monitors();
    let requirement_ids = scenario.requirement_ids();
    let mut faults = scenario.faults();
    faults.sort_by(|a, b| a.at_s.total_cmp(&b.at_s));
    let mut next_fault = 0;

    let mut stats: Vec<MonitorState> = monitors.iter().map(|_| MonitorState::new()).collect();
    let mut trace = Trace::default();
    let mut last_mode: Option<String> = None;
    let eps = 1e-9 * dt;

    let mut k = 0usize;
    let mut t = 0.0;
    loop {
        while next_fault < faults.len() && faults[next_fault].at_s <= t + eps {
            let f = &mut faults[next_fault];
            (f.apply)(scenario);
            trace.events.push(TraceEvent {
                t_s: t,
                kind: EventKind::Fault,
                name: f.name.clone(),
            });
            next_fault += 1;
        }
        trace.record(t, scenario.signals());
        note_mode(scenario, t, &mut last_mode, &mut trace);
        for (m, st) in monitors.iter().zip(stats.iter_mut()) {
            if m.sampling == Sampling::EveryStep {
                st.sample(m, scenario, t);
            }
        }
        if k >= max_steps || scenario.is_done(t) {
            break;
        }
        scenario.step(t, dt);
        k += 1;
        t = k as f64 * dt;
    }

    scenario.finalize(t);
    note_mode(scenario, t, &mut last_mode, &mut trace);

    let name = scenario.name().to_string();
    let verdicts: Vec<Verdict> = monitors
        .iter()
        .zip(stats.iter_mut())
        .map(|(m, st)| {
            if m.sampling == Sampling::AtEnd {
                st.sample(m, scenario, t);
            }
            Verdict {
                scenario: name.clone(),
                requirement_id: m.requirement_id.clone(),
                requirement_name: m.requirement_name.clone(),
                satisfied_by: m.satisfied_by.clone(),
                pass: st.pass,
                sampling: m.sampling,
                samples: st.samples,
                first_violation_s: st.first_violation_s,
                min_margin: st.min_margin,
                message: if st.pass {
                    m.pass_message.clone()
                } else {
                    m.fail_message.clone()
                },
                evidence: m
                    .evidence
                    .as_ref()
                    .map_or(serde_json::Value::Null, |e| e(scenario)),
            }
        })
        .collect();

    RunResult {
        scenario: name,
        requirement_ids,
        dt_s: dt,
        steps: k,
        end_time_s: t,
        stopped_early: k < max_steps,
        pass: verdicts.iter().all(|v| v.pass),
        verdicts,
        trace,
    }
}

fn note_mode<S: Scenario>(s: &S, t: f64, last: &mut Option<String>, trace: &mut Trace) {
    if let Some(mode) = s.mode() {
        if last.as_deref() != Some(mode) {
            trace.events.push(TraceEvent {
                t_s: t,
                kind: EventKind::Mode,
                name: mode.to_string(),
            });
            *last = Some(mode.to_string());
        }
    }
}

#[cfg(test)]
mod tests;
