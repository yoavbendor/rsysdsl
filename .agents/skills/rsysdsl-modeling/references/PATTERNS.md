# `rsysdsl` Copy-Pasteable Rust Modeling Patterns

Every pattern below compiles with `rustc` and emits OMG-valid SysML v2 (0 errors, 0 warnings against `sysml.library`).

---

## Pattern 1: Visio Boxes & Arrows (IBD-Ready Architecture)

Use **one top-level context `#[sysml::part]`** composing all boxes and wiring every arrow with `#[sysml::connect]` so VS Code `sysml-reactflow` renders a connected Internal Block Diagram.

```rust
use sysml_dsl::prelude::*;

#[sysml::package(name = "VehicleArchitecture")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod vehicle_arch {
    use super::*;

    /// Directed signal payload (for a one-way arrow).
    #[sysml::item]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct SpeedSignal {
        #[sysml::attr(default = 0.0, unit = "m/s")]
        pub velocity: Quantity<f64>,
    }

    /// Directed port: producer uses `SpeedPort`, consumer uses `Conjugated<SpeedPort>`.
    #[sysml::port]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct SpeedPort {
        #[sysml::out_item]
        pub signal: SpeedSignal,
    }

    /// Bidirectional bus port (CAN / Ethernet / mechanical coupling): connects `CanPort` <-> `CanPort`.
    #[sysml::port(symmetric)]
    #[derive(Debug, Clone, Copy, Default)]
    pub struct CanBusPort {
        #[sysml::inout_item]
        pub frame: SpeedSignal,
    }

    #[sysml::interface]
    pub struct SpeedLink {
        #[sysml::end]
        pub tx: SpeedPort,
        #[sysml::end]
        pub rx: Conjugated<SpeedPort>,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct WheelSensor {
        #[sysml::port]
        pub speed_out: SpeedPort,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Ecu {
        #[sysml::port]
        pub speed_in: Conjugated<SpeedPort>,
        #[sysml::port]
        pub can: CanBusPort,
    }

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct TelemetryGateway {
        #[sysml::port]
        pub can: CanBusPort,
    }

    /// Top-level Visio context assembly.
    #[sysml::part]
    #[sysml::connect(interface = SpeedLink, name = "wheelToEcu", from = wheel.speed_out, to = ecu.speed_in)]
    #[sysml::connect(name = "ecuToGateway", from = ecu.can, to = gateway.can)]
    #[derive(Debug, Clone, Copy)]
    pub struct VehicleContext {
        #[sysml::part]
        pub wheel: WheelSensor,
        #[sysml::part]
        pub ecu: Ecu,
        #[sysml::part]
        pub gateway: TelemetryGateway,
    }
}
```

---

## Pattern 2: Requirements, Provenance & Compiled Constraints

Record document provenance in `#[sysml::rationale("Source: ...")]` (requires `#[sysml::import("ModelingMetadata::*")]`) and express quantitative bounds in `#[sysml::require_constraint(...)]` (compiles into both SysML `require constraint` and Rust `EvaluableRequirement::evaluate(&self)`).

```rust
#[sysml::package(name = "Requirements")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
#[sysml::import("ModelingMetadata::*")]
pub mod requirements {
    use super::*;

    /// The power supply shall deliver at least 120 W continuously.
    #[sysml::requirement(id = "PWR-001")]
    #[sysml::rationale("Source: Power Budget Spec Rev 2.1, Section 4.3, Table 2")]
    #[sysml::require_constraint(delivered_power_w >= min_power_w)]
    #[derive(Debug, Clone, Copy)]
    pub struct MinimumPowerOutput {
        #[sysml::attr(default = 120.0, unit = "W")]
        pub min_power_w: Quantity<f64>,
        #[sysml::attr(default = 150.0, unit = "W")]
        pub delivered_power_w: Quantity<f64>,
    }
}

// Satisfy on the realizing part (compile-time checked ID!).
// If the requirement lives in another package, the satisfying package ALSO needs
// `#[sysml::import("Requirements::*")]` (the SysML package name), otherwise rustc passes
// but the strict audit fails with `unresolved reference 'MinimumPowerOutput'`.
// #[sysml::part]
// #[sysml::satisfy(req = requirements::MinimumPowerOutput, id = "PWR-001")]
// pub struct PowerSupplyUnit { ... }
```

---

## Pattern 3: Functional Actions & `perform action` Allocation

```rust
#[sysml::action]
pub struct MeasureVelocity;

#[sysml::action]
pub struct ComputeControlLoop;

