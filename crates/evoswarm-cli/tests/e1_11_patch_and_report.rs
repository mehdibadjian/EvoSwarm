//! e1-11: Patch and report.
//!
//! Drives `artifacts::emit` against a real on-disk git repository so every acceptance
//! criterion is checked with genuine git objects: the `evoswarm/<job-id>` branch is created
//! from the base commit, the `.patch` applies to a clean checkout, the markdown report has
//! every required section, and the user's working tree and active branch are untouched.

use std::path::PathBuf;

use evoswarm_core::{
    Candidate, JobObjective, JobStatus, JobSubmission, LineageNode, RoleUsage, ScoreBreakdown,
    SelectionOutcome, WinnerSummary,
};
use evoswarm_ledger::{JobRecord, RepoRoot};
use git2::{DiffFormat, DiffOptions, Repository, Signature};

use evoswarm_cli::artifacts::{emit, ReportInputs};

const BASE_FILE: &str = "hello.txt";
const BASE_CONTENT: &str = "line1\n";
const NEW_CONTENT: &str = "line1\nline2\n";

/// Creates a git repo with one commit on `main` writing `hello.txt`, returning the base oid.
fn init_repo_with_base(path: &std::path::Path) -> String {
    let repo = Repository::init(path).expect("init");
    let blob = repo.blob(BASE_CONTENT.as_bytes()).expect("blob");
    let mut builder = repo.treebuilder(None).expect("treebuilder");
    builder.insert(BASE_FILE, blob, 0o100_644).expect("insert");
    let tree = repo.find_tree(builder.write().expect("tree")).expect("find");
    let sig = Signature::now("Tester", "tester@example.com").expect("sig");
    let oid = repo
        .commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .expect("commit");
    oid.to_string()
}

/// Generates a git-patch that changes `hello.txt` from BASE_CONTENT to NEW_CONTENT, produced
/// entirely in the object store (base tree -> modified tree) so it is a valid `git apply` patch.
fn make_patch(repo: &Repository, base_commit: &str) -> Vec<u8> {
    let base = repo.find_commit(git2::Oid::from_str(base_commit).unwrap()).unwrap();
    let base_tree = base.tree().unwrap();
    let blob = repo.blob(NEW_CONTENT.as_bytes()).unwrap();
    let mut builder = repo.treebuilder(Some(&base_tree)).unwrap();
    builder.insert(BASE_FILE, blob, 0o100_644).unwrap();
    let new_tree = repo.find_tree(builder.write().unwrap()).unwrap();
    let mut opts = DiffOptions::new();
    let diff = repo
        .diff_tree_to_tree(Some(&base_tree), Some(&new_tree), Some(&mut opts))
        .unwrap();
    let mut buf = Vec::new();
    diff.print(DiffFormat::Patch, |_d, _h, line| {
        evoswarm_cli::git_writer::render_patch_line(&mut buf, &line);
        true
    })
    .unwrap();
    buf
}

fn candidate(patch: Vec<u8>) -> Candidate {
    Candidate {
        id: "cand-1".into(),
        patch,
        diff_hash: [0u8; 32],
        generation: 1,
        parent_ids: vec!["cand-0".into()],
        model_id: "model-x".into(),
        prompt_hash: "ph".into(),
    }
}

fn job_record(job_id: &str, base_commit: &str) -> JobRecord {
    JobRecord {
        job_id: job_id.into(),
        status: JobStatus::Completed,
        submission: JobSubmission {
            task_description: "add line2".into(),
            test_command: "pytest".into(),
            target_paths: vec![PathBuf::from("hello.txt")],
            budget_tokens: None,
            budget_dollars: None,
            objective: JobObjective::Correctness,
            timeout_secs: 30,
        },
        base_commit: Some(base_commit.into()),
        split_json: None,
    }
}

