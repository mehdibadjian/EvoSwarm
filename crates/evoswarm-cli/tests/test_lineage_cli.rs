//! e2-8: Lineage CLI acceptance tests.
//!
//! Acceptance criteria (story §4):
//! - AC1: Given a job ID, when running `evoswarm lineage <job>`, it prints the winner's
//!   ancestry with generation, model, score and gate failure reasons as an ASCII tree.
//! - AC2: Given `--json`, when run, the same lineage prints as a JSON object/array.
//! - AC3: Handles missing job or failure gracefully with non-zero exit code.

use std::fs;
use std::process::Command;
use tempfile::tempdir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_evoswarm")
}

#[test]
fn test_ascii_tree_rendering() {
    let dir = tempdir().expect("tempdir");
    let spool_dir = dir.path().join(".evoswarm/lineage_spool");
    fs::create_dir_all(&spool_dir).expect("create spool dir");

    let sample_payload = serde_json::json!({
        "task": {
            "id": "job-123",
            "spec_hash": "sha256_spec_abc",
            "repo_fingerprint": "sha256_repo_def",
            "toolchain_fingerprint": "sha256_toolchain_123"
        },
        "implementations": [
            {
                "id": "cand-0",
                "generation": 0,
                "patch_blob_sha256": "blob0",
                "model_role": "generator-seed",
                "parents": []
            },
            {
                "id": "cand-1",
                "generation": 1,
                "patch_blob_sha256": "blob1",
                "model_role": "mutator-pro",
                "parents": [
                    {
                        "parent_id": "cand-0",
                        "relation": "MUTATED_FROM",
                        "traits": "syntax_fix"
                    }
                ]
            }
        ],
        "evaluations": [
            {
                "id": "eval-0",
                "candidate_id": "cand-0",
                "score": 0.45,
                "passed_gates": false,
                "wall_time_ms": 120,
                "tests": [
                    {"name": "test_basic", "origin": "visible", "passed": true},
                    {"name": "test_edge", "origin": "visible", "passed": false}
                ]
            },
            {
                "id": "eval-1",
                "candidate_id": "cand-1",
                "score": 0.98,
                "passed_gates": true,
                "wall_time_ms": 95,
                "tests": [
                    {"name": "test_basic", "origin": "visible", "passed": true},
                    {"name": "test_edge", "origin": "visible", "passed": true}
                ]
            }
        ]
    });

    let record_file = spool_dir.join("job-123.json");
    fs::write(&record_file, serde_json::to_string_pretty(&sample_payload).unwrap())
        .expect("write sample lineage");

    let out = Command::new(bin())
        .args([
            "lineage",
            "job-123",
            "--spool-dir",
            spool_dir.to_str().unwrap(),
        ])
        .output()
        .expect("run evoswarm lineage");

    assert!(
        out.status.success(),
        "exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("job-123"), "output mentions job ID: {stdout}");
    assert!(stdout.contains("gen 0"), "output contains gen 0: {stdout}");
    assert!(stdout.contains("cand-0"), "output contains root cand-0: {stdout}");
    assert!(stdout.contains("generator-seed"), "output contains model role: {stdout}");
    assert!(stdout.contains("0.45"), "output contains candidate 0 score: {stdout}");
    assert!(stdout.contains("gen 1"), "output contains gen 1: {stdout}");
    assert!(stdout.contains("cand-1"), "output contains cand-1: {stdout}");
    assert!(stdout.contains("mutator-pro"), "output contains mutator-pro: {stdout}");
    assert!(stdout.contains("0.98"), "output contains candidate 1 score: {stdout}");
    assert!(stdout.contains("failed"), "output indicates gate failure for cand-0: {stdout}");
}

#[test]
fn test_json_lineage_export() {
    let dir = tempdir().expect("tempdir");
    let spool_dir = dir.path().join(".evoswarm/lineage_spool");
    fs::create_dir_all(&spool_dir).expect("create spool dir");

    let sample_payload = serde_json::json!({
        "task": {
            "id": "job-456",
            "spec_hash": "sha256_spec_456",
            "repo_fingerprint": "sha256_repo_456",
            "toolchain_fingerprint": "sha256_toolchain_456"
        },
        "implementations": [
            {
                "id": "cand-0",
                "generation": 0,
                "patch_blob_sha256": "blob0",
                "model_role": "generator-seed",
                "parents": []
            }
        ],
        "evaluations": [
            {
                "id": "eval-0",
                "candidate_id": "cand-0",
                "score": 0.75,
                "passed_gates": true,
                "wall_time_ms": 100,
                "tests": []
            }
        ]
    });

    let record_file = spool_dir.join("job-456.json");
    fs::write(&record_file, serde_json::to_string_pretty(&sample_payload).unwrap())
        .expect("write sample lineage");

    let out = Command::new(bin())
        .args([
            "lineage",
            "job-456",
            "--json",
            "--spool-dir",
            spool_dir.to_str().unwrap(),
        ])
        .output()
        .expect("run evoswarm lineage --json");

    assert!(
        out.status.success(),
        "exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("valid json output");
    assert_eq!(parsed["task"]["id"], "job-456");
    assert_eq!(parsed["implementations"][0]["id"], "cand-0");
    assert_eq!(parsed["evaluations"][0]["score"], 0.75);
}

#[test]
fn test_lineage_not_found() {
    let dir = tempdir().expect("tempdir");
    let spool_dir = dir.path().join(".evoswarm/lineage_spool");
    fs::create_dir_all(&spool_dir).expect("create spool dir");

    let out = Command::new(bin())
        .args([
            "lineage",
            "job-nonexistent",
            "--spool-dir",
            spool_dir.to_str().unwrap(),
        ])
        .output()
        .expect("run evoswarm lineage");

    assert_eq!(
        out.status.code(),
        Some(2),
        "validation error code when job lineage is not found"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("job-nonexistent"),
        "stderr indicates missing job"
    );
}
