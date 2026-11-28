# T5 answer key — pinned PX4 + Pixhawk FMUv6X corpus (T5.K)

Pre-registered grading key for Trial 5 (see `T5_PROTOCOL.md`). Built from the 16 corpus files alone,
fetched over `raw.githubusercontent.com` at the pinned SHAs; no other repository file about this corpus
was consulted. Everything below is quoted verbatim (straight quotes in this file; the PDFs use curly quotes
and inch marks) with its location. No modelling advice.

**Location conventions.** Markdown: `file.md:L<line>` (1-based line in the fetched file).
PDF: `DS-012 p.N` / `DS-010 p.N` (1-based PDF page = printed page number in the footer).
**Markers.** `[T]` = the fact exists only inside a PDF table (nowhere in running text of any corpus file).
`[I]` = the fact is carried only by an image/diagram; at most a caption/alt text/heading is extractable.

Pins: `PX4/PX4-user_guide@2fa62338099f18fc3092197e856615eca94d2975`,
`pixhawk/Pixhawk-Standards@a97a915bfb6e20d6c8a41a86c5673350a3e51ceb`.

---

## 1. Sources

| # | Corpus name (fetch script destination) | Upstream path | sha256 | bytes |
|---|---|---|---|---|
| 1 | `batch_1_hardware/DS-012_FMUv6X_Standard.pdf` (21 pages) | `DS-012 Pixhawk Autopilot v6X Standard.pdf` | `a434add2e261b9c1de04218abcfd6606f48566c372cd596d9daba98c16791387` | 3819249 |
| 2 | `batch_1_hardware/DS-010_Autopilot_Bus_Standard.pdf` (19 pages) | `DS-010 Pixhawk Autopilot Bus Standard.pdf` | `8fc3883e1aeade93efd15de1d2f9c3e75d3609eee9bd2ed279621ede4eb538cf` | 3561860 |
| 3 | `batch_1_hardware/DS-019_Pixhawk_versions.md` | `DS-019 Pixhawk versions and revisions.md` | `a6b31e3fba48992bcfed40c40f1fa65e10e48c823f84bef3dae452e886c7ef94` | 7382 |
| 4 | `batch_1_hardware/autopilot_pixhawk_standard.md` | `en/flight_controller/autopilot_pixhawk_standard.md` | `c0f0e3ea0eff443b517acc8cd3ea0ba166beca645132807bb8dfb3aa23e2b88e` | 1991 |
| 5 | `batch_2_sw_arch/architecture.md` | `en/concept/architecture.md` | `c77a4ae8a6037a55fc88c6a9f5bee5949a3404b76f3018abbec9f3e15f29d890` | 9022 |
| 6 | `batch_2_sw_arch/px4_systems_architecture.md` | `en/concept/px4_systems_architecture.md` | `b8124a93960bf4642b529783abeaf723545a119cb088c9e493eaf430231573b8` | 4015 |
| 7 | `batch_2_sw_arch/control_allocation.md` | `en/concept/control_allocation.md` | `367a584893bc3cee1b95f0e8d2f8ee530fc60f355da056bb448cf88dd0aa9d86` | 5179 |
| 8 | `batch_2_sw_arch/flight_modes.md` | `en/concept/flight_modes.md` | `a100cc9670b3a306ce223c0902bda164c089fb7906e2e159a29b521e21cedb70` | 6883 |
| 9 | `batch_3_comms/uorb.md` | `en/middleware/uorb.md` | `7e4d8250cd8f684a2ea7899028fe31c8595c41b36b5142eb1777182294bac693` | 14039 |
| 10 | `batch_3_comms/mavlink.md` | `en/middleware/mavlink.md` | `6a3eacaba944724ed868218d801249289888a73181603697e89f273f141046df` | 29644 |
| 11 | `batch_3_comms/uxrce_dds.md` | `en/middleware/uxrce_dds.md` | `53e46905a3cb91c7047990d8eca0602968216923390812622d454d1942936dd6` | 33330 |
| 12 | `batch_3_comms/can_index.md` | `en/can/index.md` | `6bbaf7288208efe3fe12a871facdd5dfe27b327609d1f0323802d13f428a4e4a` | 5046 |
| 13 | `batch_4_ai_safety/obstacle_avoidance.md` | `en/computer_vision/obstacle_avoidance.md` | `83616a8306a40c36bef27eaf5593385defb2807eb47d4ba320e004325579c945` | 61 |
| 14 | `batch_4_ai_safety/collision_prevention.md` | `en/computer_vision/collision_prevention.md` | `332e76ef2e90db498523e0254d561239a3cf6814b0dce19b7f0c49fcd6e149d1` | 23202 |
| 15 | `batch_4_ai_safety/path_planning_interface.md` | `en/computer_vision/path_planning_interface.md` | `651d1d71b06d178b9299291ba321f172985e510c854940f890085d1c39d4d534` | 1417 |
| 16 | `batch_4_ai_safety/pwm_limit.md` | `en/concept/pwm_limit.md` | `8714a72b9f0fbb97f5686a2f0559662f3ee1cda770706e0a397b7a789591e64d` | 914 |

Hashes are of the raw downloaded files (the fetch script's own manifest hashes the `.md` files and the
pypdf `.txt` extracts, not the PDFs, so its PDF-derived rows will differ).
`obstacle_avoidance.md` is a 1-line stub: `<Redirect to="../computer_vision/path_planning_interface" />`.

### 1.1 PDF text-extraction behaviour (observed)

| Extractor | DS-012 | DS-010 |
|---|---|---|
| `pypdf` `extract_text()` (plain; what `fetch-px4-corpus.sh` uses) | Running text on p.4 and p.11 split one word per line. Sensor-set tables p.11-15 come out **one cell per line**: row association survives only by order, and multi-line cells split into separate lines (`U6 (IMU2)` / `Not installed` / `ICM20602` / `DNP`; `0x76` / `@00=0x50`; `FMU VDD` / `3.3`). MCU pinout p.18-21: rows run together with **no separator between a signal name and the next port** (e.g. `SPI6_MISO_EXTERNAL1PA 7 ETH_CRS_DV E ETH_CRS_DVPA 8 TIM1_CH1 T FMU_CH4`). | Common External Interfaces table p.6 comes out one token per line (`Clock` / `200` / `MHz` / `200` / ...): column-to-FMU association recoverable only by counting. X1/X2 pinouts p.8-10 linearise as `FMU_CH8 1 2 GND (Pin 2) FMU_CH7 3 4 BUZZER_1 ...` (recoverable, but the odd/even column split is implicit). |
| `pypdf` `extraction_mode="layout"` | Good: tables keep columns; multi-line cells stay ambiguous (`Not installed` lands on its own line). | Good for all tables. |
| `pdfplumber` `extract_tables()` | Sensor tables and pinout clean. **Drops revision rows 0.1.0 and 0.3.0** (p.3). Splits `USB_OTG_FS_VBU S` (p.18). | **Drops revision row 0.1.0** (p.3) and the `Part Number` and `Dimensions` rows of the X1/X2 connector tables (p.7-8). X1/X2 pinouts and p.6 table clean. |
| All | Pages 1, 8 (drawing part), 9, 10, 17: content is images only (see §3.3). | Pages 1, 11, 13-19: content is images only, apart from the section headings. |

---

## 2. Compute units and aliases (alias ground truth)

### 2.1 Compute units

