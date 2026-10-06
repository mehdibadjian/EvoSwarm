//! e3-2 `job_status` MCP tool — acceptance criteria (story §4).
//!
//! Gate: `cargo test --test e3_2_job_status_tool`.
//!
//! `job_status(id)` reports a job's progress for a polling Claude Code session. Two ACs are
//! fully real here: the not-found path, and the fields the ledger actually carries (state,
//! current generation). The calibrated/accumulative fields (best score, spend so far, ETA)
//! are sourced by the engine, which does not persist them to the ledger yet — so the tool
//! returns them as explicit `null` rather than fabricating numbers. That honesty is the
//! point: see the SEAM note in the PR.

mod common;

use common::{Fixture, Tools};
use evoswarm_core::JobStatus;
use evoswarm_ledger::GenerationStatus;
use serde_json::json;

/// Submits a job through `evolve` and returns its id, so the status tests query a job the
/// server itself enqueued (the real flow, not a hand-inserted row).
fn submit_job(fx: &Fixture) -> String {
    let resp = common::call_evolve(
        &fx.server,
        common::evolve_params("make it faster", "pytest -q"),
    )
    .expect("evolve responds");
    common::structured_content(&resp["result"]).expect("ticket")["job_id"]
        .as_str()
        .expect("job_id")
        .to_string()
}

/// AC1: a running job with committed generations reports its state and current generation,
/// and the payload carries the full five-field contract (best_score/spend/eta present as
/// keys even though their engine-sourced values are null here).
#[test]
fn test_status_query() {
    let fx = Fixture::new();
    let id = submit_job(&fx);

    // Drive it to Running with three completed generations, so max_generation == 2.
    fx.server
        .ledger()
        .transition(&id, JobStatus::Running)
        .expect("queued -> running");
    for g in 0..=2 {
        fx.server
            .ledger()
            .commit_generation(&id, g, "[]", GenerationStatus::Completed)
            .expect("commit generation");
    }

    let result = fx
        .server
        .tools_call_json("job_status", json!({ "id": id.clone() }))
        .expect("job_status responds");

    assert_eq!(
        result["isError"],
        json!(false),
        "a known job is not an error: {result}"
    );
    let payload = common::structured_content(&result).expect("structured status");

    // Real, ledger-derived fields:
    assert_eq!(
        payload["state"],
        json!("running"),
        "state maps the job status"
    );
    assert_eq!(
        payload["generation"],
        json!(2),
        "current generation from the ledger"
    );

    // Contract completeness: the other three fields are present (null-valued here). They are
    // keys the schema guarantees, so a client can always read them.
    for field in ["best_score", "spend_usd", "eta_seconds"] {
        assert!(
            payload.get(field).is_some(),
            "payload must carry `{field}`: {payload}"
        );
        assert!(
            payload[field].is_null(),
            "`{field}` is engine-sourced and not yet persisted, so null (not a fake number): {:?}",
            payload[field]
        );
    }

    // The id round-trips for the caller's correlation.
    assert_eq!(payload["job_id"], json!(id));
}

/// A freshly queued job (no generations committed) reports generation 0, not null/panic —
/// the ledger's `max_generation` returns None and the tool maps it to the not-yet-started
/// generation index.
#[test]
fn test_status_queued_job_generation_zero() {
    let fx = Fixture::new();
    let id = submit_job(&fx);

    let result = fx
        .server
        .tools_call_json("job_status", json!({ "id": id }))
        .expect("responds");
    let payload = common::structured_content(&result).expect("payload");

    assert_eq!(payload["state"], json!("queued"));
    assert_eq!(
        payload["generation"],
        json!(0),
        "no committed generation == gen 0"
    );
}

/// AC2: an unknown id ⇒ a not-found tool error that names the requested id, and crucially
/// the not-found is a *tool* error (isError) not a protocol fault, so a polling client sees a
/// well-formed "this job doesn't exist" result rather than a broken socket.
#[test]
fn test_status_not_found() {
    let fx = Fixture::new();
    let result = fx
        .server
        .tools_call_json("job_status", json!({ "id": "does-not-exist" }))
        .expect("responds");

    assert_eq!(
        result["isError"],
        json!(true),
        "unknown job is a tool error, not a protocol fault: {result}"
    );
    let text = common::error_text(&result).expect("an error message");
    assert!(
        text.contains("does-not-exist"),
        "the not-found diagnostic names the requested id, got {text:?}"
    );
    assert!(
        text.to_lowercase().contains("not found") || text.to_lowercase().contains("unknown"),
        "the message says the job was not found, got {text:?}"
    );
}

/// Missing the required `id` argument is a tool error, distinguished from "not found" by a
/// message that says the id is *required* — so an implementation that lets an empty id fall
/// through to a lookup (and reports "not found") is caught, not passing on a loose isError.
#[test]
fn test_status_requires_id_argument() {
    let fx = Fixture::new();
    let result = fx
        .server
        .tools_call_json("job_status", json!({}))
        .expect("responds");
    assert_eq!(
        result["isError"],
        json!(true),
        "a status call with no id is a tool error: {result}"
    );
    assert!(
        common::structured_content(&result).is_none(),
        "no status payload when the required id is absent: {result}"
    );
    let text = common::error_text(&result).expect("an error message");
    let lower = text.to_lowercase();
    assert!(
        lower.contains("id") && (lower.contains("required") || lower.contains("must")),
        "the error must say the id is required, not just that lookup failed, got {text:?}"
    );
}

/// `job_status` is advertised in tools/list with `id` required, so a client can validate the
/// shape before calling (mirrors the evolve schema test in e3-1).
#[test]
fn test_job_status_advertised_in_tools_list() {
    let fx = Fixture::new();
    let resp = common::tools_list(&fx.server).expect("response");
    let tools = resp["result"]["tools"].as_array().expect("tools");
    let status = tools
        .iter()
        .find(|t| t["name"] == json!("job_status"))
        .expect("job_status is advertised alongside evolve");
    let required = status["inputSchema"]["required"]
        .as_array()
        .expect("required list");
    assert!(
        required.contains(&json!("id")),
        "job_status must require `id`, got {required:?}"
    );
}
