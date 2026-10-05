//! e1-1: Start a job from the CLI.
//!
//! Drives `submit` against a scripted `SandboxBackend` fake (never bwrap) so the
//! acceptance criteria are deterministic: fast queued ticket, path-escape rejection,
//! broken-baseline rejection, flaky-suite rejection, and zero-diff rejection.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use evoswarm_core::{
    ExecutionResult, JobObjective, JobStatus, JobSubmission, RunStatus, SandboxProfile,
};
use evoswarm_ledger::{JobLedger, RepoRoot};
use evoswarm_sandbox::{SandboxBackend, SandboxError};

use evoswarm_cli::{run::submit, ExitCode};

/// A sandbox fake whose `run` returns a programmed sequence of results, one per call.
/// When the sequence is exhausted it repeats the last entry, so a "stable" suite keeps
/// yielding the same outcome across the three baseline runs.
struct ScriptedBackend {
    runs: Mutex<Vec<ExecutionResult>>,
    call_index: Mutex<usize>,
}

impl ScriptedBackend {
    fn new(runs: Vec<ExecutionResult>) -> Self {
        Self {
            runs: Mutex::new(runs),
            call_index: Mutex::new(0),
        }
    }
}

#[async_trait]
impl SandboxBackend for ScriptedBackend {
    async fn prepare(
        &self,
        _profile: &SandboxProfile,
        _patch: &[u8],
    ) -> Result<PathBuf, SandboxError> {
        Ok(PathBuf::from("/fake/workdir"))
    }

    async fn run(&self, _workdir: &Path, _cmd: &str) -> Result<ExecutionResult, SandboxError> {
        let mut idx = self.call_index.lock().unwrap();
        let runs = self.runs.lock().unwrap();
        let i = (*idx).min(runs.len() - 1);
        *idx += 1;
        Ok(runs[i].clone())
    }

    async fn collect(&self, _workdir: PathBuf) -> Result<(), SandboxError> {
        Ok(())
    }
}

