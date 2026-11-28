# Brownfield Corpus-to-Model A/B Results (`PX4 + Pixhawk 6X`)

This report documents our controlled A/B experiment on a real, pinned **16-document open-source hardware/software corpus** with **3 compute units**: **PX4 Autopilot on Pixhawk 6X (`FMUv6X` STM32H753 + `IO` STM32F103 + `Companion` AI Computer)**.

- **Corpus fetcher:** [`scripts/fetch-px4-corpus.sh`](../scripts/fetch-px4-corpus.sh) (pinned public docs from `pixhawk/Pixhawk-Standards` and `PX4/PX4-user_guide`, frozen with SHA-256 manifest)
- **Total frozen corpus:** **16 documents**, **169,421 bytes** (**~42,355 tokens**) split into 4 same-type batches:
  1. `batch_1_hardware` (4 docs, ~9,165 tok): `DS-012_FMUv6X_Standard.txt`, `DS-010_Autopilot_Bus_Standard.txt`, `DS-019_Pixhawk_versions.md`, `autopilot_pixhawk_standard.md`
  2. `batch_2_sw_arch` (4 docs, ~6,272 tok): `architecture.md`, `px4_systems_architecture.md`, `control_allocation.md`, `flight_modes.md`
  3. `batch_3_comms` (4 docs, ~20,513 tok): `can_index.md`, `mavlink.md`, `uorb.md`, `uxrce_dds.md`
  4. `batch_4_ai_safety` (4 docs, ~6,397 tok): `collision_prevention.md`, `obstacle_avoidance.md`, `path_planning_interface.md`, `pwm_limit.md`

---

## 1. Experiment A — Doc-Batched Modeling (`px4_fmuv6x_brownfield.rs`)

- **Implementation:** [`crates/sysml-emit/examples/px4_fmuv6x_brownfield.rs`](../crates/sysml-emit/examples/px4_fmuv6x_brownfield.rs)
- **Tests:** [`crates/sysml-emit/tests/px4_fmuv6x_brownfield_tests.rs`](../crates/sysml-emit/tests/px4_fmuv6x_brownfield_tests.rs) (3/3 passing)

### Results
- **Cycle 1 (Batch 1 Hardware Spine — Shallow):**
  - 2 on-board `ComputeNode`s (`FmuMcu`, `IoMcu`), 4 `BusChannel`s (`SensorSpiBus`, `FmuIoUart`, `CanFdBus`, `EthernetMavlink`), 2 placeholder partitions.
  - `validate_ir`: **PASS** (`0 errors, 2 W001 warnings` for unallocated safety partitions).
  - Open-MBEE + OMG Standard Library: **0 errors** (`1,459 bytes` pure OMG SysML v2).
- **Cycle 4 (All 4 Doc-Type Batches — Enriched):**
  - **3 `ComputeNode`s** (`FmuMcu` STM32H753, `IoMcu` STM32F103, `CompanionSoc` Linux/ROS 2), **4 `BusChannel`s** (`CompanionUxrceDdsLink` with documented `200 ms` vision delay from `collision_prevention.md §CP_DELAY`), **6 `SwPartition`s**, **6 `SwConnection`s**, **6 `SwAllocation`s**, **3 safety `SwRequirement`s** (`PX4-SAFE-01..03`).
  - Documented IMU loop periods (`1.0 ms` sample / `4.0 ms` publish from `architecture.md`) included; undocumented `wcet_ms` honestly left `Option::None` (zero invented numbers).
  - `validate_ir`: **PASS (0 errors, 0 warnings)**.
  - Open-MBEE + OMG Standard Library: **PASS (0 errors, 0 warnings)** (`7,220 bytes` pure OMG SysML v2).
  - Negative safety test (`px4_negative_bypassing_commander_guardrail_triggers_e004`): bypassing `CommanderSafetyMonitor` (`ObstacleAvoidanceAI -> UnprotectedIoActuator`) is caught immediately with **`E004`**.

### Observed Friction in Experiment A (Doc-Batched Only)
1. **Manual cross-batch alias tracking:** `CompanionSoc` appeared as `"Companion Computer"`, `"Mission Computer"`, `"ROS 2"`, and `"external vision-based systems"` across Batches 2, 3, and 4. Without an index, keeping one canonical name relied on manual discipline.
2. **Coarse provenance:** Citations were written manually as doc+section strings (`"collision_prevention.md §CP_DELAY"`), not verifiable `file://` byte offsets or line numbers that detect stale docs when a spec changes.
3. **Scaling risk:** At 16 docs (~42K tokens), 4 batches were comfortable; at 100–500 docs, sequential doc-type batching cannot answer cross-cutting questions (*"every doc mentioning `STM32F103` or `PX4IO` safety overrides"*) without re-reading most of the corpus.

---

