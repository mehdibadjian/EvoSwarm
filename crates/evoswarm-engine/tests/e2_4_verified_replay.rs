//! e2-4: Verified replay acceptance tests (AD-4).
//!
//! Acceptance criteria (story §4):
//! - AC1: Given spec hash and both fingerprints match a stored winner, when the job starts,
//!   then the winner runs against the full suite including held-out tests.
//! - AC2: Given it passes, when returned, then the job is marked `replayed` and made 0 model calls.
//! - AC3: Given it fails, when checked, then a normal job starts with it as a seed and the
//!   mismatch is logged.

use async_trait::async_trait;
use evoswarm_core::{
    ExecutionResult, JobObjective, JobSubmission, SandboxProfile,
};
use evoswarm_engine::{
    check_and_execute_replay, CompletionRequest, CompletionResponse, ModelClient, ReplayDeps,
    ReplayOutcome,
};
use evoswarm_ledger::JobLedger;
use evoswarm_memory::blob_store::BlobStore;
use evoswarm_memory::falkordb::{
    EvaluationRecord, ImplementationRecord, JobRecord, LineageWriter, MockFalkorClient, TaskRecord,
    TestCaseRecord,
};
use evoswarm_sandbox::{SandboxBackend, SandboxError};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::tempdir;

struct TestModelClient {
    calls: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelClient for TestModelClient {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(CompletionResponse {
            text: "patch".into(),
            tokens_in: 10,
            tokens_out: 20,
            from_cache: false,
        })
    }
}

struct TestSandbox {
    succeeds: bool,
    runs: Arc<AtomicUsize>,
}

#[async_trait]
impl SandboxBackend for TestSandbox {
    async fn prepare(
        &self,
        _profile: &SandboxProfile,
        _patch: &[u8],
    ) -> Result<PathBuf, SandboxError> {
        Ok(PathBuf::from("/tmp"))
    }

    async fn run(&self, _workdir: &Path, _cmd: &str) -> Result<ExecutionResult, SandboxError> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        let exit_code = if self.succeeds { 0 } else { 1 };
        Ok(ExecutionResult {
            exit_code,
            stdout: String::new(),
            stderr: String::new(),
            wall_time_ms: 10,
            peak_memory_bytes: 1024,
            status: if self.succeeds {
                evoswarm_core::RunStatus::Success
            } else {
                evoswarm_core::RunStatus::Failed
            },
        })
    }

    async fn collect(&self, _workdir: PathBuf) -> Result<(), SandboxError> {
        Ok(())
    }
}

fn sample_submission() -> JobSubmission {
    JobSubmission {
        task_description: "sort array".into(),
        test_command: "pytest".into(),
        target_paths: vec![PathBuf::from("src/lib.py")],
        budget_tokens: None,
        budget_dollars: None,
        objective: JobObjective::Correctness,
        timeout_secs: 30,
    }
}

