use super::*;

fn outcome(name: &str) -> ScenarioOutcome {
    let s = scenarios().into_iter().find(|s| s.name == name).unwrap();
    let o = (s.run)();
    assert!(
        s.meets_expectation(&o),
        "{name} did not meet its expectation: {}",
        serde_json::to_string_pretty(&o.report).unwrap()
    );
    o
}

#[test]
fn fmea_faults_come_from_the_model() {
    let p = model_fmea::<PrimaryDoorInterlock>(WELDED_CLOSED);
    let m = model_fmea::<MonitorDoorInterlock>(WELDED_CLOSED);
    assert_eq!(p.fault_name(), "PrimaryDoorInterlock::ContactsWeldedClosed");
    assert_eq!(m.fault_name(), "MonitorDoorInterlock::ContactsWeldedClosed");
    // Safety-goal metadata used by the monitors is the model's.
    assert_eq!(<DoorInterlockCutoffRequirement as SafetyGoal>::FTTI_MS, 20);
    assert_eq!(
        <DoorInterlockCutoffRequirement as SafetyGoal>::SAFE_STATE,
        MicrowaveState::DoorOpenCutoff.sysml_state_name()
    );
}

#[test]
fn nominal_reheat_passes_and_completes_cycle() {
    let o = outcome(NOMINAL_SCENARIO);
    assert!(o.report.all_passed);
    assert_eq!(o.run.requirement_ids, vec!["MW-001"]);
    assert_eq!(o.run.end_time_s, REHEAT_DURATION_S);
    let modes: Vec<_> = o.run.trace.events.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(modes, vec!["cooking", "standby"]);
    let mw1 = o.run.verdict("MW-001").unwrap();
    assert_eq!(mw1.min_margin, Some(100.0));
    // 900 W x 0.98 x 0.88 absorbed for 120 s.
    let e = o.run.trace.signal("absorbed_energy_j").unwrap();
    assert!((e.last().unwrap() - 900.0 * 0.98 * 0.88 * 120.0).abs() < 1e-6);
}

#[test]
fn door_open_mid_cook_reaches_safe_state_within_limit() {
    let o = outcome(DOOR_OPEN_SCENARIO);
    assert!(o.report.all_passed, "{:#?}", o.report);
    let v = o.run.verdict("MW-004").unwrap();
    assert!(v.pass);
    // Primary interlock 5 ms + HV relay 8 ms = 13 ms <= 20 ms.
    assert_eq!(v.evidence["actual_cutoff_latency_s"], 0.013);
    assert_eq!(v.evidence["door_opened_s"], 45.0);
    assert_eq!(v.evidence["final_state"], "doorOpenCutoff");
    assert_eq!(v.evidence["final_rf_power_w"], 0.0);
    assert!((v.min_margin.unwrap() - 0.007).abs() < 1e-12);
    let rf = o.run.trace.signal("rf_power_w").unwrap();
    assert_eq!(*rf.last().unwrap(), 0.0);
    // Absorbed energy stops growing after the cutoff.
    let e = o.run.trace.signal("absorbed_energy_j").unwrap();
    assert!(*e.last().unwrap() < 900.0 * 0.98 * 0.88 * 45.1);
}

#[test]
fn primary_interlock_welded_is_tolerated_by_monitor_interlock() {
    let o = outcome(PRIMARY_WELDED_SCENARIO);
    assert!(o.report.all_passed, "{:#?}", o.report);
    let v = o.run.verdict("MW-004").unwrap();
    // Monitor interlock 8 ms + HV relay 8 ms = 16 ms <= 20 ms.
    assert_eq!(v.evidence["actual_cutoff_latency_s"], 0.016);
    assert_eq!(v.evidence["primary_interlock_reported_open"], false);
    assert_eq!(v.evidence["monitor_interlock_reported_open"], true);
    assert_eq!(
        o.run.trace.events[0].name,
        "PrimaryDoorInterlock::ContactsWeldedClosed"
    );
}

#[test]
fn broken_interlock_variant_fails_mw_004_only() {
    let o = outcome(BROKEN_INTERLOCK_SCENARIO);
    assert!(!o.report.all_passed);
    assert_eq!(o.report.failed, 1);
    assert!(o.run.verdict("MW-001").unwrap().pass);
    let v = o.run.verdict("MW-004").unwrap();
    assert!(!v.pass);
    // First sample beyond the 20 ms limit after the door opened at 45 s.
    assert!((v.first_violation_s.unwrap() - 45.021).abs() < 1e-9);
    // The door-open event never reached the MCU: the oven kept cooking with the door open
    // until the 120 s timer ended the cycle (the relay is still within its 8 ms opening delay).
    assert_eq!(v.evidence["final_state"], "standby");
    assert_eq!(v.evidence["final_rf_power_w"], 900.0);
    let modes: Vec<_> = o.run.trace.events.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        modes,
        vec![
            "PrimaryDoorInterlock::ContactsWeldedClosed",
            "MonitorDoorInterlock::ContactsWeldedClosed",
            "cooking",
            "door_opened_mid_cook",
            "standby"
        ]
    );
    assert!(v.evidence["cutoff_complete_s"].is_null());
    assert_eq!(v.evidence["injected_fmea"].as_array().unwrap().len(), 2);
}

#[test]
fn runs_are_deterministic_json() {
    let a = run_scenario(BROKEN_INTERLOCK_SCENARIO, ReheatConfig::broken_interlock());
    let b = run_scenario(BROKEN_INTERLOCK_SCENARIO, ReheatConfig::broken_interlock());
    assert_eq!(a.run.to_json(), b.run.to_json());
    assert_eq!(
        serde_json::to_string(&a.report).unwrap(),
        serde_json::to_string(&b.report).unwrap()
    );
}

#[test]
fn verification_report_merges_expected_pass_scenarios() {
    let r = verification_report();
    assert!(r.all_passed);
    let ids: Vec<_> = r.verdicts.iter().map(|v| v.req_id.as_str()).collect();
    assert_eq!(ids, vec!["MW-001", "MW-004"]);
    let mw4 = &r.verdicts[1];
    assert_eq!(
        mw4.scenario,
        format!("{DOOR_OPEN_SCENARIO}, {PRIMARY_WELDED_SCENARIO}")
    );
    assert!(mw4.evidence[DOOR_OPEN_SCENARIO].is_object());
}