## 2. Experiment B — Same Corpus via `cppognee` (`apps/docmem`, No-AI Mode)

- **Implementation:** [`tools/corpus_kg/px4_docmem_kg.py`](../tools/corpus_kg/px4_docmem_kg.py)
- **Engine:** `cppognee/apps/docmem` (`docmem.scan` + stdlib-only `cognee_egress`, **zero LLM calls, zero API keys, zero network**).

### Measured Results (`target/px4_corpus/kg_export.json`)
- **Total scan + graph build time:** **552 ms** (`0.55 s`) for all 16 docs!
- **Graph built:**
  - **16 documents** → **134 hierarchical sections** → **205 passages** (each carrying exact `file://` URI, `line`, `offset`, `length`, and `sha256` hash)
  - **366 cross-document Markdown links**
  - **9 canonical entities** resolved across **29 observed alias variants**:
    - `FmuMcu`: 43 hits across 7 docs (`FMU`, `FMUv6X`, `Flight Management Unit`, `STM32H753`)
    - `IoMcu`: 4 hits across 4 docs (`FMU_SAFETY_SWITCH_IN`, `PWM_limit`, `PX4IO`)
    - `CompanionSoc`: 48 hits across 9 docs (`Companion Computer`, `Mission Computer`, `ROS 2`, `Vision-based`)
    - `CanFdBus`: 21 hits across 3 docs (`Cyphal`, `DroneCAN`, `UAVCAN`)
    - `CompanionUxrceDdsLink`: 71 hits across 7 docs (`Fast-RTPS`, `MAVLink`, `Micro XRCE-DDS`, `uXRCE-DDS`)
    - `UorbBus`: 51 hits across 7 docs (`shared memory`, `uORB`)
    - `CommanderSafetyMonitor`: 13 hits across 3 docs (`Collision Prevention`, `FailsafeFlags`, `Return mode`, `commander`)
    - `ControlAllocator`: 4 hits across 1 doc (`Control Allocation`, `actuator_motors`, `mixer_module`)
    - `ImuSensorDriver`: 11 hits across 2 docs (`1kHz`, `250Hz`, `BMI088`, `ICM20602`)
  - **21 quantitative & compatibility claims** automatically pinned to exact `doc:section:line:offset`
  - **4 surfaced conflicts & gaps:**
    - `CONFLICT-01`: `uXRCE-DDS` client `v2.x` incompatible with agent `v3.x` (`uxrce_dds.md`)
    - `CONFLICT-02`: `UAVCAN` split into `DroneCAN` (v0) and `Cyphal` (v1) (`can_index.md`)
    - `CONFLICT-03`: 4-way terminology drift for `CompanionSoc` across 9 docs
    - `GAP-01`: 0 WCET values across all 16 docs → confirmed `Option::None` / TBD.

---

## 3. Objective A vs. B Scorecard

| Dimension | Experiment A (Doc-Batched Only) | Experiment B (`cppognee/docmem` No-AI KG) | Winner / Takeaway |
| --- | --- | --- | --- |
| **Setup & External Cost** | Zero API keys | Zero API keys (`0.55 s` pure Python) | **Tie** |
| **Model Correctness (`validate_ir` + OMG Stdlib)** | **0 errors / 0 warnings** (`7.2 KB` SysML v2) | Feeds identical `SwHwSystem` + `validate_ir` | **Shared strength** |
| **Cross-Doc Alias Resolution** | Manual discipline across batches | Automatic across all 16 docs (29 aliases / 9 entities) | **Experiment B** |
| **Provenance Granularity** | Manual `"file.md §Section"` strings | Exact `file://uri`, `line`, `byte offset`, `sha256` | **Experiment B** |
| **Cross-Doc Link Discovery** | Ad-hoc while reading | All **366 cross-doc links** indexed automatically | **Experiment B** |
| **Semantic Reasoning (e.g., safety guardrail intent)** | Strong (agent reads full section prose) | Rule-based regex only (needs agent/LLM to synthesize architecture) | **Experiment A** |

---

## 4. Recommended Next Step: Combine A + B

The experiment makes the winning architecture crystal clear:
1. **Always run `cppognee/apps/docmem` (no-AI, 0.55 s) first** on any brownfield corpus to build the deterministic **Section + Passage + Cross-Doc Link + Alias + Conflict Index** with verifiable `sha256` byte-offset provenance.
2. **Feed aspect-sliced passages from `kg_export.json` into the modeling agent** (instead of raw unindexed document dumps) to synthesize the shallow-first `SwHwSystem` Spine → Enriched `rsysdsl` model.
3. **Next operator to build:** `rsysdsl reconcile` — reads `kg_export.json` alongside a `SwHwSystem` / `Model`, checks that every component/parameter cites a valid `docmem` passage hash, and automatically flags stale citations when an upstream doc changes (`diff-corpus`).
