//! C2 property test: the IR expression evaluator (`sysml_emit::expr_eval`), which only sees
//! the emitted SysML strings, must agree with the compiled Rust of every
//! `#[sysml::requirement]` with a compiled `require_constraint` and every numeric
//! `#[sysml::calc]` in the examples and in `sysml-std`, on their defaults and on perturbed value
//! sets. Also: every expression string the examples emit must parse, before and after an
//! emit -> lift round trip.

use sysml_emit::expr_eval::agreement::{check, expression_strings, AgreementReport};
use sysml_emit::expr_eval::parse_expr;
use sysml_ir::{CompiledProbes, Model};

const RANDOM_SETS: usize = 24;

fn std_probes() -> CompiledProbes {
    use sysml_std::*;
    let mut p = CompiledProbes::default();
    p.extend(flows::sysml_std_flows::sysml_compiled_probes());
    p.extend(comms::sysml_std_comms::sysml_compiled_probes());
    p.extend(power::sysml_std_power::sysml_compiled_probes());
    p.extend(mechanics::sysml_std_mechanics::sysml_compiled_probes());
    p.extend(electronics::sysml_std_electronics::sysml_compiled_probes());
    p.extend(hmi::sysml_std_hmi::sysml_compiled_probes());
    p.extend(natural::sysml_std_natural::sysml_compiled_probes());
    p
}

fn apollo_probes() -> CompiledProbes {
    use apollo11_model::*;
    let mut p = CompiledProbes::default();
    p.extend(cosma_package::sysml_compiled_probes());
    p.extend(operations_package::sysml_compiled_probes());
    p.extend(requirements_package::sysml_compiled_probes());
    p.extend(functions_package::sysml_compiled_probes());
    p.extend(logical_components_package::sysml_compiled_probes());
    p.extend(technical_ports_package::sysml_compiled_probes());
    p.extend(technical_components_package::sysml_compiled_probes());
    p.extend(mission_phases_package::sysml_compiled_probes());
    p.extend(calculations_package::sysml_compiled_probes());
    p.extend(system_specification_package::sysml_compiled_probes());
    p
}

/// (name, model, probes of the model's own packages)
fn examples() -> Vec<(&'static str, Model, CompiledProbes)> {
    vec![
        ("sysml-std", Model::new(sysml_std::std_packages()), std_probes()),
        ("apollo11", apollo11_model::build_apollo11_model(), apollo_probes()),
        (
            "embedded-flight-controller",
            Model::new(vec![embedded_flight_controller::flight_controller::sysml_package()]),
            embedded_flight_controller::flight_controller::sysml_compiled_probes(),
        ),
        (
            "microwave",
            microwave::build_model(),
            microwave::microwave_model::sysml_compiled_probes(),
        ),
        (
            "dishwasher",
            dishwasher::build_model(),
            dishwasher::dishwasher_model::sysml_compiled_probes(),
        ),
        (
            "power-tools-family",
            power_tools_family::build_model(),
            power_tools_family::power_tools_family_model::sysml_compiled_probes(),
        ),
        (
            "bicycle",
            bicycle::build_model(),
            bicycle::bicycle_model::sysml_compiled_probes(),
        ),
        (
            "earth-water-cycle",
            earth_water_cycle::build_model(),
            earth_water_cycle::earth_water_cycle_model::sysml_compiled_probes(),
        ),
        (
            "solar-system",
            solar_system::build_model(),
            solar_system::solar_system_model::sysml_compiled_probes(),
        ),
        (
            "starter-model",
            starter_model::build_model(),
            starter_model::starter_architecture::sysml_compiled_probes(),
        ),
    ]
}

