//! e1-12: Resume after crash.
//!
//! Verifies the recovery protocol against the real SQLite ledger: resume at last committed
//! generation + 1, model calls replayed from cache with zero dispatches, sandbox runs
//! re-executed rather than cached, and a changed prompt invalidating the cache.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use evoswarm_core::{
    ExecutionResult, JobObjective, JobStatus, JobSubmission, RunStatus, SandboxProfile,
};
use evoswarm_engine::{
    dispatch_model, recover, sandbox_run, CompletionRequest, CompletionResponse, ModelClient,
};
use evoswarm_ledger::{GenerationStatus, JobLedger};
use evoswarm_models::Role;
use evoswarm_sandbox::{SandboxBackend, SandboxError};

/// A model client that counts real dispatches, so "replayed from cache" is provable by
/// observing that the call count does not rise.
struct CountingClient {
    calls: AtomicUsize,
}

#[async_trait]
impl ModelClient for CountingClient {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(CompletionResponse {
            text: "generated-patch".into(),
            tokens_in: 12,
            tokens_out: 34,
            from_cache: false,
        })
    }
}

/// A sandbox fake that counts `run` invocations and returns a canned success. Used to prove
/// a sandbox run is re-executed on resume rather than served from any cache.
struct CountingSandbox {
    runs: AtomicUsize,
}

#[async_trait]
impl SandboxBackend for CountingSandbox {
    async fn prepare(
        &self,
        _profile: &SandboxProfile,
        _patch: &[u8],
    ) -> Result<PathBuf, SandboxError> {
        Ok(PathBuf::from("/fake/workdir"))
    }

    async fn run(&self, _workdir: &Path, _cmd: &str) -> Result<ExecutionResult, SandboxError> {
        self.runs.fetch_add(1, Ordering::SeqCst);
        Ok(ExecutionResult {
            exit_code: 0,
            stdout: "<testsuites><testsuite name=\"pytest\"><testcase name=\"t\"/></testsuite></testsuites>".into(),
            stderr: String::new(),
            wall_time_ms: 1,
            peak_memory_bytes: 0,
            status: RunStatus::Success,
        })
    }

    async fn collect(&self, _workdir: PathBuf) -> Result<(), SandboxError> {
        Ok(())
    }
}

fn sub() -> JobSubmission {
    JobSubmission {
        task_description: "optimise the parser".into(),
        test_command: "pytest".into(),
        target_paths: vec![PathBuf::from("src")],
        budget_tokens: None,
        budget_dollars: None,
        objective: JobObjective::Correctness,
        timeout_secs: 30,
    }
}

fn req(prompt: &[u8]) -> CompletionRequest {
    CompletionRequest {
        role: Role::Mutator,
        model_id: "model-x".into(),
        prompt: prompt.to_vec(),
    }
}

/// AC1: killed mid-generation → the job resumes from the last completed generation. A crash
/// during Gen 2 means Gens 0–1 were fully committed (Gen 2 never committed), so resume
/// starts at Gen 2 and re-runs it — never inheriting the half-scored population.
#[test]
fn test_resume_from_last_generation() {
    let ledger = JobLedger::open_in_memory().expect("ledger");
    ledger.insert_job("job-1", &sub(), None).unwrap();
    ledger.transition("job-1", JobStatus::Running).unwrap();

    // Gens 0 and 1 completed and committed; Gen 2 was in flight when the process died, so
    // it has no committed row.
    ledger
        .commit_generation("job-1", 0, "[\"c0\"]", GenerationStatus::Completed)
        .unwrap();
    ledger
        .commit_generation("job-1", 1, "[\"c1\"]", GenerationStatus::Completed)
        .unwrap();

    let resumable = recover(&ledger).expect("recover");
    assert_eq!(resumable.len(), 1, "the Running job must be resumable");
    assert_eq!(resumable[0].job_id, "job-1");
    // Resume at last committed (1) + 1 = 2: the interrupted generation is re-run cleanly.
    assert_eq!(resumable[0].resume_at_generation, 2);

    // A job that committed nothing resumes from generation 0.
    ledger.insert_job("job-2", &sub(), None).unwrap();
    ledger.transition("job-2", JobStatus::Running).unwrap();
    let all = recover(&ledger).expect("recover");
    let job2 = all.iter().find(|j| j.job_id == "job-2").unwrap();
    assert_eq!(job2.resume_at_generation, 0);
}

/// AC2: a model call completed before the crash is replayed from its stored idempotency key
/// with zero further dispatches — no double-spend on restart.
#[tokio::test]
async fn test_model_call_idempotency_cache() {
    let ledger = JobLedger::open_in_memory().expect("ledger");
    let client = CountingClient {
        calls: AtomicUsize::new(0),
    };

    // First dispatch: a real call, stored in the cache.
    let first = dispatch_model(&ledger, &client, &req(b"static-prefix|dynamic-suffix"))
        .await
        .expect("dispatch");
    assert!(!first.from_cache);
    assert_eq!(client.calls.load(Ordering::SeqCst), 1);

    // "Crash" and restart: a fresh client with a zeroed counter, replaying the same prompt.
    let restarted = CountingClient {
        calls: AtomicUsize::new(0),
    };
    let replayed = dispatch_model(&ledger, &restarted, &req(b"static-prefix|dynamic-suffix"))
        .await
        .expect("replay");

    assert!(replayed.from_cache, "identical call must hit the cache");
    assert_eq!(replayed.text, "generated-patch");
    assert_eq!(
        restarted.calls.load(Ordering::SeqCst),
        0,
        "replay must spend zero dispatches"
    );
}

/// AC3: sandbox runs in flight at crash time are re-executed cleanly rather than read from
/// cache. Two "restarts" of the same logical run must each invoke the backend.
#[tokio::test]
async fn test_interrupted_sandbox_rerun() {
    let sandbox = CountingSandbox {
        runs: AtomicUsize::new(0),
    };
    let workdir = PathBuf::from("/fake/workdir");

    let r1 = sandbox_run(&sandbox, &workdir, "pytest").await.expect("run 1");
    assert_eq!(r1.status, RunStatus::Success);
    assert_eq!(sandbox.runs.load(Ordering::SeqCst), 1);

    // "Restart": the same run is re-executed, never served from a stored result.
    let r2 = sandbox_run(&sandbox, &workdir, "pytest").await.expect("run 2");
    assert_eq!(r2.status, RunStatus::Success);
    assert_eq!(
        sandbox.runs.load(Ordering::SeqCst),
        2,
        "a sandbox run must never be cached; each restart re-executes"
    );
}

/// AC4: after the prompt bytes change, the cache misses and a fresh dispatch occurs — the
/// idempotency key is content-addressed over the exact prompt.
#[tokio::test]
async fn test_changed_prompt_invalidates_cache() {
    let ledger = JobLedger::open_in_memory().expect("ledger");
    let client = CountingClient {
        calls: AtomicUsize::new(0),
    };

    let original = dispatch_model(&ledger, &client, &req(b"prefix|suffix"))
        .await
        .expect("original");
    assert!(!original.from_cache);
    assert_eq!(client.calls.load(Ordering::SeqCst), 1);

    // Alter one suffix byte: a different key, so a cache miss and a fresh dispatch.
    let changed = dispatch_model(&ledger, &client, &req(b"prefix|suffit"))
        .await
        .expect("changed");
    assert!(!changed.from_cache, "a changed prompt must miss the cache");
    assert_eq!(
        client.calls.load(Ordering::SeqCst),
        2,
        "the cache miss must trigger a real dispatch"
    );
}
