//! e3-4 `cancel_job` MCP tool — acceptance criteria (story §4).
//!
//! Gate: `cargo test --test e3_4_cancel_job_tool`.
//!
//! `cancel_job(id)` durably halts a job: it moves the ledger row to `Cancelled` (the state
//! change the story's contract mandates) and is idempotent — cancelling an already-cancelled
//! job returns the same result with no error, rather than re-running a transition the ledger
//! would reject.
//!
//! SEAM: the story's `best_verified_candidate` field is produced by the engine's in-memory
//! selection pass (`selection::select_verified_winner`) and is never persisted as a queryable
//! per-job field (the `job_generations` table stores only opaque population JSON), so this tool
//! returns it as explicit `null` rather than scraping a population blob or inventing an id. The
//! first AC ("no new model calls start after those in flight") is the live effect on the engine
//! loop and needs a running daemon + LLM keys (roadmap §7); the tool's durable contract here is
//! the `Cancelled` state transition, which is what a halted loop keys off.

mod common;

use common::{Fixture, Tools};
use evoswarm_core::JobStatus;
use serde_json::json;

/// Submits a job and returns its id (drives the same evolve path as e3-1/e3-3).
fn submit_job(fx: &Fixture) -> String {
    let resp = common::call_evolve(&fx.server, common::evolve_params("fix", "pytest -q"))
        .expect("evolve responds");
    common::structured_content(&resp["result"]).expect("ticket")["job_id"]
        .as_str()
        .expect("job_id")
        .to_string()
}

/// AC1: cancelling a running job durably marks it `cancelled` in the ledger and returns the
/// cancelled status. The state transition is the real, observable effect (not a stubbed echo).
#[test]
fn test_cancel_running_job() {
    let fx = Fixture::new();
    let id = submit_job(&fx);
    fx.server
        .ledger()
        .transition(&id, JobStatus::Running)
        .expect("queued -> running");

    let result = fx
        .server
        .tools_call_json("cancel_job", json!({ "id": id.clone() }))
        .expect("responds");
    assert_eq!(
        result["isError"],
        json!(false),
        "running job cancels: {result}"
    );
    let payload = common::structured_content(&result).expect("payload");

    assert_eq!(payload["job_id"], json!(id));
    assert_eq!(
        payload["status"],
        json!("cancelled"),
        "contract status is cancelled"
    );

    // The durable effect: the ledger row really moved to Cancelled, which is what the engine
    // loop reads to stop dispatching. Asserting the ledger (not just the response) proves the
    // transition happened, not that the tool merely printed the word.
    assert_eq!(
        fx.server.ledger().read_status(&id).expect("status"),
        JobStatus::Cancelled,
        "ledger durably records the cancellation"
    );

    // SEAM field: engine-computed, never persisted per-job → honest null.
    assert!(
        payload.get("best_verified_candidate").is_some(),
        "field present"
    );
    assert!(
        payload["best_verified_candidate"].is_null(),
        "best_verified_candidate is engine-in-memory, so null (not scraped/faked)"
    );
}

/// A queued job can be cancelled too (before the engine even starts), and still lands in the
/// durable `Cancelled` state.
#[test]
fn test_cancel_queued_job() {
    let fx = Fixture::new();
    let id = submit_job(&fx);
    // Still Queued — no transition to Running.

    let result = fx
        .server
        .tools_call_json("cancel_job", json!({ "id": id.clone() }))
        .expect("responds");
    assert_eq!(
        result["isError"],
        json!(false),
        "queued job cancels: {result}"
    );
    let payload = common::structured_content(&result).expect("payload");
    assert_eq!(payload["status"], json!("cancelled"));
    assert_eq!(
        fx.server.ledger().read_status(&id).expect("status"),
        JobStatus::Cancelled
    );
}

