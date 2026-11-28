//! Synthetic scale stress test (500-element and 2,000-element models) measuring:
//! 1. In-memory IR validation (`validate_ir`) time.
//! 2. SysML v2 lowering + emission (`to_sysml_model` + `emit_model`) time.
//! 3. External Open-MBEE + OMG Standard Library validation time.
//!
//! Run with `--nocapture` to inspect exact timings:
//!   cargo test -p sysml-emit --test scale_check -- --nocapture

use std::time::Instant;
use sysml_emit::{emit_model, validate_ir};
use sysml_ir::swhw::Asil;
use sysml_ir::{
    BusChannel, BusFaultMode, ComputeNode, SwAllocation, SwConnection, SwHwSystem, SwPartition,
    SwPort, SwRequirement, SwTask,
};

/// Build a synthetic SW/HW system with approximately `target_elements` elements
/// (partitions + tasks + nodes + buses + connections + allocations + requirements).
fn generate_synthetic_system(target_elements: usize) -> SwHwSystem {
    // Each subsystem group contributes:
    //   1 partition + 1 task + 1 connection + 1 allocation + 1 requirement = 5 elements
    let groups = target_elements / 5;
    let node_count = (groups / 10).max(2);

    let mut sys = SwHwSystem::new(
        format!("SyntheticScale{target_elements}"),
        format!("ScaleSystem{target_elements}"),
    )
    .bus(
        BusChannel::new("MainBackboneBus")
            .latency_ms(1.5)
            .fault_mode(BusFaultMode::Delay),
    );

    for n in 0..node_count {
        sys = sys.compute_node(
            ComputeNode::new(format!("EcuNode{n}"))
                .cores(8)
                .scheduling_policy("EDF")
                .max_utilization(0.85),
        );
    }

    for i in 0..groups {
        let req_id = format!("REQ-{i:04}");
        sys = sys.requirement(SwRequirement::new(
            &req_id,
            format!("SafetyReq{i:04}"),
            format!("Component {i} shall meet timing and safety envelope."),
        ));

        let part_name = format!("SwComp{i:04}");
        let mut p = SwPartition::new(&part_name)
            .asil(if i % 4 == 0 { Asil::D } else { Asil::B })
            .safety_guardrail(i % 4 == 0)
            .task(
                SwTask::new(format!("MainTask{i:04}"))
                    .period_ms(20.0)
                    .deadline_ms(15.0)
                    .wcet_ms(2.0),
            )
            .port(SwPort::input("dataIn", "SensorFrame"))
            .port(SwPort::output("dataOut", "SensorFrame"))
            .satisfies(&req_id);
        if i % 8 == 1 {
            p = p.ai_component(true);
        }
        sys = sys.partition(p);

        let node_name = format!("EcuNode{}", i % node_count);
        sys = sys.allocation(SwAllocation::new(
            format!("alloc{i:04}"),
            &part_name,
            node_name,
        ));

        if i > 0 {
            sys = sys.connection(
                SwConnection::new(
                    format!("link{i:04}"),
                    format!("SwComp{:04}", i - 1),
                    "dataOut",
                    &part_name,
                    "dataIn",
                )
                .over_bus("MainBackboneBus"),
            );
        }
    }

    sys
}

fn count_elements(s: &SwHwSystem) -> usize {
    let tasks: usize = s.partitions.iter().map(|p| p.tasks.len()).sum();
    s.partitions.len()
        + tasks
        + s.compute_nodes.len()
        + s.buses.len()
        + s.connections.len()
        + s.allocations.len()
        + s.requirements.len()
}

#[test]
fn scale_500_and_2000_elements_ir_validation_is_subsecond() {
    for &target in &[500usize, 2000usize] {
        let mut sys = generate_synthetic_system(target);
        let actual_elems = count_elements(&sys);

        let t_val = Instant::now();
        let report = validate_ir(&sys);
        let d_val = t_val.elapsed();
        assert!(
            report.ok,
            "clean {actual_elems}-element model failed validate_ir: {}",
            report.render_human()
        );
        assert!(
            d_val.as_millis() < 250,
            "validate_ir on {actual_elems} elements took {:?} (expected < 250ms)",
            d_val
        );

        // Introduce 1 intentional error (E003 timing violation) and measure failure detection speed
        sys.partitions[0].tasks[0].wcet_ms = Some(25.0);
        let t_err = Instant::now();
        let err_report = validate_ir(&sys);
        let d_err = t_err.elapsed();
        assert!(!err_report.ok);
        assert!(err_report.diagnostics.iter().any(|d| d.code == "E003"));

        // Restore and measure lowering + emission + external Open-MBEE stdlib validation
        sys.partitions[0].tasks[0].wcet_ms = Some(2.0);
        let t_emit = Instant::now();
        let model = sys.to_sysml_model();
        let sysml = emit_model(&model);
        let d_emit = t_emit.elapsed();

        let t_mbee = Instant::now();
        let audit = sysml_emit::audit_model_with_stdlib(
            &format!("SyntheticScale{target}"),
            &model,
            None,
            None,
            false,
        );
        let d_mbee = t_mbee.elapsed();
        assert_eq!(
            audit.validation_errors, 0,
            "Open-MBEE errors on {actual_elems}-element model: {:?}",
            audit.findings
        );

        println!(
            "[SCALE] elements={actual_elems} validate_ir_ok={:?} validate_ir_err={:?} lower+emit={:?} open_mbee_stdlib={:?} sysml_bytes={}",
            d_val,
            d_err,
            d_emit,
            d_mbee,
            sysml.len()
        );
    }
}
