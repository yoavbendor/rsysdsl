//! Experiment A (Doc-Batched Brownfield Model): PX4 Autopilot on Pixhawk 6X (FMUv6X)
//! + IO Co-processor + Companion AI Computer.
//!
//! Built strictly in 4 sequential doc-type batches from `target/px4_corpus/`
//! (fetched via `scripts/fetch-px4-corpus.sh`), with zero invented numbers:
//! - Batch 1 (Hardware): DS-012 FMUv6X, DS-010 Autopilot Bus, DS-019, autopilot_pixhawk_standard.md
//! - Batch 2 (SW Arch):  architecture.md, px4_systems_architecture.md, control_allocation.md, flight_modes.md
//! - Batch 3 (Comms):    uorb.md, mavlink.md, uxrce_dds.md, can_index.md
//! - Batch 4 (AI/Safe):  collision_prevention.md, obstacle_avoidance.md, path_planning_interface.md, pwm_limit.md
//!
//! Run with:
//!   cargo run -p sysml-emit --example px4_fmuv6x_brownfield
//!   cargo run -p sysml-emit --example px4_fmuv6x_brownfield -- --json

use sysml_emit::{emit_model, validate_ir};
use sysml_ir::swhw::Asil;
use sysml_ir::{
    BusChannel, BusFaultMode, ComputeNode, SwAllocation, SwConnection, SwHwSystem, SwPartition,
    SwPort, SwRequirement, SwTask,
};

/// Cycle 1 (Batch 1 — Hardware & Standards only): Shallow Spine with 2 on-board MCUs
/// (`FmuMcu` STM32H753 + `IoMcu` STM32F103) and physical buses; zero SW timing or allocations yet.
pub fn build_cycle1_hardware_spine() -> SwHwSystem {
    SwHwSystem::new("Px4FmuV6xShallow", "Px4AutopilotSystem")
        .compute_node(
            ComputeNode::new("FmuMcu")
                .cores(1)
                .scheduling_policy("NuttX_WorkQueue"),
        )
        .compute_node(
            ComputeNode::new("IoMcu")
                .cores(1)
                .scheduling_policy("BareMetal_Failsafe"),
        )
        .bus(BusChannel::new("SensorSpiBus"))
        .bus(BusChannel::new("FmuIoUart"))
        .bus(BusChannel::new("CanFdBus"))
        .bus(BusChannel::new("EthernetMavlink"))
        .partition(
            SwPartition::new("FmuFlightStack")
                .asil(Asil::C)
                .provenance("DS-012_FMUv6X_Standard.txt (STM32H753 FMU)")
                .port(SwPort::output("actuatorCmdOut", "ActuatorMotors")),
        )
        .partition(
            SwPartition::new("Px4IoFailsafeMixer")
                .asil(Asil::D)
                .provenance("DS-012_FMUv6X_Standard.txt (STM32F103 IO + FMU_SAFETY_SWITCH_IN)")
                .port(SwPort::input("actuatorCmdIn", "ActuatorMotors")),
        )
        .connection(
            SwConnection::new(
                "fmuToIoActuators",
                "FmuFlightStack",
                "actuatorCmdOut",
                "Px4IoFailsafeMixer",
                "actuatorCmdIn",
            )
            .over_bus("FmuIoUart"),
        )
}

