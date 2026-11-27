//! Named scenario registry used by `rsysdsl verify`.

use serde::Serialize;
use sysml_ir::VerificationReport;

use crate::RunResult;

/// What running a registered scenario produces: the requirement report and the raw run.
#[derive(Debug, Clone)]
pub struct ScenarioOutcome {
    pub report: VerificationReport,
    pub run: RunResult,
}

/// A scenario that runner crates expose to the CLI.
#[derive(Debug, Clone, Serialize)]
pub struct RegisteredScenario {
    /// Unique scenario name (`--scenario <name>`).
    pub name: &'static str,
    /// Short alternative names (`nominal`, `fault`, ...).
    pub aliases: &'static [&'static str],
    /// Key of this scenario's report in the combined `--scenario all` output.
    pub key: &'static str,
    /// Model the scenario exercises.
    pub model: &'static str,
    pub description: &'static str,
    /// `true` if every requirement is expected to pass; `false` for fault-injection
    /// scenarios that must expose at least one failure.
    pub expect_pass: bool,
    #[serde(skip)]
    pub run: fn() -> ScenarioOutcome,
}

impl RegisteredScenario {
    /// Whether an outcome matches this scenario's expectation.
    pub fn meets_expectation(&self, outcome: &ScenarioOutcome) -> bool {
        outcome.report.all_passed == self.expect_pass
    }
}

/// Ordered collection of registered scenarios.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    entries: Vec<RegisteredScenario>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add scenarios. Panics on a duplicate name or alias (a programming error).
    pub fn extend(&mut self, scenarios: impl IntoIterator<Item = RegisteredScenario>) {
        for s in scenarios {
            for n in std::iter::once(s.name).chain(s.aliases.iter().copied()) {
                assert!(self.find(n).is_none(), "duplicate scenario name '{n}'");
            }
            self.entries.push(s);
        }
    }

    /// Look up by name or alias.
    pub fn find(&self, name: &str) -> Option<&RegisteredScenario> {
        self.entries
            .iter()
            .find(|s| s.name == name || s.aliases.contains(&name))
    }

    pub fn iter(&self) -> impl Iterator<Item = &RegisteredScenario> {
        self.entries.iter()
    }
}