| ID | Unit | Every name/alias used in the corpus (verbatim) with location |
|---|---|---|
| CU-1 | **FMU / flight controller** (the autopilot module running PX4) | `FMUv6X` (DS-012 title p.2, p.5 "FMUv6X Summary", p.5 "FMUv6X has the same architecture as v5X", p.6, p.18 "Full FMUv6X Pinout"; DS-010 p.5, p.6 table header; `autopilot_pixhawk_standard.md:L15-L18`) · `Pixhawk Autopilot v6X` (DS-012 document title and every page footer: "Pixhawk Autopilot v6X Standard") · `Pixhawk 6X` (DS-010 p.5 "FMUv6X: Pixhawk 6X (2021, 400-600 MHz H7,, redund. sensors)"; `autopilot_pixhawk_standard.md:L17` "Holybro Pixhawk 6X") · `FMU v6x` / `FMUv6x` (`DS-019_Pixhawk_versions.md:L182` "FMU v6x revisions", `:L186` "FMU v6x REVISION", `:L190-L198`) · `v5/6x` (`DS-019:L140` "HWTYPE = {v5/6x}{VER}{REV}") · `FMUv5x and FMUv6x standard` (`DS-019:L3`) · `FMUvxX` (DS-012 p.17 "IMU Pinout of FMUM's IMU connector (FMUvxX)") · `VxX` (DS-012 p.3 rev 0.4.0 "Added VxX IMU pinout") · `FMUM` (DS-012 p.8 "Top view of FMUM", "clearance to FMUM pcb"; DS-010 p.9 "on the FMUM", p.11-12; `DS-019:L91`, `:L98`, `:L106`, `:L109` "Set Base ID via FMUM") · `FMU SOM` (DS-012 p.9 "Bottom view of FMU SOM"; DS-010 p.11, p.12 "under the FMU SOM") · `SOM` (DS-010 p.12 "between the SOM and base board") · `System-on-Module(SOM)` (DS-010 p.7) · `SoM` (DS-010 p.6 row "PAB / SoM") · `FMU Board` (DS-012 p.11-15 table caption) · `FMU` (`DS-019:L7`, `:L11` "the REV is on the FMU", `:L61`; DS-010 p.13 "Base to FMU Connectors", p.14 "FMU Debug Connector") · `Autopilot side (top)` (DS-010 p.7-8 connector tables) · `autopilot` (DS-010 p.7 "if autopilot and baseboard are integrated into one unit"; DS-012 p.7 "The autopilot can be powered from up to three power sources") · `Flight controller` / `flight controller` (`px4_systems_architecture.md:L3`, `:L20`, `:L41`; `can_index.md:L3`, `:L31`, `:L57`; `flight_modes.md:L7`; `collision_prevention.md:L6`, `:L28`, `:L108`) · `FC` (`px4_systems_architecture.md:L33` heading "FC and Companion Computer"; `flight_modes.md:L27`) · `flight-control board` (`architecture.md:L149`) · `Pixhawk flight controllers` (`can_index.md:L57`) · `Pixhawk standard compatible` (`can_index.md:L47`) · `fmu-v6x` (`uorb.md:L177` link path `boards/px4/fmu-v6x/default.px4board`) · `FMUv6` (`uorb.md:L177` "the FMUv6 [default.px4board]"; DS-012 p.6 "brings the proven features from FMUv6" — see C-15) · `autopilot board` (`autopilot_pixhawk_standard.md:L8`). Product names mapped to FMUv6X: `Holybro Pixhawk 6X Pro (FMUv6X)`, `CUAV Pixhawk V6X`, `Holybro Pixhawk 6X`, `RaccoonLab FMUv6X Autopilot` (`autopilot_pixhawk_standard.md:L15-L18`); `CUAV v6x`, `Holybro FMUv6x rev3`, `Holybro FMUv6x rev4`, `ClearSky AVEGA (FMUv6X)`, `Holybro FMUv6x rev6`, `Holybro FMUv6x rev8`, `Auterion FMUv6x 0.6.0` (`DS-019:L188-L198`). |
| CU-1a | **FMU processor** (part of CU-1) | `STM32H7` (DS-012 p.5 "is based on STM32H7") · `STM32H753` (DS-010 p.6 table, FMUv6X column) `[T]` · `H7` (DS-010 p.5 "400-600 MHz H7") · `FMUM microprocessor` (DS-012 p.8; DS-010 p.12) · `M7 processor U1` (DS-010 p.12 "heat from bottom side M7 processor U1") · `U1` (DS-010 p.12 "thermally connected to U1") · `CPU` (DS-012 p.7 "3.3V rail monitoring for CPU") · `MCU` (DS-010 p.6 row label) `[T]`. |
| CU-2 | **IO co-processor** | `IO` (DS-010 p.6 "PWM out ... 8 + 8 (IO)" `[T]`; signal names `USART6_TX_TO_IO__NC`, `USART6_RX_FROM_IO__RC_INPUT` on DS-012 p.18-19 `[T]` and DS-010 X1 pins 70/72 `[T]`) · `px4io` (`DS-019:L148` "v5x base without px4io"). No other description in the corpus (see G-3). |
| CU-3 | **Companion computer** | `companion computer` (`px4_systems_architecture.md:L3`, `:L35`, `:L41`, `:L45`; `flight_modes.md:L7`, `:L27`, `:L32`; `mavlink.md:L5`, `:L329-L330`; `uxrce_dds.md:L10`, `:L20`, `:L29`, `:L51`; `collision_prevention.md:L29`, `:L171`; `path_planning_interface.md:L17-L18`) · `mission computer` (`px4_systems_architecture.md:L3` "(also known as a "mission computer")", `:L35`; DS-012 p.6 "Ethernet interface for high-speed mission computer integration") · `Companion Computer` (`px4_systems_architecture.md:L33` heading) · `companion` (`uxrce_dds.md:L25` "agent running on the companion"; `uxrce_dds.md:L260` "connecting to the companion using TELEM2") · `offboard companion computer` (`collision_prevention.md:L6`) · `Companion Computers` (`collision_prevention.md:L317`) · `PC / companion computer` (`uxrce_dds.md:L565`) · `external vision system` (`mavlink.md:L332-L333`) · `vision system` (`collision_prevention.md:L174`) · `RPi` / `Raspberry Pi` (example host, `uxrce_dds.md:L210`) · `planner on a companion computer` (`path_planning_interface.md:L18`). Software on it: `micro XRCE-DDS agent` / `MicroXRCEAgent` (`uxrce_dds.md:L25`, `:L85`), ROS 2, "Linux OS" (`px4_systems_architecture.md:L45`), "Android" (`:L47`). |
| CU-4 | **Ground station** | `ground station computer` (`px4_systems_architecture.md:L24`, `:L29`) · `ground station` / `ground stations` (`flight_modes.md:L5`, `:L8`; `mavlink.md:L5`; `px4_systems_architecture.md:L43`, `:L50`) · `GCS` (`architecture.md:L85`; `mavlink.md:L328`, `:L330`, `:L341`, `:L364`; `uxrce_dds.md:L282`) · `QGroundControl` / `QGC` (`px4_systems_architecture.md:L29`; `mavlink.md:L5`, `:L477`, `:L509-L517`; `uorb.md:L225`; `collision_prevention.md:L25`, `:L57`; `control_allocation.md:L55`, `:L80-L82`) · `MAVSDK` (`px4_systems_architecture.md:L30`; `mavlink.md:L5`). |
| CU-5 | **Baseboard** (carrier of the FMU; holds VER divider, EEPROM, Ethernet transceiver, power path) | `base board` (`DS-019:L8`, `:L11`, `:L61`; DS-010 p.12 "The base board should be impedance controlled", p.12 "to the base board through metal standoffs") · `baseboard` (DS-010 p.7 "autopilot and baseboard"; `DS-019:L66` "either a baseboard version or an FMU revision") · `Baseboard side (bottom)` (DS-010 p.7-8) · `BASE` (`DS-019:L91`, `:L98`) · `base` (`DS-019:L72` "Board Type: base\|imu", `:L130`) · `Baseboards` (`DS-019:L128`) · `Baseboard Design Examples (FMUv5X, FMUv6X)` (DS-010 p.13). Named instances: `DS-019:L147-L162` (e.g. "Auterion v5x base board", "Holybro Pixhawk Jetson Baseboard", "HB CM4", "NXP T1 PHY", "ZeroOne X6 Baseboard", "Amovlab ICF6 Baseboard"). |
| CU-6 | **IMU board** (vibration-isolated sensor board on the FMU) | `IMU Board` (DS-012 p.11-15 table caption) · `IMU set of sensors` (DS-012 p.11) · `imu` (`DS-019:L72` board type, `:L82` file name pattern) · `FMUM's IMU connector` (DS-012 p.17) · `flex cable` (DS-012 p.17). |
| CU-7 | **Secure element** | `Secure element ... (SE050, I2C4)` (DS-012 p.6) · `U10 (SE)` / `SE050C1HQ1/Z 01SCZ` (DS-012 p.11-15) `[T]`. |
| CU-8 | **CAN peripherals / nodes** | `ESCs` (`can_index.md:L3`) · `Motor ESCs` (`px4_systems_architecture.md:L21`) · `CAN ESCs` (`can_index.md:L34`) · `GNSS` / `GNSS modules` (`can_index.md:L31`, `:L34`) · `CAN peripherals` (`can_index.md:L61`) · `supported CAN hardware` running "PX4 DroneCAN firmware" (`can_index.md:L63-L64`) · `NXP UAVCAN Board` (`can_index.md:L88`). |

### 2.2 Software components (PX4 on CU-1 unless stated)

| ID | Component | Names/aliases (verbatim, location) |
|---|---|---|
| SW-1 | Flight stack | `flight stack` (`architecture.md:L3`, `:L53-L56`), `PX4 flight stack` (`px4_systems_architecture.md:L20`, `:L31`, `:L41`) |
| SW-2 | Middleware | `middleware` (`architecture.md:L3-L4`, `:L83-L87`), `PX4 middleware` (`architecture.md:L105`) |
| SW-3 | uORB | `uORB` (`architecture.md:L42`, `:L104`; `uorb.md` title), `uorb` module (`uorb.md:L7-L8`), `UORB` (`uorb.md:L51`; `collision_prevention.md:L52`, `:L200`), "publish-subscribe message bus" (`architecture.md:L42`, `:L85`) |
| SW-4 | MAVLink module | `mavlink` module (`mavlink.md:L83`, `:L352`, `:L487`), `/src/modules/mavlink` (`mavlink.md:L75`) |
| SW-5 | uXRCE-DDS client | `uxrce_dds_client` (`uxrce_dds.md:L27`, `:L37`, `:L222`), `uXRCE-DDS client` (`uxrce_dds.md:L25`), `PX4 Micro XRCE-DDS Client` (`uxrce_dds.md:L60`), `client` (`uxrce_dds.md:L20`); predecessor `Fast-RTPS Bridge` (`uxrce_dds.md:L6`), `Fast-RTPS` (`uxrce_dds.md:L506`), `RTPS` (`uxrce_dds.md:L611`), `micrortps` (link path `uxrce_dds.md:L6`) |
| SW-6 | uXRCE-DDS agent (on CU-3) | `micro XRCE-DDS agent` (`uxrce_dds.md:L25`), `eProsima micro XRCE-DDS agent` (`:L29`), `Micro XRCE-DDS Agent` (`:L51`), `MicroXRCEAgent` (`:L85`), `micro-xrce-dds-agent` (`:L93`, `:L99`), `XRCE-DDS agent` / `XRCE-DDS-Agent` (`:L562-L564`), `agent` (`:L20`) |
| SW-7 | Control allocator | `control_allocator` (`control_allocation.md:L37`), "Control Allocation (Mixing)" (`:L1`), `mixer` (`architecture.md:L78`), "mixing" (`control_allocation.md:L16`) |
| SW-8 | Output drivers | "output drivers" (`control_allocation.md:L43`), `mixer_module` (`:L45`) |
| SW-9 | Commander | `commander` (`architecture.md:L135`; `flight_modes.md:L108` path `src/modules/commander/...`) |
| SW-10 | Navigator | `navigator` / `Navigator` (`architecture.md:L59`, `:L93`) |
| SW-11 | Estimators | "estimators for attitude and position" (`architecture.md:L56`), `estimator` (`:L71`); `ekf2_*` topics in `uorb.md:L242-L243` |
| SW-12 | Controllers | `controller` (`architecture.md:L73`), "position controller" (`:L76`), "rate controller" (`control_allocation.md:L36`), "attitude/rate controller" (`:L16`) |
| SW-13 | Collision Prevention | `Collision Prevention` (`collision_prevention.md:L1`, `:L3`) |
| SW-14 | Path Planning Interface (removed) | `Path Planning Interface` (`path_planning_interface.md:L1`, `:L6`), "Obstacle avoidance in Missions", "Safe Landing" (`:L6`); `obstacle_avoidance` (file 13 redirect) |
| SW-15 | PWM_limit state machine | `PWM_limit State Machine` (`pwm_limit.md:L1`, `:L3`) |
| SW-16 | Flight modes | "PX4 internal modes" / "PX4 external (ROS2) modes" (`flight_modes.md:L7`), "PX4 ROS 2 Control Interface" (`:L50`) |
| SW-17 | RTOS / OS | `NuttX` (`architecture.md:L101`, `:L147-L155`), `Linux`, `macOS`, `QuRT` (`architecture.md:L101`) |
| SW-18 | Other modules named | `dataman`, `perf`, `parameter interface` (`architecture.md:L108`), `logger` (`uorb.md:L51` "the logger"), `flight_mode_manager` (`flight_modes.md:L63`, in HTML comment) |

