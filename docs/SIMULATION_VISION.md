# Tomorrow's Vision: Multi-Domain System Simulation in `rsysdsl`

> Once systems are authored as strongly typed Rust (`rsysdsl` + `sysml-std`), every model already carries **physical SI parameters**, **conserved typed flow ports**, **pure `#[sysml::calc]` equations**, **guarded `#[sysml::state]` machines**, and **executable `#[sysml::require_constraint]` checks**. Turning a static SysML v2 architecture into a live, multi-domain executable simulator becomes a natural compilation target rather than a separate rewrite.

---

## 1. Why `rsysdsl` Has an Unfair Advantage for Simulation

Traditional MBSE workflows suffer from the "two-model chasm": systems engineers draw diagrams in Cameo/Capella, while simulation engineers manually re-implement the physics in Simulink/Modelica/Python — and the two immediately drift apart.

In `rsysdsl`, **the architecture IS already compiled Rust**:
- **Parameters** (`#[sysml::attr(default = ..., unit = "...")]`) auto-generate `impl Default` with real SI values.
- **Formulas** (`#[sysml::calc]`) compile to zero-overhead Rust functions.
- **Modes & Safety Interlocks** (`#[sysml::state]` + `#[sysml::transition(guard = ...)]`) compile to deterministic `step_with(&subject, event)` state machines (even in `#![no_std]` firmware!).
- **Requirements** (`#[sysml::require_constraint(...)]`) compile to `evaluate(&self) -> bool` monitors that can run *at every simulation timestep*.
- **Connections** (`ElectricalLink`, `MechanicalLink`, `ThermalLink`, `RadiativeLink`, `FluidLink`, `AcousticLink`) already encode the exact energy/mass/information topology of the system.

---

## 2. Four Progressive Simulation Tiers

### Tier 1 — Steady-State Multi-Domain Budget & Variant Trade-Study Solver (`sysml-sim-steady`)
**Fastest win (~1 sprint):**
- Walk the `#[sysml::connect]` graph over `sysml-std` ports to automatically roll up:
  - **Electrical power budget** (mains/battery draw vs. every load & converter efficiency)
  - **Thermal budget** (`Q_total = sum(P_in * (1 - eta))`, steady-state `T_junction = T_ambient + Q * R_th`)
  - **Acoustic rollup** (`L_total = 10 * log10(sum(10^(L_i / 10)))` across motors, fans, pumps, transformers)
  - **Mass & fluid balance** (`sum(mass_kg)`, `sum(inflow) - sum(outflow)`)
- **Automated `#[sysml::variation]` Trade Studies:** Automatically instantiate every combination of `BatteryPackVariant` × `PowerToolVariant` (or `CanFdChannel` vs `UartChannel` vs `EthernetChannel`), evaluate all `EvaluableRequirement` predicates, and emit a Pareto trade-study table (`rsysdsl trade-study`).

### Tier 2 — Hybrid Co-Simulation: Continuous Physics + Discrete State Machines (`sysml-sim-hybrid`)
**Couples continuous time (`dt`) with discrete logic:**
- Each stateful physical component implements a clean `#![no_std]`-friendly `SimDynamics` trait in a companion `*_sim` crate (preserving strict zero-semantic-leakage inside `#[sysml::package]`!):
  ```rust
  pub trait SimComponent {
      type Inputs;
      type Outputs;
      fn step(&mut self, dt_s: f64, inputs: &Self::Inputs) -> Self::Outputs;
  }
  ```
- **Concrete scenarios across our 6 reference systems:**
  - **Solar System:** Symplectic Velocity-Verlet N-body orbital integrator -> real elliptical orbits, seasonal insolation variation, planetary equilibrium temperature curves.
  - **Earth Water Cycle:** Coupled 5-reservoir ODE (`dM_i/dt = inflow - outflow`, latent heat exchange) -> simulate +2 K warming perturbation and watch atmospheric vapor & runoff shift while verifying global mass conservation to `1e-12`.
  - **Bicycle:** Rider cadence + road grade profile -> speed vs. aero/rolling drag ODE + brake disc thermal spike during alpine descent -> verify `BIKE-003` brake fade margin.
  - **Power Tool Family:** Duty-cycle torque pulse train on `Battery18V2Ah` vs `Battery18V5Ah` -> battery SOC discharge, inverter/motor temperature rise, automatic `ThermalCutoff` transition when `T > T_max`.
  - **Dishwasher:** Full 90-minute wash cycle (`Fill -> WashHeat -> Rinse -> Dry -> Drain`) tracking tub water mass, water temperature (`C_w * dT/dt = P_heater - Q_loss`), pump power, acoustic dB profile over time, and mid-cycle `DoorOpenPause` / `FloodFault` recovery.
  - **Microwave:** 120-second reheating run tracking food temperature curve, magnetron thermal rise under cooling fan airflow, and a live fault injection at `t = 45 s` (door opened mid-cook -> dual interlock kills HV relay in `< 20 ms` -> RF power drops to `0 W` -> magnetron cools exponentially).

### Tier 3 — Bond-Graph Conservation & Acausal Connections (Modelica-in-Rust)
- Map `sysml-std::flows` onto **Paynter Bond Graph** power-conjugate `(effort, flow)` pairs so `effort * flow = Power [W]` is enforced across every domain:

| Domain | Effort (`e`) | Flow (`f`) | Product (`e * f`) |
| --- | --- | --- | --- |
| Electrical | Voltage `V` (`V`) | Current `I` (`A`) | `W` |
| Mechanical (Linear) | Force `F` (`N`) | Velocity `v` (`m/s`) | `W` |
| Mechanical (Rotary) | Torque `tau` (`N*m`) | Angular speed `omega` (`rad/s`) | `W` |
| Fluidic | Pressure `P` (`Pa`) | Volumetric flow `Q` (`m^3/s`) | `W` |
| Thermal | Temperature `T` (`K`) | Entropy flow `dS/dt` (`W/K`) | `W` |