#[sysml::part]
#[sysml::perform(measure: MeasureVelocity)]
#[sysml::perform(compute: ComputeControlLoop)]
#[derive(Debug, Clone, Copy)]
pub struct ControlModule {
    #[sysml::attr(default = 0.01, unit = "s")]
    pub cycle_time: Quantity<f64>,
}
```

---

## Pattern 4: Executable Guarded State Machine

Guards reference `event.<field>` (from the `on = EventItem` struct) and `subject.<field>` (from the `subject = Part` struct). The exact same guard emits SysML `if ...` AND runs in `mode.step_with(&event, &subject)`.

```rust
#[sysml::item]
#[derive(Debug, Clone, Copy, Default)]
pub struct ThermalAlert {
    #[sysml::attr(default = 0.0, unit = "K")]
    pub measured_temp: f64,
}

#[sysml::part]
#[derive(Debug, Clone, Copy)]
pub struct ThermalPlant {
    #[sysml::attr(default = 350.0, unit = "K")]
    pub shutdown_temp: Quantity<f64>,
}

#[sysml::state(subject = ThermalPlant)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlantMode {
    #[sysml::initial]
    #[sysml::transition(
        on = ThermalAlert,
        guard = event.measured_temp > subject.shutdown_temp,
        to = SafeShutdown
    )]
    Operating,
    SafeShutdown,
}

// Driving it (tests or a separate sim crate). `step_with` takes `&mut self` and returns
// `Result<PlantMode, NoStdTransitionError>`: `Err` means no transition matched the event.
#[cfg(test)]
#[test]
fn plant_shuts_down_on_overtemp() {
    let plant = ThermalPlant::default();
    let mut mode = PlantMode::Operating;
    let alert = ThermalAlert { measured_temp: 360.0 };
    assert_eq!(mode.step_with(&alert, &plant), Ok(PlantMode::SafeShutdown));
}
```

---

## Pattern 5: Pure Modeled Calculations (`#[sysml::calc]`)

Allowed inside `#[sysml::package]`: pure arithmetic (`+`, `-`, `*`, `/`), `let` bindings, `if/else` expressions, and calls to other `#[sysml::calc]` functions. Opaque Rust functions fail `rustc`.

```rust
#[sysml::calc]
pub fn power_margin_w(available_w: f64, required_w: f64) -> f64 {
    let margin = available_w - required_w;
    margin
}
```

---

## Pattern 6: ISO 26262 Safety Goals, FMEA & ASIL Decomposition

When using ISO 26262 annotations, include `sysml_emit::iso26262_metadata_package()` in `Model::new(...)` and `#[sysml::import("ISO26262Metadata::*")]` on the package so OMG stdlib validation passes with 0 warnings.

```rust
#[sysml::requirement(id = "SG-001")]
#[sysml::safety_goal(asil = AsilD, safe_state = "ValvesClosed", ftti_ms = 50)]
#[sysml::require_constraint(cutoff_s <= ftti_s)]
#[derive(Debug, Clone, Copy)]
pub struct PreventUncommandedThrust {
    #[sysml::attr(default = 0.050, unit = "s")]
    pub ftti_s: Quantity<f64>,
    #[sysml::attr(default = 0.018, unit = "s")]
    pub cutoff_s: Quantity<f64>,
}

#[sysml::part]
#[sysml::independent(MonitorChannel)]
#[sysml::fmea(
    failure_mode = "SpuriousThrottleHigh",
    effect = "UncommandedThrust",
    severity = 10,
    occurrence = 2,
    detection = 3,
    asil = AsilB
)]
#[derive(Debug, Clone, Copy)]
pub struct PrimaryChannel {
    #[sysml::attr(default = 0.012, unit = "s")]
    pub latency: Quantity<f64>,
}

#[sysml::part]
#[sysml::independent(PrimaryChannel)]
#[derive(Debug, Clone, Copy)]
pub struct MonitorChannel {
    #[sysml::attr(default = 0.006, unit = "s")]
    pub latency: Quantity<f64>,
}

/// `rustc` enforces BOTH ISO 26262-9 Table 1 (D -> B + B) AND mutual `#[sysml::independent]`.
#[sysml::part]
#[sysml::satisfy(req = PreventUncommandedThrust, id = "SG-001")]
#[sysml::asil_decompose(
    goal = PreventUncommandedThrust,
    left = PrimaryChannel,
    left_asil = AsilB,
    right = MonitorChannel,
    right_asil = AsilB
)]
#[derive(Debug, Clone, Copy)]
pub struct SafetyController {
    #[sysml::part]
    pub primary: PrimaryChannel,
    #[sysml::part]
    pub monitor: MonitorChannel,
}
```

---

## Pattern 7: Bare-Metal Ferrocene `#![no_std]` Model

Add `no_std` to `#[sysml::package(no_std, name = "...")]`, set `sysml-dsl = { path = "../../crates/sysml-dsl", default-features = false }` in `Cargo.toml` with `reflect = ["sysml-dsl/reflect"]`, and gate `extern crate std;` behind `#[cfg(any(test, feature = "reflect"))]`. See [`examples/embedded_flight_controller/`](../../../examples/embedded_flight_controller/) for a complete working example.