---

## 3. Buses, interfaces, connectors

### 3.1 Hardware buses/interfaces

| ID | Name (verbatim) | Endpoints | Protocol | Quantitative facts (verbatim, location) |
|---|---|---|---|---|
| IF-1 | **Pixhawk Autopilot Bus (PAB)**, connector **X1** ("PAB X1") | FMU/SoM (top) <-> baseboard (bottom) | board-to-board connector, "Mandatory main bus with the critical Pixhawk interfaces." (DS-010 p.8) | "Connector: 100-pos Hirose DF40" (DS-010 p.5); "The 100-pin connector is automotive grade" (p.7); part numbers "Hirose DF40HC(3.0)-100DS-0.4V(58)" (baseboard) / "Hirose DF40C-100DP-0.4V(51)" (autopilot) `[T]` p.7; dimensions baseboard "A = 22.6mm" "B = 19.6mm", autopilot "A = 21.52mm" "B = 19.6mm" `[T]` p.7; full 100-pin pinout `[T]` p.8-9 (see §3.2); "The DF40 connectors establish a 3mm board to board spacing" (p.12) |
| IF-2 | **PAB X2** | FMU/SoM <-> baseboard | "Advanced bus (optional) containing ethernet and external SPI port." (DS-010 p.10) | "Connector: 50-pos Hirose DF40" (p.5); "The 50-pin connector" (p.8); part numbers "Hirose DF40HC(3.0)-50DS-0.4V(51)" / "Hirose DF40C-50DP-0.4V(51)" `[T]` p.8; dimensions "A = 12.6mm" "B = 9.6mm" / "A = 11.52mm" "B = 9.6mm" `[T]` p.8; 50-pin pinout `[T]` p.10 |
| IF-3 | **Ethernet** | FMU <-> mission/companion computer | "100-Base T" `[T]` (DS-010 p.6); RMII signals ETH_* (DS-010 p.12) | DS-012 p.6 "Ethernet interface for high-speed mission computer integration"; X2 pins 2-27 ETH_MDIO, ETH_REF_CLK, ETH_MDC, ETH_POWER_EN, ETH_CRS_DV, ETH_RX_ER, ETH_RXD0, ETH_PHY_nINT, ETH_RXD1, ETH_TXD0, ETH_TXD1, ETH_TX_EN `[T]` (DS-010 p.10); "100 ohms differential for Ethernet" (p.12); RMII signals "kept under 6" in length with length matching to each other within 2"" (p.12); "ETH_TXD0 is ~1" on the FMUM board and ~0.5" longer"; "ETH_TXD0 should be less than 5" in length" (p.12); MCU pins PA1, PA2, PA7, PB11, PB13, PC1, PC4, PC5, PG13, PG15 `[T]` (DS-012 p.18-20). Baseboard "Ethernet Transceiver" section is image-only `[I]` (DS-010 p.18). |
| IF-4 | **USB** | FMU <-> host | USB | "USB-C 480 Mbit" (FMUv6X) `[T]` DS-010 p.6; X1 pins 76 `USB_D_P`, 78 `USB_D_N`, 80 `VBUS_SENSE` `[T]` p.9; MCU AF `USB_OTG_FS_DM/DP/VBUS` (PA11/PA12/PA9) `[T]` DS-012 p.18; "90 ohms differential for USB" (DS-010 p.12) |
| IF-5 | **CAN1, CAN2, CAN3** | FMU <-> CAN peripherals (ESCs, GNSS, smart batteries) | CAN; software "DroneCAN" / "Cyphal" (`can_index.md:L7-L12`); "UAVCAN" (DS-012 p.6) | FMUv6X "CAN 3" `[T]` DS-010 p.6; X1 pins 57/59 `CAN2_TX/RX`, 63/65 `CAN1_TX/RX` `[T]` p.9; X2 pins 40/42 `CAN3_TX/RX` `[T]` p.10; MCU PD0/PD1 CAN1, PB6/PB12 CAN2 `[T]` DS-012 p.18-19 (no CAN3 pins listed); transceivers "TJA1051TK/3", "MCP2562FD-H/MF" (DS-010 p.6, list); "a 120Ω termination resistor" (`can_index.md:L30`); "4 pin JST-GH connectors for CAN" (`can_index.md:L47`); "All Pixhawk flight controllers come with 2 CAN interfaces" (`can_index.md:L57`) — see C-1 |
| IF-6 | **I2C1** "BASE_GPS1_MAG_LED_PM1" | FMU <-> GPS1 mag, LED, power module 1 | I2C | X1 pins 16/18 `I2C1_SDA/SCL_BASE_GPS1_MAG_LED_PM1` `[T]`; MCU PB7/PB8 `[T]` |
| IF-7 | **I2C2** "BASE_GPS2_MAG_LED_PM2" | FMU <-> GPS2 mag, LED, PM2; on-FMU BARO2 | I2C | X1 pins 12/14 `[T]`; MCU PF0/PF1 `[T]`; BARO2 BMP388 `0x76` (Rev 1-4) / CP-20100 `0x63` (Rev 5) on I2C2 `[T]` DS-012 p.11-15 |
| IF-8 | **I2C3** "BASE_MS5611_BARBED_EXTERNAL1" | FMU <-> barbed baro on base | I2C | X1 pins 8/10 `[T]`; MCU PH7/PH8 `[T]`; DS-012 p.6 "High accuracy barbed baro" |
| IF-9 | **I2C4** (internal FMU/IMU-board bus) | FMU <-> SE050, BARO1, MAG1/MAG2, EEPROM, RM3100 | I2C | DS-012 p.6 "(SE050, I2C4)"; MCU PF14/PF15 `I2C4_SCL_FMU`/`I2C4_SDA_FMU`, PG5 `I2C4_DRDY1_BMP388` `[T]`; addresses SE `0x48`, BARO1 `0x77` (BMP388) / `0x63` (ICP-20100), EEPROM `0x50`, BMM150 `0x10`, MAG2 `0x1e` (LIS2MDLTR, Rev 1) / `0x0C` (VCM5883, Rev 2-5), RM3100 `0x20` (Rev 5) `[T]` DS-012 p.11-15. All I2C: "All I2C buses are pulled up with 1.5K Resistors on the FMUM." (DS-010 p.9). Count "I2C 3 (+NFC)" `[T]` DS-010 p.6. NFC: "For NFC one external I2C port needs to have an additional GPIO line and 5V" (DS-012 p.7); `NFC_GPIO` X2 pin 48 `[T]`, MCU PH3 `[T]` |
| IF-10 | **SPI1** (IMU3, power domain 1) | FMU <-> IMU3 | SPI | Table: ICM20649 (Rev 1-2), ICM42670-P (Rev 3-4), ICM42649 (Rev 5); CS1, DRDY1 `[T]` DS-012 p.11-15. Pinout names `SPI1_*_SENSOR1_ICM20602` (PA5, PB4, PB5, PI9, PF2) `[T]` |
| IF-11 | **SPI2** (IMU2, power domain 2) | FMU <-> IMU2 | SPI | ICM-42688-P, CS1, "DRDY2 INT2" `[T]`; pinout names `SPI2_*_SENSOR2_ISM330`/`SPI2_nCS1_ISM330`/`SPI2_DRDY2_ISM330_INT2` (PI1-PI3, PH5, PH12) `[T]` |
| IF-12 | **SPI3** (IMU1, power domain 3) | FMU <-> IMU1 | SPI | BMI088 ACCEL CS1, BMI088 GYRO CS2 "DRDY2 INT3" (Rev 1, 3, 5); "ICM20649 ACCEL" CS1 (Rev 2, 4) `[T]`; pinout `SPI3_*_BMI088`, `SPI3_nCS1_BMI088_ACCEL` (PI4), `SPI3_nCS2_BMI088_GYRO` (PI8), `SPI3_DRDY1_BMI088_INT1_ACCEL` (PI6), `SPI3_DRDY2_BMI088_INT3_GYRO` (PI7) `[T]` |
| IF-13 | **SPI4** | FMU <-> BMM150 (per pinout) | SPI | pinout `SPI4_MOSI/SCK/MISO_SENSOR4_BMM150` (PE6, PE12, PE13), `SPI4_nCS1_BMM150` (PH15), `SPI4_DRDY1_BMM150_DRDY` (PF3) `[T]` — conflicts with tables (C-5) |
| IF-14 | **SPI5** (FRAM) | FMU <-> FRAM FM25V02A-DG | SPI | FRAM "SPI5 CS1", power "FMU VDD 3.3" `[T]` DS-012 p.11-15; pinout `SPI5_*_FRAM`, `SPI5_nCS1_FRAM` (PF7, PF8, PF11, PG7) `[T]`; prose says "FRAM memory for configuration data (SPI2)" and "External sensor bus (SPI5)" (DS-012 p.6-7) — C-3, C-4 |
| IF-15 | **SPI6** "EXTERNAL1" (external SPI port) | FMU <-> external sensor | SPI | X2 pins 31-47 `SPI6_MISO/MOSI/SCK (SWO)/nRESET/nCS1/nCS2/DRDY2/DRDY1_EXTERNAL1` `[T]` DS-010 p.10; MCU PA6, PB3, PG14, PI10, PA15, PF10, PD11, PD12 `[T]`; `SPIX_SYNC` X2 pin 49 / PH10 `[T]` |
| IF-16 | **TELEM1** (UART7, RTS/CTS) | FMU <-> GCS radio by default | UART / MAVLink | X1 pins 22, 24, 64, 66 `[T]`; MCU PE8, PF6, PE9, PE10 `[T]`; "`TELEM1` and `TELEM2` are set up by default to connect via MAVLink to a GCS and a companion computer (respectively)" (`uxrce_dds.md:L282`); `MAV_0_CONFIG` (`:L283`) |
| IF-17 | **TELEM2** (UART5, RTS/CTS) | FMU <-> companion computer by default | UART / MAVLink, or uXRCE-DDS | X1 pins 52, 54, 58, 60 `[T]`; MCU PB9, PD2, PC8, PC9 `[T]`; `UXRCE_DDS_CFG` "such as `TELEM2`" (`uxrce_dds.md:L231`); `SER_TEL2_BAUD` (`:L258-L260`); `MAV_1_CONFIG` (`:L283`) |
| IF-18 | **TELEM3** (USART2, RTS/CTS) | FMU <-> (unspecified) | UART | X1 pins 40, 42, 46, 48 `[T]`; MCU PD5, PA3, PD4, PD3 `[T]` |
| IF-19 | **GPS1** (USART1) / **GPS2** (UART8) | FMU <-> GPS modules | UART (+ I2C1/I2C2 mag) | X1 34/36 `USART1_RX/TX_GPS1`, 28/30 `UART8_TX/RX_GPS2` `[T]`; MCU PB14/PB15, PE0/PE1 `[T]`; DS-012 p.6 "GPS external mag + baro #1", "#2" |
| IF-20 | **DEBUG console** (USART3) | FMU <-> debug adapter | UART | X1 69/71 `USART3_TX/RX_DEBUG` `[T]`; MCU PD8/PD9 `[T]` |
| IF-21 | **UART4** | FMU <-> (unspecified) | UART | X1 96/98 `[T]`; MCU PH13/PH14 `[T]` |
| IF-22 | **FMU <-> IO link** (USART6) | FMU <-> IO co-processor / RC input | UART | X1 70 `USART6_RX_FROM_IO__RC_INPUT`, 72 `USART6_TX_TO_IO__NC` `[T]`; MCU PC7/PC6 `[T]`; `SER_URT6_BAUD` (`uxrce_dds.md:L258`) |
| IF-23 | **SWD debug** | FMU <-> JLink / "Pixhawk Debug Adapter" | SWD | X1 75 `FMU_SWDIO`, 77 `FMU_SWCLK`, 87 `FMU_nRST` `[T]`; MCU PA13, PA14, PE2 `TRACECLK` `[T]`; "Debug 10-pos" (FMUv6X) `[T]` DS-010 p.6; `DS-019:L92` "JLink + Pixhawk Debug Adapter"; "FMU Debug Connector" drawing `[I]` DS-010 p.14 |
| IF-24 | **PWM outputs / capture / RC** | FMU (and IO) -> ESCs/servos; RC receiver -> FMU | PWM / PPM | "PWM out 8 + 8 (IO)" `[T]` DS-010 p.6; X1 FMU_CH1..FMU_CH8 (pins 17,15,13,11,7,5,3,1) `[T]`; X2 FMU_CH9..FMU_CH12 (pins 16,18,20,22) `[T]`; X1 97 `FMU_CAP1`, 39 `FMU_PPM_INPUT` `[T]`; MCU TIM1/TIM4/TIM12 channels for FMU_CH1-8, PI0 `FMU_CAP1`, PI5 `FMU_PPM_INPUT` `[T]`; "RC Inputs" example drawing `[I]` DS-010 p.15 |
| IF-25 | **Power inputs / rails** | power sources -> baseboard -> FMU | power | X1 49/51/53/55 `VDD_5V_IN`, 84/86 `FMU_VDD_3V3`, 29 `V_RTC_BAT`, 89/91/93 `nPOWER_IN_A/B/C`, 35/37 `VDD_5V_PERIPH_nEN/nOC`, 81/83 `VDD_5V_HIPOWER_nEN/nOC`, 33 `VDD_3V3_SPEKTRUM_POWER_EN`, 90 `ADC1_6V6`, 92 `ADC1_3V3` `[T]`; DS-012 p.7 "can be powered from up to three power sources"; "Power input digital" `[T]` DS-010 p.6; "Powerpath Selector", "Peripheral Power Protection" drawings `[I]` DS-010 p.16-17 |
| IF-26 | **Sensor power domains** | FMU power control -> sensor sets | LDO enable + ADC monitor | Enables `VDD_3V3_SENSORS1_EN` (PI11), `..2_EN` (PD15), `..3_EN` (PE7), `..4_EN` (PG8); monitors `SCALED_VDD_3V3_SENSORS1..4` (ADC1_IN0/IN4/IN8/IN12) `[T]` DS-012 p.18-21; "Power Domain" column values 1-4 and "FMU VDD 3.3" `[T]` p.11-15; prose: "Three redundancy domains" (p.6) — C-2 |
| IF-27 | **Smart battery bus** | batteries -> FMU | SMBus / UAVCAN | "Two smart batteries on SMBus or more on UAVCAN" (DS-012 p.6). No SMBus pins named anywhere (G-6). |
| IF-28 | **SD card** | FMU <-> SD card | SDMMC2 | MCU PD6, PD7, PG9-PG12 `SDMMC2_*`; PC13 `VDD_3V3_SD_CARD_EN` `[T]` |
| IF-29 | **HW VER/REV sense** | baseboard VER divider / FMU REV divider -> FMU ADC | ADC + EEPROM | X1 25 `HW_VER_REV_DRIVE`, 27 `HW_VER_SENSE` `[T]`; MCU PF4 `HW_VER_SENSE`, PF5 `HW_REV_SENSE`, PG0 `HW_VER_REV_DRIVE` `[T]`; resistor and DN tables in `DS-019:L18-L54` (Markdown table, not PDF) |
| IF-30 | **IMU connector** (FMUM <-> IMU board, flex) | FMU <-> IMU board | — | Pinout exists only as image `[I]` DS-012 p.17 (caption "IMU Pinout of FMUM's IMU connector (FMUvxX)") |
| IF-31 | **Safety switch, LEDs, buzzer, heater, armed** | FMU <-> baseboard | GPIO/PWM | X1 21 `FMU_SAFETY_SWITCH_IN`, 23 `FMU_nSAFETY_SWITCH_LED_OUT`, 4 `BUZZER_1`, 85 `nARMED` `[T]`; MCU PE3-PE5 `nLED_RED/GREEN/BLUE`, PB10 `HEATER` `[T]` |
| IF-32 | **Telemetry radio / LTE** | GCS <-> FMU or companion | MAVLink | "Telemetry radios for connecting to a ground station computer/software." (`px4_systems_architecture.md:L24`); "cloud or ground station connection via LTE" (`:L50`) |

