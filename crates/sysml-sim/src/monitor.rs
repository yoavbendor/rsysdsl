//! Requirement monitors sampled by the runner, and the verdicts they produce.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sysml_ir::EvaluableRequirement;

/// When a monitor is sampled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sampling {
    /// At `t = 0` and after every fixed step (invariants: "at all times ...").
    EveryStep,
    /// Once, after `Scenario::finalize` (terminal conditions: "at touchdown ...").
    AtEnd,
}

type Check<S> = Box<dyn Fn(&S) -> bool>;
type Margin<S> = Box<dyn Fn(&S) -> f64>;
type Evidence<S> = Box<dyn Fn(&S) -> Value>;

/// A requirement check over the scenario state `S`.
///
/// Build one from an arbitrary predicate ([`Monitor::new`]) or from a model's
/// macro-generated requirement ([`Monitor::requirement`]), which calls
/// `EvaluableRequirement::evaluate()` so the predicate is never duplicated.
pub struct Monitor<S> {
    pub(crate) requirement_id: String,
    pub(crate) requirement_name: String,
    pub(crate) satisfied_by: String,
    pub(crate) pass_message: String,
    pub(crate) fail_message: String,
    pub(crate) sampling: Sampling,
    pub(crate) check: Check<S>,
    pub(crate) margin: Option<Margin<S>>,
    pub(crate) evidence: Option<Evidence<S>>,
    pub(crate) evidence_at_violation: bool,
}

impl<S: 'static> Monitor<S> {
    /// Monitor from any closure returning `true` while the requirement holds.
    pub fn new(requirement_id: impl Into<String>, check: impl Fn(&S) -> bool + 'static) -> Self {
        let id = requirement_id.into();
        Self {
            requirement_name: id.clone(),
            requirement_id: id,
            satisfied_by: String::new(),
            pass_message: "requirement held".to_string(),
            fail_message: "requirement violated".to_string(),
            sampling: Sampling::EveryStep,
            check: Box::new(check),
            margin: None,
            evidence: None,
            evidence_at_violation: false,
        }
    }

    /// Monitor that instantiates a model requirement from the current state and calls
    /// its generated `evaluate()`. ID and name come from `R::REQ_ID` / `R::SYSML_NAME`.
    /// Also requires `R::evidence_populated()` so unpopulated `NaN` evidence fields fail.
    pub fn requirement<R: EvaluableRequirement>(build: impl Fn(&S) -> R + 'static) -> Self {
        let mut m = Self::new(R::REQ_ID, move |s| {
            let r = build(s);
            r.evidence_populated() && r.evaluate()
        });
        m.requirement_name = R::SYSML_NAME.to_string();
        m
    }

    /// Sample only once at the end of the run instead of every step.
    pub fn at_end(mut self) -> Self {
        self.sampling = Sampling::AtEnd;
        self
    }

    /// Signed margin (`>= 0` means inside the limit); the minimum over all samples is reported.
    pub fn with_margin(mut self, margin: impl Fn(&S) -> f64 + 'static) -> Self {
        self.margin = Some(Box::new(margin));
        self
    }

    /// Evidence JSON, captured at the end of the run (or at the first failing sample if
    /// [`Monitor::evidence_at_violation`] is also set).
    pub fn with_evidence(mut self, evidence: impl Fn(&S) -> Value + 'static) -> Self {
        self.evidence = Some(Box::new(evidence));
        self
    }

    /// Capture [`Monitor::with_evidence`] at the first failing sample instead of at the end
    /// of the run (useful for `EveryStep` invariants whose state recovers or changes later).
    pub fn evidence_at_violation(mut self) -> Self {
        self.evidence_at_violation = true;
        self
    }

    /// Model element path that claims to satisfy this requirement (for reports).
    pub fn satisfied_by(mut self, path: impl Into<String>) -> Self {
        self.satisfied_by = path.into();
        self
    }

    /// Human-readable pass / fail messages (for reports).
    pub fn messages(mut self, pass: impl Into<String>, fail: impl Into<String>) -> Self {
        self.pass_message = pass.into();
        self.fail_message = fail.into();
        self
    }

    pub fn requirement_id(&self) -> &str {
        &self.requirement_id
    }
}

/// Running statistics of one monitor during a run.
pub(crate) struct MonitorState {
    pub samples: usize,
    pub pass: bool,
    pub first_violation_s: Option<f64>,
    pub first_violation_evidence: Option<Value>,
    pub min_margin: Option<f64>,
}

impl MonitorState {
    pub fn new() -> Self {
        Self {
            samples: 0,
            pass: true,
            first_violation_s: None,
            first_violation_evidence: None,
            min_margin: None,
        }
    }

    pub fn sample<S>(&mut self, m: &Monitor<S>, s: &S, t: f64) {
        self.samples += 1;
        if !(m.check)(s) {
            self.pass = false;
            if self.first_violation_s.is_none() {
                self.first_violation_s = Some(t);
                if m.evidence_at_violation {
                    if let Some(ev) = &m.evidence {
                        self.first_violation_evidence = Some(ev(s));
                    }
                }
            }
        }
        if let Some(margin) = &m.margin {
            let v = margin(s);
            self.min_margin = Some(self.min_margin.map_or(v, |cur| cur.min(v)));
        }
    }
}

/// Outcome of one requirement monitor over one scenario run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Verdict {
    pub scenario: String,
    pub requirement_id: String,
    pub requirement_name: String,
    pub satisfied_by: String,
    pub pass: bool,
    pub sampling: Sampling,
    pub samples: usize,
    /// Simulation time of the first failing sample, if any.
    pub first_violation_s: Option<f64>,
    /// Minimum margin over all samples, when a margin closure was given.
    pub min_margin: Option<f64>,
    pub message: String,
    pub evidence: Value,
}
