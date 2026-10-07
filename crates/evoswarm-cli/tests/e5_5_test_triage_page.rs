//! e5-5: Test triage web page acceptance tests (AD-5, §4).
//!
//! Acceptance criteria:
//! - AC1: Given the triage page, when started, then it binds to 127.0.0.1 only (refuses external connections).
//! - AC2: Given candidate tests for a repo, when listed, each shows its code and failing candidates.
//! - AC3: Given I approve or reject a test via HTTP endpoints, then the same promotion/rejection
//!   path as E2-6 is used.

use axum::http::StatusCode;
use evoswarm_cli::triage::{create_triage_router, TriageServerState};
use serde_json::json;
use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;

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

#[tokio::test]
async fn test_binds_localhost_only() {
    let dir = tempdir().expect("tempdir");
    let repo_path = dir.path().join("repo");
    fs::create_dir_all(&repo_path).expect("create repo");
    init_git_repo(&repo_path);

    let state = Arc::new(TriageServerState {
        repo_path: repo_path.clone(),
        spool_dir: repo_path.join(".evoswarm/lineage_spool"),
    });

    let _router = create_triage_router(state);

    // Verify binding to 127.0.0.1 succeeds
    let loopback: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(loopback).await;
    assert!(listener.is_ok(), "bind to 127.0.0.1 succeeds");
    let local_addr = listener.unwrap().local_addr().unwrap();
    assert_eq!(local_addr.ip().to_string(), "127.0.0.1");

    // Attempting to bind non-loopback (e.g. 0.0.0.0 or arbitrary public IP) must be disallowed by policy
    assert!(
        evoswarm_cli::triage::validate_bind_address("127.0.0.1:8080").is_ok(),
        "127.0.0.1 allowed"
    );
    assert!(
        evoswarm_cli::triage::validate_bind_address("0.0.0.0:8080").is_err(),
        "0.0.0.0 rejected"
    );
    assert!(
        evoswarm_cli::triage::validate_bind_address("192.168.1.1:8080").is_err(),
        "external IP rejected"
    );
}

#[tokio::test]
async fn test_triage_approval_and_rejection_actions() {
    let dir = tempdir().expect("tempdir");
    let repo_path = dir.path().join("repo");
    fs::create_dir_all(&repo_path).expect("create repo dir");
    init_git_repo(&repo_path);

    let spool_dir = repo_path.join(".evoswarm/lineage_spool");
    fs::create_dir_all(&spool_dir).expect("create spool dir");

    let sample_payload = json!({
        "task": {
            "id": "job-triage-1",
            "spec_hash": "hash_triage",
            "repo_fingerprint": "repo_triage",
            "toolchain_fingerprint": "tool_triage"
        },
        "implementations": [
            {
                "id": "cand-failing",
                "generation": 1,
                "patch_blob_sha256": "blob_fail",
                "model_role": "mutator",
                "parents": []
            }
        ],
        "evaluations": [
            {
                "id": "eval-1",
                "candidate_id": "cand-failing",
                "score": 0.4,
                "passed_gates": false,
                "wall_time_ms": 100,
                "tests": [
                    {"name": "test_adversary_boundary", "origin": "adversary", "passed": false},
                    {"name": "test_adversary_edge", "origin": "adversary", "passed": false}
                ]
            }
        ]
    });

    let record_file = spool_dir.join("job-triage-1.json");
    fs::write(&record_file, serde_json::to_string_pretty(&sample_payload).unwrap())
        .expect("write sample lineage");

    let state = Arc::new(TriageServerState {
        repo_path: repo_path.clone(),
        spool_dir: spool_dir.clone(),
    });

    let router = create_triage_router(state);

    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    // 1. GET /triage renders candidate tests
    let req = Request::builder()
        .uri("/triage")
        .body(Body::empty())
        .unwrap();

    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body_bytes = axum::body::to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let html = String::from_utf8_lossy(&body_bytes);
    assert!(html.contains("test_adversary_boundary"), "html contains test: {html}");
    assert!(html.contains("cand-failing"), "html mentions failing candidate: {html}");

    // 2. POST /triage/approve?job_id=job-triage-1&id=test_adversary_boundary
    let req_approve = Request::builder()
        .method("POST")
        .uri("/triage/approve?job_id=job-triage-1&id=test_adversary_boundary")
        .body(Body::empty())
        .unwrap();

    let resp_approve = router.clone().oneshot(req_approve).await.unwrap();
    assert_eq!(resp_approve.status(), StatusCode::OK);

    // Verify test promoted to trusted in metadata
    let status_file = repo_path.join(".evoswarm/approved_tests.json");
    assert!(status_file.exists());
    let status_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&status_file).unwrap()).unwrap();
    assert_eq!(
        status_json["trusted_tests"][0]["name"],
        "test_adversary_boundary"
    );

    // 3. POST /triage/reject?job_id=job-triage-1&id=test_adversary_edge
    let req_reject = Request::builder()
        .method("POST")
        .uri("/triage/reject?job_id=job-triage-1&id=test_adversary_edge")
        .body(Body::empty())
        .unwrap();

    let resp_reject = router.oneshot(req_reject).await.unwrap();
    assert_eq!(resp_reject.status(), StatusCode::OK);

    let status_json2: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&status_file).unwrap()).unwrap();
    assert_eq!(
        status_json2["rejected_tests"][0]["name"],
        "test_adversary_edge"
    );
}