Pixhawk connector standard (external connectors): "Pixhawk connector standards v2 (2015-)" "Connector: JST GH" (DS-010 p.5); obsolete v1 "Connector: Hirose DF13" (p.5). "DS-009 Pixhawk Connector Standard" is referenced (DS-012 p.5) but not in the corpus.

### 3.2 Software/logical interfaces

| ID | Name | Endpoints | Protocol | Quantitative facts (verbatim, location) |
|---|---|---|---|---|
| SI-1 | **uORB** | PX4 modules <-> modules (intra-FMU) | "asynchronous `publish()` / `subscribe()` messaging API" (`uorb.md:L5`); "based on shared memory" (`architecture.md:L104`) | `uorb top` example: "update: 1s, num topics: 77" (`uorb.md:L235`); column "publishing frequency in Hz" (`:L252`); example rows e.g. `sensor_accel 0 1 242 0 1`, `sensor_baro 0 1 42 0 1`, `battery_status 0 6 500 2694 1` (`:L237-L249`); "listener sensor_accel 5" (5 messages, `:L189`) |
| SI-2 | **MAVLink** | FMU <-> GCS, MAVSDK, companion, cameras, gimbal, OSD | MAVLink 2 ("generates the MAVLink 2 C header files", `mavlink.md:L78`) | Command "include up to 7 numeric "param" values" (`:L26`); example message id `11514` (`:L132`, `:L141`); example stream "50Hz on UDP port `14556`" (`:L353-L356`); "stream CA_TRAJECTORY at 5 Hz" (`:L342`, code `configure_stream_local("BATTERY_STATUS_DEMO", 5.0f)` `:L347`); "mavlink stream -u 14556 -s CA_TRAJECTORY -r 300" (`:L497`); modes `MAVLINK_MODE_NORMAL`, `_ONBOARD`, `_ONBOARD_LOW_BANDWIDTH`, `_GIMBAL`, `_EXTVISION`, `_EXTVISIONMIN`, `_OSD`, `_CUSTOM`, `_MAGIC`, `_CONFIG`, `_MINIMAL`, `_IRIDIUM` (`:L328-L339`) |
| SI-3 | **uXRCE-DDS bridge** | `uxrce_dds_client` (FMU) <-> micro XRCE-DDS agent (companion) <-> ROS 2 / DDS | XRCE-DDS "over a serial or UDP link" (`uxrce_dds.md:L20`); "PX4 only supports UDP and serial connections" (`:L197`) | UDP port "`8888`" default (`:L203`, `:L226`, `:L236`, `:L306`); `UXRCE_DDS_AG_IP` default "`2130706433`" = "`127.0.0.1`" (`:L240`); serial example "-b 921600" (`:L213`); agent "v2.4.2" (`:L68`); client "based on version `v2.x` which is not compatible with the latest `v3.x` Agent" (`:L60`); "By default, ROS 2 operates on ID 0" (`:L271`); example domain `3`, port `9999` (`:L315`); example host `192.168.0.100:8888` (`:L295`); namespaces `/fmu/in/`, `/fmu/out/` (`:L464-L466`); PX4 publisher QoS TRANSIENT_LOCAL / BEST_EFFORT / KEEP_LAST / depth 0 (`:L390-L395`); subscriber VOLATILE / BEST_EFFORT / KEEP_LAST / depth queue_depth (`:L401-L406`); ROS 2 default "queue size of 10", "reliable", "volatile" (`:L409`) |
| SI-4 | **Control allocation pipeline** | rate controller -> `control_allocator` -> output drivers | uORB topics | "The rate controller outputs torque and thrust setpoints" (`control_allocation.md:L36`); `actuator_motors` (`:L48`); `PWM_MAIN_FUNCx` (`:L47-L48`); pipeline diagram `[I]` (`:L32`) |
| SI-5 | **OBSTACLE_DISTANCE** feed | companion/external sensor -> FMU (Collision Prevention) | MAVLink `OBSTACLE_DISTANCE` or ROS 2 (`collision_prevention.md:L171`, `:L319`) | test "4 m/s with `OBSTACLE_DISTANCE` messages being emitted at 10Hz (the maximum rate supported by the vision system)" (`:L174`) |
| SI-6 | **Path planning interface** (removed) | FMU <-> companion planner | MAVLink (API only in v1.14 docs) | none in corpus (`path_planning_interface.md:L17-L21`) |
| SI-7 | **Standard Modes Protocol** | GCS <-> FMU | MAVLink | "from PX4 v1.15" (`flight_modes.md:L119`) |
| SI-8 | **CAN application protocols** | FMU <-> CAN nodes | DroneCAN (= UAVCAN v0), Cyphal (= UAVCAN v1) | "In 2022 the project split into two" (`can_index.md:L17`); KDECAN not supported (`:L22`) |

