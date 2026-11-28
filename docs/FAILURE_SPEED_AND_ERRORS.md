# Failure Speed & Error Quality Audit (`rsysdsl`)

This report benchmarks `rsysdsl` at **500-element** and **2,000-element** scale and audits both **compile-time macro checks (`rustc`)** and **fast in-memory IR validation (`validate_ir`)** + **external Open-MBEE OMG Standard Library validation**.

All timings were measured on this Linux devbox (`rustup run 1.97.0`, debug profile, warm Cargo cache) using:
- `crates/sysml-emit/tests/scale_check.rs` (510 and 2,040 elements via `SwHwSystem` + `validate_ir` + `emit_model` + `audit_model_with_stdlib`)
- A synthetic standalone macro crate (100 and 400 component groups = ~500 and ~2,000 elements in a single `#[sysml::package]` module using `#[sysml::part]`, `#[sysml::connect]`, `#[sysml::satisfy]`, `#[sysml::require_constraint]`).

---

## 1. Measured Timings (500 vs. 2,000 Elements)

| Pipeline Stage | ~500 Elements (510 IR / 100 macro groups) | ~2,000 Elements (2,040 IR / 400 macro groups) | Status (3 s budget) |
| --- | --- | --- | --- |
| **In-memory IR validation (`validate_ir`, clean — debug / release)** | **0.74 ms** / **0.24 ms** | **2.81 ms** / **0.83 ms** | PASS (< 1 ms release) |
| **In-memory IR validation (`validate_ir`, 1 error — debug / release)** | **0.58 ms** / **0.12 ms** | **2.96 ms** / **0.61 ms** | PASS (< 1 ms release) |
| **IR Lowering + `.sysml` emission (debug / release)** | **16.3 ms** / **7.1 ms** (`76.6 KB`) | **59.9 ms** / **12.6 ms** (`306.3 KB`) | PASS (< 15 ms release) |
| **Open-MBEE + OMG Standard Library validation (`--release`)** | **0.77 s** (`770 ms`) | **1.03 s** (`1,031 ms`) | **PASS (~9× faster than debug)** |
| **Open-MBEE + OMG Standard Library validation (debug)** | **7.02 s** | **8.90 s** | **SLOW (> 3 s in debug only)** |
| **Macro crate incremental `cargo check` (clean)** | **1.60 s** | **8.14 s** | **SLOW (> 3 s at 2k)** |
| **Macro crate incremental `cargo check` (`bad_port`)** | **1.58 s** | **6.17 s** | **SLOW (> 3 s at 2k)** |
| **Macro crate incremental `cargo check` (`missing_req`)** | **2.29 s** | **9.11 s** | **SLOW (> 3 s at 2k)** |
| **Macro crate incremental `cargo check` (`unit_mismatch`)** | **1.57 s** | **6.37 s** | **SLOW (> 3 s at 2k)** |

### Key Bottlenecks Flagged (> 3 s)

1. **Monolithic `#[sysml::package]` proc-macro expansion (`6.17 s – 9.11 s` at 2,000 elements):**
   - `#[sysml::package]` parses and re-emits the entire module's token stream in a single proc-macro invocation. Changing a single line inside a 400-part package forces `rustc` to re-expand all 400 `#[sysml::part]`, `#[sysml::requirement]`, and `#[sysml::connect]` macros and re-typecheck every generated `const _: fn()` closure.
   - **Root cause:** Encoding cross-element connections, allocations, and large assemblies in `#[sysml::part]` + `#[sysml::package]` proc-macros scales super-linearly with module size.
   - **Mitigation:** Keep `rustc` lightweight (plain Rust structs/builders like `SwHwSystem` or per-subsystem crates of ~100–300 elements) and run architectural graph validation in `validate_ir` (**0.83 ms** release / **2.81 ms** debug at 2,040 elements — **> 3,000× faster** than incremental `cargo check`).

2. **Open-MBEE + OMG Standard Library loading (`7.02 s – 8.90 s` in debug → `0.77 s – 1.03 s` in `--release`):**
   - In debug builds, ~6.5 s is a fixed one-time cost of parsing the ~90 OMG `sysml.library` files.
   - **Verified fix (`--release`):** Compiling with `--release` (which is how `rsysdsl` is installed via `cargo install --path crates/sysml-cli`) cuts full OMG Standard Library validation by **~9×**, down to **0.77 s** at 510 elements and **1.03 s** at 2,040 elements!

---

## 2. Exact Verbatim Error Outputs (3 Intentional Mistakes)

### Mistake 1: Broken Port Connection / Polarity Mismatch

#### A. Macro `#[sysml::connect]` (`rustc`, 1.58 s @ 500 elem / 6.17 s @ 2k elem) — **UNREADABLE / BAD SPAN**
```text
error[E0308]: mismatched types
    --> src/lib.rs:1519:5
     |
1519 |     #[sysml::part]
     |     ^^^^^^^^^^^^^^
     |     |
     |     expected `&Conjugated<SigPort>`, found `&SigPort`
     |     arguments to this function are incorrect
     |
     = note: expected reference `&sysml_dsl::Conjugated<SigPort>`
                found reference `&SigPort`
note: function defined here
    --> /home/antigravity/yoavbendor/rsysdsl/crates/sysml-core/src/traits.rs:184:14
     |
 184 | pub const fn assert_interface_connect_core<I>(_a: &I::EndA, _b: &I::EndB)
     |              ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
     = note: this error originates in the attribute macro `sysml::part` (in Nightly builds, run with -Z macro-backtrace for more info)
```
**Critique:** **Flagged as hard to follow.** On a 400-connection assembly, `rustc` points to `#[sysml::part]` on the outer struct rather than the offending `#[sysml::connect(name = "l0001", ...)]` attribute, leaks the internal helper `assert_interface_connect_core<I>`, and never names the connection or field path.

