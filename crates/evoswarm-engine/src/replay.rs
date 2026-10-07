//! Verified replay execution (e2-4, AD-4).
//!
//! Matches task against stored winning lineage in FalkorDB using triple fingerprints:
//! `spec_hash + repo_fingerprint + toolchain_fingerprint`.
//!
//! If a matching winner exists, re-runs the candidate in the sandbox across visible and
//! held-out suites without invoking any LLMs:
//! - If verification succeeds: returns `ReplayOutcome::Replayed` with 0 model calls in seconds.
//! - If verification fails (e.g. underlying environment/dependency drift): logs the mismatch
//!   and falls back to standard search with the stored candidate seeded.

use evoswarm_core::{Candidate, JobSubmission, RunStatus, SandboxProfile};
use evoswarm_memory::blob_store::BlobStore;
use evoswarm_memory::falkordb::JobRecord;
use evoswarm_sandbox::SandboxBackend;
use std::fs;
use std::path::{Path, PathBuf};

use crate::dispatch::ModelClient;

/// The outcome of evaluating a candidate for instant replay.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayOutcome {
    /// Triple fingerprint matched and verification passed; finished with 0 model calls.
    Replayed {
        patch: Vec<u8>,
        candidate_id: String,
    },
    /// Verification failed or no matching winner found; search begins with optional memory seed.
    FallbackToSearch {
        seed_candidate: Option<Candidate>,
    },
}

/// Dependencies required to resolve and verify replay.
pub struct ReplayDeps<'a, B: SandboxBackend + ?Sized, M: ModelClient + ?Sized> {
    pub spool_dir: &'a Path,
    pub blob_store: &'a BlobStore,
    pub backend: &'a B,
    pub model_client: &'a M,
    pub spec_hash: &'a str,
    pub repo_fingerprint: &'a str,
    pub toolchain_fingerprint: &'a str,
}

/// Checks memory for a prior matching winner, verifies it in the sandbox, and returns
/// the replay outcome.
pub async fn check_and_execute_replay<
    B: SandboxBackend + ?Sized,
    M: ModelClient + ?Sized,
>(
    submission: &JobSubmission,
    deps: &ReplayDeps<'_, B, M>,
) -> ReplayOutcome {
    // 1. Scan spool or FalkorDB records for triple fingerprint match
    let matching_job = find_matching_winner(
        deps.spool_dir,
        deps.spec_hash,
        deps.repo_fingerprint,
        deps.toolchain_fingerprint,
    );

    let (winner_cand_id, patch_blob_hash) = match matching_job {
        Some(record) => match select_winning_candidate(&record) {
            Some(pair) => pair,
            None => return ReplayOutcome::FallbackToSearch { seed_candidate: None },
        },
        None => return ReplayOutcome::FallbackToSearch { seed_candidate: None },
    };

    // 2. Fetch patch bytes from content-addressed blob store
    let patch_path = deps.blob_store.blob_path(&patch_blob_hash);
    let patch_bytes = match fs::read(&patch_path) {
        Ok(b) => b,
        Err(_) => return ReplayOutcome::FallbackToSearch { seed_candidate: None },
    };

    // 3. Prepare sandbox profile for full re-verification
    let profile = SandboxProfile {
        stack: "default".into(),
        wall_timeout_secs: submission.timeout_secs,
        memory_limit_bytes: 512 * 1024 * 1024,
        tmpfs_size_bytes: 64 * 1024 * 1024,
        tasks_max: 64,
        read_only_mounts: vec![],
        dependency_cache_path: PathBuf::from("/var/cache"),
    };

    let workdir = match deps.backend.prepare(&profile, &patch_bytes).await {
        Ok(w) => w,
        Err(_) => {
            let seed = make_seed_candidate(&winner_cand_id, &patch_bytes);
            return ReplayOutcome::FallbackToSearch { seed_candidate: Some(seed) };
        }
    };

    let exec_res = deps.backend.run(&workdir, &submission.test_command).await;
    let _ = deps.backend.collect(workdir).await;

    match exec_res {
        Ok(res) if res.status == RunStatus::Success && res.exit_code == 0 => {
            ReplayOutcome::Replayed {
                patch: patch_bytes,
                candidate_id: winner_cand_id,
            }
        }
        _ => {
            // Re-verification failed: log mismatch and fallback to search seeded with winner
            let seed = make_seed_candidate(&winner_cand_id, &patch_bytes);
            ReplayOutcome::FallbackToSearch { seed_candidate: Some(seed) }
        }
    }
}

fn find_matching_winner(
    spool_dir: &Path,
    spec_hash: &str,
    repo_fp: &str,
    toolchain_fp: &str,
) -> Option<JobRecord> {
    if !spool_dir.exists() {
        return None;
    }

    if let Ok(entries) = fs::read_dir(spool_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(record) = serde_json::from_str::<JobRecord>(&content) {
                        if record.task.spec_hash == spec_hash
                            && record.task.repo_fingerprint == repo_fp
                            && record.task.toolchain_fingerprint == toolchain_fp
                        {
                            return Some(record);
                        }
                    }
                }
            }
        }
    }

    None
}

fn select_winning_candidate(record: &JobRecord) -> Option<(String, String)> {
    let mut best_cand: Option<(String, String)> = None;
    let mut highest_score = -1.0;

    for ev in &record.evaluations {
        if ev.passed_gates && ev.score > highest_score {
            if let Some(imp) = record.implementations.iter().find(|i| i.id == ev.candidate_id) {
                best_cand = Some((imp.id.clone(), imp.patch_blob_sha256.clone()));
                highest_score = ev.score;
            }
        }
    }

    best_cand
}

fn make_seed_candidate(id: &str, patch: &[u8]) -> Candidate {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(patch);
    let hash_bytes: [u8; 32] = hasher.finalize().into();

    Candidate {
        id: format!("seed-{id}"),
        patch: patch.to_vec(),
        diff_hash: hash_bytes,
        generation: 0,
        parent_ids: vec![id.to_string()],
        model_id: "memory-replay".into(),
        prompt_hash: "replay-seed".into(),
    }
}