### 3.3 Facts carried only by images/diagrams `[I]`

Not expected from text extraction; only the caption/alt text/heading below is extractable.

| # | Location | Extractable caption / surrounding text | What only the image carries |
|---|---|---|---|
| I-1 | DS-012 p.6 | heading "Detailed Block Diagram" | FMUv6X block diagram (bus-to-device wiring as a picture) |
| I-2 | DS-012 p.8 | "Top view of FMUM"; "Locate centers of Red, Green, and Blue LEDs and SD Card connector pin 1 as shown in the Top View FMUM above." | LED and SD-card pin-1 positions, mechanical dimensions |
| I-3 | DS-012 p.9 | "Bottom view of FMU SOM" | bottom-side layout |
| I-4 | DS-012 p.10 | heading "Sensors Locations" | sensor placement; IMU flex pin-1 location ("can be seen in FMUv6X Sensors Locations section", p.17) |
| I-5 | DS-012 p.17 | "IMU Pinout of FMUM's IMU connector (FMUvxX)" | the whole IMU-connector pinout (pin count, signals) |
| I-6 | DS-010 p.7-8 | "Dimensions" with "A = ..." "B = ..." text | connector drawings (A/B values are text; the meaning of A/B is only in the drawing) |
| I-7 | DS-010 p.11 | "Top view of FMUM", "Bottom view of FMU SOM" | mechanical drawings |
| I-8 | DS-010 p.13 | "Base to FMU Connectors (X1, X2)" | reference schematic |
| I-9 | DS-010 p.14 | "FMU Debug Connector" | debug connector pinout (10-pos per p.6 table) |
| I-10 | DS-010 p.15 | "RC Inputs" | RC input circuit |
| I-11 | DS-010 p.16 | "Powerpath Selector" | power-path circuit (3 inputs) |
| I-12 | DS-010 p.17 | "Peripheral Power Protection" | protection circuit, current limits |
| I-13 | DS-010 p.18 | "Baseboard EEPROM and Sensor Connections"; "Ethernet Transceiver" | EEPROM/sensor wiring, PHY part and wiring |
| I-14 | DS-010 p.19 | "GPS / Audio Interface" | GPS/buzzer circuit |
| I-15 | `architecture.md:L20` | `![PX4 Architecture](../../assets/diagrams/PX4_Architecture.svg)`; "The top part of the diagram contains middleware blocks, while the lower section shows the components of the flight stack." | module graph and "information flow for the _most important_ connections between the modules" (`:L39`) |
| I-16 | `architecture.md:L61` | `PX4 High-Level Flight Stack` | pipeline sensors/RC/Navigator -> Actuators |
| I-17 | `px4_systems_architecture.md:L14` | `PX4 architecture - FC only system` | FC-only HW/SW mapping |
| I-18 | `px4_systems_architecture.md:L37` | `PX4 architecture - FC + Companion Computer` | FC + companion links (incl. LTE, `:L50`) |
| I-19 | `control_allocation.md:L24`, `:L32` | `Mixing Overview`, `Pipeline Overview` | module/topic pipeline |
| I-20 | `flight_modes.md:L106` | `Mode requirements diagram` | `mode_req_manual_control` flow |
| I-21 | `can_index.md:L36` | `CAN Wiring`; "a CAN bus connecting a flight controller to 4 CAN ESCs and a GNSS" | topology/termination placement |
| I-22 | `pwm_limit.md:L27` | `PWM Limit state machine diagram` | **all state transitions** of INIT/OFF/RAMP/ON (text lists states only) |
| I-23 | `DS-019:L16` | `rev-ver-sensing`; "The diagram shows R1 and R2, with the signals `HW_xxx_SENSE` and `HW_VER_REV_DRIVE`." | divider topology (which resistor is high-side) |
| I-24 | `uxrce_dds.md:L23` | `Architecture uXRCE-DDS with ROS 2` | client/agent/DDS topology |
| I-25 | `collision_prevention.md:L151`, `:L297`, `:L190` | `Scalefactor`; `Obstacle_Distance Angles`; `RViz image ...` | scale curve (its generating Python is in an HTML comment `:L322-L350`), sector angle convention |

---

## 4. Numeric values

### 4.1 Hardware (DS-012, DS-010, DS-019)

| Quantity | Value (verbatim) | Location | Note |
|---|---|---|---|
| FMUv6X MCU | STM32H753 | DS-010 p.6 | `[T]`; DS-012 p.5 says "STM32H7" |
| FMUv6X clock | 480 MHz | DS-010 p.6 | `[T]`; p.5 text "400-600 MHz H7" (C-6) |
| FMUv6X RAM | 1 MB | DS-010 p.6 | `[T]` |
| FMUv6X FLASH | 2MB | DS-010 p.6 | `[T]` |
| FMUv6X stable date | 02/2021 | DS-010 p.6 | `[T]` |
| UART (RTS/CTS) count, FMUv6X | 3 | DS-010 p.6 | `[T]` |
| UART (2-wire) count, FMUv6X | 4 | DS-010 p.6 | `[T]` |
| Debug connector, FMUv6X | 10-pos | DS-010 p.6 | `[T]` |
| Ethernet, FMUv6X | 100-Base T | DS-010 p.6 | `[T]` |
| USB, FMUv6X | USB-C 480 Mbit | DS-010 p.6 | `[T]` |
| CAN count, FMUv6X | 3 | DS-010 p.6 | `[T]` (C-1) |
| I2C count, FMUv6X | 3 (+NFC) | DS-010 p.6 | `[T]` |
| Power input, FMUv6X | digital | DS-010 p.6 | `[T]` |
| PWM out, FMUv6X | 8 + 8 (IO) | DS-010 p.6 | `[T]` (C-8) |
| Same row set for FMUv5 / FMUv5X / FMUv6U | e.g. FMUv5 "STM32F765", "200 MHz", "512 KB", "2MB", "06/2018", CAN "2", USB "2.0 12 MBit", PWM "6 + 8 (IO)"; FMUv5X "06/2019", "8 + 8 (IO)", CAN "2"; FMUv6U "STM32H753", "480 MHz", "12/2020", CAN "1", PWM "8" | DS-010 p.6 | `[T]` |
| FMU generation list | "FMUv1: No product name (2012, 168 MHz M4)" ... "FMUv5: Pixhawk 4 (2018, 200 MHz M7)", "FMUv5X: Pixhawk 5X (2021 , 200 MHz M7,, redund. sensors)", "FMUv6U: Pixhawk 6U (2021 , 400-600 MHz H7)", "FMUv6X: Pixhawk 6X (2021, 400-600 MHz H7,, redund. sensors)" | DS-010 p.5 | list text |
| PAB X1 pin count | 100-pos / 100-pin | DS-010 p.5, p.7 | |
| PAB X2 pin count | 50-pos / 50-pin | DS-010 p.5, p.8 | |
| X1 connector dims | baseboard A = 22.6mm, B = 19.6mm; autopilot A = 21.52mm, B = 19.6mm | DS-010 p.7 | `[T]` |
| X2 connector dims | baseboard A = 12.6mm, B = 9.6mm; autopilot A = 11.52mm, B = 9.6mm | DS-010 p.8 | `[T]` |
| Board-to-board spacing | 3mm | DS-010 p.12 | |
| I2C pull-ups | 1.5K Resistors | DS-010 p.9 | note under X1 table |
| Impedance | "50 ohms single ended, 90 ohms differential for USB, and 100 ohms differential for Ethernet" | DS-010 p.12 | |
| Diff-pair clearance | "3x the gap separation" | DS-010 p.12 | |
| RMII trace length | "under 6"", matched "within 2""; ETH_TXD0 "~1"" on FMUM, "~0.5"" longer; "less than 5"" on base | DS-010 p.12 | |
| Mounting holes | "four 2.0 mm mounting holes with 3.6 mm pads" | DS-010 p.12 | |
| VDD_5V_IN trace | "at least 1mm", vias "at least 2" | DS-010 p.13 | |
| Case outline | "no greater than 31.8mm x 38.8mm"; "a total of 1.4mm" for wall + clearance | DS-012 p.8; DS-010 p.12 | identical text in both |
| Operating temperature | "-40 to +85°C" | DS-012 p.6 | |
| Redundancy domains | "Three redundancy domains" | DS-012 p.6 | (C-2) |
| Power sources | "up to three power sources" | DS-012 p.7 | X1 `nPOWER_IN_A/B/C` |
| Smart batteries | "Two smart batteries on SMBus" | DS-012 p.6 | |
| Rails monitored | "5V rail", "3.3V rail ... for CPU", "3.3V rail ... for each sensor domain" | DS-012 p.7 | |
| NFC supply | "5V" | DS-012 p.7 | |
| Oscillators | `16_MHZ_IN`/`16_MHZ_OUT` (PH0/PH1); `32KHZ_IN`/`32KHZ_OUT` (PC14/PC15) | DS-012 p.19-20 | `[T]` |
| MCU pinout rows | 140 rows, ports PA0-PI11 | DS-012 p.18-21 | `[T]` |
| Sensor-set revisions | Rev 1 ... Rev 5 | DS-012 p.11-15 | text says "(Rev 1, Rev 2, Rev 3)" p.11 |
| I2C addresses | SE050 0x48; BMP388 BARO2 0x76 "@00=0x50"; BMP388 BARO1 0x77 "@00=0x50"; CP-20100/ICP-20100 0x63 "@0xefc8=0x08"; EEPROM 0x50; BMM150 0x10; LIS2MDLTR 0x1e; VCM5883 0x0C; RM3100 0x20 | DS-012 p.11-15 | `[T]` |
| Power domain per device | IMU3 1; BARO2 2; FRAM "FMU VDD 3.3"; SE 4; IMU1 3; IMU2 2; BARO1, MAGs, EEPROM, RM3100 4 | DS-012 p.11-15 | `[T]` |
| Document revisions | DS-012: 0.1.0, 0.2.0, 0.3.0 (7/23/2022), 0.4.0 (12/08/2022). DS-010: 0.1.0 ... 0.7.0 ("Updated PAB Pinout to RC03") | DS-012 p.3; DS-010 p.3 | `[T]` |
| VER/REV resistor configs | Config 0 "NO POP"; 1: R1 442 K / R2 24.9 K; ... 10: 24.9 / 442 (11 configs) | `DS-019:L18-L30` | Markdown table |
| VER/REV ADC windows | e.g. "0 \| 204 \| 0 \| 0.166 \| 0" ... "\| 3946 \| 4095 \| 3.147 \| 3.363" (DN and V) | `DS-019:L32-L54` | Markdown table; max DN 4095 |
| EEPROM escape code | "0x007" | `DS-019:L61`, `:L63` | |
| Board IDs | base 0x000-0x00a, 0x010, 0x011, 0x100, 0x150, 0x200; FMU v6x revisions 0x000-0x00a, 0x010 | `DS-019:L147-L198` | |