#[tokio::test]
async fn test_instant_replay_success() {
    let dir = tempdir().expect("tempdir");
    let spool_dir = dir.path().join("spool");
    let blob_dir = dir.path().join("blobs");
    let _ledger = JobLedger::open_in_memory().unwrap();
    let blob_store = BlobStore::new(blob_dir).unwrap();

    let patch_bytes = b"diff --git a/src/lib.py b/src/lib.py\n+ def sort(): pass";
    let patch_blob_hash = blob_store.write(patch_bytes).unwrap();

    let client = MockFalkorClient::new_failing();
    let writer = LineageWriter::new(client.clone(), &spool_dir);

    let winner_job = JobRecord {
        task: TaskRecord {
            id: "prev-job-1".into(),
            spec_hash: "hash_spec_1".into(),
            repo_fingerprint: "repo_fp_1".into(),
            toolchain_fingerprint: "tool_fp_1".into(),
        },
        implementations: vec![ImplementationRecord {
            id: "cand-winner".into(),
            generation: 2,
            patch_blob_sha256: patch_blob_hash.clone(),
            model_role: "mutator".into(),
            parents: vec![],
        }],
        evaluations: vec![EvaluationRecord {
            id: "eval-winner".into(),
            candidate_id: "cand-winner".into(),
            score: 1.0,
            passed_gates: true,
            wall_time_ms: 50,
            tests: vec![
                TestCaseRecord {
                    name: "test_visible".into(),
                    origin: "visible".into(),
                    passed: true,
                },
                TestCaseRecord {
                    name: "test_held_out".into(),
                    origin: "held_out".into(),
                    passed: true,
                },
            ],
        }],
    };
    writer.record_job(&winner_job).await.unwrap();

    let model_calls = Arc::new(AtomicUsize::new(0));
    let model_client = TestModelClient {
        calls: Arc::clone(&model_calls),
    };
    let sandbox_runs = Arc::new(AtomicUsize::new(0));
    let sandbox = TestSandbox {
        succeeds: true,
        runs: Arc::clone(&sandbox_runs),
    };

    let deps = ReplayDeps {
        spool_dir: &spool_dir,
        blob_store: &blob_store,
        backend: &sandbox,
        model_client: &model_client,
        spec_hash: "hash_spec_1",
        repo_fingerprint: "repo_fp_1",
        toolchain_fingerprint: "tool_fp_1",
    };

    let start_time = std::time::Instant::now();
    let outcome = check_and_execute_replay(&sample_submission(), &deps).await;
    let elapsed = start_time.elapsed();

    match outcome {
        ReplayOutcome::Replayed {
            patch,
            candidate_id,
        } => {
            assert_eq!(patch, patch_bytes);
            assert_eq!(candidate_id, "cand-winner");
        }
        _ => panic!("expected ReplayOutcome::Replayed"),
    }

    assert_eq!(model_calls.load(Ordering::SeqCst), 0, "0 model calls on replay");
    assert_eq!(sandbox_runs.load(Ordering::SeqCst), 1, "1 sandbox verification run");
    assert!(elapsed.as_secs() < 5, "finishes in seconds");
}

#[tokio::test]
async fn test_replay_verification_failure_fallback() {
    let dir = tempdir().expect("tempdir");
    let spool_dir = dir.path().join("spool");
    let blob_dir = dir.path().join("blobs");
    let blob_store = BlobStore::new(blob_dir).unwrap();

    let patch_bytes = b"diff --git a/src/lib.py b/src/lib.py\n+ broken code";
    let patch_blob_hash = blob_store.write(patch_bytes).unwrap();

    let client = MockFalkorClient::new_failing();
    let writer = LineageWriter::new(client.clone(), &spool_dir);

    let winner_job = JobRecord {
        task: TaskRecord {
            id: "prev-job-2".into(),
            spec_hash: "hash_spec_2".into(),
            repo_fingerprint: "repo_fp_2".into(),
            toolchain_fingerprint: "tool_fp_2".into(),
        },
        implementations: vec![ImplementationRecord {
            id: "cand-broken".into(),
            generation: 1,
            patch_blob_sha256: patch_blob_hash.clone(),
            model_role: "mutator".into(),
            parents: vec![],
        }],
        evaluations: vec![EvaluationRecord {
            id: "eval-broken".into(),
            candidate_id: "cand-broken".into(),
            score: 0.9,
            passed_gates: true,
            wall_time_ms: 50,
            tests: vec![],
        }],
    };
    writer.record_job(&winner_job).await.unwrap();

    let model_calls = Arc::new(AtomicUsize::new(0));
    let model_client = TestModelClient {
        calls: Arc::clone(&model_calls),
    };
    let sandbox_runs = Arc::new(AtomicUsize::new(0));
    let sandbox = TestSandbox {
        succeeds: false, // verification fails in sandbox
        runs: Arc::clone(&sandbox_runs),
    };

    let deps = ReplayDeps {
        spool_dir: &spool_dir,
        blob_store: &blob_store,
        backend: &sandbox,
        model_client: &model_client,
        spec_hash: "hash_spec_2",
        repo_fingerprint: "repo_fp_2",
        toolchain_fingerprint: "tool_fp_2",
    };

    let outcome = check_and_execute_replay(&sample_submission(), &deps).await;

    match outcome {
        ReplayOutcome::FallbackToSearch { seed_candidate } => {
            assert!(seed_candidate.is_some(), "fallback carries winner as seed candidate");
            assert_eq!(seed_candidate.unwrap().patch, patch_bytes);
        }
        _ => panic!("expected ReplayOutcome::FallbackToSearch"),
    }
}
