# Trial 3 grading (S7 TBD suite): 12 runs

Graded from each run's `src/lib.rs` (non-test part; line numbers below are file lines), checked against the emitted `target/*.sysml` (the `@TBD`/`TbdMetadata` count in every emitted file equals the source TBD count below, plus one import line). LOG.md was used only for invocation counts. Paths are relative to
`/tmp/claude-0/-home-user-rsysdsl/3a3c748d-8950-55d7-8040-3d0498a5a2e1/scratchpad/trial3/`.

Grading rules used:

- **Spec numbers:** from the key in `docs/trials/T3_PROTOCOL.md`. Where three independent runs agree on spec wording that the key does not list (e-bike "3-phase", water heater "Overtemp (cutoff opened)"), I treat it as spec text.
- **TBD channel (`connect(tbd)`) rule:** honest when the spec does not mention the link at all (e.g. battery pack to BMS). Borderline when the spec names the signal flow, the model already had a matching typed port pair, and the TBD only removes port checking.
- **Design parameter vs bound:** a TBD on a device setting where the spec gives only a bound (water heater cutoff trip point vs "opens at or below 368 K") is borderline, not 2a, because the 368 K limit stays on the requirement.
- **"Invented trigger unmarked":** an event or condition that the spec does not give, driving a transition with no `tbd` on the transition and no `#[sysml::tbd]` on the event item.
- **Leakage:** no run has a numeric literal in a `default`, guard or constraint outside categories (a)–(d). All numbers are listed per run so this can be checked.
- **Std defaults through ports and interfaces:** sysml-std item, port and interface defaults reached through std port or interface types (`ElectricalPowerPort` → `ElectricalEnergy` 12 V / 1 A; `ThermalPort` → `ThermalEnergy` 293.15 K; `AcousticPort` → 0.02 Pa / 1000 Hz; `ElectricalLink.transmissionEfficiency` 0.98) are listed separately. They are **not** reported by `lint inherited_std_default`, which covers part specialization only. Per the key they are not leakage, because no agent claims them as spec values. They are a tool gap.

---

## greenhouse_r1

**TBD inventory (5)**

| # | Line | Marker | Target | Reason (abridged) | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 39 | `tbd_unit` | `SoilMoistureReading.moisture` | no soil moisture unit/range | honest (incidental) |
| 2 | 200 | `#[tbd]` (preliminary) | `DcSupply24V` | 24 V supply input source not stated | honest |
| 3 | 285 | transition `tbd` | Normal → Irrigating on `IrrigationDemand` | soil trigger threshold not stated | honest (incidental) |
| 4 | 327 | `#[tbd]` (preliminary) | `VentilationFan` | fan power supply not stated | honest (incidental) |
| 5 | 339 | `#[tbd]` (preliminary) | `IrrigationPump` | pump/valve ratings not stated | honest (incidental) |

**Numbers in the model:** 288, 303 (135, 138, 246, 248), 600 (152, 250), 1500 (166, 317), 24 (203), 230 (213), 0.5 m^3/s (330). All are spec values. Conventions: none numeric. Leakage: 0.

**Std defaults:** no std part is specialized. Through ports, `ElectricalPowerPort` (types L102, used at 206, 216, 224, 234, 253, 320, 346) carries the `ElectricalEnergy` payload defaults 12 V / 1 A on both the 24 V and the 230 V supplies. Lint does not see these.

**Invented structure:**

- `IrrigationDemand` (L55) is marked (TBD transition, L282–286).
- **Unmarked:** SensorFault entry and recovery use an invented `sensor_ok: bool` field on `AirTempReading` (L32). The transitions are L271, L288, L295 (`!event.sensor_ok` → SensorFault) and L308 (`event.sensor_ok` → Normal). The key lists SensorFault detection/recovery as an incidental gap, so these 4 transitions should carry a `tbd`. They contain no number, so this is structural invention, not numeric leakage.

**LOG:**