### 4.2 Software / comms / behaviour

| Quantity | Value (verbatim) | Location |
|---|---|---|
| IMU sampling / publish | "sample the data at 1kHz, integrate it and publish with 250Hz" | `architecture.md:L92` |
| Example task | priority "SCHED_PRIORITY_DEFAULT + 40", stack "3600" | `architecture.md:L137-L138` |
| Execution modes | "There are 2 different ways that a module can be executed" | `architecture.md:L111` |
| uORB timestamp unit | "time since system start (microseconds)" | `uorb.md:L64` |
| uorb top | "update: 1s, num topics: 77"; per-topic #MSG e.g. 242, 249, 42, 500, 98, 4 | `uorb.md:L235-L249` |
| MAVLink command params | "up to 7 numeric "param" values" | `mavlink.md:L26` |
| MAVLink example | id "11514"; "50Hz on UDP port `14556`"; "5 Hz"; "-r 300" | `mavlink.md:L141`, `:L353`, `:L342`, `:L497` |
| uXRCE-DDS UDP port | "8888" (default) | `uxrce_dds.md:L203`, `:L226`, `:L236`, `:L306` |
| Agent IP default | "2130706433" = "127.0.0.1" | `uxrce_dds.md:L240` |
| Serial baud example | "921600" | `uxrce_dds.md:L213` |
| Agent version | "v2.4.2"; client "v2.x", agent "v3.x" incompatible | `uxrce_dds.md:L68`, `:L60` |
| DDS domain | "ROS 2 operates on ID 0"; example "3", port "9999" | `uxrce_dds.md:L271`, `:L315` |
| QoS depth | publishers ".depth = 0"; ROS 2 default "queue size of 10" | `uxrce_dds.md:L394`, `:L409` |
| subscriptions_multi example | "already `2` instances ... instance `3`" | `uxrce_dds.md:L483` |
| CAN termination | "120Ω" | `can_index.md:L30` |
| CAN connector | "4 pin JST-GH" | `can_index.md:L47` |
| CAN interfaces per FC | "2 CAN interfaces" | `can_index.md:L57` |
| CAN example topology | "4 CAN ESCs and a GNSS" | `can_index.md:L34` |
| Collision prevention sectors | "36 sectors" / "72 sectors" / "72 Sectors" | `collision_prevention.md:L69` / `:L137` / `:L291` (C-9) |
| Bin example | "9.99° to 10.01°" -> bins "5° and 10°", arc "2.5° and 12.5°" | `collision_prevention.md:L311` |
| Range-data timeout | "longer than 0.5s" -> xy velocity setpoints zero | `collision_prevention.md:L90-L91` |
| Hold switch | "After 5 seconds of not receiving any data" -> HOLD mode | `collision_prevention.md:L92` |
| Sensor delay | direct "can be assumed to be 0"; external vision "as high as 0.2s" | `collision_prevention.md:L108-L109` |
| Tracking delay | "typically between 0.1 and 0.5 seconds" | `collision_prevention.md:L113` |
| Guidance angle | "30 degrees is a good balance" | `collision_prevention.md:L127` |
| SF45 coverage | "320 degree sensing" | `collision_prevention.md:L43` |
| Companion test | "4 m/s", "10Hz (the maximum rate supported by the vision system)" | `collision_prevention.md:L174` |
| CP_GO_NO_DATA | "Set to 1 ... (default is 0/`False`)" | `collision_prevention.md:L64` |
| CP_DIST | "Set negative to disable" | `collision_prevention.md:L61` |
| Orientation example | "-R 25" = "ROTATION_DOWNWARD_FACING" | `collision_prevention.md:L51` |
| Distance sentinel | "65535" (Lua) | `collision_prevention.md:L235` |
| Airframe example | "CA_AIRFRAME 4", "CA_ROTOR_COUNT 2", "CA_ROTOR0_KM -0.05", "CA_ROTOR0_PY 0.2" | `control_allocation.md:L71-L74` |
| Mode/feature versions | external modes "first appeared in PX4 v1.15"; Standard Modes "from PX4 v1.15"; control allocation replaces mixing of "v1.13 and earlier"; uXRCE-DDS "PX4 v1.14"; path planning "Removed PX4 v1.15"; message versioning "PX4 v1.16+" | `flight_modes.md:L50`, `:L119`; `control_allocation.md:L4`; `uxrce_dds.md:L3`; `path_planning_interface.md:L3`; `uorb.md:L119` |

---

## 5. Requirements and normative statements

| ID | Statement (verbatim) | Location | Kind |
|---|---|---|---|
| R-1 | "This list describes the mandatory external interfaces." (introduces the FMUv5/5X/6U/6X table) | DS-010 p.6 | interfaces |
| R-2 | "The usage of this bus is mandatory for all System-on-Module(SOM) designs . However, if autopilot and baseboard are integrated into one unit, this connector pair (PAB X1 and PAB X2) can be omitted." | DS-010 p.7 | interface |
| R-3 | "Mandatory main bus with the critical Pixhawk interfaces." (X1) | DS-010 p.8 | interface |
| R-4 | "Advanced bus (optional) containing ethernet and external SPI port." (X2) | DS-010 p.10 | interface |
| R-5 | "The outer case dimensions shall be no greater than 31.8mm x 38.8mm." | DS-012 p.8; DS-010 p.12 | mechanical |
| R-6 | "It is necessary to provide heat sinking of the FMUM microprocessor." | DS-012 p.8; DS-010 p.12 | thermal |
| R-7 | "Implementations of the standard must be compliant with the full specification." | DS-012 p.4; DS-010 p.4 | compliance |
| R-8 | "The trademark shall only be used for compliant products and in combination with a signed adopter agreement." | DS-012 p.4; DS-010 p.4 | compliance |
| R-9 | "The following CAN transceivers are recommended. ● TJA1051TK/3 ● MCP2562FD-H/MF" | DS-010 p.6 | interface |
| R-10 | "The base board should be impedance controlled with 50 ohms single ended, 90 ohms differential for USB, and 100 ohms differential for Ethernet." | DS-010 p.12 | interface/PCB |
| R-11 | "Impedance controlled signal traces should not be routed such that they cross a split in their respective reference planes." | DS-010 p.12 | PCB |
| R-12 | "Ethernet RMII interface signals ... should each be kept under 6" in length with length matching to each other within 2"." / "ETH_TXD0 should be less than 5" in length" | DS-010 p.12 | interface |
| R-13 | "Port protection diodes and series resistors should be placed close to the connectors they are providing protection for." | DS-010 p.12 | protection |
| R-14 | "The impedance from the input connector, through power path selector, to VDD_5V_IN should be given special attention to reduce voltage drops. Keep traces wide (at least 1mm) and use multiple vias when changing layers (at least 2)." | DS-010 p.13 | power |
| R-15 | "The design examples are not part of the formal specification and implementing the board differently is permitted." | DS-010 p.13 | scope |
| R-16 | "For NFC one external I2C port needs to have an additional GPIO line and 5V to supply the external NFC reader." | DS-012 p.7 | interface |
| R-17 | "Redundant sensors on separate buses, allowing continuous operation while losing a complete redundancy domain." | DS-012 p.6 | safety/redundancy (feature claim) |
| R-18 | "every sensor set is powered by an independent LDO with independent power control" | DS-012 p.7 | safety/power |
| R-19 | "Note: device names may reflect legacy devices names. What matters is the BUSn,CSn,DRDYn and the INTn." | DS-012 p.16 | interface naming rule |
| R-20 | "To comply with the standard, both software maintainers as well as hardware manufacturers must agree to follow the same guidelines." | `DS-019:L3` | compliance |
| R-21 | "this desire shall be indicated to the PX4 board support team by issuing a PR to this repository" | `DS-019:L66` | process |
| R-22 | "The project is no longer accepting resistor-based REV/VER pairs and is moving all new hardware to use EEPROM logic." | `DS-019:L59` | HW identification |
| R-23 | "At either end of the chain, a 120Ω termination resistor should be connected between the two data lines." | `can_index.md:L30` | interface |
| R-24 | "if your peripherals support 2 CAN interfaces as well, it is recommended to wire both up for increased safety." | `can_index.md:L57` | safety |
| R-25 | "All message definitions **must** include the `uint64_t timestamp` field" | `uorb.md:L50` | interface |
| R-26 | "All _versioned_ messages definitions must include the `uint32 MESSAGE_VERSION` field." | `uorb.md:L55` | interface |
| R-27 | "the topic needs to be advertised and published outside of an interrupt context (at least once) before it can be published in an interrupt context." | `uorb.md:L155` | interface |
| R-28 | "Make sure not to mix `orb_advertise_multi` and `orb_advertise` for the same topic!" | `uorb.md:L169` | interface |
| R-29 | "Removed or renamed messages must be added to the `deprecated_msgs` list ... Removed or renamed fields must be commented and marked as deprecated. ... the field must be renamed as well" | `uorb.md:L111-L115` | interface versioning |
| R-30 | "in order to communicate both ends of the communication must use libraries generated from the same XML definition." | `mavlink.md:L54` | interface |
| R-31 | "To be part of a PX4 release, any MAVLink definitions that you use must be in `common.xml`" | `mavlink.md:L70` | interface |
| R-32 | "It should be used for development only!" (DEBUG_VECT repurposing) | `mavlink.md:L461` | interface |
| R-33 | "PX4 Micro XRCE-DDS Client is based on version `v2.x` which is not compatible with the latest `v3.x` Agent version." | `uxrce_dds.md:L60` | interface compatibility |
| R-34 | "ROS 2 applications need to be built in a workspace that includes the _same_ message definitions that were used to create the uXRCE-DDS client module" | `uxrce_dds.md:L45` (also `:L330`) | interface |
| R-35 | "PX4 only supports UDP and serial connections." / "You should create a single instance of the agent for each channel" | `uxrce_dds.md:L197`, `:L200` | interface |
| R-36 | "The IP address must be provided in `int32` format as PX4 does not support string parameters." | `uxrce_dds.md:L239` | interface |
| R-37 | "each client should have a unique non-zero key" (multi-client, single agent) | `uxrce_dds.md:L266` | interface |
| R-38 | "To use these ports you must first disable the existing configuration" | `uxrce_dds.md:L280` | interface |
| R-39 | "The topic _base name_, which **must** coincide with the desired uORB topic name" | `uxrce_dds.md:L460` | interface |
| R-40 | "PX4 QoS settings for publishers are incompatible with the default QoS settings for ROS 2 subscribers." | `uxrce_dds.md:L383` | interface |
| R-41 | "All the tasks must behave co-operatively as they cannot interrupt each other." / "_work queue tasks_ are not allowed to sleep or poll on a message, or do blocking IO" | `architecture.md:L116`, `:L121` | SW architecture |
| R-42 | "It should also have some form of real-time scheduling (e.g. FIFO)." | `architecture.md:L102` | runtime |
| R-43 | "PX4 external modes cannot be used in the following cases: ... Modes that require low-level access, strict timing, and/or high update rate requirements. ... Safety critical modes, such as Return mode." | `flight_modes.md:L30-L36` | safety/allocation |
| R-44 | "If the condition of restriction is not met: - arming is not allowed, while the mode is selected - when already armed, the mode cannot be selected - when armed and the mode is selected, the relevant failsafe is triggered" | `flight_modes.md:L97-L101` | safety |
| R-45 | "Process termination of a ROS 2 mode results in a fallback to an internal flight mode" / "if the ROS 2 mode crashes the original mode will be engaged." | `flight_modes.md:L44`, `:L46` | safety |
| R-46 | "modes in PX4 source code must explicitly specify them." | `flight_modes.md:L114` | SW |
| R-47 | "If there is no sensor data in a particular direction, velocity in that direction is restricted to 0" | `collision_prevention.md:L78` (also `:L143`) | safety |
| R-48 | "If the autopilot does not receive range data from any sensor for longer than 0.5s, it will output a warning ... This will force the velocity setpoints in xy to zero. After 5 seconds of not receiving any data, the vehicle will switch into HOLD mode." | `collision_prevention.md:L90-L92` | safety |
| R-49 | "The velocity is restricted such that the vehicle will stop in time to maintain the distance specified in CP_DIST." | `collision_prevention.md:L74` | safety |
| R-50 | "The guidance feature will never direct the vehicle in a direction without sensor data." | `collision_prevention.md:L130` | safety |
| R-51 | "it needs to supply a stream of OBSTACLE_DISTANCE messages" / "The minimum rate at which messages _must_ be sent depends on vehicle speed" | `collision_prevention.md:L171`, `:L173` | interface |
| R-52 | "The **Path Planning Interface**, along with the features **Obstacle avoidance in Missions** and **Safe Landing** are no longer supported or maintained, and _should not_ be used in any PX4 version." | `path_planning_interface.md:L6` | scope |
| R-53 | PWM_limit: "armed: asserted to enable dangerous behaviors such as spinning propellers"; "assertion of pre-armed immediately forces behavior of state ON, regardless of current state"; "INIT and OFF - pwm outputs set to disarmed values."; "RAMP - pwm outputs ramp from disarmed values to min values."; "ON - pwm outputs set according to control values." | `pwm_limit.md:L10-L23` | safety/states |
| R-54 | "Provides a delay between assertion of "armed" and a ramp-up of throttle" | `pwm_limit.md:L4` | safety |
| R-55 | control_allocator "handles motor failures" | `control_allocation.md:L40` | safety |
| R-56 | "PX4 does not support other CAN software protocols for drones such as KDECAN" | `can_index.md:L22` | scope |