- Adding a lightweight acausal solver (or exporting to **FMI 3.0** Functional Mock-up Units via Rust `cdylib`) lets `rsysdsl` models plug straight into industrial Simulink, OpenModelica, or HIL test benches!

### Tier 4 — Fault Injection, Monte Carlo & Interactive Browser Time-Series Playback
- **Automatic `@FMEA` Fault Injection:** Every `#[sysml::fmea(...)]` annotation on a part becomes a runnable fault scenario (e.g., *cooling fan seized*, *NTC thermistor stuck low*, *door switch welded closed*), verifying that `SafetyGoal` FTTI deadlines (`ftti_ms`) hold across worst-case Monte Carlo parameter sweeps (`±10%` resistance, ambient `263 K..323 K`).
- **Interactive Simulation Replay in `rsysdsl view`:**
  - `rsysdsl sim <model> --trace out.json` exports a deterministic time-series trace (`t`, node states, temperatures, power flows, active requirement pass/fail margins).
  - Embed a lightweight playback scrubber inside our standalone offline HTML viewer (`rsysdsl view --trace out.json -o sim.html`):
    - **Click any subsystem box** -> sparkline of its Power (`W`), Temperature (`K`), Noise (`dB`), and State Machine transitions.
    - **Live heatmap & animated flow thickness** on IBD arrows (watch electrical power turn into RF energy + waste heat in real time, and watch arrows turn red the instant a safety requirement margin shrinks!).

---

## 3. Recommended First Step Tomorrow

Start with **Tier 1 + Tier 2 MVP (`crates/sysml-sim`)**:
1. Define a minimal `#![no_std]`-compatible `SimClock` + `TraceRecorder` + `ScenarioVerdict` crate (~250 lines, zero heavy deps).
2. Run a live 120-second coupled thermal/interlock simulation on **`microwave`** and a 90-minute wash cycle on **`dishwasher`** + a **2Ah vs 5Ah** battery discharge comparison on **`power_tools_family`**.
3. Wire `--trace` playback into our offline HTML viewer and publish live simulation traces right alongside the SysML v2 diagrams on GitHub Pages!

---

## 4. Implemented Today: `crates/sysml-sim` (Stage S2)

A small, deterministic fixed-step runner (deps: `sysml-ir`, `serde`, `serde_json`; no ODE framework).
Three-tier rule: `model crate <- runner crate (e.g. apollo11-sim) -> sysml-sim`; models never depend on it.

```rust
pub trait Scenario: Sized + 'static {           // the struct IS the simulation state
    fn name(&self) -> &str;
    fn dt(&self) -> f64;                          // fixed step [s]; t_k = k * dt (no drift)
    fn duration(&self) -> f64;                    // max simulated time [s]
    fn requirement_ids(&self) -> Vec<String>;     // default: the monitors' IDs
    fn initialize(&mut self) {}                   // e.g. drive the mode machine to its start phase
    fn step(&mut self, t: f64, dt: f64);          // plant + controller; may call step_with(...)
    fn is_done(&self, t: f64) -> bool { false }   // early stop (touchdown, ...)
    fn finalize(&mut self, t: f64) {}             // terminal event (e.g. TouchdownTelemetry)
    fn signals(&self) -> Vec<(&'static str, f64)>;// recorded into the trace every sample
    fn mode(&self) -> Option<&str> { None }       // e.g. sysml_state_name(); changes -> trace events
    fn monitors(&self) -> Vec<Monitor<Self>>;
    fn faults(&self) -> Vec<Fault<Self>> { vec![] } // Fault::at(t_s, name, |s| ...)
}

pub fn run<S: Scenario>(s: &mut S) -> RunResult;   // trace + Vec<Verdict>; .to_json(), .to_report(model)
pub fn rk4_step<const N: usize>(f, t, x: &[f64; N], dt) -> [f64; N];
pub struct ZeroCrossing;                          // .update(t, g) -> Option<t_cross> (event detection)

Monitor::requirement(|s| MyReq { actual: .., ..Default::default() })  // calls EvaluableRequirement::evaluate()
    .at_end()                    // or sampled every step (default)
    .with_margin(|s| limit - actual)
    .with_evidence(|s| json!({..}))
    .satisfied_by("Path.to.part").messages("pass msg", "fail msg");
Monitor::new("REQ-ID", |s| predicate)             // any closure

pub struct Verdict { scenario, requirement_id, requirement_name, satisfied_by, pass, sampling,
                     samples, first_violation_s: Option<f64>, min_margin: Option<f64>, message, evidence }
```

Runner crates expose `pub fn scenarios() -> Vec<sysml_sim::RegisteredScenario>`; the CLI registers them:

```bash
rsysdsl verify --list                                   # registered scenarios (JSON)
rsysdsl verify --scenario nominal                       # name or alias; default: nominal
rsysdsl verify --scenario all                           # {"ok", "<key>": report, ...}
rsysdsl verify --scenario fault --trace target/fault.json   # full time-series trace + verdicts
```

Apollo 11 (`examples/apollo11_sim::PoweredDescentScenario`) is the first port; its verdict JSON is
byte-identical to the pre-port output. Limitations: one fixed `dt` per scenario (no adaptive
stepping), event times inside a step are interpolated by `ZeroCrossing` but the trace records the
step-end time, and evidence is captured at the end of the run.
