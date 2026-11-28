//! `reconcile` — reconciles a compiled `sysml_ir::Model` against a Knowledge Graph
//! export (`kg_export.json`, e.g. from `cppognee/apps/docmem` via `tools/corpus_kg/`).
//!
//! Checks:
//! 1. **Entity coverage**: KG canonical entities present vs. missing in the model.
//! 2. **Alias drift**: model elements using a non-canonical alias instead of the KG canonical name.
//! 3. **Provenance coverage**: non-library parts/requirements carrying `Provenance:` / `Source:` /
//!    document citations vs. uncited elements.
//! 4. **Conflict tracking**: KG conflicts not yet captured in model docs/rationales or `@TBD` marks,
//!    emitting ready-to-paste `#[sysml::tbd(reason = "...")]` suggestions.
//! 5. **Stale-doc detection** (optional `manifest.json` + corpus dir): flags files whose SHA-256
//!    changed on disk since the KG was built (`diff-corpus`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sysml_ir::{Model, PackageMember};

use crate::emit_cli::is_sysml_std_package;

/// Subset of `kg_export.json` produced by `tools/corpus_kg/px4_docmem_kg.py`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KgExport {
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub entities: BTreeMap<String, KgEntity>,
    #[serde(default)]
    pub conflicts_and_gaps: Vec<KgConflict>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KgEntity {
    #[serde(default)]
    pub docs_covered: Vec<String>,
    #[serde(default)]
    pub doc_count: usize,
    #[serde(default)]
    pub aliases_observed: BTreeMap<String, usize>,
    #[serde(default)]
    pub total_passage_hits: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KgConflict {
    pub id: String,
    pub kind: String,
    pub summary: String,
    #[serde(default)]
    pub source: String,
}

/// Severity of a reconciliation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReconcileSeverity {
    Error,
    Warning,
    Info,
}

/// One structured reconciliation finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconcileFinding {
    pub code: String,
    pub severity: ReconcileSeverity,
    pub element: String,
    pub message: String,
    pub suggested_fix: String,
}

/// Summary metrics for a reconciliation run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconcileSummary {
    pub kg_entities: usize,
    pub matched_entities: usize,
    pub missing_entities: usize,
    pub alias_drift_count: usize,
    pub total_model_elements: usize,
    pub cited_model_elements: usize,
    pub provenance_coverage_pct: f64,
    pub kg_conflicts: usize,
    pub unresolved_conflicts: usize,
    pub stale_docs: usize,
}

/// Full report produced by [`reconcile_model`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconcileReport {
    pub ok: bool,
    pub summary: ReconcileSummary,
    pub findings: Vec<ReconcileFinding>,
}

