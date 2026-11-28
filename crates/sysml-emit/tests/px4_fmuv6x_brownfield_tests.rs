//! Integration & negative tests for Experiment A (PX4 FMUv6X + IO MCU + Companion AI).

use sysml_emit::{audit_model_with_stdlib, emit_model, validate_ir, IrSeverity};
use sysml_ir::swhw::Asil;
use sysml_ir::{
    BusChannel, BusFaultMode, ComputeNode, SwAllocation, SwConnection, SwHwSystem, SwPartition,
    SwPort, SwRequirement, SwTask,
};

#[path = "../examples/px4_fmuv6x_brownfield.rs"]
#[allow(dead_code)]
mod px4_example;

#[test]
fn px4_cycle1_shallow_passes_with_w001_only_and_validates_omg_stdlib() {
    let sys = px4_example::build_cycle1_hardware_spine();
    let rep = validate_ir(&sys);
    assert!(rep.ok, "Cycle 1 failed:\n{}", rep.render_human());
    assert_eq!(rep.error_count, 0);
    assert_eq!(rep.warning_count, 2);
    assert!(rep
        .diagnostics
        .iter()
        .all(|d| d.code == "W001" && d.severity == IrSeverity::Warning));

    let model = sys.to_sysml_model();
    let audit = audit_model_with_stdlib("Px4FmuV6xShallow", &model, None, None, true);
    assert_eq!(audit.validation_errors, 0, "{:?}", audit.findings);
}

#[test]
fn px4_cycle4_full_brownfield_passes_ir_and_omg_stdlib_zero_errors() {
    let sys = px4_example::build_cycle4_full_brownfield();
    let rep = validate_ir(&sys);
    assert!(rep.ok, "Cycle 4 failed:\n{}", rep.render_human());
    assert_eq!(rep.error_count, 0);
    assert_eq!(rep.warning_count, 0);
    assert_eq!(sys.compute_nodes.len(), 3);
    assert_eq!(sys.partitions.len(), 6);
    assert_eq!(sys.allocations.len(), 6);

    let model = sys.to_sysml_model();
    let sysml = emit_model(&model);
    assert!(sysml.contains("allocation allocAiOnCompanion allocate obstacleAvoidanceAI to companionSoc;"));
    assert!(sysml.contains("allocation allocMixerOnIo allocate px4IoFailsafeMixer to ioMcu;"));
    assert!(sysml.contains("latencyMs : Real = 200;"));

    let audit = audit_model_with_stdlib("Px4FmuV6xBrownfield", &model, None, None, true);
    assert_eq!(audit.validation_errors, 0, "{:?}", audit.findings);
    assert_eq!(audit.validation_warnings, 0, "{:?}", audit.findings);
}

#[test]
fn px4_negative_bypassing_commander_guardrail_triggers_e004() {
    // If Companion AI connects directly to an unprotected ASIL-D actuator partition -> E004!
    let unsafe_sys = SwHwSystem::new("Px4UnsafeBypass", "Px4UnsafeSystem")
        .compute_node(ComputeNode::new("CompanionSoc").cores(4))
        .compute_node(ComputeNode::new("IoMcu").cores(1))
        .bus(BusChannel::new("DirectUart").fault_mode(BusFaultMode::Corrupt))
        .requirement(SwRequirement::new("PX4-SAFE-03", "PwmLimitReq", "Safe disarm."))
        .partition(
            SwPartition::new("ObstacleAvoidanceAI")
                .asil(Asil::QM)
                .ai_component(true)
                .task(SwTask::new("VisionTask").period_ms(50.0))
                .port(SwPort::output("motorOut", "ActuatorMotors")),
        )
        .partition(
            SwPartition::new("UnprotectedIoActuator")
                .asil(Asil::D)
                .port(SwPort::input("motorIn", "ActuatorMotors"))
                .satisfies("PX4-SAFE-03"),
        )
        .connection(
            SwConnection::new(
                "unsafeAiToIo",
                "ObstacleAvoidanceAI",
                "motorOut",
                "UnprotectedIoActuator",
                "motorIn",
            )
            .over_bus("DirectUart"),
        )
        .allocation(SwAllocation::new("a1", "ObstacleAvoidanceAI", "CompanionSoc"))
        .allocation(SwAllocation::new("a2", "UnprotectedIoActuator", "IoMcu"));

    let rep = validate_ir(&unsafe_sys);
    assert!(!rep.ok);
    assert!(
        rep.diagnostics.iter().any(|d| d.code == "E004" && d.element == "unsafeAiToIo"),
        "expected E004, got:\n{}",
        rep.render_human()
    );
}
