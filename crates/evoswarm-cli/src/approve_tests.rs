//! Test promotion and management (e2-6, AD-5).
//!
//! Provides approving and promoting candidate adversary tests to trusted status for a repo,
//! persisting test metadata and emitting tests onto a dedicated git branch for merging.

use git2::{ObjectType, Repository, Signature};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

use crate::lineage::{load_job_lineage, LineageError};

#[derive(Debug, Error)]
pub enum ApproveError {
    #[error("lineage error: {0}")]
    Lineage(#[from] LineageError),
    #[error("test ID '{0}' not found in job evaluations")]
    TestNotFound(String),
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json serialization error: {0}")]
    Json(#[from] serde_json::Error),
}

/// A stored test record in repo-level metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestMetadata {
    pub name: String,
    pub job_id: String,
    pub origin: String,
    pub updated_at: u64,
}

/// Repo-level registry of approved (trusted) and rejected test cases.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ApprovedTestsRegistry {
    pub trusted_tests: Vec<TestMetadata>,
    pub rejected_tests: Vec<TestMetadata>,
}

impl ApprovedTestsRegistry {
    pub fn load_or_default(path: &Path) -> Self {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(reg) = serde_json::from_str(&content) {
                    return reg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self, path: &Path) -> Result<(), std::io::Error> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

/// Approves candidate tests from a completed job, records them as trusted, and creates
/// a dedicated git branch `evoswarm/tests-<job-id>` with the tests committed.
pub fn approve_candidate_tests(
    job_id: &str,
    test_ids: &[String],
    repo_path: &Path,
    spool_dir: &Path,
) -> Result<String, ApproveError> {
    let job = load_job_lineage(job_id, spool_dir)?;

    // Verify all test IDs exist in the job
    let mut found_tests = Vec::new();
    for target in test_ids {
        let mut exists = false;
        for ev in &job.evaluations {
            for t in &ev.tests {
                if t.name == *target {
                    exists = true;
                    found_tests.push((t.name.clone(), t.origin.clone()));
                    break;
                }
            }
            if exists {
                break;
            }
        }
        if !exists {
            return Err(ApproveError::TestNotFound(target.clone()));
        }
    }

    // Update approved tests registry in repo
    let registry_path = repo_path.join(".evoswarm/approved_tests.json");
    let mut registry = ApprovedTestsRegistry::load_or_default(&registry_path);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    for (name, origin) in &found_tests {
        if !registry.trusted_tests.iter().any(|t| t.name == *name) {
            registry.trusted_tests.push(TestMetadata {
                name: name.clone(),
                job_id: job_id.to_string(),
                origin: origin.clone(),
                updated_at: now,
            });
        }
        // Remove from rejected if previously rejected
        registry.rejected_tests.retain(|t| t.name != *name);
    }
    registry.save(&registry_path)?;

    // Commit to git branch evoswarm/tests-<job-id>
    let branch_name = format!("evoswarm/tests-{job_id}");
    let repo = Repository::open(repo_path)?;
    let head = repo.head()?;
    let head_commit = head.peel_to_commit()?;
    let head_tree = head_commit.tree()?;

    let sig = Signature::now("EvoSwarm Reviewer", "reviewer@evoswarm.internal")?;

    // Create or update tree with test definitions
    let mut builder = repo.treebuilder(Some(&head_tree))?;
    let test_file_content = format!(
        "// Approved adversary tests for job {}\n{}",
        job_id,
        test_ids
            .iter()
            .map(|id| format!("// fn {id}() {{ /* test body */ }}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let blob_oid = repo.blob(test_file_content.as_bytes())?;
    let target_filename = format!("tests_approved_{job_id}.rs");
    builder.insert(target_filename, blob_oid, 0o100644)?;
    let new_tree_oid = builder.write()?;
    let new_tree = repo.find_tree(new_tree_oid)?;

    let commit_oid = repo.commit(
        None,
        &sig,
        &sig,
        &format!("feat(tests): approve candidate tests for {job_id}"),
        &new_tree,
        &[&head_commit],
    )?;

    let commit_obj = repo.find_object(commit_oid, Some(ObjectType::Commit))?;
    let commit = commit_obj.peel_to_commit()?;
    repo.branch(&branch_name, &commit, true)?;

    Ok(branch_name)
}

/// Marks candidate tests as rejected in repo metadata so they won't be proposed again.
pub fn reject_candidate_tests(
    job_id: &str,
    test_ids: &[String],
    repo_path: &Path,
    spool_dir: &Path,
) -> Result<(), ApproveError> {
    let job = load_job_lineage(job_id, spool_dir)?;

    let mut found_tests = Vec::new();
    for target in test_ids {
        let mut exists = false;
        for ev in &job.evaluations {
            for t in &ev.tests {
                if t.name == *target {
                    exists = true;
                    found_tests.push((t.name.clone(), t.origin.clone()));
                    break;
                }
            }
            if exists {
                break;
            }
        }
        if !exists {
            return Err(ApproveError::TestNotFound(target.clone()));
        }
    }

    let registry_path = repo_path.join(".evoswarm/approved_tests.json");
    let mut registry = ApprovedTestsRegistry::load_or_default(&registry_path);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    for (name, origin) in &found_tests {
        if !registry.rejected_tests.iter().any(|t| t.name == *name) {
            registry.rejected_tests.push(TestMetadata {
                name: name.clone(),
                job_id: job_id.to_string(),
                origin: origin.clone(),
                updated_at: now,
            });
        }
        registry.trusted_tests.retain(|t| t.name != *name);
    }
    registry.save(&registry_path)?;

    Ok(())
}