---

## 6. Inconsistencies and conflicts

| ID | Topic | Statement A (verbatim, location) | Statement B (verbatim, location) | Kind |
|---|---|---|---|---|
| C-1 | CAN interface count | "All Pixhawk flight controllers come with 2 CAN interfaces" (`can_index.md:L57`) | FMUv6X "CAN 3", FMUv6U "CAN 1" `[T]` (DS-010 p.6); X2 carries `CAN3_TX/RX` (DS-010 p.10) | cross-doc value |
| C-2 | Number of sensor redundancy/power domains | "Three redundancy domains: Completely isolated sensor domains with separate buses and separate power control." (DS-012 p.6) | Power Domain column uses 1, 2, 3, 4 plus "FMU VDD 3.3" `[T]` (DS-012 p.11-15); four enables `VDD_3V3_SENSORS1..4_EN` and monitors `SCALED_VDD_3V3_SENSORS1..4` `[T]` (p.18-21) | intra-doc value |
| C-3 | FRAM bus | "FRAM memory for configuration data (SPI2)" (DS-012 p.6) | "U9 (FRAM) FM25V02A-DG SPI5" `[T]` (p.11-14), "U7 (FRAM) ... SPI5" (p.15); pinout `SPI5_*_FRAM` (p.20) | intra-doc |
| C-4 | External sensor bus | "External sensor bus (SPI5)" (DS-012 p.7) | External SPI is `SPI6_*_EXTERNAL1` (DS-012 p.18-21; DS-010 X2 p.10 "external SPI port"); SPI5 is FRAM (C-3) | intra-doc / cross-doc |
| C-5 | Device names per bus (tables vs pinout) | Tables: SPI1 = ICM20649 / ICM42670-P / ICM42649; SPI2 = ICM-42688-P; BMM150 on "I2C4 0x10" `[T]` (DS-012 p.11-15) | Pinout: `SPI1_*_ICM20602`, `SPI2_*_ISM330`, `SPI4_*_BMM150`, `I2C4_DRDY1_BMP388` (p.18-21); acknowledged: "device names may reflect legacy devices names" (p.16) | intra-doc naming (BMM150 bus is a real bus conflict: I2C4 vs SPI4) |
| C-6 | FMUv6X clock | "FMUv6X: Pixhawk 6X (2021, 400-600 MHz H7,, redund. sensors)" (DS-010 p.5) | "Clock ... 480 MHz" `[T]` (DS-010 p.6) | intra-doc value |
| C-7 | FMUv5X date | "FMUv5X: Pixhawk 5X (2021 , 200 MHz M7,, redund. sensors)" (DS-010 p.5) | FMUv5X "Stable 06/2019" `[T]` (DS-010 p.6) | intra-doc value |
| C-8 | PWM output count | "PWM out 8 + 8 (IO)" `[T]` (DS-010 p.6); MCU pinout lists only FMU_CH1..FMU_CH8 (DS-012 p.18-21) | X2 adds "FMU_CH9" ... "FMU_CH12" (DS-010 p.10) | cross-doc |
| C-9 | Collision-prevention sector count | "an internal representation of 36 sectors around the vehicle" (`collision_prevention.md:L69`) | "an internal representation of 72 sectors" (`:L137`), "divides the plane around the drone into 72 Sectors" (`:L291`); 5° bin example (`:L311`) | intra-doc value (two "Algorithm Description" sections) |
| C-10 | Collision-prevention position-mode setting | "It can be enabled for multicopter vehicles when using acceleration-based Position mode" (`collision_prevention.md:L4`) | MPC_POS_MODE: "Set to `Direct velocity` or `Smoothed velocity` to enable Collision Prevention in Position Mode (default is `Acceleration based`)." (`:L65`) | intra-doc |
| C-11 | MAVLink streaming example | "you could just add the message to the `MAVLINK_MODE_NORMAL` case ... For example, to stream CA_TRAJECTORY at 5 Hz:" (`mavlink.md:L341-L342`) | code: "case MAVLINK_MODE_CONFIG: // USB ... configure_stream_local("BATTERY_STATUS_DEMO", 5.0f);" (`:L345-L348`) | intra-doc |
| C-12 | CAN protocol naming | "Two smart batteries on SMBus or more on UAVCAN" (DS-012 p.6) | "the original version of UAVCAN (UAVCAN v0) was renamed to DroneCAN, and the newer UAVCAN v1 was renamed Cyphal" (`can_index.md:L17`) | cross-doc alias (UAVCAN is ambiguous) |
| C-13 | FMUv4X / Pixhawk 3 Pro | "FMUv4X: Pixhawk 3 Pro (2017, ...)" (DS-010 p.5) | "Drotek Pixhawk 3 Pro (FMUv4pro)" (`autopilot_pixhawk_standard.md:L25`) | cross-doc alias |
| C-14 | FMU family coverage | DS-010 p.5-6 define FMUv6U ("Pixhawk 6U") and no FMUv6C / FMUv6X-RT | `autopilot_pixhawk_standard.md:L13-L21` lists FMUv6X-RT and FMUv6C boards, no FMUv6U | cross-doc |
| C-15 | "FMUv6" identity | "The FMUv6X generation brings the proven features from FMUv6 to a hardened form factor." (DS-012 p.6) — FMUv6 as a predecessor | "the FMUv6 [default.px4board](.../boards/px4/fmu-v6x/default.px4board)" (`uorb.md:L177`) — FMUv6 used for fmu-v6x; DS-012 p.5 "FMUv6X has the same architecture as v5X" | cross-doc alias |
| C-16 | Sensor-set revision count | "These are revisioned in pairs. (Rev 1, Rev 2, Rev 3)" (DS-012 p.11) | Rev 4 and Rev 5 tables (p.14-15); rev 0.3.0 "Add Sensor Set 5" (p.3) | intra-doc |
| C-17 | FMUv6X revision ids vs sensor sets | DS-012 defines sensor sets Rev 1-5 only | DS-019 lists "Holybro FMUv6x rev6" (0x006) and "Holybro FMUv6x rev8" (0x008), `DS-019:L193`, `:L195`; no mapping from DS-019 revision values to DS-012 sensor-set revs | cross-doc |
| C-18 | BARO part number in Rev 5 | FMU set "U5 (BARO2) CP-20100" `[T]` (DS-012 p.15) | IMU set "U2 (BARO1) ICP-20100" `[T]` (p.15) | intra-doc spelling |
| C-19 | IMU3 part in Rev 5 | "U9 (IMU3) ICM42649" `[T]` (DS-012 p.15) | Rev 1-2 "ICM20649"; summary "TDK Invensense ICM-20649 (TBC)" (p.6) | intra-doc (likely typo, unresolved by corpus) |
| C-20 | Rev 5 reference designators | IMU board "U2 (MAG2)" and "U2 (BARO1)"; "U6 (IMU2) Not installed" and "U6 BMM150" `[T]` (DS-012 p.15) | same designator for two parts | intra-doc |
| C-21 | Sensor list vs tables | Summary sensors (DS-012 p.6): "Bosch BMI088 (TBC)", "ICM-42688-P (TBC)", "ICM-20649 (TBC)", "Bosch BMM150 compass", "Bosch BMP388 pressure sensor" | Rev 2/4 IMU1 is "ICM20649 ACCEL" (no BMI088); Rev 5 baros are ICP-20100; Rev 5 adds RM3100 `[T]` | intra-doc |
| C-22 | BMI088 ACCEL DRDY | Table "U1 (IMU1) BMI088 ACCEL SPI3 CS1 NA" `[T]` (DS-012 p.11, p.13, p.15) | pinout "PI6 ... SPI3_DRDY1_BMI088_INT1_ACCEL" (p.21) | intra-doc |
| C-23 | X2 signals absent from MCU pinout | X2: "ETH_RX_ER", "ETH_PHY_nINT", "CAN3_TX/RX", "FMU_CH9..12", "SPARE09..15" (DS-010 p.10) | DS-012 MCU pinout (p.18-21) has none of these | cross-doc |
| C-24 | USB speed vs MCU peripheral | "USB-C 480 Mbit" `[T]` (DS-010 p.6) | MCU USB signals are on "USB_OTG_FS_DM/DP" (DS-012 p.18) while FMUv5 with "2.0 12 MBit" also uses the term USB 2.0; the corpus never states FS speed | possible conflict (needs external knowledge; flag only) |
| C-25 | Path planning status | Badge "Removed PX4 v1.15" (`path_planning_interface.md:L3`) | "This actual code is still present in code at time of writing (PX4 v1.15)." (`:L20`) | intra-doc |
| C-26 | Copyright holder | "Pixhawk Special Interests Group (SIG) of Dronecode Foundation" (DS-012 p.4) | "Pixhawk Special Interests Group (SIG)" (DS-010 p.4) | cross-doc (non-technical) |
| C-27 | DDS link wording | "over a serial or UDP link" (`uxrce_dds.md:L20`) | FC-companion "connected using a fast serial or IP link, and typically communicate using the MAVLink protocol" (`px4_systems_architecture.md:L42`) | cross-doc (protocol for the same FC-companion link differs: MAVLink vs uXRCE-DDS; both may coexist, `uxrce_dds.md:L282`) |
| C-28 | YAML file name | "dds_topics.yaml" (`uxrce_dds.md:L41`, `:L323`) | "dds_topic.yaml" (`uxrce_dds.md:L540`, `:L558`, `:L577`) | intra-doc spelling |
| C-29 | Serial device example | "if you're using the RPi `UART0` serial port" | "`--dev /dev/AMA0`" (`uxrce_dds.md:L210-L213`) | intra-doc (device naming) |

