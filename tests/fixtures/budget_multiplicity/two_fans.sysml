package TwoFansUnit {
    doc /* C1.6 regression: one fan definition used twice (and a [2] filter-heater bank). */
    private import ScalarValues::*;
    private import SI::*;

    part def Fan {
        doc /* 170 W EC fan. */
        attribute ratedPower : Real default 170.0 [W] {
            @SysmlStdBudget::BudgetRole { role = "power"; }
        }
        attribute mass : Real default 2.5 [kg] {
            @SysmlStdBudget::BudgetRole { role = "mass"; }
        }
    }

    part def FrostHeater {
        doc /* 50 W frost-protection heater, two per unit. */
        attribute ratedPower : Real default 50.0 [W] {
            @SysmlStdBudget::BudgetRole { role = "power"; }
        }
    }

    part def VentilationUnit {
        doc /* Top-level context: supply and exhaust fans share one definition. */
        part supplyFan : Fan;
        part exhaustFan : Fan;
        part heaters[2] : FrostHeater;
    }
}