fn inputs(verified: bool) -> ReportInputs {
    ReportInputs {
        winner: WinnerSummary {
            candidate_id: "cand-1".into(),
            verified,
            visible_passed: vec!["test_visible_a".into()],
            held_out_passed: vec!["test_secret_b".into()],
            proposed_tests: vec!["test_proposed_edge".into()],
        },
        score: ScoreBreakdown {
            score: 0.82,
            adversary: 0.9,
            runtime: 0.7,
            parsimony: 0.85,
            w_adversary: 0.4,
            w_runtime: 0.4,
            w_parsimony: 0.2,
        },
        usage: vec![RoleUsage {
            role: "mutator".into(),
            model_id: "model-x".into(),
            calls: 4,
            tokens_in: 1200,
            tokens_out: 600,
            cost_usd: 0.4321,
        }],
        lineage: vec![LineageNode {
            candidate_id: "cand-1".into(),
            generation: 1,
            parent_ids: vec!["cand-0".into()],
            model_id: "model-x".into(),
        }],
    }
}

fn temp_repo_dir() -> (tempfile::TempDir, RepoRoot) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = RepoRoot::new(dir.path().to_path_buf());
    (dir, root)
}

/// AC1: branch `evoswarm/<job-id>` is created from the base commit and the `.patch` file is
/// emitted; the active branch (main) is unchanged.
#[test]
fn test_git_branch_emission() {
    let (dir, repo) = temp_repo_dir();
    let base = init_repo_with_base(dir.path());
    let g = Repository::open(dir.path()).expect("open");
    let active_before = evoswarm_cli::git_writer::current_branch(&g);
    let patch = make_patch(&g, &base);

    let artifacts = emit(
        &job_record("job-42", &base),
        &SelectionOutcome::Verified { candidate: candidate(patch) },
        &repo,
        &inputs(true),
    )
    .expect("emit");

    // The branch exists in the object store and points at a commit whose parent is the base.
    let emitted = g
        .find_branch("evoswarm/job-42", git2::BranchType::Local)
        .expect("branch must exist");
    let head = emitted.get().peel_to_commit().expect("commit");
    assert_eq!(
        head.parent_id(0).ok().map(|o| o.to_string()),
        Some(base.clone()),
        "emitted commit must be branched from the base commit"
    );
    assert_eq!(artifacts.branch.branch_name, "evoswarm/job-42");
    // The patch file was emitted at the spec'd path and is non-empty.
    assert!(artifacts.patch_path.exists(), "patch file must exist");
    assert!(
        artifacts.patch_path.ends_with(".evoswarm/patches/job-42.patch"),
        "patch path per spec §1, got {:?}",
        artifacts.patch_path
    );
    assert!(!std::fs::read(&artifacts.patch_path).unwrap().is_empty());
    // The user's active branch is unchanged (never switched to the emitted branch).
    assert_eq!(
        evoswarm_cli::git_writer::current_branch(&g),
        active_before,
        "active branch must be untouched"
    );
}