Compared and found consistent: case-outline text (DS-012 p.8 = DS-010 p.12); SE050 on I2C4 (DS-012 p.6 = tables = pinout); TELEM1/2/3 UART mapping (DS-012 pinout = DS-010 X1); GPS1/GPS2, DEBUG, UART4, USART6 mapping; I2C1-3 signal names; SPI6 external signals (DS-012 = DS-010 X2, except "(SWO)" annotation only in DS-010); three power inputs (DS-012 p.7 = X1 `nPOWER_IN_A/B/C`); IMU rate 1 kHz/250 Hz vs `uorb top` sample counts (242/249, sample output only); TELEM1→GCS, TELEM2→companion defaults (`uxrce_dds.md:L282`, single source).

---

## 7. Gaps a model would hit

| ID | Gap | Where the corpus names it |
|---|---|---|
| G-1 | No electrical limits for FMU: no supply voltage range, current, power consumption, or mass anywhere (only rail names `VDD_5V_IN`, `FMU_VDD_3V3`, "5V rail", "3.3V rail"). | DS-012 p.6-7; DS-010 p.9 |
| G-2 | No bus clock rates / baud rates for SPI, I2C, CAN, UARTs on FMUv6X (only an example 921600 on a companion, and Ethernet "100-Base T", USB "480 Mbit"). | DS-010 p.6; `uxrce_dds.md:L213` |
| G-3 | IO co-processor: no MCU, firmware, protocol, rate or PWM spec; only "8 + 8 (IO)" and `USART6_*_IO` names; `TO_IO__NC` suggests not connected. | DS-010 p.6; DS-012 p.18-19 |
| G-4 | IMU-connector pinout exists only as an image. | DS-012 p.17 |
| G-5 | Debug connector (10-pos), RC input, power-path selector, peripheral power protection, EEPROM/sensor connections, Ethernet transceiver, GPS/audio: schematics only as images; no part numbers or ratings in text. | DS-010 p.13-19 |
| G-6 | SMBus for smart batteries: no SMBus signals/pins defined. | DS-012 p.6 |
| G-7 | "Battery-backed real time clock": only `V_RTC_BAT` pin and 32 kHz oscillator names; no battery spec. | DS-012 p.7; DS-010 X1 pin 29 |
| G-8 | Heater (`HEATER`, TIM2_CH3) present, but "Removed requirement for temperature calibration" and no thermal set-point. | DS-012 p.3, p.19 |
| G-9 | NFC: "5V" and a GPIO named, no NFC reader/protocol spec. | DS-012 p.7 |
| G-10 | Secure element use ("secure authentication of the drone") with no protocol/key spec. | DS-012 p.6 |
| G-11 | "The official pinout is covered in this pinout sheet." — the sheet is not in the corpus. | DS-012 p.18 |
| G-12 | "DS-009 Pixhawk Connector Standard" referenced, not in corpus (JST-GH external connector pinouts unknown). | DS-012 p.5; DS-010 p.5 |
| G-13 | Peripheral power: `VDD_5V_PERIPH_nOC`, `VDD_5V_HIPOWER_nOC` over-current flags but no current limits. | DS-010 X1 pins 35-37, 81-83 |
| G-14 | Mapping DS-019 revision values (e.g. 0x003 rev3, 0x006 rev6) to DS-012 sensor-set revs not given. | `DS-019:L186-L198`; DS-012 p.11-15 |
| G-15 | TELEM3, UART4 have no stated default role. | DS-010 X1 |
| G-16 | Companion computer: no hardware spec (CPU, power, OS version) — Linux/Android "typically"; Raspberry Pi only an example. | `px4_systems_architecture.md:L45-L47`; `uxrce_dds.md:L210` |
| G-17 | `dds_topics.yaml` topic set is release-specific and not included; only examples shown. | `uxrce_dds.md:L323-L326`, `:L421-L455` |
| G-18 | MAVLink message set/rates actually streamed by PX4 per mode are not listed (only mode names and examples). | `mavlink.md:L324-L339` |
| G-19 | PWM_limit transitions (and ramp time/delay value) only in an image; no numeric delay. | `pwm_limit.md:L4`, `:L27` |
| G-20 | Collision prevention: CP_DIST, CP_DELAY, CP_GUIDE_ANG default values/units not given (only tuning guidance); minimum OBSTACLE_DISTANCE rate "depends on vehicle speed". | `collision_prevention.md:L59-L65`, `:L173` |
| G-21 | Mode list per vehicle is linked out (MC/FW/VTOL/rover pages not in corpus); `FailsafeFlags` per-mode requirement mapping is in source code, not in corpus. | `flight_modes.md:L18-L23`, `:L108` |
| G-22 | Control allocation: geometry parameters and output functions are in linked YAML files, not in corpus. | `control_allocation.md:L49`, `:L60` |
| G-23 | Path planning interface API only in external v1.14 docs; `obstacle_avoidance.md` is a redirect stub with no content. | `path_planning_interface.md:L21`; file 13 |
| G-24 | uORB topic rates other than the IMU example and `uorb top` sample are not given (navigator "run considerably slower", no number). | `architecture.md:L93` |
| G-25 | CAN bus bit rate, node count limits, and whether CAN supplies power: "Refer to your manufacturer instructions". | `can_index.md:L38-L39` |
| G-26 | Ethernet PHY part / magnetics: only a heading (image) and baseboard name "NXP T1 PHY". | DS-010 p.18; `DS-019:L150` |
| G-27 | Several sensors marked "(TBC)" by the standard itself: BMI088, ICM-42688-P, ICM-20649. | DS-012 p.6 |
| G-28 | X2 `SPARE09`-`SPARE15` (no SPARE13), `PG6`, `PH11` have no assigned function. | DS-010 p.10 |
| G-29 | Safety/failsafe configuration (RC loss etc.) is linked out ("Safety (Failsafe) Configuration"), not in corpus. | `flight_modes.md:L101-L102` |

---

## 8. Counts

| Section | Count |
|---|---|
| Compute units (§2.1) | 8 (CU-1..CU-8; CU-1a is the FMU's processor) |
| Distinct backticked names in the CU-1 row (FMU) | 39 (of which 11 vendor product/revision names, 1 board-config path) |
| Distinct backticked names in the CU-1a row (FMU processor) | 8 |
| Distinct backticked names, all CU rows (CU-1..CU-8, incl. CU-1a) | 95 (a few are signal names or hosted software, e.g. `USART6_TX_TO_IO__NC`, `MicroXRCEAgent`, `MAVSDK`) |
| Software components (§2.2) | 18 |
| Hardware interfaces (§3.1) | 32 |
| Software/logical interfaces (§3.2) | 8 |
| `[T]` (PDF-table-only) markers in this file | 69 |
| Image-only facts (§3.3) | 25 |
| Numeric value rows (§4.1 + §4.2) | 44 + 32 = 76 |
| Requirements / normative statements (§5) | 56 |
| Conflicts (§6) | 29 |
| Gaps (§7) | 29 |
