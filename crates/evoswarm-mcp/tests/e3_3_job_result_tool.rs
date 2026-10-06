//! e3-3 `job_result` MCP tool — acceptance criteria (story §4).
//!
//! Gate: `cargo test --test e3_3_job_result_tool`.
//!
//! `job_result(id)` fetches a finished job's artifacts. AC1 and AC2 are fully real here: the
//! branch name, patch path and report path are derived from the exact deterministic layout
//! e1-11's `artifacts::emit` writes, and existence is checked against the filesystem, so the
//! tool reports artifacts that are actually on disk — not a fabricated path.
//!
//! SEAM: two of the story's five fields (`score`, `tests_passed`) are computed by the engine
//! and only rendered *into* the markdown report; they are never persisted as structured,
//! per-job data, so this tool returns them as explicit `null` rather than scraping markdown or
//! inventing numbers. AC3 ("Claude Code applies the patch and the tests pass") is the live
//! end-to-end and needs a running daemon + LLM keys (roadmap §7), so it is not asserted here —
//! see the PR's SEAM note.

mod common;

use common::{Fixture, Tools};
use evoswarm_core::JobStatus;
use serde_json::json;
use std::fs;

/// Submits a job and returns its id.
fn submit_job(fx: &Fixture) -> String {
    let resp = common::call_evolve(&fx.server, common::evolve_params("fix", "pytest -q"))
        .expect("evolve responds");
    common::structured_content(&resp["result"]).expect("ticket")["job_id"]
        .as_str()
        .expect("job_id")
        .to_string()
}

/// Mirrors e1-11's `artifacts::emit` layout exactly: the repo-relative paths it writes.
fn artifact_paths(id: &str) -> (String, String) {
    (
        format!(".evoswarm/patches/{id}.patch"),
        format!(".evoswarm/reports/{id}.md"),
    )
}

/// AC1: a completed job with artifacts on disk reports the branch and both paths, each marked
/// present. The patch/report files are really created (by the test, standing in for e1-11) and
/// the tool reads their existence from the filesystem — proving it is not echoing a path.
#[test]
fn test_fetch_completed_result() {
    let fx = Fixture::new();
    let id = submit_job(&fx);

    // Drive to Completed and write the artifacts where e1-11 would have written them.
    fx.server
        .ledger()
        .transition(&id, JobStatus::Completed)
        .expect("queued -> completed");
    let (patch_rel, report_rel) = artifact_paths(&id);
    let patch_abs = fx.root.path().join(&patch_rel);
    let report_abs = fx.root.path().join(&report_rel);
    fs::create_dir_all(patch_abs.parent().unwrap()).unwrap();
    fs::create_dir_all(report_abs.parent().unwrap()).unwrap();
    fs::write(&patch_abs, b"diff --git a/x b/x\n").expect("write patch");
    fs::write(&report_abs, b"# report\n").expect("write report");

    let result = fx
        .server
        .tools_call_json("job_result", json!({ "id": id.clone() }))
        .expect("responds");
    assert_eq!(result["isError"], json!(false), "completed job: {result}");
    let payload = common::structured_content(&result).expect("payload");

    assert_eq!(payload["job_id"], json!(id));
    assert_eq!(payload["state"], json!("completed"));
    // Branch name is e1-11's `evoswarm/<id>` convention.
    assert_eq!(payload["branch"], json!(format!("evoswarm/{id}")));
    // Paths are the deterministic artifact locations, reported as present.
    assert_eq!(payload["patch_path"], json!(patch_rel));
    assert_eq!(
        payload["patch_exists"],
        json!(true),
        "filesystem says patch is on disk"
    );
    assert_eq!(payload["report_path"], json!(report_rel));
    assert_eq!(payload["report_exists"], json!(true));

    // The engine-computed fields are honestly unavailable (markdown-only, not persisted).
    for field in ["score", "tests_passed"] {
        assert!(payload.get(field).is_some(), "key present: {field}");
        assert!(
            payload[field].is_null(),
            "`{field}` is markdown-only, so null (not scraped/faked)"
        );
    }
}

/// A completed job whose artifacts are NOT on disk (e.g. a resume that never re-emitted them)
/// reports the expected paths but marks them absent — so the caller can tell a missing patch
/// from a tool failure. Existence is a real filesystem read.
#[test]
fn test_completed_result_reports_absent_artifacts() {
    let fx = Fixture::new();
    let id = submit_job(&fx);
    fx.server
        .ledger()
        .transition(&id, JobStatus::Completed)
        .expect("queued -> completed");
    // Deliberately create NO files.

    let result = fx
        .server
        .tools_call_json("job_result", json!({ "id": id.clone() }))
        .expect("responds");
    let payload = common::structured_content(&result).expect("payload");

    assert_eq!(payload["state"], json!("completed"));
    assert_eq!(
        payload["patch_exists"],
        json!(false),
        "no file => absent, not fabricated"
    );
    assert_eq!(payload["report_exists"], json!(false));
}

