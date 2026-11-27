use super::*;
use serde_json::json;
use sysml_ir::{EvaluableRequirement, SysmlRequirementCore};

/// dx/dt = -x, x(0) = 1, integrated with RK4; mode switches when x crosses 0.5.
struct Decay {
    x: [f64; 1],
    dt: f64,
    crossing: ZeroCrossing,
    crossed_at: Option<f64>,
    mode: &'static str,
}

impl Decay {
    fn new(dt: f64) -> Self {
        Self {
            x: [1.0],
            dt,
            crossing: ZeroCrossing::new(Crossing::Falling),
            crossed_at: None,
            mode: "high",
        }
    }
}

impl Scenario for Decay {
    fn name(&self) -> &str {
        "decay"
    }
    fn dt(&self) -> f64 {
        self.dt
    }
    fn duration(&self) -> f64 {
        5.0
    }
    fn initialize(&mut self) {
        self.crossing.update(0.0, self.x[0] - 0.5);
    }
    fn step(&mut self, t: f64, dt: f64) {
        self.x = rk4_step(|_t, x: &[f64; 1]| [-x[0]], t, &self.x, dt);
        if let Some(tc) = self.crossing.update(t + dt, self.x[0] - 0.5) {
            self.crossed_at = Some(tc);
            self.mode = "low";
        }
    }
    fn signals(&self) -> Vec<(&'static str, f64)> {
        vec![("x", self.x[0])]
    }
    fn mode(&self) -> Option<&str> {
        Some(self.mode)
    }
    fn monitors(&self) -> Vec<Monitor<Self>> {
        vec![Monitor::new("DECAY-001", |s: &Decay| s.x[0] > 0.0)
            .with_margin(|s: &Decay| s.x[0])
            .with_evidence(|s: &Decay| json!({ "x_end": s.x[0] }))]
    }
}

#[test]
fn rk4_matches_exponential_decay_within_1e_6() {
    // Stated tolerance: |x_rk4(t) - exp(-t)| < 1e-6 for t in [0, 5] with dt = 0.1
    // (RK4 global error is O(dt^4); measured max error is ~1.4e-7).
    let mut s = Decay::new(0.1);
    let r = run(&mut s);
    assert_eq!(r.steps, 50);
    let xs = r.trace.signal("x").unwrap();
    let max_err = r
        .trace
        .time_s
        .iter()
        .zip(xs)
        .map(|(t, x)| (x - (-t).exp()).abs())
        .fold(0.0, f64::max);
    assert!(max_err < 1e-6, "max error {max_err}");

    // Fourth-order convergence: halving dt shrinks the end-point error ~16x.
    let err_at_end = |dt: f64| {
        let mut s = Decay::new(dt);
        run(&mut s);
        (s.x[0] - (-5.0f64).exp()).abs()
    };
    let ratio = err_at_end(0.2) / err_at_end(0.1);
    assert!((12.0..20.0).contains(&ratio), "convergence ratio {ratio}");
}

#[test]
fn zero_crossing_fires_mode_change_mid_run() {
    let mut s = Decay::new(0.1);
    let r = run(&mut s);
    let tc = s.crossed_at.expect("x crosses 0.5");
    assert!((tc - std::f64::consts::LN_2).abs() < 0.01, "crossing at {tc}");
    let modes: Vec<_> = r
        .trace
        .events
        .iter()
        .filter(|e| e.kind == EventKind::Mode)
        .map(|e| (e.name.as_str(), e.t_s))
        .collect();
    assert_eq!(modes.len(), 2);
    assert_eq!(modes[0], ("high", 0.0));
    assert_eq!(modes[1].0, "low");
    assert!((modes[1].1 - 0.7).abs() < 1e-12, "first sample after ln 2");
    let v = r.verdict("DECAY-001").unwrap();
    assert!(v.pass);
    assert_eq!(v.samples, 51);
    assert_eq!(v.min_margin, Some(xs_last(&r)));
}

fn xs_last(r: &RunResult) -> f64 {
    *r.trace.signal("x").unwrap().last().unwrap()
}

/// x(t) = t ramp with a fault that freezes it; monitors check x <= 0.35.
struct Ramp {
    x: f64,
    frozen: bool,
}

struct RampLimit {
    actual: f64,
    limit: f64,
}
impl SysmlRequirementCore for RampLimit {
    const REQ_ID: &'static str = "RAMP-002";
    const SYSML_NAME: &'static str = "RampLimit";
}
impl EvaluableRequirement for RampLimit {
    fn evaluate(&self) -> bool {
        self.actual <= self.limit
    }
}

