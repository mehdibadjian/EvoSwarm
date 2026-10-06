//! Shared test support for the e3 MCP tool stories.
//!
//! Lives under `tests/common/` so Cargo does not treat it as its own `--test` target; each
//! e3 test pulls it in with `mod common;`.
//!
//! The server is driven in-process through its JSON-RPC `handle` boundary — a real request
//! object in, a real response object out. This is the roadmap §5 SEAM for Epic 3: the schema,
//! framing, path guard and ledger writes are exercised for real; only the stdio pipe to an
//! actual Claude Code client (which needs a running daemon + LLM) is standing in for.

#![allow(dead_code)]

use evoswarm_ledger::{JobLedger, RepoRoot};
use evoswarm_mcp::Server;
use serde_json::{json, Value};
use tempfile::TempDir;

/// A fixture: an isolated repository root (temp dir) and a server wired to an in-memory
/// ledger rooted at it, so each test gets a clean job table and a clean boundary to guard.
pub struct Fixture {
    pub root: TempDir,
    pub server: Server,
}

impl Fixture {
    pub fn new() -> Self {
        let root = TempDir::new().expect("temp repo root");
        let ledger = JobLedger::open_in_memory().expect("in-memory ledger");
        let server =
            Server::new(ledger, RepoRoot::new(root.path())).expect("server wires to a valid root");
        Self { root, server }
    }
}

/// Builds a JSON-RPC `tools/call` request for `tool` with `arguments`.
pub fn request_with_id(id: u64, method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
}

/// Sends a `tools/call` for `tool` with the given `arguments`, returning the raw response
/// (`None` only for a notification, which a `tools/call` never is).
pub fn tools_call(server: &Server, tool: &str, arguments: Value) -> Option<Value> {
    server.handle(&request_with_id(
        1,
        "tools/call",
        json!({ "name": tool, "arguments": arguments }),
    ))
}

/// Sends a `tools/list` request.
pub fn tools_list(server: &Server) -> Option<Value> {
    server.handle(&request_with_id(2, "tools/list", json!({})))
}

/// Sends an `initialize` request (the MCP lifecycle handshake).
pub fn initialize_request(server: &Server) -> Option<Value> {
    server.handle(&request_with_id(
        3,
        "initialize",
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test-client", "version": "0" }
        }),
    ))
}

/// A well-formed `evolve` arguments object: the four story params, with a single in-root
/// path and a default budget. Callers mutate fields to exercise rejection cases.
///
/// The `paths` entry is `"src"` — relative, so the server resolves it against its own
/// `RepoRoot` (the fixture's temp dir). A non-existent in-root path still passes the guard's
/// lexical containment check, so the happy path needs no directory pre-created.
pub fn evolve_params(task: &str, test_command: &str) -> Value {
    json!({
        "task": task,
        "test_command": test_command,
        "paths": ["src"],
        "budget": { "tokens": 100_000, "dollars": 5.0 }
    })
}

/// Convenience: `tools/call evolve` with `arguments`, returning the response.
pub fn call_evolve(server: &Server, arguments: Value) -> Option<Value> {
    tools_call(server, "evolve", arguments)
}

/// Extracts the `structuredContent` object from a `tools/call` result, or `None` if the
/// result carries none (which is the shape of a tool error).
pub fn structured_content(result: &Value) -> Option<&Value> {
    result.get("structuredContent").filter(|v| v.is_object())
}

/// Concatenates the text of every `content` block in a result — used to assert that a tool
/// error message names the offending input.
pub fn error_text(result: &Value) -> Option<String> {
    let blocks = result.get("content")?.as_array()?;
    let joined = blocks
        .iter()
        .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join("\n");
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}
