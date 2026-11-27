//! Fixed-step numerical helpers: classic RK4 and zero-crossing event detection.

/// One classic 4th-order Runge-Kutta step for `dx/dt = f(t, x)` on a fixed-size
/// state vector. Returns `x(t + dt)`; allocation-free and deterministic.
///
/// ```
/// use sysml_sim::rk4_step;
/// // dx/dt = -x, x(0) = 1
/// let x1 = rk4_step(|_t, x: &[f64; 1]| [-x[0]], 0.0, &[1.0], 0.1);
/// assert!((x1[0] - (-0.1f64).exp()).abs() < 1e-6);
/// ```
pub fn rk4_step<const N: usize, F>(f: F, t: f64, x: &[f64; N], dt: f64) -> [f64; N]
where
    F: Fn(f64, &[f64; N]) -> [f64; N],
{
    let offset = |x: &[f64; N], k: &[f64; N], h: f64| -> [f64; N] {
        let mut out = *x;
        for i in 0..N {
            out[i] += h * k[i];
        }
        out
    };
    let k1 = f(t, x);
    let k2 = f(t + 0.5 * dt, &offset(x, &k1, 0.5 * dt));
    let k3 = f(t + 0.5 * dt, &offset(x, &k2, 0.5 * dt));
    let k4 = f(t + dt, &offset(x, &k3, dt));
    let mut out = *x;
    for i in 0..N {
        out[i] += dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
    out
}

/// Which sign change of a guard function counts as an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crossing {
    /// `g` goes from `< 0` to `>= 0`.
    Rising,
    /// `g` goes from `> 0` to `<= 0`.
    Falling,
    /// Either direction.
    Either,
}

/// Event detector for a scalar guard `g(t)` sampled once per fixed step.
///
/// Use it inside `Scenario::step` to decide when a discrete event fires (e.g. feed a
/// typed event into the model's generated `step_with(&event, &subject)` the step the
/// altitude crosses zero). [`ZeroCrossing::update`] returns the linearly interpolated
/// crossing time inside the step, so evidence can report sub-step timing.
#[derive(Debug, Clone)]
pub struct ZeroCrossing {
    direction: Crossing,
    prev: Option<(f64, f64)>,
}

impl ZeroCrossing {
    pub fn new(direction: Crossing) -> Self {
        Self {
            direction,
            prev: None,
        }
    }

    /// Feed the guard value `g` at time `t`. Returns `Some(t_cross)` on the first sample
    /// after a sign change matching the configured direction.
    pub fn update(&mut self, t: f64, g: f64) -> Option<f64> {
        let hit = match self.prev {
            Some((t0, g0)) => {
                let rising = g0 < 0.0 && g >= 0.0;
                let falling = g0 > 0.0 && g <= 0.0;
                let fired = match self.direction {
                    Crossing::Rising => rising,
                    Crossing::Falling => falling,
                    Crossing::Either => rising || falling,
                };
                if fired {
                    let frac = if g == g0 { 1.0 } else { g0 / (g0 - g) };
                    Some(t0 + frac.clamp(0.0, 1.0) * (t - t0))
                } else {
                    None
                }
            }
            None => None,
        };
        self.prev = Some((t, g));
        hit
    }
}
