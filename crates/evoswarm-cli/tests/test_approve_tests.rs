//! e2-6: Approve candidate tests acceptance tests.
//!
//! Acceptance criteria (story §4):
//! - AC1: Given a job report, when running `evoswarm approve-tests <job> --ids <ids>`, then the
//!   chosen tests become trusted for that repo and are added on a branch for merge.
//! - AC2: Given a test is rejected (`evoswarm reject-tests <job> --ids <ids>` or status updated),
//!   when later jobs run, it is recorded in FalkorDB/metadata and omitted from future proposals.
//! - AC3: Missing job or invalid test ID produces a validation error with non-zero exit code.

use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_evoswarm")
}

fn init_git_repo(path: &std::path::Path) {
    let repo = git2::Repository::init(path).expect("init git repo");
    let sig = git2::Signature::now("EvoSwarm Test", "test@evoswarm.internal").expect("sig");
    let tree_id = {
        let mut index = repo.index().expect("index");
        let sample_file = path.join("README.md");
        fs::write(&sample_file, "# Test Repo\n").expect("write readme");
        index.add_path(std::path::Path::new("README.md")).expect("add");
        index.write().expect("write index");
        index.write_tree().expect("write tree")
    };
    let tree = repo.find_tree(tree_id).expect("find tree");
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
        .expect("commit");
}

#[test]
fn test_promote_adversary_test() {
    let dir = tempdir().expect("tempdir");
    let repo_path = dir.path().join("repo");
    fs::create_dir_all(&repo_path).expect("create repo dir");
    init_git_repo(&repo_path);

    let spool_dir = repo_path.join(".evoswarm/lineage_spool");
    fs::create_dir_all(&spool_dir).expect("create spool dir");

    let sample_payload = serde_json::json!({
        "task": {
            "id": "job-789",
            "spec_hash": "sha256_spec_789",
            "repo_fingerprint": "sha256_repo_789",
            "toolchain_fingerprint": "sha256_toolchain_789"
        },
        "implementations": [
            {
                "id": "cand-win",
                "generation": 1,
                "patch_blob_sha256": "blob_win",
                "model_role": "mutator",
                "parents": []
            }
        ],
        "evaluations": [
            {
                "id": "eval-win",
                "candidate_id": "cand-win",
                "score": 0.95,
                "passed_gates": true,
                "wall_time_ms": 150,
                "tests": [
                    {"name": "test_adv_edge_case", "origin": "adversary", "passed": true},
                    {"name": "test_adv_boundary", "origin": "adversary", "passed": true}
                ]
            }
        ]
    });

    let record_file = spool_dir.join("job-789.json");
    fs::write(&record_file, serde_json::to_string_pretty(&sample_payload).unwrap())
        .expect("write sample lineage");

    let out = Command::new(bin())
        .args([
            "approve-tests",
            "job-789",
            "--ids",
            "test_adv_edge_case",
            "--repo",
            repo_path.to_str().unwrap(),
        ])
        .output()
        .expect("run approve-tests");

    assert!(
        out.status.success(),
        "exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("test_adv_edge_case"), "stdout mentions approved test: {stdout}");
    assert!(stdout.contains("evoswarm/tests-job-789"), "stdout mentions branch: {stdout}");

    // Verify git branch exists
    let repo = git2::Repository::open(&repo_path).expect("open repo");
    let branch = repo.find_branch("evoswarm/tests-job-789", git2::BranchType::Local);
    assert!(branch.is_ok(), "git branch created for approved tests");

    // Verify test status record updated
    let status_file = repo_path.join(".evoswarm/approved_tests.json");
    assert!(status_file.exists(), "approved tests file created");
    let status_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&status_file).unwrap()).unwrap();
    assert_eq!(
        status_json["trusted_tests"][0]["name"],
        "test_adv_edge_case"
    );
}

#[test]
fn test_rejected_test_suppression() {
    let dir = tempdir().expect("tempdir");
    let repo_path = dir.path().join("repo");
    fs::create_dir_all(&repo_path).expect("create repo dir");
    init_git_repo(&repo_path);

    let spool_dir = repo_path.join(".evoswarm/lineage_spool");
    fs::create_dir_all(&spool_dir).expect("create spool dir");

    let sample_payload = serde_json::json!({
        "task": {
            "id": "job-790",
            "spec_hash": "sha256_spec_790",
            "repo_fingerprint": "sha256_repo_790",
            "toolchain_fingerprint": "sha256_toolchain_790"
        },
        "implementations": [],
        "evaluations": [
            {
                "id": "eval-adv",
                "candidate_id": "cand-1",
                "score": 0.5,
                "passed_gates": false,
                "wall_time_ms": 100,
                "tests": [
                    {"name": "test_adv_flaky", "origin": "adversary", "passed": false}
                ]
            }
        ]
    });

    let record_file = spool_dir.join("job-790.json");
    fs::write(&record_file, serde_json::to_string_pretty(&sample_payload).unwrap())
        .expect("write sample lineage");

    let out = Command::new(bin())
        .args([
            "reject-tests",
            "job-790",
            "--ids",
            "test_adv_flaky",
            "--repo",
            repo_path.to_str().unwrap(),
        ])
        .output()
        .expect("run reject-tests");

    assert!(
        out.status.success(),
        "exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let status_file = repo_path.join(".evoswarm/approved_tests.json");
    assert!(status_file.exists(), "status file created");
    let status_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&status_file).unwrap()).unwrap();
    assert_eq!(
        status_json["rejected_tests"][0]["name"],
        "test_adv_flaky"
    );
}
