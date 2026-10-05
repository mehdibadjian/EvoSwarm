//! Artifact emission (e1-11): ties the git writer and the report renderer into one call that
//! produces the local artifact set for a completed job — the `evoswarm/<job-id>` branch, the
//! `.evoswarm/patches/<job-id>.patch` file, and the `.evoswarm/reports/<job-id>.md` audit report.
//!
//! The base commit is read from the persisted `JobRecord` (recorded at submission, e1-1), never
//! from HEAD, so completion does not depend on the working tree state. Nothing is pushed to a
//! remote and the user's checkout is never switched (isolation invariant).

use std::path::PathBuf;

use evoswarm_core::{
    LineageNode, RoleUsage, ScoreBreakdown, SelectionOutcome, WinnerSummary,
};
use evoswarm_ledger::{JobRecord, RepoRoot};
use git2::Repository;

use crate::git_writer::{self, EmittedBranch};
use crate::report::{render_markdown, ReportData};

/// The report inputs produced upstream by the engine and needed to render a complete audit
/// report. Grouped so `emit` takes one owned bundle rather than six loose parameters.
#[derive(Debug, Clone)]
pub struct ReportInputs {
    pub winner: WinnerSummary,
    pub score: ScoreBreakdown,
    pub usage: Vec<RoleUsage>,
    pub lineage: Vec<LineageNode>,
}

#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    #[error("report generation failed: {0}")]
    Report(#[from] crate::report::ReportError),
    #[error("git artifact emission failed: {0}")]
    Git(#[from] git_writer::GitError),
    #[error("io error writing artifact: {0}")]
    Io(#[from] std::io::Error),
    #[error("job has no recorded base commit; cannot emit artifacts")]
    BaseCommitMissing,
}

/// The local artifacts emitted for a job.
#[derive(Debug, Clone)]
pub struct Artifacts {
    pub branch: EmittedBranch,
    pub patch_path: PathBuf,
    pub report_path: PathBuf,
    pub report_markdown: String,
}

/// Emits the full artifact set for a completed job. The winning candidate's patch is committed to
/// `evoswarm/<job-id>` from the recorded base commit, written out as a `.patch`, verified with
/// `git apply --check`, and accompanied by the markdown audit report. A `NoVerifiedWinner` still
/// emits the best-effort patch and report (spec §4); the report header states the outcome.
pub fn emit(
    job: &JobRecord,
    winner: &SelectionOutcome,
    repo: &RepoRoot,
    inputs: &ReportInputs,
) -> Result<Artifacts, ReportError> {
    let base_commit = job
        .base_commit
        .clone()
        .ok_or(ReportError::BaseCommitMissing)?;

    let git_repo = Repository::open(repo.as_path()).map_err(git_writer::GitError::Git)?;
    let candidate = winner.candidate();

    // Render the report first: if a mandatory section's data is absent the job fails loudly
    // before any git object is written, so a partial artifact set is never left behind.
    let data = ReportData {
        job_id: &job.job_id,
        winner: &inputs.winner,
        score: &inputs.score,
        usage: &inputs.usage,
        lineage: &inputs.lineage,
    };
    let report_markdown = render_markdown(&data)?;

    let message = format!("evoswarm({}): winning candidate {}", job.job_id, candidate.id);
    let mut branch = git_writer::emit_branch_from_patch(
        &git_repo,
        &job.job_id,
        &base_commit,
        &candidate.patch,
        &message,
    )?;

    // Artifact paths under the repo root, mirroring spec §1.
    let patch_path = repo
        .as_path()
        .join(".evoswarm")
        .join("patches")
        .join(format!("{}.patch", job.job_id));
    let report_path = repo
        .as_path()
        .join(".evoswarm")
        .join("reports")
        .join(format!("{}.md", job.job_id));

    git_writer::write_patch(&git_repo, &base_commit, &branch, &patch_path)?;
    // Integrity gate (spec §3): an unappliable patch is a job failure, not a warning.
    git_writer::verify_patch_applies(&git_repo, &base_commit, &patch_path)?;
    branch.patch_path = patch_path.clone();

    if let Some(parent) = report_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&report_path, &report_markdown)?;

    Ok(Artifacts {
        branch,
        patch_path,
        report_path,
        report_markdown,
    })
}
