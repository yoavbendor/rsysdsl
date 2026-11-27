//! Requirement limit vs. evidence analysis (S6.5).
//!
//! A requirement def holds spec **limits** (from the source document) and **evidence** values
//! (measured / achieved, supplied by a verification case). `#[sysml::attr(..., evidence)]`
//! marks the latter. A model value on an evidence attribute is a placeholder: it never counts
//! as verification, only a `VerificationReport` verdict does. A requirement whose constraint
//! compares two attributes holding the same model value is *hollow* (the "measured" value was
//! copied from the limit).

use sysml_ir::RequirementDef;

/// Evidence classification of one requirement def (independent of verification verdicts).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RequirementEvidence {
    /// Attributes marked `evidence`.
    pub evidence_attributes: Vec<String>,
    /// The subset of [`Self::evidence_attributes`] that carry a model value (a default):
    /// placeholders, as opposed to evidence attributes left without a value.
    pub evidence_with_model_value: Vec<String>,
    /// Comparisons `a op b` whose two attribute operands carry the same model value.
    pub hollow_comparisons: Vec<String>,
}

fn attr_value(r: &RequirementDef, name: &str) -> Option<f64> {
    r.attributes
        .iter()
        .find(|a| a.name.as_deref() == Some(name))
        .and_then(|a| a.value.as_deref())
        .and_then(|v| v.trim().parse::<f64>().ok())
        .filter(|v| v.is_finite())
}

fn is_ident(s: &str) -> bool {
    let s = s.trim();
    !s.is_empty()
        && s.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Classify the evidence of a requirement def.
pub fn analyze_requirement(r: &RequirementDef) -> RequirementEvidence {
    let evidence_attributes: Vec<String> = r
        .attributes
        .iter()
        .filter(|a| a.is_evidence)
        .filter_map(|a| a.name.clone())
        .collect();
    let evidence_with_model_value: Vec<String> = r
        .attributes
        .iter()
        .filter(|a| a.is_evidence && a.value.as_deref().is_some_and(|v| !v.trim().is_empty()))
        .filter_map(|a| a.name.clone())
        .collect();
    let mut hollow = Vec::new();
    for c in &r.require_constraints {
        for clause in c.split(&['&', '|', ';'][..]).flat_map(|s| s.split(" and ")) {
            let clause = clause.trim().trim_matches(|ch| ch == '(' || ch == ')' || ch == '{' || ch == '}');
            for op in ["<=", ">=", "==", "<", ">"] {
                if let Some((lhs, rhs)) = clause.split_once(op) {
                    let (lhs, rhs) = (lhs.trim(), rhs.trim().trim_start_matches('='));
                    let rhs = rhs.trim();
                    if is_ident(lhs) && is_ident(rhs) {
                        if let (Some(a), Some(b)) = (attr_value(r, lhs), attr_value(r, rhs)) {
                            if a == b {
                                hollow.push(format!("{lhs} {op} {rhs} (both = {a})"));
                            }
                        }
                    }
                    break;
                }
            }
        }
    }
    RequirementEvidence {
        evidence_attributes,
        evidence_with_model_value,
        hollow_comparisons: hollow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysml_ir::AttributeUsage;

    fn attr(name: &str, v: Option<&str>, evidence: bool) -> AttributeUsage {
        AttributeUsage {
            name: Some(name.into()),
            value: v.map(Into::into),
            is_evidence: evidence,
            ..Default::default()
        }
    }

    #[test]
    fn detects_copied_limit_and_evidence_marker() {
        let r = RequirementDef {
            name: "R".into(),
            attributes: vec![
                attr("maxW", Some("800.0"), false),
                attr("measuredW", Some("800.0"), true),
            ],
            require_constraints: vec!["measuredW <= maxW".into()],
            ..Default::default()
        };
        let e = analyze_requirement(&r);
        assert_eq!(e.evidence_attributes, vec!["measuredW".to_string()]);
        assert_eq!(e.hollow_comparisons.len(), 1);

        let r2 = RequirementDef {
            attributes: vec![attr("maxW", Some("800.0"), false), attr("measuredW", None, true)],
            ..r
        };
        assert!(analyze_requirement(&r2).hollow_comparisons.is_empty());
    }
}
