//! Git artifact emission (e1-11 §1–3) built on `git2`.
//!
//! The isolation invariant is the whole point: the branch and its commit are written straight
//! into the object store and the branch ref is created with `branch()`, which never touches
//! HEAD, the index, or the working tree. A dirty checkout therefore cannot block or corrupt
//! emission, and the user's active branch is provably unchanged (AC4). No remote push occurs;
//! the artifact set is local-only by design.

use std::path::{Path, PathBuf};

use git2::{DiffOptions, ObjectType, Repository, Signature};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    #[error("base commit not found: {0}")]
    BaseCommitMissing(String),
    #[error("emitted patch failed integrity check: {0}")]
    PatchUnappliable(String),
}

/// The local artifacts produced for a job: the branch name, the patch path, and the commit oid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmittedBranch {
    pub branch_name: String,
    pub commit_oid: String,
    pub patch_path: PathBuf,
}

/// Branch name for a job: `evoswarm/<job-id>`.
pub fn branch_name(job_id: &str) -> String {
    format!("evoswarm/{job_id}")
}

/// Emits a winning candidate whose change is a raw git patch (the `Candidate.patch` bytes) as a
/// single atomic commit on `evoswarm/<job-id>`, branched from `base_commit`, without checking
/// anything out. The patch is applied to the base commit's tree in the object store
/// (`apply_to_tree`), so a dirty working tree cannot affect the result.
pub fn emit_branch_from_patch(
    repo: &Repository,
    job_id: &str,
    base_commit: &str,
    patch: &[u8],
    message: &str,
) -> Result<EmittedBranch, GitError> {
    let base_oid = git2::Oid::from_str(base_commit)
        .map_err(|_| GitError::BaseCommitMissing(base_commit.to_string()))?;
    let base = repo
        .find_object(base_oid, Some(ObjectType::Commit))
        .map_err(|_| GitError::BaseCommitMissing(base_commit.to_string()))?
        .peel_to_commit()?;
    let base_tree = base.tree()?;

    let diff = git2::Diff::from_buffer(patch)
        .map_err(|e| GitError::PatchUnappliable(format!("cannot parse patch: {e}")))?;
    let mut index = repo
        .apply_to_tree(&base_tree, &diff, None)
        .map_err(|e| GitError::PatchUnappliable(format!("patch does not apply to base tree: {e}")))?;
    let tree_oid = index.write_tree_to(repo)?;
    let new_tree = repo.find_tree(tree_oid)?;

    let sig = Signature::now("EvoSwarm", "evoswarm@localhost")?;
    let commit_oid = repo.commit(None, &sig, &sig, message, &new_tree, &[&base])?;
    let commit = repo.find_commit(commit_oid)?;
    let name = branch_name(job_id);
    repo.branch(&name, &commit, false)?;

    Ok(EmittedBranch {
        branch_name: name,
        commit_oid: commit_oid.to_string(),
        patch_path: PathBuf::new(),
    })
}

/// Produces the `base -> branch` diff in git-patch format and writes it to `patch_path`. The
/// patch is generated from the object store (never the working tree), so it reflects exactly the
/// committed change.
pub fn write_patch(
    repo: &Repository,
    base_commit: &str,
    branch: &EmittedBranch,
    patch_path: &Path,
) -> Result<(), GitError> {
    let base_oid = git2::Oid::from_str(base_commit)
        .map_err(|_| GitError::BaseCommitMissing(base_commit.to_string()))?;
    let base = repo.find_commit(base_oid)?;
    let head = repo.find_commit(
        git2::Oid::from_str(&branch.commit_oid)
            .map_err(|_| GitError::BaseCommitMissing(branch.commit_oid.clone()))?,
    )?;

    let base_tree = base.tree()?;
    let head_tree = head.tree()?;
    let mut opts = DiffOptions::new();
    let diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut opts))?;

    if let Some(parent) = patch_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| GitError::PatchUnappliable(format!("cannot create patch dir: {e}")))?;
    }
    let mut buf = Vec::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        render_patch_line(&mut buf, &line);
        true
    })?;
    std::fs::write(patch_path, &buf)
        .map_err(|e| GitError::PatchUnappliable(format!("cannot write patch: {e}")))?;
    Ok(())
}

/// Verifies the emitted patch applies cleanly to `base_commit` (spec §3). The check runs entirely
/// in the object store via `apply_to_tree`, so it never touches the index or working tree — a
/// dirty checkout cannot make it fail spuriously (isolation invariant / AC4). A patch that does
/// not apply to the base tree is a job failure, not a warning.
pub fn verify_patch_applies(
    repo: &Repository,
    base_commit: &str,
    patch_path: &Path,
) -> Result<(), GitError> {
    let patch = std::fs::read(patch_path)
        .map_err(|e| GitError::PatchUnappliable(format!("cannot read patch: {e}")))?;
    let diff = git2::Diff::from_buffer(&patch)
        .map_err(|e| GitError::PatchUnappliable(format!("cannot parse patch: {e}")))?;

    let base_oid = git2::Oid::from_str(base_commit)
        .map_err(|_| GitError::BaseCommitMissing(base_commit.to_string()))?;
    let base = repo
        .find_object(base_oid, Some(ObjectType::Commit))
        .map_err(|_| GitError::BaseCommitMissing(base_commit.to_string()))?
        .peel_to_commit()?;
    let base_tree = base.tree()?;

    // Success = the patch produces a valid new tree from the base tree. Any error means it would
    // not apply cleanly to the base commit.
    repo.apply_to_tree(&base_tree, &diff, None)
        .map_err(|e| GitError::PatchUnappliable(format!("patch does not apply to base: {e}")))?;
    Ok(())
}

/// Returns the currently checked-out branch name (or `None` if HEAD is detached), used by tests
/// to assert the active branch is unchanged after emission.
pub fn current_branch(repo: &Repository) -> Option<String> {
    let head = repo.head().ok()?;
    head.shorthand().map(|s| s.to_string())
}

/// Serialises one diff line into the patch buffer. libgit2's `content()` returns the line body
/// *without* its origin marker for context/addition/deletion lines, so writing it verbatim drops
/// the leading ` ` / `+` / `-` and yields a patch `git apply` cannot parse ("invalid patch hunk").
/// Prepending `origin()` restores the marker; header and EOFNL lines already carry their full
/// text and must not be prefixed.
pub fn render_patch_line(buf: &mut Vec<u8>, line: &git2::DiffLine<'_>) {
    use git2::DiffLineType;
    match line.origin_value() {
        DiffLineType::Context | DiffLineType::Addition | DiffLineType::Deletion => {
            buf.push(line.origin() as u8);
            buf.extend_from_slice(line.content());
        }
        _ => buf.extend_from_slice(line.content()),
    }
}