impl Scenario for Ramp {
    fn name(&self) -> &str {
        "ramp"
    }
    fn dt(&self) -> f64 {
        0.1
    }
    fn duration(&self) -> f64 {
        1.0
    }
    fn step(&mut self, t: f64, dt: f64) {
        if !self.frozen {
            self.x = t + dt;
        }
    }
    fn is_done(&self, _t: f64) -> bool {
        self.x > 0.85
    }
    fn signals(&self) -> Vec<(&'static str, f64)> {
        vec![("x", self.x)]
    }
    fn monitors(&self) -> Vec<Monitor<Self>> {
        vec![
            Monitor::new("RAMP-001", |s: &Ramp| s.x <= 0.35).with_margin(|s: &Ramp| 0.35 - s.x),
            Monitor::requirement(|s: &Ramp| RampLimit {
                actual: s.x,
                limit: 0.35,
            })
            .at_end()
            .satisfied_by("Ramp.x")
            .messages("ok", "too high"),
        ]
    }
}

#[test]
fn monitor_records_first_violation_at_the_right_step() {
    let mut s = Ramp {
        x: 0.0,
        frozen: false,
    };
    let r = run(&mut s);
    // x = 0.4 at k = 4 is the first sample above 0.35.
    let every = r.verdict("RAMP-001").unwrap();
    assert!(!every.pass);
    assert_eq!(every.first_violation_s, Some(0.4));
    // is_done stops the run once x > 0.85, i.e. at t = 0.9 (k = 9).
    assert!(r.stopped_early);
    assert_eq!(r.steps, 9);
    assert_eq!(every.samples, 10);
    assert!((every.min_margin.unwrap() - (0.35 - 0.9)).abs() < 1e-12);

    let end = r.verdict("RAMP-002").unwrap();
    assert_eq!(end.requirement_name, "RampLimit");
    assert_eq!(end.samples, 1);
    assert!(!end.pass);
    assert_eq!(end.message, "too high");
    assert!((end.first_violation_s.unwrap() - 0.9).abs() < 1e-12);

    let report = r.to_report("RampModel");
    assert_eq!(report.failed, 2);
    assert_eq!(report.verdicts[1].satisfied_by, "Ramp.x");
    assert_eq!(r.requirement_ids, vec!["RAMP-001", "RAMP-002"]);
}

#[test]
fn fault_schedule_applies_at_its_time() {
    struct Frozen(Ramp);
    impl Scenario for Frozen {
        fn name(&self) -> &str {
            "ramp_frozen"
        }
        fn dt(&self) -> f64 {
            0.1
        }
        fn duration(&self) -> f64 {
            1.0
        }
        fn step(&mut self, t: f64, dt: f64) {
            self.0.step(t, dt)
        }
        fn monitors(&self) -> Vec<Monitor<Self>> {
            vec![Monitor::new("RAMP-001", |s: &Frozen| s.0.x <= 0.35)]
        }
        fn faults(&self) -> Vec<Fault<Self>> {
            vec![Fault::at(0.3, "freeze", |s: &mut Frozen| s.0.frozen = true)]
        }
    }
    let mut s = Frozen(Ramp {
        x: 0.0,
        frozen: false,
    });
    let r = run(&mut s);
    assert!(r.pass, "x frozen at 0.3 never exceeds 0.35");
    assert!((s.0.x - 0.3).abs() < 1e-12);
    assert_eq!(r.steps, 10);
    assert_eq!(
        r.trace.events,
        vec![TraceEvent {
            t_s: 0.30000000000000004,
            kind: EventKind::Fault,
            name: "freeze".into()
        }]
    );
}

#[test]
fn runs_are_deterministic() {
    let a = run(&mut Decay::new(0.05)).to_json();
    let b = run(&mut Decay::new(0.05)).to_json();
    assert_eq!(a, b);
    let parsed: RunResult = serde_json::from_str(&a).unwrap();
    assert_eq!(parsed.verdicts.len(), 1);
}

#[test]
fn registry_finds_by_name_and_alias() {
    fn outcome() -> ScenarioOutcome {
        let run = super::run(&mut Decay::new(0.1));
        ScenarioOutcome {
            report: run.to_report("Decay"),
            run,
        }
    }
    let mut reg = Registry::new();
    reg.extend([RegisteredScenario {
        name: "decay_nominal",
        aliases: &["decay"],
        key: "decay",
        model: "Decay",
        description: "test",
        expect_pass: true,
        run: outcome,
    }]);
    let s = reg.find("decay").unwrap();
    assert_eq!(s.name, "decay_nominal");
    assert!(s.meets_expectation(&(s.run)()));
    assert!(reg.find("nope").is_none());
}