#### B. Runtime IR Validator (`validate_ir`, 0.58 ms @ 510 elem / 2.96 ms @ 2,040 elem) — **CRISP & ACTIONABLE**
```text
error[E001]: port payload type mismatch on `perceptionToMonitor`: `PerceptionAI` emits `RawLidarPointCloud` but `BrakeSafetyMonitor` expects `ObstacleList` (PerceptionAI.obstaclesOut (RawLidarPointCloud) -> BrakeSafetyMonitor.obstaclesIn (ObstacleList))
  element: perceptionToMonitor
  fix: align payload types on `PerceptionAI.obstaclesOut` and `BrakeSafetyMonitor.obstaclesIn` or insert an explicit adapter partition
```

---

### Mistake 2: Missing / Dangling Requirement Target

#### A. Macro `#[sysml::satisfy]` (`rustc`, 2.29 s @ 500 elem / 9.11 s @ 2k elem) — **READABLE, SLOW AT 2K**
```text
error[E0425]: cannot find type `MissingReq9999` in this scope
  --> src/lib.rs:27:28
   |
27 |     #[sysml::satisfy(req = MissingReq9999, id = "REQ-9999")]
   |                            ^^^^^^^^^^^^^^ not found in this scope
```
**Critique:** Clear span and message, but **9.11 s** incremental turnaround at 2,000 elements makes iterative editing sluggish.

#### B. Runtime IR Validator (`validate_ir`, < 3 ms @ 2,040 elem) — **CRISP & ACTIONABLE**
```text
error[E002]: partition `BrakeSafetyMonitor` satisfies unknown requirement `REQ-NONEXISTENT-99` (BrakeSafetyMonitor.satisfies)
  element: BrakeSafetyMonitor
  fix: declare `SwRequirement::new("REQ-NONEXISTENT-99", ...)` or fix the requirement ID on `BrakeSafetyMonitor`
```

---

### Mistake 3: Invalid Timing Budget / Dimensional Mismatch

#### A. Macro `#[sysml::require_constraint]` (`rustc`, 1.57 s @ 500 elem / 6.37 s @ 2k elem) — **READABLE, SLOW AT 2K**
```text
error: dimension mismatch in `<=`: `wcet_s` [s] has dimension s, but `max_power_w` [W] has dimension kg*m^2*s^-3 (both sides of comparisons, `+` and `-` must have the same dimension)
  --> src/lib.rs:20:33
   |
20 |     #[sysml::require_constraint(wcet_s <= max_power_w)]
   |                                 ^^^^^^^^^^^^^^^^^^^^^
```
**Critique:** Excellent local diagnostic (keep local dimensional checks!), but **6.37 s** at 2,000 elements when placed in a monolithic `#[sysml::package]` — and `rustc` cannot check cross-task CPU utilization (`sum(wcet_ms / period_ms) <= max_utilization`) across allocations.

#### B. Runtime IR Validator (`validate_ir`, < 3 ms @ 2,040 elem) — **CRISP, CROSS-ELEMENT & ACTIONABLE**
```text
error[E003]: task `BrakeSafetyMonitor.PlausibilityCheckTask` has wcet_ms (9 ms) > deadline_ms (5 ms) (BrakeSafetyMonitor.PlausibilityCheckTask (wcet=9 ms, deadline=5 ms))
  element: BrakeSafetyMonitor.PlausibilityCheckTask
  fix: reduce `wcet_ms` to <= 5 ms or relax `deadline_ms` on `BrakeSafetyMonitor.PlausibilityCheckTask`

error[E003]: compute node `SafetyMcu` utilization 150.0% exceeds limit 140.0% (2 core(s) x 70%) (SafetyMcu (used=1.500, limit=1.400 across 2 core(s)))
  element: SafetyMcu
  fix: offload a task from `SafetyMcu`, reduce task `wcet_ms`, or add cores
```

---

## 3. Architectural Takeaway

1. **Keep `rustc` lightweight:** Plain Rust structs/enums/builders (`SwHwSystem`, `SwPartition`, `SwTask`, `ComputeNode`, `BusChannel`, `SwAllocation`) give instant local type safety without proc-macro bloat or cryptic `assert_interface_connect_core` trait-bound spans.
2. **Run all graph/timing/safety rules in `validate_ir`:** Sub-millisecond at 500 elements and **< 3 ms** at 2,040 elements, with structured JSON + human-readable `code`, `element`, `source_hint`, `message`, and `suggested_fix`.
3. **Keep Open-MBEE + OMG stdlib validation as a CI / release-binary gate:** Emitted `.sysml` is 100% standard OMG SysML v2 and validates with 0 errors, without slowing down the inner edit loop.