/// AC2: an unfinished (running) job returns its state and no patch, and is not an error — the
/// tool answers "still running", it does not fail. Artifacts are not surfaced while running.
#[test]
fn test_unfinished_job_returns_state_no_patch() {
    let fx = Fixture::new();
    let id = submit_job(&fx);
    fx.server
        .ledger()
        .transition(&id, JobStatus::Running)
        .expect("queued -> running");
    // Even if a file were present, a non-completed job without partial must not surface it.
    let (patch_rel, _) = artifact_paths(&id);
    let p = fx.root.path().join(&patch_rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, b"premature").unwrap();

    let result = fx
        .server
        .tools_call_json("job_result", json!({ "id": id.clone() }))
        .expect("responds");
    assert_eq!(
        result["isError"],
        json!(false),
        "a running job is a valid query: {result}"
    );
    let payload = common::structured_content(&result).expect("payload");

    assert_eq!(payload["state"], json!("running"));
    assert!(
        payload["patch_path"].is_null(),
        "no patch surfaced while running: {payload}"
    );
    assert!(payload["branch"].is_null(), "no branch until completion");
}

/// AC2 `partial` clause ("...unless partial=true"): with `partial` set, the tool DOES surface
/// whatever artifacts exist mid-run, so the flag has a real, observable effect — it is not an
/// accepted-but-ignored argument. Same job, same on-disk patch; only `partial` differs.
#[test]
fn test_partial_flag_surfaces_mid_run_artifacts() {
    let fx: Fixture = Fixture::new();
    let id = submit_job(&fx);
    fx.server
        .ledger()
        .transition(&id, JobStatus::Running)
        .expect("queued -> running");

    let (patch_rel, report_rel) = artifact_paths(&id);
    fs::create_dir_all(fx.root.path().join(&patch_rel).parent().unwrap()).unwrap();
    fs::write(fx.root.path().join(&patch_rel), b"mid-run").unwrap();
    fs::create_dir_all(fx.root.path().join(&report_rel).parent().unwrap()).unwrap();
    fs::write(fx.root.path().join(&report_rel), b"# partial").unwrap();

    // partial=false: nothing surfaced (covered above); partial=true: surfaced.
    let result = fx
        .server
        .tools_call_json("job_result", json!({ "id": id.clone(), "partial": true }))
        .expect("responds");
    let payload = common::structured_content(&result).expect("payload");

    assert_eq!(
        payload["state"],
        json!("running"),
        "still reports the true state"
    );
    assert_eq!(
        payload["patch_path"],
        json!(patch_rel),
        "partial=true surfaces the mid-run patch"
    );
    assert_eq!(payload["patch_exists"], json!(true));
    assert!(
        payload["partial"].is_boolean(),
        "the payload echoes whether partial data is being reported: {payload}"
    );
}

/// AC-not-found: an unknown id is a tool error that names the requested id (same discipline as
/// e3-2's not-found and e3-1's path rejection).
#[test]
fn test_result_not_found() {
    let fx = Fixture::new();
    let result = fx
        .server
        .tools_call_json("job_result", json!({ "id": "ghost-job" }))
        .expect("responds");
    assert_eq!(result["isError"], json!(true), "unknown job: {result}");
    let text = common::error_text(&result).expect("message");
    assert!(
        text.contains("ghost-job"),
        "not-found names the id, got {text:?}"
    );
    assert!(
        common::structured_content(&result).is_none(),
        "no artifact payload on failure"
    );
}

/// Missing `id` is a required-argument tool error, distinct from a not-found (the message says
/// the id is required).
#[test]
fn test_result_requires_id() {
    let fx = Fixture::new();
    let result = fx
        .server
        .tools_call_json("job_result", json!({}))
        .expect("responds");
    assert_eq!(
        result["isError"],
        json!(true),
        "no id => tool error: {result}"
    );
    let text = common::error_text(&result).expect("message").to_lowercase();
    assert!(
        text.contains("id") && (text.contains("required") || text.contains("must")),
        "the message says the id is required, got {text:?}"
    );
}

/// `job_result` is advertised with `id` required so a client can validate the shape before
/// calling.
#[test]
fn test_job_result_advertised_in_tools_list() {
    let fx = Fixture::new();
    let resp = common::tools_list(&fx.server).expect("response");
    let tools = resp["result"]["tools"].as_array().expect("tools");
    let r = tools
        .iter()
        .find(|t| t["name"] == json!("job_result"))
        .expect("job_result advertised");
    let required = r["inputSchema"]["required"].as_array().expect("required");
    assert!(
        required.contains(&json!("id")),
        "job_result requires id, got {required:?}"
    );
}