/// Cycle 4 (After all 4 doc-type batches): 3 ComputeNodes (`FmuMcu`, `IoMcu`, `CompanionSoc`),
/// 6 SwPartitions, documented loop periods (`1 ms` IMU / `4 ms` published IMU; WCETs left `None`
/// because not in public docs), typed uORB/MAVLink/uXRCE-DDS/DroneCAN connections, AI guardrail,
/// and full document provenance.
pub fn build_cycle4_full_brownfield() -> SwHwSystem {
    SwHwSystem::new("Px4FmuV6xBrownfield", "Px4AutopilotSystem")
        // Requirements extracted from Batch 2 & Batch 4 docs
        .requirement(SwRequirement::new(
            "PX4-SAFE-01",
            "ExternalModeFallbackReq",
            "Safety-critical modes (e.g. ReturnMode) shall remain on FMU and engage if an external companion/ROS 2 mode crashes (flight_modes.md).",
        ))
        .requirement(SwRequirement::new(
            "PX4-SAFE-02",
            "CollisionPreventionGuardrailReq",
            "Collision prevention on FMU shall bound velocity setpoints using companion obstacle data and handle up to 200 ms vision delay or range data loss (collision_prevention.md).",
        ))
        .requirement(SwRequirement::new(
            "PX4-SAFE-03",
            "PwmLimitDisarmOverrideReq",
            "PWM_limit state machine on IO MCU shall override actuator outputs to disarmed values whenever safety switch or disarm input is asserted (pwm_limit.md, DS-012).",
        ))
        // 3 ComputeNodes across Batch 1 & Batch 2/4
        .compute_node(
            ComputeNode::new("FmuMcu")
                .cores(1)
                .scheduling_policy("NuttX_WorkQueue"),
        )
        .compute_node(
            ComputeNode::new("IoMcu")
                .cores(1)
                .scheduling_policy("BareMetal_Failsafe"),
        )
        .compute_node(
            ComputeNode::new("CompanionSoc")
                .cores(4)
                .scheduling_policy("Linux_ROS2"),
        )
        // 4 BusChannels from Batch 1, Batch 3 & Batch 4
        .bus(
            BusChannel::new("SensorSpiBus")
                .fault_mode(BusFaultMode::Drop),
        )
        .bus(
            BusChannel::new("FmuIoUart")
                .fault_mode(BusFaultMode::Delay)
                .fault_mode(BusFaultMode::Corrupt),
        )
        .bus(
            BusChannel::new("CanFdBus")
                .fault_mode(BusFaultMode::Drop)
                .fault_mode(BusFaultMode::Delay),
        )
        .bus(
            // collision_prevention.md §CP_DELAY: "For external vision-based systems the sensor delay may be as high as 0.2s"
            BusChannel::new("CompanionUxrceDdsLink")
                .latency_ms(200.0)
                .fault_mode(BusFaultMode::Drop)
                .fault_mode(BusFaultMode::Delay),
        )
        // Partitions on CompanionSoc (AI / QM)
        .partition(
            SwPartition::new("ObstacleAvoidanceAI")
                .asil(Asil::QM)
                .ai_component(true)
                .provenance("px4_systems_architecture.md (Companion/Mission Computer)")
                .provenance("path_planning_interface.md (companion planner)")
                .provenance("collision_prevention.md (external vision obstacle data, <= 200 ms delay)")
                .provenance("uxrce_dds.md (Micro-XRCE-DDS-Agent v2.x)")
                .port(SwPort::output("obstacleMapOut", "ObstacleDistance"))
                .port(SwPort::output("trajectoryOut", "TrajectorySetpoint")),
        )
        // Partitions on FmuMcu (STM32H753)
        .partition(
            SwPartition::new("ImuSensorDriver")
                .asil(Asil::C)
                .provenance("DS-012_FMUv6X_Standard.txt (ICM20602 / ISM330 / BMI088 triple SPI IMUs)")
                .provenance("architecture.md §Update Rates (1 kHz sample, 250 Hz publish)")
                .task(
                    // Documented 1 kHz sampling; WCET is not in public corpus -> left None (honest gap)
                    SwTask::new("ImuSampleTask").period_ms(1.0),
                )
                .task(SwTask::new("ImuPublishTask").period_ms(4.0))
                .port(SwPort::output("imuOut", "SensorCombined")),
        )
        .partition(
            SwPartition::new("Ekf2Estimator")
                .asil(Asil::C)
                .provenance("architecture.md §Flight Stack")
                .provenance("uorb.md (shared-memory pub-sub)")
                .port(SwPort::input("imuIn", "SensorCombined"))
                .port(SwPort::output("vehicleStateOut", "VehicleLocalPosition")),
        )
        .partition(
            SwPartition::new("CommanderSafetyMonitor")
                .asil(Asil::D)
                .safety_guardrail(true)
                .fallback_state("ReturnMode")
                .provenance("architecture.md §Background Tasks (commander)")
                .provenance("flight_modes.md (ReturnMode fallback on ROS 2 crash)")
                .provenance("collision_prevention.md (velocity setpoint bounding & CP_GO_NO_DATA)")
                .port(SwPort::input("vehicleStateIn", "VehicleLocalPosition"))
                .port(SwPort::input("obstacleMapIn", "ObstacleDistance"))
                .port(SwPort::input("trajectoryIn", "TrajectorySetpoint"))
                .port(SwPort::output("safeSetpointOut", "VehicleTorqueThrustSetpoint"))
                .satisfies("PX4-SAFE-01")
                .satisfies("PX4-SAFE-02"),
        )
        .partition(
            SwPartition::new("ControlAllocator")
                .asil(Asil::C)
                .provenance("control_allocation.md (torque/thrust -> actuator_motors)")
                .provenance("can_index.md (DroneCAN / Cyphal/CAN ESCs)")
                .port(SwPort::input("safeSetpointIn", "VehicleTorqueThrustSetpoint"))
                .port(SwPort::output("actuatorCmdOut", "ActuatorMotors")),
        )
        // Partition on IoMcu (STM32F103)
        .partition(
            SwPartition::new("Px4IoFailsafeMixer")
                .asil(Asil::D)
                .fallback_state("Disarmed")
                .provenance("DS-012_FMUv6X_Standard.txt (STM32F103 IO + FMU_SAFETY_SWITCH_IN)")
                .provenance("pwm_limit.md (PWM_limit state machine disarm override)")
                .port(SwPort::input("actuatorCmdIn", "ActuatorMotors"))
                .satisfies("PX4-SAFE-03"),
        )
        // Connections (uORB internal, uXRCE-DDS / MAVLink companion, FMU->IO UART)
        .connection(
            SwConnection::new(
                "imuToEkf2",
                "ImuSensorDriver",
                "imuOut",
                "Ekf2Estimator",
                "imuIn",
            )
            .over_bus("SensorSpiBus"),
        )
        .connection(SwConnection::new(
            "ekf2ToCommander",
            "Ekf2Estimator",
            "vehicleStateOut",
            "CommanderSafetyMonitor",
            "vehicleStateIn",
        ))
        .connection(
            SwConnection::new(
                "aiObstaclesToCommander",
                "ObstacleAvoidanceAI",
                "obstacleMapOut",
                "CommanderSafetyMonitor",
                "obstacleMapIn",
            )
            .over_bus("CompanionUxrceDdsLink"),
        )
        .connection(
            SwConnection::new(
                "aiTrajectoryToCommander",
                "ObstacleAvoidanceAI",
                "trajectoryOut",
                "CommanderSafetyMonitor",
                "trajectoryIn",
            )
            .over_bus("CompanionUxrceDdsLink"),
        )
        .connection(SwConnection::new(
            "commanderToAllocator",
            "CommanderSafetyMonitor",
            "safeSetpointOut",
            "ControlAllocator",
            "safeSetpointIn",
        ))
        .connection(
            SwConnection::new(
                "allocatorToIoMixer",
                "ControlAllocator",
                "actuatorCmdOut",
                "Px4IoFailsafeMixer",
                "actuatorCmdIn",
            )
            .over_bus("FmuIoUart"),
        )
        // Allocations across all 3 ComputeNodes
        .allocation(SwAllocation::new(
            "allocAiOnCompanion",
            "ObstacleAvoidanceAI",
            "CompanionSoc",
        ))
        .allocation(SwAllocation::new("allocImuOnFmu", "ImuSensorDriver", "FmuMcu"))
        .allocation(SwAllocation::new("allocEkf2OnFmu", "Ekf2Estimator", "FmuMcu"))
        .allocation(SwAllocation::new(
            "allocCommanderOnFmu",
            "CommanderSafetyMonitor",
            "FmuMcu",
        ))
        .allocation(SwAllocation::new(
            "allocAllocatorOnFmu",
            "ControlAllocator",
            "FmuMcu",
        ))
        .allocation(SwAllocation::new(
            "allocMixerOnIo",
            "Px4IoFailsafeMixer",
            "IoMcu",
        ))
}

fn main() {
    let as_json = std::env::args().any(|a| a == "--json");
    let c1 = build_cycle1_hardware_spine();
    let r1 = validate_ir(&c1);
    let s1 = emit_model(&c1.to_sysml_model());

    let c4 = build_cycle4_full_brownfield();
    let r4 = validate_ir(&c4);
    let s4 = emit_model(&c4.to_sysml_model());

    if as_json {
        println!(
            "{}",
            serde_json::json!({
                "cycle1_hardware_spine": r1,
                "cycle4_full_brownfield": r4,
            })
        );
    } else {
        println!("=== Cycle 1 (Batch 1 Hardware Spine — Shallow) ===");
        println!("{}", r1.render_human());
        println!("Emitted {} bytes of OMG SysML v2.\n", s1.len());
        println!("=== Cycle 4 (All 4 Doc Batches — 3 ComputeNodes) ===");
        println!("{}", r4.render_human());
        println!("Emitted {} bytes of OMG SysML v2.", s4.len());
    }
}