/// AC3 (idempotency): cancelling an already-cancelled job returns the same `cancelled` result
/// with NO error — the tool must guard on current state rather than blindly re-issuing a
/// `Cancelled -> Cancelled` transition the ledger forbids (that would surface an error).
#[test]
fn test_cancel_idempotency() {
    let fx = Fixture::new();
    let id = submit_job(&fx);
    fx.server
        .ledger()
        .transition(&id, JobStatus::Running)
        .expect("queued -> running");

    let first_resp = fx
        .server
        .tools_call_json("cancel_job", json!({ "id": id.clone() }))
        .expect("first cancel responds");
    let first = common::structured_content(&first_resp).expect("first payload");

    let second = fx
        .server
        .tools_call_json("cancel_job", json!({ "id": id.clone() }))
        .expect("second cancel responds");
    // The second call is NOT an error (AC3) — it returns the same cancelled result.
    assert_eq!(
        second["isError"],
        json!(false),
        "re-cancelling is idempotent, not an error: {second}"
    );
    let second_payload = common::structured_content(&second).expect("second payload");
    assert_eq!(second_payload["status"], json!("cancelled"));
    assert_eq!(second_payload, first, "identical result on the repeat call");
    // And the durable state is still Cancelled (the no-op didn't corrupt it).
    assert_eq!(
        fx.server.ledger().read_status(&id).expect("status"),
        JobStatus::Cancelled
    );
}

/// A job that already finished a terminal state other than cancelled (Completed) must NOT be
/// force-cancelled — that would rewrite a real outcome. It is a tool error that names the state,
/// distinct from the idempotent already-cancelled path. This is the guard that keeps AC3 from
/// collapsing into "always return cancelled no matter what".
#[test]
fn test_cannot_cancel_completed_job() {
    let fx = Fixture::new();
    let id = submit_job(&fx);
    fx.server
        .ledger()
        .transition(&id, JobStatus::Completed)
        .expect("queued -> completed");

    let result = fx
        .server
        .tools_call_json("cancel_job", json!({ "id": id.clone() }))
        .expect("responds");
    assert_eq!(
        result["isError"],
        json!(true),
        "a finished job is not cancellable: {result}"
    );
    let text = common::error_text(&result).expect("message").to_lowercase();
    // The refusal must be the *semantic* "cannot be cancelled" path, not any incidental error
    // that mentions the word "completed". A naive implementation that falls through to a raw
    // ledger.transition call for terminal states surfaces `LedgerError::IllegalTransition`
    // whose Debug is "illegal status transition Completed -> Cancelled" — that also contains
    // "completed", so a loose message check would wrongly accept it. Requiring the explicit
    // refusal wording pins the guard down.
    assert!(
        text.contains("cannot be cancelled"),
        "the error states the refusal explicitly, got {text:?}"
    );
    // The ledger is untouched: the completed job stays completed.
    assert_eq!(
        fx.server.ledger().read_status(&id).expect("status"),
        JobStatus::Completed,
        "a rejected cancel must not mutate the job"
    );
}

/// AC-not-found: an unknown id is a tool error that names the requested id (same discipline as
/// e3-2/e3-3).
#[test]
fn test_cancel_not_found() {
    let fx = Fixture::new();
    let result = fx
        .server
        .tools_call_json("cancel_job", json!({ "id": "ghost-job" }))
        .expect("responds");
    assert_eq!(result["isError"], json!(true), "unknown job: {result}");
    let text = common::error_text(&result).expect("message");
    assert!(
        text.contains("ghost-job"),
        "not-found names the id, got {text:?}"
    );
    assert!(
        common::structured_content(&result).is_none(),
        "no status payload on failure"
    );
}

/// Missing `id` is a required-argument tool error, distinct from a not-found.
#[test]
fn test_cancel_requires_id() {
    let fx = Fixture::new();
    let result = fx
        .server
        .tools_call_json("cancel_job", json!({}))
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

/// `cancel_job` is advertised with `id` required so a client can validate the shape before calling.
#[test]
fn test_cancel_advertised_in_tools_list() {
    let fx = Fixture::new();
    let resp = common::tools_list(&fx.server).expect("response");
    let tools = resp["result"]["tools"].as_array().expect("tools");
    let c = tools
        .iter()
        .find(|t| t["name"] == json!("cancel_job"))
        .expect("cancel_job advertised");
    let required = c["inputSchema"]["required"].as_array().expect("required");
    assert!(
        required.contains(&json!("id")),
        "cancel_job requires id, got {required:?}"
    );
}
