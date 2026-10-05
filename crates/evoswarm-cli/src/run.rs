//! `evoswarm run` submission orchestration (e1-1).
//!
//! The 2 s ticket deadline is met by persisting and returning the queued ticket *before*
//! baseline validation, not by making validation fast: validation failure is reported as a
//! job state transition and a rejection outcome, never as a lost ticket.

use std::path::PathBuf;
use std::time::Instant;

use evoswarm_core::{
    require_within_root, JobStatus, JobSubmission, JobTicket, PathEscape, SandboxProfile,
};
use evoswarm_ledger::{JobLedger, RepoRoot};
use evoswarm_sandbox::SandboxBackend;

use crate::baseline::validate_baseline;
use crate::exit_codes::ExitCode;

/// Everything `submit` reports back: the fast ticket, the process exit code, and (on
/// rejection) the diagnostic shown on stderr. A successful queue returns `code == Ok`.
#[derive(Debug, Clone, PartialEq)]
pub struct SubmitOutcome {
    pub ticket: JobTicket,
    pub code: ExitCode,
    pub message: Option<String>,
    /// Milliseconds from submission start to the ticket being produced.
    pub ticket_elapsed_ms: u128,
}

impl SubmitOutcome {
    pub fn accepted(&self) -> bool {
        self.code == ExitCode::Ok
    }
}

/// Failures that prevent even a ticket from being issued (as opposed to baseline
/// rejections, which still yield a queued ticket before transitioning to failed).
#[derive(Debug, thiserror::Error)]
pub enum SubmitError {
    #[error("{0}")]
    PathEscape(PathEscape),
    #[error("ledger error: {0}")]
    Ledger(#[from] evoswarm_ledger::LedgerError),
    #[error("invalid submission: {0}")]
    Validation(String),
}

impl SubmitError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            SubmitError::PathEscape(_) => ExitCode::PathEscape,
            SubmitError::Ledger(_) => ExitCode::Validation,
            SubmitError::Validation(_) => ExitCode::Validation,
        }
    }
}

/// Validates the submission, records the job as `Queued`, and produces the fast ticket.
/// Baseline validation runs *after* the ticket is produced; a rejection transitions the
/// persisted job to `Failed` and is returned in `SubmitOutcome`, so the ticket is never lost.
///
/// Generic over `SandboxBackend` so tests inject a scripted fake instead of bwrap.
pub async fn submit<B>(
    submission: JobSubmission,
    repo: &RepoRoot,
    ledger: &JobLedger,
    backend: &B,
) -> Result<SubmitOutcome, SubmitError>
where
    B: SandboxBackend + ?Sized,
{
    let started = Instant::now();

    // 1. Flag validation before any work.
    validate_flags(&submission)?;

    // 2. Repository boundary check: every --paths entry must sit within the repo root.
    for p in &submission.target_paths {
        require_within_root(repo.as_path(), p).map_err(SubmitError::PathEscape)?;
    }

    // 3. Record the base commit so e1-11 can emit a patch without depending on HEAD later.
    let base_commit = resolve_base_commit(repo.as_path());

    // 4. Assign the id and persist in Queued state — this is the durable ticket.
    let job_id = uuid::Uuid::new_v4().to_string();
    ledger.insert_job(&job_id, &submission, base_commit.as_deref())?;
    let ticket = JobTicket {
        job_id: job_id.clone(),
        status: JobStatus::Queued,
    };
    let ticket_elapsed_ms = started.elapsed().as_millis();

    // 5. Baseline validation runs after the ticket exists. A rejection is a state
    //    transition, not a lost ticket.
    let profile = profile_for(&submission);
    match validate_baseline(
        backend,
        &profile,
        &submission.test_command,
        submission.objective,
    )
    .await
    {
        Ok(()) => {
            // A trustworthy, improvable baseline leaves the job Queued: the search engine
            // transitions it to Running when it actually starts, not at submission.
            Ok(SubmitOutcome {
                ticket,
                code: ExitCode::Ok,
                message: None,
                ticket_elapsed_ms,
            })
        }
        Err(rejection) => {
            ledger.transition(&job_id, JobStatus::Failed)?;
            Ok(SubmitOutcome {
                ticket: JobTicket {
                    job_id,
                    status: JobStatus::Failed,
                },
                code: rejection.code,
                message: Some(rejection.message),
                ticket_elapsed_ms,
            })
        }
    }
}

fn validate_flags(sub: &JobSubmission) -> Result<(), SubmitError> {
    if sub.task_description.trim().is_empty() {
        return Err(SubmitError::Validation("--task must not be empty".into()));
    }
    if sub.test_command.trim().is_empty() {
        return Err(SubmitError::Validation("--cmd must not be empty".into()));
    }
    if sub.target_paths.is_empty() {
        return Err(SubmitError::Validation("--paths must list at least one path".into()));
    }
    if sub.target_paths.iter().any(|p| p.as_os_str().is_empty()) {
        return Err(SubmitError::Validation("--paths entry must not be empty".into()));
    }
    Ok(())
}

/// Builds the sandbox profile for baseline validation from the submission's timeout. The
/// memory/pids caps are carried but not enforced in this environment (see roadmap §3:
/// cgroup writes are denied), so they are set to advisory values only.
fn profile_for(sub: &JobSubmission) -> SandboxProfile {
    SandboxProfile {
        stack: "python".to_string(),
        wall_timeout_secs: sub.timeout_secs,
        memory_limit_bytes: 512 * 1024 * 1024,
        tmpfs_size_bytes: 64 * 1024 * 1024,
        tasks_max: 64,
        read_only_mounts: Vec::new(),
        dependency_cache_path: PathBuf::new(),
    }
}

/// Resolves the current HEAD commit of `repo`, if it is a git repository. Returns `None`
/// for a non-git directory so submission still succeeds (the patch step degrades later).
fn resolve_base_commit(repo: &std::path::Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evoswarm_core::JobObjective;

    fn sub(paths: Vec<PathBuf>) -> JobSubmission {
        JobSubmission {
            task_description: "make it faster".into(),
            test_command: "pytest -q".into(),
            target_paths: paths,
            budget_tokens: None,
            budget_dollars: None,
            objective: JobObjective::Correctness,
            timeout_secs: 30,
        }
    }

    #[test]
    fn empty_task_is_validation_error() {
        let mut s = sub(vec![PathBuf::from("src")]);
        s.task_description = "  ".into();
        let err = validate_flags(&s).unwrap_err();
        assert_eq!(err.exit_code(), ExitCode::Validation);
    }

    #[test]
    fn empty_paths_is_validation_error() {
        let s = sub(vec![]);
        let err = validate_flags(&s).unwrap_err();
        assert_eq!(err.exit_code(), ExitCode::Validation);
    }

    #[test]
    fn profile_uses_submission_timeout() {
        let s = sub(vec![PathBuf::from("src")]);
        let p = profile_for(&s);
        assert_eq!(p.wall_timeout_secs, 30);
        assert_eq!(p.stack, "python");
    }
}
