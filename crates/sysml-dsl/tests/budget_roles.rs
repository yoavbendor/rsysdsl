//! C1.1 / D1: `#[sysml::attr(..., budget = "<role>")]` tags an attribute with the budget it
//! contributes to. The unit's dimension must match the role (checked by `rustc`, see the
//! `bad_budget_*` trybuild cases); the tag is emitted as `@SysmlStdBudget::BudgetRole`
//! metadata (package auto-included), passes the strict OMG audit and is lifted back into the IR.

use sysml_dsl::ir::{BudgetRole, PackageMember};
use sysml_dsl::prelude::*;

#[sysml::package(name = "BudgetRoleDemo")]
#[sysml::import("ScalarValues::*")]
#[sysml::import("SI::*")]
pub mod budget_role_demo {
    use super::*;

    #[sysml::part]
    #[derive(Debug, Clone, Copy)]
    pub struct Heater {
        #[sysml::attr(budget = "power", default = 2000.0, unit = "W")]
        pub rated_power_w: Quantity<f64>,
        #[sysml::attr(budget = "mass", default = 1.2, unit = "kg")]
        pub mass_kg: Quantity<f64>,
        #[sysml::attr(budget = "efficiency", default = 0.97, dimensionless)]
        pub eta: f64,
        /// Unit cost in the model's currency (EUR); the SI table has no currency unit.
        #[sysml::attr(budget = "cost", default = 35.0, dimensionless)]
        pub bom_cost: f64,
        #[sysml::attr(budget = "latency", default = 0.5, unit = "s")]
        pub response_time: Quantity<f64>,
        /// Supply capacity, not a consumer: explicitly out of the power budget.
        #[sysml::attr(budget = "none", default = 3000.0, unit = "W")]
        pub supply_capacity: Quantity<f64>,
        #[sysml::attr(default = 230.0, unit = "V")]
        pub voltage: Quantity<f64>,
    }
}

fn role_of(model: &Model, attr: &str) -> Option<BudgetRole> {
    model.packages.iter().find_map(|p| {
        p.members.iter().find_map(|m| match m {
            PackageMember::PartDef(d) if d.name == "Heater" => d
                .attributes
                .iter()
                .find(|a| a.name.as_deref() == Some(attr))
                .and_then(|a| a.budget_role),
            _ => None,
        })
    })
}

#[test]
fn budget_roles_emit_audit_and_round_trip() {
    let model = Model::new(vec![budget_role_demo::sysml_package()]);
    assert_eq!(role_of(&model, "ratedPowerW"), Some(BudgetRole::Power));
    assert_eq!(role_of(&model, "voltage"), None);

    let sysml = sysml_emit::emit_model(&model);
    assert!(
        sysml.contains("@SysmlStdBudget::BudgetRole { role = \"power\"; }"),
        "{sysml}"
    );
    assert!(sysml.contains("library package SysmlStdBudget"), "auto-included: {sysml}");

    let audit = sysml_emit::audit_model_with_stdlib("budget_roles", &model, None, None, true);
    assert!(
        audit.ok && audit.validation_errors == 0 && audit.validation_warnings == 0,
        "{audit:#?}\n{sysml}"
    );

    let lifted = sysml_lift::lift_sysml_units(&[("roles.sysml".to_string(), sysml)]);
    assert!(lifted.parse_errors.is_empty(), "{:#?}", lifted.parse_errors);
    let m = &lifted.model;
    assert_eq!(role_of(m, "ratedPowerW"), Some(BudgetRole::Power));
    assert_eq!(role_of(m, "massKg"), Some(BudgetRole::Mass));
    assert_eq!(role_of(m, "eta"), Some(BudgetRole::Efficiency));
    assert_eq!(role_of(m, "bomCost"), Some(BudgetRole::Cost));
    assert_eq!(role_of(m, "responseTime"), Some(BudgetRole::Latency));
    assert_eq!(role_of(m, "supplyCapacity"), Some(BudgetRole::None));
    assert_eq!(role_of(m, "voltage"), None);
}

#[test]
fn omit_std_split_keeps_budget_package_with_the_std_half() {
    let model = Model::new(vec![budget_role_demo::sysml_package()]);
    let (own, std_half) = sysml_emit::emit_model_split_std(&model);
    assert!(own.contains("@SysmlStdBudget::BudgetRole"), "{own}");
    assert!(!own.contains("library package SysmlStdBudget"), "{own}");
    assert!(std_half.contains("library package SysmlStdBudget"), "{std_half}");
}