/// AC2: the emitted `.patch` applies without conflict to a clean checkout of the base commit.
#[test]
fn test_patch_file_integrity() {
    let (dir, repo) = temp_repo_dir();
    let base = init_repo_with_base(dir.path());
    let g = Repository::open(dir.path()).expect("open");
    let patch = make_patch(&g, &base);

    let artifacts = emit(
        &job_record("job-43", &base),
        &SelectionOutcome::Verified { candidate: candidate(patch) },
        &repo,
        &inputs(true),
    )
    .expect("emit");

    // Clone the repo to a clean checkout at the base commit, then `git apply --check` the patch.
    let clean = tempfile::tempdir().expect("clean dir");
    let status = std::process::Command::new("git")
        .arg("clone")
        .arg(dir.path())
        .arg(clean.path())
        .status()
        .expect("git clone");
    assert!(status.success(), "clone must succeed");
    let co = Repository::open(clean.path()).expect("open clone");
    co.set_head_detached(git2::Oid::from_str(&base).unwrap())
        .expect("detach at base");
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(clean.path())
        .args(["apply", "--check", "--verbose"])
        .arg(&artifacts.patch_path)
        .output()
        .expect("git apply");
    assert!(
        out.status.success(),
        "patch must apply cleanly to a base checkout; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// AC3: the markdown report displays score breakdown, held-out passes, model calls, cost,
/// lineage and proposed tests — all present and non-empty.
#[test]
fn test_markdown_report_sections() {
    let (dir, repo) = temp_repo_dir();
    let base = init_repo_with_base(dir.path());
    let g = Repository::open(dir.path()).expect("open");
    let patch = make_patch(&g, &base);

    let artifacts = emit(
        &job_record("job-44", &base),
        &SelectionOutcome::Verified { candidate: candidate(patch) },
        &repo,
        &inputs(true),
    )
    .expect("emit");

    let md = &artifacts.report_markdown;
    for section in [
        "## Score breakdown",
        "## Test results",
        "## Model usage & cost",
        "## Lineage",
        "## Proposed adversary tests",
    ] {
        assert!(md.contains(section), "missing section: {section}");
    }
    // Held-out passes, cost figure, model calls, lineage and proposed test all appear.
    assert!(md.contains("test_secret_b"), "held-out pass must be listed");
    assert!(md.contains("0.4321"), "per-role cost must appear");
    assert!(md.contains("| 4 |"), "model call count must appear");
    assert!(md.contains("cand-1"), "lineage node must appear");
    assert!(md.contains("test_proposed_edge"), "proposed test must appear");
    assert!(md.contains("verified winner"), "verified outcome must be stated");
    // The report file is also written to disk at the spec'd path.
    assert!(
        artifacts.report_path.ends_with(".evoswarm/reports/job-44.md"),
        "report path per spec §1, got {:?}",
        artifacts.report_path
    );
    assert!(artifacts.report_path.exists());
}

/// AC4: a dirty working tree and uncommitted changes survive job completion, and the active
/// branch is unchanged — emission never touches the checkout.
#[test]
fn test_working_tree_untouched() {
    let (dir, repo) = temp_repo_dir();
    let base = init_repo_with_base(dir.path());
    let g = Repository::open(dir.path()).expect("open");
    let active_before = evoswarm_cli::git_writer::current_branch(&g);

    // Dirty the working tree with an uncommitted, untracked change before the job runs.
    let dirty_path = dir.path().join("uncommitted.txt");
    std::fs::write(&dirty_path, "user work in progress\n").expect("write dirty file");
    std::fs::write(dir.path().join(BASE_FILE), "line1\nMODIFIED BY USER\n").expect("modify tracked");

    let patch = make_patch(&g, &base);
    let _ = emit(
        &job_record("job-45", &base),
        &SelectionOutcome::Verified { candidate: candidate(patch) },
        &repo,
        &inputs(true),
    )
    .expect("emit");

    // The untracked change survives untouched.
    assert_eq!(
        std::fs::read_to_string(&dirty_path).unwrap(),
        "user work in progress\n",
        "uncommitted untracked file must survive emission"
    );
    // The tracked-file working-tree modification survives (emission worked on the object store).
    assert_eq!(
        std::fs::read_to_string(dir.path().join(BASE_FILE)).unwrap(),
        "line1\nMODIFIED BY USER\n",
        "uncommitted tracked change must survive emission"
    );
    // HEAD still points at the original branch, not the emitted one.
    assert_eq!(
        evoswarm_cli::git_writer::current_branch(&g),
        active_before,
        "active branch must remain unchanged"
    );
}

/// Spec §4: a NoVerifiedWinner still emits the best-effort patch and report, and the report
/// states the outcome explicitly rather than implying success.
#[test]
fn test_no_verified_winner_still_emits_best_effort() {
    let (dir, repo) = temp_repo_dir();
    let base = init_repo_with_base(dir.path());
    let g = Repository::open(dir.path()).expect("open");
    let patch = make_patch(&g, &base);

    let artifacts = emit(
        &job_record("job-46", &base),
        &SelectionOutcome::NoVerifiedWinner { best_effort: candidate(patch) },
        &repo,
        &inputs(false),
    )
    .expect("emit best-effort");

    assert!(artifacts.patch_path.exists(), "best-effort patch emitted");
    assert!(artifacts.report_path.exists(), "best-effort report emitted");
    assert!(
        artifacts.report_markdown.contains("no verified winner"),
        "report must state the no-winner outcome explicitly"
    );
}