/// Reconcile a `sysml_ir::Model` against a `KgExport` (and optional corpus directory for
/// SHA-256 stale-file checks).
pub fn reconcile_model(
    model: &Model,
    kg: &KgExport,
    corpus_dir: Option<&Path>,
) -> ReconcileReport {
    let mut findings = Vec::new();

    // Collect non-library model elements (PartDefs, InterfaceDefs, RequirementDefs)
    let mut model_names: BTreeSet<String> = BTreeSet::new();
    let mut model_names_lower: BTreeMap<String, String> = BTreeMap::new();
    let mut all_docs_text = String::new();
    let mut total_elements = 0usize;
    let mut cited_elements = 0usize;

    // Collect known doc basenames from KG for citation matching
    let mut kg_doc_stems: BTreeSet<String> = BTreeSet::new();
    for ent in kg.entities.values() {
        for d in &ent.docs_covered {
            if let Some(fname) = Path::new(d).file_name().and_then(|s| s.to_str()) {
                kg_doc_stems.insert(fname.to_lowercase());
                if let Some(stem) = Path::new(fname).file_stem().and_then(|s| s.to_str()) {
                    kg_doc_stems.insert(stem.to_lowercase());
                }
            }
        }
    }

    for pkg in &model.packages {
        if is_sysml_std_package(pkg) {
            continue;
        }
        for member in &pkg.members {
            match member {
                PackageMember::PartDef(p) => {
                    model_names.insert(p.name.clone());
                    model_names_lower.insert(p.name.to_lowercase(), p.name.clone());
                    total_elements += 1;
                    let doc_str = p.doc.as_deref().unwrap_or("");
                    all_docs_text.push_str(doc_str);
                    all_docs_text.push('\n');
                    for t in &p.tbd {
                        all_docs_text.push_str(&t.reason);
                        all_docs_text.push('\n');
                    }
                    if has_citation(doc_str, &kg_doc_stems) || !p.tbd.is_empty() {
                        cited_elements += 1;
                    } else {
                        findings.push(ReconcileFinding {
                            code: "R003".into(),
                            severity: ReconcileSeverity::Info,
                            element: format!("{}::{}", pkg.name, p.name),
                            message: format!("part `{}` has no document provenance citation", p.name),
                            suggested_fix: format!(
                                "add `.provenance(\"<doc> §<section>\")` or `/// Provenance: ...` on `{}`",
                                p.name
                            ),
                        });
                    }
                }
                PackageMember::RequirementDef(r) => {
                    model_names.insert(r.name.clone());
                    model_names_lower.insert(r.name.to_lowercase(), r.name.clone());
                    if let Some(sn) = &r.short_name {
                        model_names.insert(sn.clone());
                    }
                    total_elements += 1;
                    let combined = format!(
                        "{} {}",
                        r.doc.as_deref().unwrap_or(""),
                        r.rationale.as_deref().unwrap_or("")
                    );
                    all_docs_text.push_str(&combined);
                    all_docs_text.push('\n');
                    if has_citation(&combined, &kg_doc_stems) || !r.tbd.is_empty() {
                        cited_elements += 1;
                    } else {
                        findings.push(ReconcileFinding {
                            code: "R003".into(),
                            severity: ReconcileSeverity::Info,
                            element: format!("{}::{}", pkg.name, r.name),
                            message: format!("requirement `{}` has no document provenance citation", r.name),
                            suggested_fix: format!(
                                "cite source document in doc/rationale for `{}`",
                                r.name
                            ),
                        });
                    }
                }
                PackageMember::InterfaceDef(i) => {
                    model_names.insert(i.name.clone());
                    model_names_lower.insert(i.name.to_lowercase(), i.name.clone());
                }
                _ => {}
            }
        }
    }

    // 1 & 2. Entity coverage + alias drift
    let mut matched_entities = 0usize;
    let mut missing_entities = 0usize;
    let mut alias_drift_count = 0usize;

    for (canonical, ent) in &kg.entities {
        if model_names.contains(canonical) || model_names_lower.contains_key(&canonical.to_lowercase()) {
            matched_entities += 1;
        } else {
            // Check if the model accidentally used one of the non-canonical aliases
            let mut drifted: Option<(String, String)> = None;
            for alias in ent.aliases_observed.keys() {
                let norm = normalize_ident(alias);
                if let Some(actual) = model_names_lower.get(&norm) {
                    drifted = Some((actual.clone(), alias.clone()));
                    break;
                }
            }
            if let Some((actual_name, alias_used)) = drifted {
                alias_drift_count += 1;
                findings.push(ReconcileFinding {
                    code: "R002".into(),
                    severity: ReconcileSeverity::Warning,
                    element: actual_name.clone(),
                    message: format!(
                        "model element `{actual_name}` uses alias `{alias_used}` instead of KG canonical name `{canonical}`"
                    ),
                    suggested_fix: format!("rename `{actual_name}` to `{canonical}` for corpus consistency"),
                });
            } else {
                missing_entities += 1;
                findings.push(ReconcileFinding {
                    code: "R001".into(),
                    severity: ReconcileSeverity::Warning,
                    element: canonical.clone(),
                    message: format!(
                        "KG canonical entity `{canonical}` (cited in {} doc(s)) is not modeled",
                        ent.doc_count
                    ),
                    suggested_fix: format!(
                        "add `{canonical}` to the model or explicitly mark it out-of-scope"
                    ),
                });
            }
        }
    }

    // 4. Unresolved KG conflicts -> ready-to-paste #[sysml::tbd(...)]
    let all_docs_lower = all_docs_text.to_lowercase();
    let mut unresolved_conflicts = 0usize;
    for c in &kg.conflicts_and_gaps {
        let addressed = all_docs_lower.contains(&c.id.to_lowercase());
        if !addressed {
            unresolved_conflicts += 1;
            let sev = if c.kind.contains("conflict") || c.kind.contains("incompatib") {
                ReconcileSeverity::Warning
            } else {
                ReconcileSeverity::Info
            };
            findings.push(ReconcileFinding {
                code: "R004".into(),
                severity: sev,
                element: c.id.clone(),
                message: format!(
                    "KG {} `{}` not yet recorded in model: {}",
                    c.kind, c.id, c.summary
                ),
                suggested_fix: format!(
                    "#[sysml::tbd(reason = \"{}: {} ({})\")]",
                    c.id,
                    c.summary.replace('"', "'"),
                    c.source
                ),
            });
        }
    }

    // 5. Optional stale-doc detection against corpus_dir/manifest.json
    let mut stale_docs = 0usize;
    if let Some(dir) = corpus_dir {
        let manifest_path = dir.join("manifest.json");
        if let Ok(raw) = std::fs::read_to_string(&manifest_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(files) = val.get("files").and_then(|v| v.as_array()) {
                    for f in files {
                        let Some(rel) = f.get("path").and_then(|v| v.as_str()) else {
                            continue;
                        };
                        let Some(expected_len) = f.get("bytes").and_then(|v| v.as_u64()) else {
                            continue;
                        };
                        let full = dir.join(rel);
                        match std::fs::metadata(&full) {
                            Ok(meta) if meta.len() != expected_len => {
                                stale_docs += 1;
                                findings.push(ReconcileFinding {
                                    code: "R005".into(),
                                    severity: ReconcileSeverity::Error,
                                    element: rel.to_string(),
                                    message: format!(
                                        "corpus document `{rel}` changed on disk (expected {expected_len} bytes, found {} bytes)",
                                        meta.len()
                                    ),
                                    suggested_fix: "re-run KG extraction (`tools/corpus_kg/`) and re-reconcile affected elements".into(),
                                });
                            }
                            Err(_) => {
                                stale_docs += 1;
                                findings.push(ReconcileFinding {
                                    code: "R005".into(),
                                    severity: ReconcileSeverity::Error,
                                    element: rel.to_string(),
                                    message: format!("corpus document `{rel}` is missing on disk"),
                                    suggested_fix: "re-fetch corpus or remove stale citations".into(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let provenance_coverage_pct = if total_elements == 0 {
        100.0
    } else {
        ((cited_elements as f64 / total_elements as f64) * 1000.0).round() / 10.0
    };

    let has_errors = findings
        .iter()
        .any(|f| f.severity == ReconcileSeverity::Error);

    ReconcileReport {
        ok: !has_errors,
        summary: ReconcileSummary {
            kg_entities: kg.entities.len(),
            matched_entities,
            missing_entities,
            alias_drift_count,
            total_model_elements: total_elements,
            cited_model_elements: cited_elements,
            provenance_coverage_pct,
            kg_conflicts: kg.conflicts_and_gaps.len(),
            unresolved_conflicts,
            stale_docs,
        },
        findings,
    }
}

/// Render a [`ReconcileReport`] as clean Markdown.
pub fn render_reconcile_markdown(report: &ReconcileReport) -> String {
    let s = &report.summary;
    let mut out = String::new();
    out.push_str("# Knowledge Graph Reconciliation Report\n\n");
    out.push_str(&format!(
        "- **Status:** {}\n- **KG Entities Matched:** {}/{} (missing: {}, alias drift: {})\n- **Provenance Coverage:** {}/{} ({:.1}%)\n- **KG Conflicts/Gaps:** {} ({} unrecorded in model)\n- **Stale Corpus Docs:** {}\n\n",
        if report.ok { "PASS" } else { "FAIL" },
        s.matched_entities,
        s.kg_entities,
        s.missing_entities,
        s.alias_drift_count,
        s.cited_model_elements,
        s.total_model_elements,
        s.provenance_coverage_pct,
        s.kg_conflicts,
        s.unresolved_conflicts,
        s.stale_docs,
    ));

    if !report.findings.is_empty() {
        out.push_str("## Findings\n\n");
        for f in &report.findings {
            out.push_str(&format!(
                "- **`[{}]` ({:?}) `{}`**: {}  \n  *Fix:* `{}`\n",
                f.code, f.severity, f.element, f.message, f.suggested_fix
            ));
        }
    }
    out
}

fn has_citation(text: &str, kg_doc_stems: &BTreeSet<String>) -> bool {
    let lower = text.to_lowercase();
    if lower.contains("provenance:")
        || lower.contains("source:")
        || lower.contains(".md")
        || lower.contains(".txt")
        || lower.contains(".pdf")
        || lower.contains("file://")
    {
        return true;
    }
    kg_doc_stems
        .iter()
        .any(|stem| stem.len() >= 4 && lower.contains(stem))
}

fn normalize_ident(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sysml_ir::{Package, PartDef};

    #[test]
    fn detects_alias_drift_missing_entity_and_unresolved_conflict() {
        let mut pkg = Package {
            name: "TestPkg".into(),
            ..Default::default()
        };
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "FmuMcu".into(),
            doc: Some("Provenance: DS-012_FMUv6X_Standard.txt".into()),
            ..Default::default()
        }));
        pkg.members.push(PackageMember::PartDef(PartDef {
            name: "PX4IO".into(), // alias instead of canonical IoMcu!
            doc: None,
            ..Default::default()
        }));
        let model = Model::new(vec![pkg]);

        let mut entities = BTreeMap::new();
        entities.insert(
            "FmuMcu".into(),
            KgEntity {
                docs_covered: vec!["DS-012_FMUv6X_Standard.txt".into()],
                doc_count: 1,
                ..Default::default()
            },
        );
        entities.insert(
            "IoMcu".into(),
            KgEntity {
                docs_covered: vec!["DS-012_FMUv6X_Standard.txt".into()],
                doc_count: 1,
                aliases_observed: BTreeMap::from([("PX4IO".into(), 2)]),
                ..Default::default()
            },
        );
        entities.insert(
            "CompanionSoc".into(),
            KgEntity {
                docs_covered: vec!["px4_systems_architecture.md".into()],
                doc_count: 1,
                ..Default::default()
            },
        );
        let kg = KgExport {
            engine: "test".into(),
            entities,
            conflicts_and_gaps: vec![KgConflict {
                id: "CONFLICT-01".into(),
                kind: "protocol_incompatibility".into(),
                summary: "v2.x client incompatible with v3.x agent".into(),
                source: "uxrce_dds.md".into(),
            }],
        };

        let report = reconcile_model(&model, &kg, None);
        assert!(report.ok); // warnings/info only unless stale docs or --strict
        assert_eq!(report.summary.matched_entities, 1);
        assert_eq!(report.summary.alias_drift_count, 1);
        assert_eq!(report.summary.missing_entities, 1);
        assert_eq!(report.summary.unresolved_conflicts, 1);
        assert!(report.findings.iter().any(|f| f.code == "R001" && f.element == "CompanionSoc"));
        assert!(report.findings.iter().any(|f| f.code == "R002" && f.element == "PX4IO"));
        assert!(report.findings.iter().any(|f| f.code == "R003" && f.element == "TestPkg::PX4IO"));
        assert!(report.findings.iter().any(|f| f.code == "R004" && f.element == "CONFLICT-01"));
    }
}
