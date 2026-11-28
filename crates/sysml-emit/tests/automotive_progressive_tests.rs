//! Integration & negative-diagnostic tests for the 2-stage progressive automotive example
//! and `validate_ir` (`E001`–`E004`, `W001`).

use std::time::Instant;
use sysml_emit::{audit_model_with_stdlib, emit_model, validate_ir, IrSeverity};
use sysml_ir::swhw::Asil;
use sysml_ir::{
    BusChannel, BusFaultMode, ComputeNode, SwAllocation, SwConnection, SwHwSystem, SwPartition,
    SwPort, SwRequirement, SwTask,
};

fn build_stage1_shallow() -> SwHwSystem {
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

fn build_stage2_enriched() -> SwHwSystem {
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

#[test]
fn stage1_shallow_passes_ir_with_w001_warnings_and_validates_omg_stdlib() {
    let sys = build_stage1_shallow();
    let report = validate_ir(&sys);
    assert!(report.ok, "Stage 1 should pass with warnings only:\n{}", report.render_human());
    assert_eq!(report.error_count, 0);
    assert_eq!(report.warning_count, 2); // BrakeSafetyMonitor and BrakeActuator unallocated
    assert!(report
        .diagnostics
        .iter()
        .all(|d| d.code == "W001" && d.severity == IrSeverity::Warning));

    let model = sys.to_sysml_model();
    let sysml = emit_model(&model);
    assert!(sysml.contains("part def PerceptionAI"));
    assert!(sysml.contains("requirement def <'REQ-SAFE-01'> AebSafeBrakingGoal"));

    let audit = audit_model_with_stdlib("AutomotiveAebShallow", &model, None, None, true);
    assert_eq!(
        audit.validation_errors, 0,
        "Stage 1 emitted SysML had errors:\n{sysml}\n{:?}",
        audit.findings
    );
}

#[test]
fn stage2_enriched_passes_ir_cleanly_and_validates_omg_stdlib() {
    let sys = build_stage2_enriched();
    let report = validate_ir(&sys);
    assert!(report.ok, "Stage 2 should pass cleanly:\n{}", report.render_human());
    assert_eq!(report.error_count, 0);
    assert_eq!(report.warning_count, 0);

    let model = sys.to_sysml_model();
    let sysml = emit_model(&model);
    assert!(sysml.contains("allocation allocMonitor allocate brakeSafetyMonitor to safetyMcu;"));
    assert!(sysml.contains("part def CanFdBus"));

    let audit = audit_model_with_stdlib("AutomotiveAebEnriched", &model, None, None, true);
    assert_eq!(
        audit.validation_errors, 0,
        "Stage 2 emitted SysML had errors:\n{sysml}\n{:?}",
        audit.findings
    );
    assert_eq!(
        audit.validation_warnings, 0,
        "Stage 2 emitted SysML had warnings:\n{sysml}\n{:?}",
        audit.findings
    );
}

#[test]
fn negative_e001_broken_port_connection_and_type_mismatch() {
    let mut sys = build_stage2_enriched();
    // Break payload type on PerceptionAI output + add a missing-port connection
    sys.partitions[0].ports[0].payload_type = "RawLidarPointCloud".into();
    sys.connections.push(SwConnection::new(
        "danglingLink",
        "PerceptionAI",
        "nonExistentPort",
        "BrakeActuator",
        "brakeCmdIn",
    ));

    let t0 = Instant::now();
    let report = validate_ir(&sys);
    let elapsed = t0.elapsed();

    assert!(!report.ok);
    assert!(elapsed.as_millis() < 50, "validate_ir took {:?}", elapsed);
    assert!(report.diagnostics.iter().any(|d| d.code == "E001" && d.message.contains("payload type mismatch")));
    assert!(report.diagnostics.iter().any(|d| d.code == "E001" && d.message.contains("missing port `nonExistentPort`")));
}

#[test]
fn negative_e002_dangling_requirement_satisfy() {
    let mut sys = build_stage2_enriched();
    sys.partitions[1].satisfies.push("REQ-NONEXISTENT-99".into());

    let report = validate_ir(&sys);
    assert!(!report.ok);
    let d = report.diagnostics.iter().find(|d| d.code == "E002").expect("E002 expected");
    assert!(d.message.contains("REQ-NONEXISTENT-99"));
    assert!(d.suggested_fix.contains("SwRequirement::new"));
}

#[test]
fn negative_e003_timing_sanity_and_node_overutilization() {
    let mut sys = build_stage2_enriched();
    // Make WCET > deadline AND overload SafetyMcu (2 cores * 0.70 = 1.40 max util)
    sys.partitions[1].tasks[0].wcet_ms = Some(9.0); // > deadline 5.0 -> E003 (0.90 util)
    sys.partitions[2].tasks[0].wcet_ms = Some(6.0); // > deadline 4.0 -> E003 (0.60 util; total 1.50 > 1.40)

    let report = validate_ir(&sys);
    assert!(!report.ok);
    let e003s: Vec<_> = report.diagnostics.iter().filter(|d| d.code == "E003").collect();
    assert!(e003s.len() >= 3, "expected 2 task timing errors + 1 node utilization error, got: {e003s:#?}");
    assert!(e003s.iter().any(|d| d.message.contains("wcet_ms (9 ms) > deadline_ms (5 ms)")));
    assert!(e003s.iter().any(|d| d.message.contains("utilization 150.0% exceeds limit 140.0%")));
}

#[test]
fn negative_e004_high_asil_consuming_ai_without_guardrail() {
    // Direct PerceptionAI (QM, AI) -> BrakeActuator (ASIL-D, no guardrail, no fallback)
    let sys = SwHwSystem::new("UnsafeAeb", "UnsafeAebSystem")
        .partition(
            SwPartition::new("PerceptionAI")
                .asil(Asil::QM)
                .ai_component(true)
                .port(SwPort::output("brakeCmdOut", "BrakeCommand")),
        )
        .partition(
            SwPartition::new("BrakeActuator")
                .asil(Asil::D)
                .port(SwPort::input("brakeCmdIn", "BrakeCommand")),
        )
        .connection(SwConnection::new(
            "unsafeDirectAiToBrake",
            "PerceptionAI",
            "brakeCmdOut",
            "BrakeActuator",
            "brakeCmdIn",
        ));

    let report = validate_ir(&sys);
    assert!(!report.ok);
    let d = report.diagnostics.iter().find(|d| d.code == "E004").expect("E004 expected");
    assert!(d.message.contains("BrakeActuator"));
    assert!(d.message.contains("PerceptionAI"));
    assert!(d.suggested_fix.contains("safety_guardrail"));
}