#[test]
fn ir_evaluator_agrees_with_compiled_requirements_and_calcs() {
    let mut total = AgreementReport::default();
    let mut lines = Vec::new();
    for (name, model, probes) in examples() {
        let r = check(&model, &probes, RANDOM_SETS);
        lines.push(format!(
            "{name}: {} requirement(s) ({} pass / {} fail cases), {} calc(s), {} compared case(s), {} unknown on defaults, without probe: {:?}",
            r.requirements_compared,
            r.requirement_passes,
            r.requirement_fails,
            r.calcs_compared,
            r.cases_compared,
            r.cases_unknown,
            r.requirements_without_probe
        ));
        total.requirement_passes += r.requirement_passes;
        total.requirement_fails += r.requirement_fails;
        total.requirements_without_probe.extend(r.requirements_without_probe.clone());
        total.requirements_compared += r.requirements_compared;
        total.calcs_compared += r.calcs_compared;
        total.cases_compared += r.cases_compared;
        total.cases_unknown += r.cases_unknown;
        total
            .disagreements
            .extend(r.disagreements.into_iter().map(|d| format!("{name}: {d}")));
        total
            .unmatched_probes
            .extend(r.unmatched_probes.into_iter().map(|d| format!("{name}: {d}")));
    }
    eprintln!("{}", lines.join("\n"));
    eprintln!(
        "TOTAL: {} requirements, {} calcs, {} compared cases, {} unknown",
        total.requirements_compared, total.calcs_compared, total.cases_compared, total.cases_unknown
    );
    assert!(
        total.unmatched_probes.is_empty(),
        "probes without an IR definition: {:#?}",
        total.unmatched_probes
    );
    assert!(
        total.disagreements.is_empty(),
        "{} disagreement(s):\n{}",
        total.disagreements.len(),
        total.disagreements.join("\n")
    );
    // Coverage floor: the examples have dozens of compiled requirements and calcs; a probe
    // generator regression must not make this test vacuous.
    assert!(total.requirements_compared >= 26, "{}", lines.join("\n"));
    assert!(total.calcs_compared >= 28, "{}", lines.join("\n"));
    // The perturbed sets must exercise both outcomes, or agreement says little.
    assert!(total.requirement_passes > 50 && total.requirement_fails > 50, "{}", lines.join("\n"));
    // Every example requirement with a constraint has a compiled probe (no string-literal
    // constraints left unchecked).
    assert!(
        total.requirements_without_probe.is_empty(),
        "requirements with constraints but no compiled probe: {:?}",
        total.requirements_without_probe
    );
}

#[test]
fn every_emitted_expression_parses_before_and_after_lift() {
    let mut failures = Vec::new();
    let mut count = 0usize;
    for (name, model, _) in examples() {
        for (loc, src) in expression_strings(&model) {
            count += 1;
            if let Err(e) = parse_expr(&src) {
                failures.push(format!("{name}: {loc}: `{src}`: {e}"));
            }
        }
        let text = sysml_emit::emit_model(&model);
        let lifted = sysml_lift::lift_sysml_source(&format!("{name}.sysml"), &text).model;
        for (loc, src) in expression_strings(&lifted) {
            count += 1;
            if let Err(e) = parse_expr(&src) {
                failures.push(format!("{name} (lifted): {loc}: `{src}`: {e}"));
            }
        }
    }
    eprintln!("parsed {count} expression strings");
    assert!(count > 100);
    assert!(failures.is_empty(), "{} unparsed:\n{}", failures.len(), failures.join("\n"));
}

/// Models authored by other agents (Haiku trial 3) and lifted from `.sysml`: every stored
/// expression parses too.
#[test]
fn every_trial3_fixture_expression_parses() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/haiku_t3");
    let mut count = 0usize;
    let mut failures = Vec::new();
    for f in std::fs::read_dir(&dir).unwrap() {
        let f = f.unwrap().path();
        if f.extension().is_none_or(|e| e != "sysml") {
            continue;
        }
        let text = std::fs::read_to_string(&f).unwrap();
        let model = sysml_lift::lift_sysml_source(&f.display().to_string(), &text).model;
        for (loc, src) in expression_strings(&model) {
            count += 1;
            if let Err(e) = parse_expr(&src) {
                failures.push(format!("{}: {loc}: `{src}`: {e}", f.display()));
            }
        }
    }
    assert!(count >= 80, "only {count} expressions found");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