- 8 invocations.
- Friction failures: 1 (#1 `cargo test`, missing test-module import).
- Excluded: #8 `--maturity preliminary` probe (deliberate).
- Verify rounds: 1.

---

## greenhouse_r2

**TBD inventory (8)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 33 | `tbd_unit` (+ `default = 0.0`) | `SoilMoistureReading.moisture` | no soil moisture unit | honest |
| 2 | 265 | `tbd_value` | `GreenhouseSensor.sample_rate` | sample rate not given | honest (self-introduced attribute) |
| 3 | 306 | `#[tbd]` (preliminary) | `VentilationFan` | fan supply/rating not given | honest |
| 4 | 320 | `#[tbd]` (preliminary) | `IrrigationPump` | pump run command path not given | honest |
| 5 | 326 | `tbd_value` | `IrrigationPump.flow_rate` | pump flow not given | honest |
| 6 | 350 | `tbd_value` | `DcPowerSupply24V.output_current` | 24 V supply current not given | honest (self-introduced) |
| 7 | 397 | transition `tbd` | Normal → Irrigating | soil trigger threshold | honest |
| 8 | 398 | transition `tbd` | Normal → SensorFault on `SensorDropout` | fault criteria not given | honest |

**Numbers:** 288/303/600/1500/0.5/24/230 (41, 49, 215, 217, 230, 243, 296, 312, 338, 348, 368, 371, 374). All are spec values. Leakage: 0.

**Conventions:** `default = 0.0` on item fields at L25 (K), L33 (moisture, unit TBD), L57 (W), L79 (s).

**Std defaults:** none. The run uses only its own ports and items.

**Invented structure:** `SensorDropout` (L86) is marked (L398). SensorFault is latched with no exit (L407, documented), so no invented recovery. Unmarked: none.

**LOG:**

- 11 invocations logged.
- Friction failures: 0.
- Excluded: #11 `--maturity preliminary` probe. Also a 12th call, a misrouted `cargo test` that exited 127 through `ventilation_r1`'s overwritten `trial3/logrun.sh`. It is recorded as `ventilation_r1` attempt 1; excluded here and noted.
- Verify rounds: 2.

---

## greenhouse_r3

**TBD inventory (10)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 39 | `tbd_unit` | `SoilMoistureReading.moisture` | no unit/range | honest |
| 2 | 340–343 | `tbd_value` | `VentilationFan.supply_voltage` | no fan supply source/voltage | honest |
| 3 | 354 | `tbd_value` | `IrrigationPump.rated_power_w` | no pump power | honest |
| 4–9 | 381, 383, 385, 387, 389–393, 395–399 | `redefines` + `tbd_value` | `MainsSupply` (`AcMainsSupply`): mass, ratedPower, efficiency, wasteHeat, maxCurrent, mainsFrequency | not in GH-CONOPS | blanked-std-default ×6 |
| 10 | 413–417 | transition `tbd` | Normal → Irrigating | no soil trigger threshold | honest |

**Numbers:** 288/303/600/1500/0.5/24 (206, 209, 223, 237, 279, 282, 285, 326, 338, 366). 230 is redefined from the std `nominalVoltage` (378). Leakage: 0.

**Conventions:** `default = 0.0` at L47 (HeaterCommand W) and L55 (FanCommand m^3/s).

**Std defaults:**

- All 7 `AcMainsSupply` part attributes are redefined or blanked. Lint is clean.
- Port level: `ElectricalPowerPort` payload (12 V / 1 A) on all power links (types L183–185, L369, L402).
- `MainsSupply` inherits `heat_out: ThermalPort` from `PowerElement`, which carries 293.15 K.

**Invented structure: unmarked.**

- `SensorFaultDetected` (L77) drives 4 transitions with no `tbd`: L418, L421, L424, L427.
- `SensorRecovered` (L82) drives SensorFault → Normal at L429, also with no `tbd`.
- The source says nothing about fault detection or recovery (incidental gap in the key). This is the same gap that greenhouse_r2 marked.

**LOG:**

- 11 invocations.
- Friction failures: 0.
- Verify rounds: 2. Round 2 came after the lint info `inherited_std_default` prompted blanking.

---

## ebike_r1

**TBD inventory (8)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 50 | `tbd_unit` | `CanFrame.state_of_charge` | no SoC unit/scale | honest (incidental) |
| 2 | 52 | `tbd_unit` | `CanFrame.assist_level` | no unit/levels | honest |
| 3 | 68–72 | `#[tbd]` (preliminary) | event item `FaultDetected` | Fault detection criteria not given | honest (incidental) |
| 4 | 276 | transition `tbd` | Off → Assist on `PowerOn` | no switch-on event | honest |
| 5 | 280 | transition `tbd` | Assist → Off on `PowerOff` | no switch-off event | honest |
| 6 | 284 | transition `tbd` | NoAssist → Off on `PowerOff` | same | honest |
| 7 | 286 | transition `tbd` | Fault → Off on `ResetFault` | no reset path | honest (incidental) |
| 8 | 361 | `connect(tbd)` | `packToBms` | pack-to-BMS interface not described | honest (see note) |

Note on 8: a typed `DcPowerLink` with exactly matching port types existed (L127). The spec apparently does not mention this link, so I graded it honest.

**Numbers:** 250 (186, 317), 6.94 (200, 298), 42 (214), 36 (251), 14 A*h (253). Leakage: 0. Conventions: none.

**Std defaults:** none (own ports only).

**Invented structure:** `PowerOn`, `PowerOff` and `ResetFault` transitions are marked. The `FaultDetected` item is TBD-marked, so its transitions (L279, L283) count as marked. Unmarked: none.

**LOG:**

- 13 invocations.
- Friction failures: 0.
- Excluded: #7 and #13 `--maturity preliminary` probes (exit 1 by design).
- Verify rounds: 2.

---

## ebike_r2

**TBD inventory (3)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 37–40 | `#[tbd]` | item `TorqueSignal` (no fields) | no torque unit/range | honest (incidental) |
| 2 | 54–57 | `#[tbd]` | item `CanFrame` (no fields) | no message IDs/payload | honest (incidental) |
| 3 | 353–358 | `connect(tbd)` | `packToBms` | link not specified | honest (typed `DcPowerLink` was available, L123) |

**Numbers:** 250 (182, 311), 6.94 (196, 273), 42 (210), 36 (247), 14 A*h (249). Leakage: 0.

**Conventions:** `default = 0.0` at L23 (V), L31 (W), L48 (m/s).

**Std defaults:** none.

**Invented structure: unmarked.**

- `PowerOnRequest` (L64): Off → Assist at L292.
- `PowerOffRequest` (L69): L295, L299.
- `FaultDetected` (L74): L296, L300.
- `FaultCleared` (L79): Fault → Off at L302.

That is 6 transitions on 4 invented events, none with a `tbd`. Fault entry/reset is an incidental gap in the key, and power-on/off are not in the spec. ebike_r1 and ebike_r3 marked these same transitions.

**LOG:**

- 6 invocations.
- Friction failures: 1 (#1 `cargo test`, test-module imports).
- Verify rounds: 1.

---

## ebike_r3

**TBD inventory (16)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 28 | `tbd_unit` | `CanFrame.state_of_charge` | no unit/range | honest |
| 2 | 30 | `tbd_unit` | `CanFrame.assist_level` | no scale | honest |
| 3 | 38 | `tbd_unit` | `PedalTorqueSignal.torque` | no torque unit | honest |
| 4–8 | 230–259 | `redefines` + `tbd_value` | `HubMotor` (`ElectricMotor` → `PhysicalComponent`): ratedRotationFreq, efficiency, mass, heatDissipation, noiseLevelDb | not in EB-SPEC | blanked-std-default ×5 |
| 9 | 299 | transition `tbd` | Off → Assist (`PowerOnRequest`) | no power-on trigger | honest |
| 10, 11 | 303, 308 | transition `tbd` | Assist/NoAssist → Off (`PowerOffRequest`) | no power-off trigger | honest |
| 12, 13 | 304, 309 | transition `tbd` | Assist/NoAssist → Fault (`FaultReport`) | no fault triggers | honest |
| 14 | 329–334 | `connect(tbd)` | `batteryToBms` | wiring not described | honest |
| 15 | 359–364 | `connect(tbd)` | `pedalTorqueToMotorController` | "names the signal but not its interface" | **borderline** |
| 16 | 365–370 | `connect(tbd)` | `wheelSpeedToMotorController` | same | **borderline** |

Why 15–16 are borderline: the spec says the motor controller reads pedal torque and wheel speed. The run had directed `PedalTorquePort` / `WheelSpeedPort` (L77, L85) and their conjugates on the controller (L212, L214). A typed interface was one struct away, and the other two e-bike runs used one. The TBD channel turns off polarity/type checking on a flow the spec does give. The reason is true, since the signal electrical interface is not specified, so this is not 2b.

**Numbers:**

- Spec values: 250 (116, 228 redefines std `ratedPower`), 6.94 (131, 202), 42 (145), 36 (174), 14 A*h (176).
- 3.0 `phase_count` (205), dimensionless, from the spec's "3-phase" (all three e-bike runs model 3-phase). I treat it as a spec number, though it is not in the key's table.

Leakage: 0. Conventions: none.

**Std defaults:**

- Part level: all handled. `ratedPower` is redefined from the spec, the other 5 are blanked, and lint is clean.
- Not lint-visible:
  - `ElectricalPowerPort` payload (12 V / 1 A) on the 36 V battery and on the BMS/MC ports (L179, 188, 190, 208, 210).
  - `ElectricalLink.transmissionEfficiency = 0.98` on the 2 connections `bmsToMotorController` and `motorControllerToHubMotor` (L335–346).
  - `HubMotor` inherits unconnected `heat_out` (`ThermalPort` 293.15 K), `noise_out` (`AcousticPort` 0.02 Pa / 1000 Hz), `mech_out` and `cmd_in` from `ElectricMotor` / `PhysicalComponent`.

**Invented structure:** all marked (L299, 303, 304, 308, 309). Fault is latched (L312, documented). Unmarked: none.

**LOG:**

- 10 invocations.
- Friction failures: 0.
- Verify rounds: 2. Round 2 came after the lint `inherited_std_default` info prompted blanking 3 `PhysicalComponent` defaults.

---

## water_heater_r1

**TBD inventory (5)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 306 | `tbd_value` | `ThermostatController.setpoint_k` | setpoint not given | honest (incidental) |
| 2 | 320–324 | transition `tbd` | Idle → Heating (`HeatDemand`) | on-threshold vs setpoint not given | honest |
| 3 | 328–332 | transition `tbd` | Heating → Idle (`HeatSatisfied`) | off-threshold not given | honest |
| 4 | 336–340 | transition `tbd` | Overtemp → Idle (`ResetRequest`) | reset not given | honest (incidental) |
| 5 | 350–353 | `tbd_value` | `ThermalCutoff.trip_temp_k` | "spec gives only the upper bound (≤ 368 K), not the rated trip point" | **borderline** |

Why 5 is borderline: the spec does give 368 K as a bound, and the run keeps it on the WH-002 limit (L200). The cutoff's own trip setting is a separate design parameter, so the reason is true. It is not 2a, but a reader could argue it is.

**Numbers:** 358 (186), 368 (200), 2000 (214, 266), 230 (255), 80 L (282), 1 Hz (292). Leakage: 0. Conventions: none.

**Std defaults:** none (own ports).

**Invented structure:**

- `HeatDemand`, `HeatSatisfied` and `ResetRequest` are marked.
- `CutoffTripped` (L325, L333) is not invented: the spec's Overtemp is "cutoff opened".
- Unmarked: none.

**LOG:**

- 5 invocations.
- Friction failures: 0.
- Verify rounds: 1.

---

## water_heater_r2

**TBD inventory (12)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1–3 | 167, 169, 171 | `redefines` + `tbd_value` | `HeatingElement` (`ResistiveHeater` → `PowerElement`): mass, efficiency, wasteHeat | not stated | blanked-std-default ×3 |
| 4–7 | 190, 192, 194, 196 | `redefines` + `tbd_value` | `NtcTemperatureSensor` (`TemperatureSensor` → `SensorBase` → `ElectronicUnit`): maxMeasurableTemp, mass, supplyVoltage, powerDraw | not stated | blanked-std-default ×4 |
| 8 | 210 | `tbd_value` | `ThermostatController.setpoint_temp` | setpoint not stated | honest |
| 9 | 236 | `tbd_value` | `ThermalCutoff.trip_temp_k` | "design trip point not stated; WH-002 gives only the 368 K limit" | **borderline** (as water_heater_r1 #5) |
| 10 | 257–261 | transition `tbd` | Overtemp → Idle (`ResetPressed`) | reset not stated | honest |
| 11 | 316–321 | `connect(tbd)` | `tankToSensor` | NTC-to-water coupling not stated | honest (see Unsure) |
| 12 | 322–327 | `connect(tbd)` | `tankToCutoff` | cutoff sensing coupling not stated | honest (see Unsure) |

**Numbers:**

- 358 (81), 368 (95), 2000 (109; and 165, which redefines std `ratedPower`), 230 (151).
- 1 Hz (188), redefining std `sampleRate`.
- 0.08 m^3 (222), an exact conversion of 80 L.

Leakage: 0. Conventions: none.

**Std defaults:**

- Part level: all handled (`ratedPower` and `sampleRate` redefined from the spec, 7 blanked).
- Not lint-visible:
  - `ElectricalLink.transmissionEfficiency = 0.98` on `mainsToCutoff` and `cutoffToElement` (L286–297).
  - `ElectricalPowerPort` payload 12 V / 1 A on the 230 V mains port (L154) and the cutoff ports (L239, L241).
  - `ThermalPort` payload 293.15 K on the heater, tank, sensor and cutoff thermal ports (L176, 201, 225, 227, 243).

**Invented structure:**

- `HeatDemand` (L251) and `HeatSatisfied` (L254) are unmarked. Graded minor: the thermostat implies these events, and the open threshold is covered by the setpoint TBD (L210). water_heater_r1 marked the same transitions.
- `ResetPressed` is marked.
- `CutoffOpened` comes from the spec.

**LOG:**

- 5 invocations.
- Friction failures: 0.
- Verify rounds: 1.

---

## water_heater_r3

**TBD inventory (2)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 256 | `tbd_value` | `ThermostatController.setpoint_temp_k` | setpoint not given | honest |
| 2 | 275–279 | transition `tbd` | Overtemp → Idle (`CutoffResetRequest`) | reset method not given | honest |

**Numbers:**

- 230 (25, 201), 358 (145), 368 (159, 212), 2000 (173, 225), 1 Hz (246).
- 0.08 m^3 (238), an exact conversion of 80 L.

Leakage: 0.

Note on L212: `ThermalCutoff.trip_temp_k` defaults to **368 K, the WH-002 bound used as the device trip setting**. This is the opposite choice from water_heater_r1 and water_heater_r2. It is a spec number, so not leakage, but it conflates the limit with the design value: a cutoff set exactly at the bound has zero margin, and the requirement then verifies against itself if evidence is copied.

**Conventions:** bool-flag guards `event.demand` / `!event.demand` (L269, L272), no number.

**Std defaults:** none. `build_model` does not even load `std_packages()` (L318–325).

**Invented structure:** `HeatDemand { demand: bool }` (L47) is unmarked at L269 and L272. Minor, same reasoning as water_heater_r2. The reset is marked.

**Doc inaccuracy:** the header (L9) says "sensor accuracy" is a TBD, but no such TBD exists in the source.

**LOG:**

- 6 invocations.
- Friction failures: 0.
- Verify rounds: 1.

---

## ventilation_r1

**TBD inventory (8)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 128 | `tbd_value` | HRV-3 `max_power_w` | pending energy-label class | honest (D3) |
| 2 | 142 | `tbd_value` (owner = vendor) | HRV-4 `min_efficiency` | datasheet not received | honest (D2) |
| 3 | 277 | transition `tbd` | Normal → Boost (`BoostRequest`) | Boost entry not given | honest |
| 4 | 278 | transition `tbd` | Normal → FrostProtection (`FrostEntry`) | entry condition to be defined | honest (D4) |
| 5 | 280 | transition `tbd` | Boost → Normal (`ModeReset`) | exit not given | honest |
| 6 | 282 | transition `tbd` | FilterAlarm → Normal | clear condition not given | honest |
| 7 | 284 | transition `tbd` | FrostProtection → Normal | exit not given | honest |
| 8 | 317–322 | `connect(tbd)` | `controllerToBmsGateway` | protocol decided by BMS vendor | honest (D1) |

**D1–D5:**

| Gap | Honest? | How |
| --- | --- | --- |
| D1 | yes | `connect(tbd)` at L317 |
| D2 | yes | `tbd_value` on the requirement limit at L142. The exchanger has no own attribute, so the gap is counted once. |
| D3 | yes | L128 |
| D4 | yes | TBD transition at L278 |
| D5 | yes | 40 Hz exact conversion (L184), documented at L183 |

**Numbers:** 0.14 (98, 203, 213; exhaust equals supply per spec), 150 Pa (114, 249), 170 W (181), 40 Hz (184, from 2400 rpm), 230 V (326). Leakage: 0.

**Conventions:** `default = 0.0` Pa (L31).

**Std defaults:** none (no std use at all).

**Invented structure:** `BoostRequest`, `FrostEntry` and `ModeReset` are all on TBD transitions. Unmarked: none.

**LOG:**

- 8 lines logged. **Attempt 1 (exit 127, "cargo test: command not found", label "cargo test #1") is greenhouse_r2's misrouted call** through the overwritten shared `trial3/logrun.sh`, so I excluded it.
- Adjusted: 7 invocations.
- Friction failures: 1 (attempt 2: in-crate audit test failed on `unresolved reference 'Real'`, a missing `ScalarValues::*` import in the Context package).
- Verify rounds: 1.

---

## ventilation_r2

**TBD inventory (19)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 103 | `tbd_value` | HRV-3 `max_total_power_w` | pending energy-label | honest (D3) |
| 2 | 115 | `tbd_value` | HRV-4 `min_efficiency` | datasheet not received | honest (D2) |
| 3–6 | 157, 159, 161, 163 | `redefines` + `tbd_value` | `EcFanMotor` (`BrushlessDcMotor` → `ElectricMotor` → `PhysicalComponent`): efficiency, mass, heatDissipation, noiseLevelDb | not in spec | blanked-std-default ×4 |
| 7 | 193 | `tbd_value` | `PlateHeatExchanger.efficiency` | datasheet not received | honest (D2) |
| 8–13 | 221–231 | `redefines` + `tbd_value` | `MainsSupply` (`AcMainsSupply`): maxCurrent, mainsFrequency, mass, ratedPower, efficiency, wasteHeat | not in spec | blanked-std-default ×6 |
| 14 | 270 | transition `tbd` | Normal → Boost | entry not given | honest |
| 15 | 271 | transition `tbd` | Normal → FrostProtection | to be defined | honest (D4) |
| 16 | 273 | transition `tbd` | Boost → Normal | exit not given | honest |
| 17 | 280–285 | transition `tbd` (guarded `dp <= threshold`) | FilterAlarm → Normal | clear condition not given | honest |
| 18 | 287 | transition `tbd` | FrostProtection → Normal | exit not given | honest |
| 19 | 317–322 | `connect(tbd)` | `controllerToBms` | BMS vendor decides | honest (D1) |

**D1–D5:**

| Gap | Honest? | How |
| --- | --- | --- |
| D1 | yes | L317 |
| D2 | yes | L115 and L193 |
| D3 | yes | L103 |
| D4 | yes | L271 |
| D5 | yes | `redefines = "ratedRotationFreq", default = 40.0, unit = "Hz"` (L155). This also replaces the std 50 Hz placeholder. |

**Numbers:**

- 0.14 (77, 172, 182), 150 (91, 243).
- 170 (153), redefining std `ratedPower`.
- 40 Hz (155), from 2400 rpm.
- 230 (219), redefining std `nominalVoltage`.

Leakage: 0. Conventions: none.

**Std defaults:**

- Part level: all 13 inherited attributes are redefined from the spec or blanked. Lint is clean with 0 info.
- Not lint-visible:
  - `EcFanMotor` inherits unconnected `power_in` (12 V / 1 A payload), `heat_out` (293.15 K), `noise_out` (0.02 Pa / 1000 Hz), `mech_out` and `cmd_in`.
  - `MainsSupply` inherits `power_out` (12 V / 1 A) and `heat_out` (293.15 K).

**Invented structure:** all transitions with invented triggers carry `tbd`. Unmarked: none.

Structural gap, not TBD-marked: the DP sensor (L200) is never connected to the Controller. The Controller has no DP input port (L242–247), but its FilterAlarm transitions consume `FilterPressureSignal`.

**LOG:**

- 10 invocations. A2 and A6 were logged after the fact; disclosed.
- Friction failures: 0.
- Verify rounds: 2. Round 2 came after lint info prompted blanking 7 inherited defaults and adding the FrostProtection exit.

---

## ventilation_r3

**TBD inventory (11)**

| # | Line | Marker | Target | Reason | Class |
| --- | --- | --- | --- | --- | --- |
| 1 | 99 | `tbd_value` | HRV-3 `max_total_power` | pending energy-label | honest (D3) |
| 2 | 111 | `tbd_value` | HRV-4 `min_efficiency` | datasheet not received | honest (D2) |
| 3 | 165 | `tbd_value` | `PlateHeatExchanger.efficiency` | datasheet not received | honest (D2) |
| 4 | 208 | transition `tbd` | Normal → Boost (on `BmsExchange`) | Boost entry not given | honest |
| 5 | 209 | transition `tbd` | Normal → FrostProtection (on `BmsExchange`) | to be defined | honest (D4) |
| 6 | 211 | `#[tbd]` | state Boost | entry/exit not given | honest |
| 7 | 212 | transition `tbd` | Boost → Normal | exit not given | honest |
| 8 | 214 | transition `tbd` | FilterAlarm → Normal | clearance not given | honest |
| 9 | 216 | `#[tbd]` | state FrostProtection | entry to be defined | honest (D4) |
| 10 | 217 | transition `tbd` | FrostProtection → Normal | exit not given | honest |
| 11 | 243–248 | `connect(tbd)` | `controllerToBmsGateway` | BMS vendor decides | honest (D1) |

**D1–D5:**

| Gap | Honest? | How |
| --- | --- | --- |
| D1 | yes | L243 |
| D2 | yes | L111 and L165 |
| D3 | yes | L99 |
| D4 | yes | state TBD (L216) and transition TBD (L209) |
| D5 | yes | 40 Hz (L135). The first attempt was `unit = "rpm", custom_unit = ...`, the documented Pattern 8 route, but strict audit rejected it with `unresolved reference 'rpm'` (LOG entry 1), so the run converted. |

**Numbers:** 0.14 (74, 146, 156), 150 (87, 182), 170 (132), 40 Hz (135), 230 (251). Leakage: 0.

**Conventions:** `default = 0.0` Pa (L25).

**Std defaults:** none (no std packages loaded, L271–278).

**Invented structure:**

- Boost and FrostProtection entries and exits reuse the BMS exchange item `BmsExchange` as a placeholder trigger (L208, 209, 212, 217). All are TBD-marked, so they count as marked.
- Semantically they suggest the BMS triggers frost protection, which the spec does not say. The run disclosed this in its log.
- Unmarked: none.

**LOG:**

- 7 invocations.
- Friction failures: 2.
  - Entry 1: `custom_unit` rpm audit error, plus `unreachable_state` lint warnings for Boost and FrostProtection.
  - Entry 2: a re-run of the same code to read the messages.
- Verify rounds: 1.

---

## Summary table

Abbreviations:

- **TBDs** = TBDs total (equals the `rsysdsl tbd` ledger and the emitted `@TBD` count in every run).
- **Honest** = honest-gap TBDs.
- **Blanked** = blanked-std-default TBDs.
- **Borderline** = borderline TBDs.
- **Unmarked triggers** = invented triggers unmarked: transitions on events or conditions not in the spec, with no `tbd`.

| Run | Invocations | Friction failures | Verify rounds | TBDs | Honest | Blanked | Misuse (a/b/c) | Borderline | Leakage | Unmarked triggers |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| greenhouse_r1 | 8 (+1 maturity probe incl.) | 1 | 1 | 5 | 5 | 0 | 0/0/0 | 0 | 0 | **yes**: 4 (`sensor_ok` fault entry/recovery, L271/288/295/308) |
| greenhouse_r2 | 11 (+1 maturity probe incl.; +1 misrouted exit-127 excluded) | 0 | 2 | 8 | 8 | 0 | 0/0/0 | 0 | 0 | no |
| greenhouse_r3 | 11 | 0 | 2 | 10 | 4 | 6 | 0/0/0 | 0 | 0 | **yes**: 5 (`SensorFaultDetected` L418/421/424/427, `SensorRecovered` L429) |
| ebike_r1 | 13 (+2 maturity probes incl.) | 0 | 2 | 8 | 8 | 0 | 0/0/0 | 0 | 0 | no |
| ebike_r2 | 6 | 1 | 1 | 3 | 3 | 0 | 0/0/0 | 0 | 0 | **yes**: 6 (PowerOn/PowerOff/FaultDetected/FaultCleared, L292/295/296/299/300/302) |
| ebike_r3 | 10 | 0 | 2 | 16 | 9 | 5 | 0/0/0 | 2 | 0 | no |
| water_heater_r1 | 5 | 0 | 1 | 5 | 4 | 0 | 0/0/0 | 1 | 0 | no |
| water_heater_r2 | 5 | 0 | 1 | 12 | 4 | 7 | 0/0/0 | 1 | 0 | minor: 2 (HeatDemand/HeatSatisfied L251/254, thermostat-implied) |
| water_heater_r3 | 6 | 0 | 1 | 2 | 2 | 0 | 0/0/0 | 0 | 0 | minor: 2 (HeatDemand bool L269/272) |
| ventilation_r1 | 7 (8 logged − 1 contaminated) | 1 | 1 | 8 | 8 | 0 | 0/0/0 | 0 | 0 | no |
| ventilation_r2 | 10 | 0 | 2 | 19 | 9 | 10 | 0/0/0 | 0 | 0 | no |
| ventilation_r3 | 7 | 2 | 1 | 11 | 11 | 0 | 0/0/0 | 0 | 0 | no (placeholder trigger reused, but marked) |

"Maturity probe incl." means the probe is counted in invocations but not in friction failures.

---

## Aggregate

**Totals (12 runs):**

| Metric | Value |
| --- | --- |
| Invocations | 99 (mean 8.25, max 13, all within the 15 cap) |
| Friction failures | 5 |
| Deliberate `--maturity` probe failures (excluded) | 4 |
| Contaminated exit-127 call (excluded) | 1 |
| Verify rounds | 17 |

**TBD classification:**

| Category | Count |
| --- | --- |
| TBDs total | 107 |
| Honest gaps | 75 |
| Blanked std defaults | 28 (in 4 runs: greenhouse_r3, ebike_r3, water_heater_r2, ventilation_r2) |
| Misuse 2a / 2b / 2c | **0 / 0 / 0** |
| Borderline | 4 (water heater cutoff trip point ×2; ebike_r3 sensor TBD channels ×2) |

**Leakage:** 0 numeric literals outside spec values, exact conversions (40 Hz, 0.08 m^3) and redefined std values.

- Conventions: 11 `default = 0.0` initializers across 5 runs (greenhouse_r2 4, ebike_r2 3, greenhouse_r3 2, ventilation_r1 1, ventilation_r3 1), plus bool-flag guards.
- Spec-derived but not in the key: `phase_count = 3` (ebike_r3 L205).

**Std defaults left in place:**

- Part-level `inherited_std_default`: 0 in every final model.
- Port, item and interface-level defaults that lint does not see are present in 5 runs:
  - 12 V / 1 A `ElectricalEnergy`: greenhouse_r1, greenhouse_r3, ebike_r3, water_heater_r2, ventilation_r2.
  - 293.15 K `ThermalEnergy`: greenhouse_r3, ebike_r3, water_heater_r2, ventilation_r2.
  - 0.02 Pa / 1000 Hz acoustic: ebike_r3, ventilation_r2.
  - `ElectricalLink` 0.98: ebike_r3, water_heater_r2.

**Unmarked invented triggers:**

- 3 runs clearly: greenhouse_r1, greenhouse_r3, ebike_r2 (15 transitions).
- 2 runs minor: water_heater_r2 and water_heater_r3 (thermostat-implied HeatDemand).

**Ventilation D1–D5:** 15/15 handled honestly.

- D1: `connect(tbd)` ×3.
- D2: `tbd_value` on the HRV-4 limit ×3, plus the exchanger attribute in r2 and r3.
- D3: `tbd_value` ×3.
- D4: TBD transition ×3, plus a state `#[tbd]` in r3.
- D5: exact 40 Hz ×3, with 0 non-conversion numbers.

### Most important patterns

1. **No TBD misuse and no numeric leakage in 107 TBDs across 12 runs.** No TBD sits on a spec value. No `tbd_unit` or `custom_unit` dodges a dimension check. Every measured value is `evidence` with no default. Every requirement limit carries the spec number. The only debatable items are the 4 borderline ones.
2. **The lint nudge for std defaults works, but only at part level.**
   - 4 runs specialized std parts, and every one of them reached 0 `inherited_std_default` by blanking: greenhouse_r3 6, ebike_r3 5, water_heater_r2 7, ventilation_r2 10. In greenhouse_r3, ebike_r3 and ventilation_r2 it took a second verify round triggered by the lint info.
   - The same runs, plus greenhouse_r1, carry unflagged std numbers through std ports and interfaces: 12 V / 1 A on 24 V, 36 V and 230 V supplies; 0.98 link efficiency; 293.15 K; 0.02 Pa / 1000 Hz.
   - 8 of 12 runs specialized no std part (greenhouse_r1, greenhouse_r2, ebike_r1, ebike_r2, water_heater_r1, water_heater_r3, ventilation_r1, ventilation_r3); most of them cited the spec-less defaults as the reason. So 0 part-level leakage is partly bought by less reuse.
3. **Invented triggers are the main remaining honesty gap.** 5/12 runs drive transitions from events the spec does not give, without `tbd`. Sibling runs on the same spec marked the identical transitions, so the behaviour is inconsistent, not impossible:
   - greenhouse_r1 and greenhouse_r3 (sensor fault/recovery) vs greenhouse_r2, which marked them.
   - ebike_r2 (power on/off, fault) vs ebike_r1 and ebike_r3, which marked them.
   - water_heater_r2 and water_heater_r3 (HeatDemand) vs water_heater_r1, which marked them.

   ebike_r1 added its transition TBDs only on a self-review (LOG #8). No tool flags an unguarded transition on an invented event.
4. **The documented D5 route is broken, and state-level TBDs are not enough for lint.**
   - ventilation_r3 tried `unit = "rpm", custom_unit = "..."` exactly as PATTERNS Pattern 8 shows, and strict audit failed with `unresolved reference 'rpm'`. All three runs ended on the 40 Hz conversion. ventilation_r1 and ventilation_r2 avoided `custom_unit` pre-emptively.
   - ventilation_r3 also found that `#[sysml::tbd]` on a state does not clear `unreachable_state`. That pushed it to invent a placeholder trigger by reusing `BmsExchange`. It is TBD-marked but semantically misleading.
5. **The water heater "bound vs design setting" split.**
   - water_heater_r1 (L350) and water_heater_r2 (L236): TBD on the cutoff trip point, reason "spec gives only the ≤ 368 K bound".
   - water_heater_r3 (L212): sets the trip point to the bound 368 K.

   Neither is leakage, but the protocol needs a rule. The r3 choice equates the design value with the limit, which is the conflation trial 1 flagged. Similarly, all three ventilation runs model HRV-2 as "alarm trigger ≤ 150 Pa" and put 150 Pa on the controller as the design threshold: same pattern, spec-given, not graded down.

Smaller notes:

- Friction is low. All 5 friction failures are Rust or import slips (test-module imports ×2, missing `ScalarValues::*`) or the custom_unit and lint issue above. There were no audit or lint failures on the final models.
- The shared `trial3/` helper scripts caused one cross-run log contamination (greenhouse_r2's call appears in ventilation_r1/LOG.md attempt 1).

---

## Unsure how to classify

- **TBD channels where a typed interface was available but the spec is silent on the link:** e-bike `packToBms` in all 3 runs (`DcPowerLink` / `ElectricalLink` had matching ports), and water_heater_r2 `tankToSensor` / `tankToCutoff` (`ThermalLink` matched, L316–327). I graded these honest, and graded ebike_r3's pedal/speed channels borderline because the spec names those flows. A stricter grader could call all of them borderline (2b-adjacent: the TBD bypasses port checking).
- **Water heater trip point TBD** (water_heater_r1 L350, water_heater_r2 L236): graded borderline. If the rule is "368 K is spec-given, therefore any TBD on a 368 K-related attribute is 2a", these become 2 misuse-2a.
- **Self-introduced attributes then TBD'd** (greenhouse_r2 `sample_rate` L265, `output_current` L350; greenhouse_r3 fan `supply_voltage` L340): graded honest, since the reasons are true. They add debt the spec never asked for, though not leakage.
- **HeatDemand in water_heater_r2 and water_heater_r3:** counted as "minor unmarked". If the thermostat's call-for-heat is considered spec-implied, these are not invented at all.
- **Port and interface-level std defaults** (12 V / 1 A, 293.15 K, 0.98, 0.02 Pa / 1000 Hz): not counted as leakage per the key, because they are not claimed as spec values and lint does not report them. If the protocol intends "any std default not redefined or blanked" to count, greenhouse_r1, greenhouse_r3, ebike_r3, water_heater_r2 and ventilation_r2 each get non-zero leakage.
- **ebike_r3 `phase_count = 3`:** treated as spec ("3-phase", corroborated by all 3 e-bike runs). The full spec text for specs 1–3 was not available to me; only the key's number table.
