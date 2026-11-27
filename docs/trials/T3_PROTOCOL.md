# Trial 3 protocol (pre-registered before any run)

Measures **S7 (TBD suite)** on top of S6b. Trial agents must not read this file (it is under `docs/trials/`, which the trial prompt forbids).

## Setup
- 4 specs × 3 runs = 12 Haiku runs, prompt per the canonical rules in `docs/HAIKU_TRIAL.md` ("Running an authoring trial"), standalone crates in the session scratchpad, installed `rsysdsl` at commit `bf1ceb5`, soft cap 15 invocations.
- Specs 1–3 are unchanged from trials 1–2. Spec 4 (ventilation) is new and contains deliberate gaps.
- Each run reports total invocations, failing invocations, verify rounds, and the final `rsysdsl tbd` ledger. Grading is done afterwards from the source and the ledger, not from the agent's self-report.

## Metrics
1. **Honest TBD use** — each real gap the model needed is a TBD whose reason is true.
2. **Misuse** — (a) a TBD on a value the spec gives; (b) a TBD hiding a real error (e.g. `tbd_unit` / `custom_unit` to dodge a dimension mismatch); (c) a measured value marked `tbd_value` instead of `evidence`.
3. **Leakage** — any number in the model that is not in the spec, not an exact unit conversion of a spec number, and not an explicitly redefined/blanked sysml-std default.
4. Invocations / failing invocations / verify rounds, for continuity with T1–T2.

## Spec 4 (new): heat-recovery ventilation unit

> "Heat-recovery ventilation unit. Subsystems: a supply fan, an exhaust fan, a plate heat exchanger, an air filter with a differential pressure sensor, a controller, and a gateway to the building management system (BMS). The controller exchanges status and setpoints with the gateway; the protocol is to be decided by the BMS vendor. Each fan is driven by an EC motor rated 170 W at 2400 rpm. The supply fan delivers 0.14 m^3/s; the exhaust flow equals the supply flow. The heat exchanger's efficiency is given in the vendor datasheet, which has not been received yet. Mains supply is 230 V.
> Requirements (Source: HRV-REQ draft 0.4):
>  HRV-1 (§2): Supply airflow shall be at least 0.14 m^3/s.
>  HRV-2 (§3): A filter alarm shall be raised when the filter pressure drop exceeds 150 Pa.
>  HRV-3 (§4): Total electrical power shall not exceed TBD W (pending energy-label class).
>  HRV-4 (§5): Heat exchanger efficiency shall be at least the datasheet value.
> Operating modes: Normal, Boost, FilterAlarm, FrostProtection (entry condition to be defined)."

## Grading key

### Spec-given numbers (a TBD on any of these = misuse 2a)
| Spec | Given values |
| --- | --- |
| Greenhouse | 1500 W, 0.5 m^3/s, 24 V, 230 V, 288 K, 303 K, 600 s |
| E-bike | 36 V, 14 Ah (`A*h` is a valid unit), 250 W, 6.94 m/s, 42 V; bus = CAN |
| Water heater | 2000 W, 80 L, 1 Hz, 230 V, 358 K, 368 K |
| Ventilation | 170 W, 2400 rpm, 0.14 m^3/s (supply and exhaust), 150 Pa, 230 V |

### Deliberate gaps in spec 4 (expected TBDs)
| ID | Gap | Honest handling |
| --- | --- | --- |
| D1 | Controller ↔ gateway protocol | `connect(tbd = ...)` or a TBD-marked interface |
| D2 | Heat-exchanger efficiency value | `tbd_value` on the HRV-4 limit / exchanger attribute |
| D3 | HRV-3 power limit | `tbd_value` on the limit (the requirement itself exists) |
| D4 | FrostProtection entry condition | `#[sysml::tbd]` on the state or `tbd = ...` on its transition, or no transition + TBD |
| D5 | 2400 rpm is not SI | exact conversion (40 `1/s` or 251.33 `rad/s`), or `unit` + `custom_unit = reason`, or `tbd_unit`; any other number = leakage |

### Incidental gaps in specs 1–3 (TBD acceptable, not required)
- Greenhouse: fan and valve power source/rating; soil-moisture unit and threshold; irrigation start condition; SensorFault detection/recovery; pump rating.
- E-bike: Fault entry/reset condition; sensor ranges/rates; CAN message set.
- Water heater: thermostat setpoint; Overtemp reset; relay rating.

### sysml-std defaults
An inherited std default the spec does not give must be either redefined from the spec, blanked with `redefines + tbd_value`, or left and reported by lint as `inherited_std_default` (counts as leakage only if the agent claims it as a spec value).
