//! e3-1 `evolve` MCP tool — acceptance criteria (story §4).
//!
//! Gate: `cargo test --test e3_1_evolve_tool`.
//!
//! The tool is exposed over stdio JSON-RPC (MCP). This test drives the server's in-process
//! request handler directly, so the JSON-RPC framing, the MCP tool schema, the path-escape
//! guard (`require_within_root`), and the durable `Queued` ticket (`JobLedger`) are all
//! exercised for real — no stub, no LLM.
//!
//! SEAM: the transport is exercised at the handler boundary, not through a real Claude Code
//! stdio handshake. Per roadmap §5/§6 that end-to-end clause needs a running daemon + LLM
//! keys; the deferred clause is named in the PR. What is verified here is every behavioural
//! AC the story lists (fast ticket, missing-test rejection, path-escape rejection) plus the
//! MCP protocol scaffolding the sibling tools (e3-2..e3-4) build on.

mod common;

use common::{call_evolve, evolve_params, initialize_request, tools_call, tools_list, Fixture};
use serde_json::{json, Value};

/// AC1: a valid `evolve` call returns a `{job_id, status:"queued"}` ticket, and the job is
/// really persisted in `Queued` state in the ledger — proving the ticket is durable, not a
/// fabricated echo.
#[test]
fn test_evolve_submission_fast_return() {
    let fx = Fixture::new();
    let started = std::time::Instant::now();

    let resp = call_evolve(&fx.server, evolve_params("make it faster", "pytest -q"));
    let elapsed = started.elapsed();

    assert!(
        elapsed.as_secs() < 2,
        "AC1 requires a ticket within 2s; took {:?}",
        elapsed
    );

    let result = resp.expect("evolve returns a response (it carries a request id)");
    let result = result
        .get("result")
        .expect("no JSON-RPC error on a valid evolve call");
    // MCP tools/call result: content blocks + a structured payload.
    let payload = common::structured_content(result).expect("structured job ticket");
    let job_id = payload["job_id"]
        .as_str()
        .expect("job_id present")
        .to_string();
    assert_eq!(
        payload["status"],
        json!("queued"),
        "ticket status is queued"
    );
    assert!(!job_id.is_empty(), "job id assigned");

    // The ticket is durable: the ledger holds the job in Queued with the submitted params.
    let status = fx
        .server
        .ledger()
        .read_status(&job_id)
        .expect("job persisted");
    assert_eq!(status, evoswarm_core::JobStatus::Queued);
    let record = fx.server.ledger().read_job(&job_id).expect("read back");
    assert_eq!(record.submission.task_description, "make it faster");
    assert_eq!(record.submission.test_command, "pytest -q");
}

/// AC2: no test command ⇒ the tool call errors out and the diagnostic says a test suite is
/// required. In MCP a tool-level failure is an `isError` result (not a JSON-RPC protocol
/// error), because the request was well-formed and only the tool's precondition failed.
#[test]
fn test_evolve_requires_test_command() {
    let fx = Fixture::new();
    let mut params = evolve_params("make it faster", "pytest -q");
    params["test_command"] = json!("   "); // whitespace-only counts as missing

    let resp = tools_call(&fx.server, "evolve", params).expect("response");
    let result = &resp["result"];
    assert_eq!(
        result["isError"],
        json!(true),
        "a missing test command is a tool error, not a protocol fault: {:?}",
        resp
    );
    let text = common::error_text(result).expect("an error message");
    assert!(
        text.contains("test"),
        "error must explain a test suite is required, got {text:?}"
    );
    assert!(
        common::structured_content(result).is_none(),
        "a rejected call returns no job ticket: {result}"
    );
}

/// AC3: a path outside the repo root ⇒ rejected, and the offending path is named in the
/// diagnostic (so the caller learns *which* path crossed the boundary).
#[test]
fn test_evolve_path_traversal_rejection() {
    let fx = Fixture::new();
    let mut params = evolve_params("make it faster", "pytest -q");
    params["paths"] = json!(["../../etc"]);

    let resp = tools_call(&fx.server, "evolve", params).expect("response");
    let result = &resp["result"];
    assert_eq!(
        result["isError"],
        json!(true),
        "a path outside the root is a tool error: {:?}",
        resp
    );
    let text = common::error_text(result).expect("an error message");
    assert!(
        text.contains("../../etc"),
        "the rejected path must be named in the diagnostic, got {text:?}"
    );
}

/// The tool advertises itself over `tools/list` with a schema that requires the four story
/// params (task, test_command, paths, budget) — so a client can validate before calling.
#[test]
fn test_tools_list_advertises_evolve_schema() {
    let fx = Fixture::new();
    let resp = tools_list(&fx.server).expect("response");
    let tools = resp["result"]["tools"].as_array().expect("tools array");
    let evolve = tools
        .iter()
        .find(|t| t["name"] == json!("evolve"))
        .expect("evolve tool is advertised");

    let required = evolve["inputSchema"]["required"]
        .as_array()
        .expect("required list");
    for field in ["task", "test_command", "paths", "budget"] {
        assert!(
            required.contains(&json!(field)),
            "schema must require `{field}` (story §2 contract), got {required:?}"
        );
    }
}

/// The MCP lifecycle handshake: `initialize` returns a protocol version, server info and
/// tool capability — the shape a real client negotiates before calling tools.
#[test]
fn test_initialize_handshake_shape() {
    let fx = Fixture::new();
    let resp = initialize_request(&fx.server).expect("response");
    let result = &resp["result"];
    assert!(
        result["protocolVersion"].is_string(),
        "initialize reports a protocol version: {result}"
    );
    assert!(
        result["capabilities"]["tools"].is_object(),
        "tools capability"
    );
    assert!(
        result["serverInfo"]["name"].is_string(),
        "server identifies itself"
    );
}

/// Protocol robustness (the handler, not the tool): an unknown method is a JSON-RPC error
/// with code -32601 (method not found), and a well-formed `initialized` notification gets no
/// response body — the two framing rules every MCP client relies on.
#[test]
fn test_unknown_method_and_notification_framing() {
    let fx = Fixture::new();

    // Unknown method with an id ⇒ JSON-RPC error.
    let resp = fx
        .server
        .handle(&json!({"jsonrpc":"2.0","id":9,"method":"does_not_exist","params":{}}));
    let resp: Value = resp.expect("method-not-found produces a response");
    assert_eq!(
        resp["error"]["code"],
        json!(-32601),
        "method not found code: {resp}"
    );
    assert!(
        resp.get("result").is_none(),
        "an error response carries no result"
    );

    // `initialized` is a notification (no id) ⇒ no response body, per JSON-RPC 2.0.
    let notification = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
    assert!(
        fx.server.handle(&notification).is_none(),
        "a notification must not produce a response"
    );
}
