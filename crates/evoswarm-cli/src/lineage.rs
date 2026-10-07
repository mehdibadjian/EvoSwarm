//! Lineage CLI presentation and graph traversal (e2-8).
//!
//! Provides rendering of job ancestry into formatted ASCII trees or JSON representation,
//! reporting generation numbers, candidate IDs, model roles, fitness scores, and gate failure reasons.

use evoswarm_memory::falkordb::JobRecord;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LineageError {
    #[error("job lineage not found for '{0}'")]
    NotFound(String),
    #[error("failed to read lineage record: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to deserialize lineage record: {0}")]
    Deserialization(#[from] serde_json::Error),
}

/// Reads a recorded job lineage from disk (local retry spool or archive directory).
pub fn load_job_lineage(job_id: &str, spool_dir: &Path) -> Result<JobRecord, LineageError> {
    let candidate_path = spool_dir.join(format!("{job_id}.json"));
    if !candidate_path.exists() {
        return Err(LineageError::NotFound(job_id.to_string()));
    }
    let content = fs::read_to_string(&candidate_path)?;
    let record: JobRecord = serde_json::from_str(&content)?;
    Ok(record)
}

/// Formats the job lineage into a human-readable ASCII ancestry tree.
pub fn format_ascii_tree(record: &JobRecord) -> String {
    let mut out = String::new();
    out.push_str(&format!("Lineage Tree for Task `{}`\n", record.task.id));
    out.push_str(&format!(
        "Spec Hash: {}\nRepo: {}\nToolchain: {}\n\n",
        record.task.spec_hash, record.task.repo_fingerprint, record.task.toolchain_fingerprint
    ));

    // Map candidate IDs to their evaluations
    let mut evals_by_cand = HashMap::new();
    for ev in &record.evaluations {
        evals_by_cand.insert(ev.candidate_id.as_str(), ev);
    }

    // Sort implementations by generation, then id
    let mut impls = record.implementations.clone();
    impls.sort_by_key(|i| (i.generation, i.id.clone()));

    for imp in impls {
        let eval_info = if let Some(ev) = evals_by_cand.get(imp.id.as_str()) {
            let status = if ev.passed_gates {
                "passed gates".to_string()
            } else {
                let failed_tests: Vec<_> = ev
                    .tests
                    .iter()
                    .filter(|t| !t.passed)
                    .map(|t| t.name.as_str())
                    .collect();
                if failed_tests.is_empty() {
                    "failed gates".to_string()
                } else {
                    format!("failed ({})", failed_tests.join(", "))
                }
            };
            format!("score: {:.2}, status: {}", ev.score, status)
        } else {
            "score: n/a".to_string()
        };

        let parents_str = if imp.parents.is_empty() {
            "(root)".to_string()
        } else {
            let p_list: Vec<_> = imp
                .parents
                .iter()
                .map(|p| format!("{} ({})", p.parent_id, p.relation))
                .collect();
            p_list.join(", ")
        };

        let indent = "  ".repeat(imp.generation as usize);
        out.push_str(&format!(
            "{indent}* [gen {}] `{}` ← {} [model: {}] [{}]\n",
            imp.generation, imp.id, parents_str, imp.model_role, eval_info
        ));
    }

    out
}

/// Exports the job lineage as formatted JSON.
pub fn format_json_export(record: &JobRecord) -> Result<String, LineageError> {
    Ok(serde_json::to_string_pretty(record)?)
}
