//! 2-stage progressive automotive AEB (Autonomous Emergency Braking) example:
//! - Stage 1 (Shallow): Perception AI + Brake Safety Monitor + Brake Actuator + Safety Goal
//!   (`REQ-SAFE-01`), with zero timing/hardware details. Passes `validate_ir` cleanly (warnings
//!   only) and emits 100% standard OMG SysML v2.
//! - Stage 2 (Enriched): Incrementally extends the exact same Stage 1 model with `Asil::D`,
//!   `SwTask` timing (`period_ms`, `deadline_ms`, `wcet_ms`), `ComputeNode`s (`AdasSoc`,
//!   `SafetyMcu`), `BusChannel` (`CanFdBus`), and `SwAllocation`s. Passes `validate_ir` with
//!   0 errors / 0 warnings and validates cleanly against Open-MBEE + OMG Standard Library.
//!
//! Run with:
//!   cargo run -p sysml-emit --example automotive_progressive
//!   cargo run -p sysml-emit --example automotive_progressive -- --json

use sysml_emit::{emit_model, validate_ir};
use sysml_ir::swhw::Asil;
use sysml_ir::{
    BusChannel, BusFaultMode, ComputeNode, SwAllocation, SwConnection, SwHwSystem, SwPartition,
    SwPort, SwRequirement, SwTask,
};

/// Stage 1 (Day-1 Shallow Model): no timing, no compute nodes, no bus details.
pub fn build_stage1_shallow() -> SwHwSystem {
    SwHwSystem::new("AutomotiveAebShallow", "AebSystem")
        .requirement(SwRequirement::new(
            "REQ-SAFE-01",
            "AebSafeBrakingGoal",
            "AEB shall command emergency braking safely even if PerceptionAI outputs are erroneous.",
        ))
        .partition(
            SwPartition::new("PerceptionAI")
                .ai_component(true)
                .provenance("System Concept §2.1")
                .port(SwPort::output("obstaclesOut", "ObstacleList")),
        )
        .partition(
            SwPartition::new("BrakeSafetyMonitor")
                .asil(Asil::D)
                .safety_guardrail(true)
                .fallback_state("SafeStop")
                .provenance("Safety Concept §3.1")
                .port(SwPort::input("obstaclesIn", "ObstacleList"))
                .port(SwPort::output("brakeCmdOut", "BrakeCommand"))
                .satisfies("REQ-SAFE-01"),
        )
        .partition(
            SwPartition::new("BrakeActuator")
                .asil(Asil::D)
                .port(SwPort::input("brakeCmdIn", "BrakeCommand")),
        )
        .connection(SwConnection::new(
            "perceptionToMonitor",
            "PerceptionAI",
            "obstaclesOut",
            "BrakeSafetyMonitor",
            "obstaclesIn",
        ))
        .connection(SwConnection::new(
            "monitorToActuator",
            "BrakeSafetyMonitor",
            "brakeCmdOut",
            "BrakeActuator",
            "brakeCmdIn",
        ))
}

/// Stage 2 (Enriched Model): extends Stage 1 with tasks, timing, ECUs, CAN-FD bus, and allocations.
pub fn build_stage2_enriched() -> SwHwSystem {
    let mut sys = build_stage1_shallow();
    sys.package_name = "AutomotiveAebEnriched".to_string();

    for p in &mut sys.partitions {
        match p.name.as_str() {
            "PerceptionAI" => {
                p.asil = Some(Asil::QM);
                p.provenance.push("src/perception/detector.cpp:88".into());
                p.tasks.push(
                    SwTask::new("ObjectDetectTask")
                        .period_ms(50.0)
                        .deadline_ms(40.0)
                        .wcet_ms(22.0),
                );
            }
            "BrakeSafetyMonitor" => {
                p.provenance.push("src/safety/aeb_monitor.cpp:142".into());
                p.tasks.push(
                    SwTask::new("PlausibilityCheckTask")
                        .period_ms(10.0)
                        .deadline_ms(5.0)
                        .wcet_ms(1.8),
                );
            }
            "BrakeActuator" => {
                p.provenance.push("src/actuation/brake_drv.cpp:64".into());
                p.tasks.push(
                    SwTask::new("HydraulicValveTask")
                        .period_ms(10.0)
                        .deadline_ms(4.0)
                        .wcet_ms(1.2),
                );
            }
            _ => {}
        }
    }

    for c in &mut sys.connections {
        c.bus = Some("CanFdBus".into());
    }

    sys.compute_node(
        ComputeNode::new("AdasSoc")
            .cores(4)
            .scheduling_policy("EDF")
            .max_utilization(0.80),
    )
    .compute_node(
        ComputeNode::new("SafetyMcu")
            .cores(2)
            .scheduling_policy("FixedPriority")
            .max_utilization(0.70),
    )
    .bus(
        BusChannel::new("CanFdBus")
            .latency_ms(2.0)
            .fault_mode(BusFaultMode::Drop)
            .fault_mode(BusFaultMode::Delay)
            .fault_mode(BusFaultMode::Corrupt),
    )
    .allocation(SwAllocation::new("allocPerception", "PerceptionAI", "AdasSoc"))
    .allocation(SwAllocation::new(
        "allocMonitor",
        "BrakeSafetyMonitor",
        "SafetyMcu",
    ))
    .allocation(SwAllocation::new(
        "allocActuator",
        "BrakeActuator",
        "SafetyMcu",
    ))
}

fn main() {
    let as_json = std::env::args().any(|a| a == "--json");

    let stage1 = build_stage1_shallow();
    let r1 = validate_ir(&stage1);
    let sysml1 = emit_model(&stage1.to_sysml_model());

    let stage2 = build_stage2_enriched();
    let r2 = validate_ir(&stage2);
    let sysml2 = emit_model(&stage2.to_sysml_model());

    if as_json {
        println!(
            "{}",
            serde_json::json!({
                "stage1_shallow": r1,
                "stage2_enriched": r2,
            })
        );
    } else {
        println!("=== Stage 1 (Shallow) ===");
        println!("{}", r1.render_human());
        println!("Emitted {} bytes of OMG SysML v2.\n", sysml1.len());
        println!("=== Stage 2 (Enriched) ===");
        println!("{}", r2.render_human());
        println!("Emitted {} bytes of OMG SysML v2.", sysml2.len());
    }
}
