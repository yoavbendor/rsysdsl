//! Executable verification report and state-transition error types.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Pass/fail verdict status for a requirement verification check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Pass,
    Fail,
    Skipped,
}

/// Traceable verification result linked directly to a SysML requirement ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationVerdict {
    pub req_id: String,
    pub req_name: String,
    pub status: VerificationStatus,
    pub scenario: String,
    pub satisfied_by: String,
    pub evidence: serde_json::Value,
    pub message: String,
}

/// Complete JSON-serializable verification report across one or more scenarios.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerificationReport {
    pub model: String,
    pub passed: usize,
    pub failed: usize,
    pub all_passed: bool,
    pub verdicts: Vec<VerificationVerdict>,
}

impl VerificationReport {
    pub fn from_verdicts(model: impl Into<String>, verdicts: Vec<VerificationVerdict>) -> Self {
        let passed = verdicts
            .iter()
            .filter(|v| v.status == VerificationStatus::Pass)
            .count();
        let failed = verdicts
            .iter()
            .filter(|v| v.status == VerificationStatus::Fail)
            .count();
        Self {
            model: model.into(),
            passed,
            failed,
            all_passed: failed == 0,
            verdicts,
        }
    }
}

/// Error returned when an executable `#[sysml::state]` state machine receives
/// an illegal trigger in its current state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionError {
    pub state_machine: String,
    pub from: String,
    pub trigger: String,
    pub allowed_triggers: Vec<String>,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "illegal transition in {} from '{}' on trigger '{}' (allowed: {:?})",
            self.state_machine, self.from, self.trigger, self.allowed_triggers
        )
    }
}

impl std::error::Error for TransitionError {}