/// JUnit XML for a suite of `n` tests all passing.
fn all_pass_xml(n: usize) -> String {
    let cases: String = (0..n)
        .map(|i| format!(r#"<testcase classname="t" name="test_{i}" time="0.001"/>"#))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"<testsuites><testsuite name="pytest" tests="{n}">{cases}</testsuite></testsuites>"#
    )
}

/// JUnit XML where `fail_idx` fails and the rest pass.
fn one_fail_xml(n: usize, fail_idx: usize) -> String {
    let cases: String = (0..n)
        .map(|i| {
            if i == fail_idx {
                format!(r#"<testcase classname="t" name="test_{i}" time="0.001"><failure message="boom">AssertionError</failure></testcase>"#)
            } else {
                format!(r#"<testcase classname="t" name="test_{i}" time="0.001"/>"#)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"<testsuites><testsuite name="pytest" tests="{n}">{cases}</testsuite></testsuites>"#
    )
}

fn ok_run(xml: String) -> ExecutionResult {
    ExecutionResult {
        exit_code: 0,
        stdout: xml,
        stderr: String::new(),
        wall_time_ms: 1,
        peak_memory_bytes: 0,
        status: RunStatus::Success,
    }
}

fn failed_run(stderr: &str) -> ExecutionResult {
    ExecutionResult {
        exit_code: 127,
        stdout: String::new(),
        stderr: stderr.to_string(),
        wall_time_ms: 1,
        peak_memory_bytes: 0,
        status: RunStatus::Failed,
    }
}

fn submission(paths: Vec<PathBuf>, objective: JobObjective) -> JobSubmission {
    JobSubmission {
        task_description: "reduce runtime of the hot loop".into(),
        test_command: "pytest -q --junitxml=/dev/stdout".into(),
        target_paths: paths,
        budget_tokens: Some(100_000),
        budget_dollars: Some(5.0),
        objective,
        timeout_secs: 30,
    }
}

fn temp_repo() -> (tempfile::TempDir, RepoRoot) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    let repo = RepoRoot::new(dir.path().to_path_buf());
    (dir, repo)
}

/// AC1: a valid submission prints a queued ticket in under 2 s and the SQLite state is
/// `queued` (not Running — the engine moves it later).
#[tokio::test]
async fn test_submit_valid_job_fast_return() {
    let (_dir, repo) = temp_repo();
    let ledger = JobLedger::open_in_memory().expect("ledger");
    // A baseline with one failing test: trustworthy (stable) AND improvable under a
    // correctness objective, so the job is accepted and stays queued.
    let backend = ScriptedBackend::new(vec![ok_run(one_fail_xml(3, 1))]);

    let outcome = submit(
        submission(vec![PathBuf::from("src")], JobObjective::Correctness),
        &repo,
        &ledger,
        &backend,
    )
    .await
    .expect("submit ok");

    assert_eq!(outcome.code, ExitCode::Ok, "valid job must be accepted");
    assert!(outcome.accepted());
    assert_eq!(outcome.ticket.status, JobStatus::Queued);
    assert!(
        outcome.ticket_elapsed_ms < 2000,
        "ticket must be produced in under 2 s, took {} ms",
        outcome.ticket_elapsed_ms
    );
    // The durable SQLite state is queued.
    assert_eq!(
        ledger.read_status(&outcome.ticket.job_id).expect("status"),
        JobStatus::Queued
    );
    // Ticket JSON shape matches the spec contract.
    let json = serde_json::to_value(&outcome.ticket).expect("json");
    assert_eq!(json["status"], "queued");
    assert!(!json["job_id"].as_str().unwrap().is_empty());
}

/// AC2: a `--paths` entry escaping the repo root is rejected with PathEscape and the
/// resolved offending path surfaced in the error.
#[tokio::test]
async fn test_reject_path_traversal_outside_repo() {
    let (_dir, repo) = temp_repo();
    let ledger = JobLedger::open_in_memory().expect("ledger");
    let backend = ScriptedBackend::new(vec![ok_run(one_fail_xml(3, 1))]);

    let err = submit(
        submission(vec![PathBuf::from("../outside")], JobObjective::Correctness),
        &repo,
        &ledger,
        &backend,
    )
    .await
    .expect_err("path escape must error");

    assert_eq!(err.exit_code(), ExitCode::PathEscape);
    let msg = err.to_string();
    assert!(
        msg.contains("escapes repository root"),
        "error should name the escape, got: {msg}"
    );
    // The offending candidate path is included so the user can see what was rejected.
    assert!(msg.contains("outside"), "error should show the path, got: {msg}");
}

/// AC3: a baseline command that cannot execute is rejected carrying the child's exit code.
#[tokio::test]
async fn test_reject_broken_baseline_command() {
    let (_dir, repo) = temp_repo();
    let ledger = JobLedger::open_in_memory().expect("ledger");
    // The child exits non-zero with no test report: a missing/broken binary.
    let backend = ScriptedBackend::new(vec![failed_run("sh: pytest: command not found")]);

    let outcome = submit(
        submission(vec![PathBuf::from("src")], JobObjective::Correctness),
        &repo,
        &ledger,
        &backend,
    )
    .await
    .expect("submit returns outcome, rejection is a transition");

    assert_eq!(outcome.code, ExitCode::BaselineCommandFailed);
    let msg = outcome.message.expect("rejection message");
    assert!(
        msg.contains("command not found"),
        "must echo child stderr, got: {msg}"
    );
    // The ticket was still produced before validation, then transitioned to Failed.
    assert_eq!(outcome.ticket.status, JobStatus::Failed);
    assert_eq!(
        ledger.read_status(&outcome.ticket.job_id).expect("status"),
        JobStatus::Failed
    );
}

/// AC4: baseline results that differ across the three runs are rejected and the varying
/// test names are listed.
#[tokio::test]
async fn test_reject_flaky_baseline_suite() {
    let (_dir, repo) = temp_repo();
    let ledger = JobLedger::open_in_memory().expect("ledger");
    // test_1 passes on runs 1 and 3 but fails on run 2 → flaky.
    let backend = ScriptedBackend::new(vec![
        ok_run(one_fail_xml(3, 1)), // run 1: test_1 fails
        ok_run(all_pass_xml(3)),    // run 2: test_1 passes
        ok_run(one_fail_xml(3, 1)), // run 3: test_1 fails
    ]);

    let outcome = submit(
        submission(vec![PathBuf::from("src")], JobObjective::Correctness),
        &repo,
        &ledger,
        &backend,
    )
    .await
    .expect("submit returns outcome");

    assert_eq!(outcome.code, ExitCode::FlakyTestDetected);
    let msg = outcome.message.expect("rejection message");
    assert!(
        msg.contains("test_1"),
        "must name the varying test, got: {msg}"
    );
    assert_eq!(outcome.ticket.status, JobStatus::Failed);
}

/// AC5: an already-passing suite without `--objective perf` is rejected with
/// NothingToImprove, and the message points at the flag.
#[tokio::test]
async fn test_reject_already_passing_suite_without_perf_flag() {
    let (_dir, repo) = temp_repo();
    let ledger = JobLedger::open_in_memory().expect("ledger");
    // Every baseline test passes, stable across runs.
    let backend = ScriptedBackend::new(vec![ok_run(all_pass_xml(3))]);

    let outcome = submit(
        submission(vec![PathBuf::from("src")], JobObjective::Correctness),
        &repo,
        &ledger,
        &backend,
    )
    .await
    .expect("submit returns outcome");

    assert_eq!(outcome.code, ExitCode::NothingToImprove);
    let msg = outcome.message.expect("rejection message");
    assert!(
        msg.contains("--objective perf"),
        "must suggest the perf flag, got: {msg}"
    );
    assert_eq!(outcome.ticket.status, JobStatus::Failed);
}

/// AC5 complement: the same all-pass suite WITH `--objective perf` is accepted, proving
/// the rejection is specifically about the missing perf objective.
#[tokio::test]
async fn test_all_passing_suite_with_perf_flag_is_accepted() {
    let (_dir, repo) = temp_repo();
    let ledger = JobLedger::open_in_memory().expect("ledger");
    let backend = ScriptedBackend::new(vec![ok_run(all_pass_xml(3))]);

    let outcome = submit(
        submission(vec![PathBuf::from("src")], JobObjective::Performance),
        &repo,
        &ledger,
        &backend,
    )
    .await
    .expect("submit ok");

    assert_eq!(outcome.code, ExitCode::Ok);
    assert_eq!(
        ledger.read_status(&outcome.ticket.job_id).expect("status"),
        JobStatus::Queued
    );
}
