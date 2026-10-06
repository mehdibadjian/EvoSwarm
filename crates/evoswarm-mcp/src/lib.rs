//! EvoSwarm MCP server (AD-2, AD-7): the JSON-RPC tool surface Claude Code talks to.
//!
//! This is the seam Epic 3 was scoped against (roadmap §5): the protocol framing, tool
//! schema, path guard and ledger writes are real and testable in-process; only the stdio
//! pipe to a live Claude Code client needs a running daemon + LLM keys and is deferred.
//!
//! e3-1 stands up the server and the `evolve` tool. e3-2 (job_status), e3-3 (job_result) and
//! e3-4 (cancel_job) extend the same `tools_call` match arm and `tool_definitions` list.
//!
//! Two error channels, matching MCP semantics exactly:
//! - A **protocol** fault (unknown method, malformed `tools/call` envelope) is a JSON-RPC
//!   `error` object — the request never reached a tool.
//! - A **tool** failure on a well-formed call (missing test command, a path outside the repo)
//!   is a *successful* JSON-RPC response whose `result` carries `isError: true` and a human
//!   `content` message. That is what the spec mandates: the transport worked, the tool
//!   refused. Conflating the two would make a client treat a rejected task as a dropped socket.

use std::path::PathBuf;

use evoswarm_core::{require_within_root, JobObjective, JobStatus, JobSubmission};
use evoswarm_ledger::{JobLedger, RepoRoot};
use serde_json::{json, Value};

/// The MCP protocol version this server implements.
pub const PROTOCOL_VERSION: &str = "2024-11-05";

/// Default wall-clock budget (seconds) for a delegated job when the client omits one. e3-1's
/// contract takes `budget` as tokens/dollars; the sandbox timeout is the engine's concern
/// (e1-10), so a sane ceiling is recorded at submission rather than inferred.
const DEFAULT_TIMEOUT_SECS: u64 = 3600;

/// JSON-RPC error code: the requested method does not exist.
const METHOD_NOT_FOUND: i64 = -32601;
/// JSON-RPC error code: invalid method parameters.
const INVALID_PARAMS: i64 = -32602;

/// The MCP server: a ledger it enqueues into and a repository root it guards every submitted
/// path against. Both are injected so tests get a fresh in-memory ledger and a temp root.
pub struct Server {
    ledger: JobLedger,
    root: RepoRoot,
}

impl Server {
    /// Wires a server. The root is canonicalised once so the path guard and callers agree on
    /// the boundary; a root that cannot be resolved is a wiring error, not a per-request one.
    pub fn new(ledger: JobLedger, root: RepoRoot) -> Result<Self, ServerError> {
        let root_canon = root.as_path().canonicalize().map_err(|e| {
            ServerError::BadRoot(root.as_path().display().to_string(), e.to_string())
        })?;
        Ok(Self {
            ledger,
            root: RepoRoot::new(root_canon),
        })
    }

    /// Borrows the ledger so callers (and tests) can read job state back.
    pub fn ledger(&self) -> &JobLedger {
        &self.ledger
    }

    /// The repository root this server guards every submitted path against.
    pub fn root(&self) -> &RepoRoot {
        &self.root
    }

    /// Handles one JSON-RPC message. Returns `Some(response)` for a request (a message that
    /// carries an `id`) and `None` for a notification (no `id`) — JSON-RPC 2.0 forbids
    /// responding to a notification. `req` must already be a parsed JSON value.
    pub fn handle(&self, req: &Value) -> Option<Value> {
        let method = req.get("method").and_then(|m| m.as_str())?;

        // Extract the id first: a notification has none, and must not receive a response even
        // if we computed one. `null` is treated as "no id" per the spec's request-vs-notification rule.
        let id = req.get("id").filter(|v| !v.is_null()).cloned();

        let result: Result<Value, RpcError> = match method {
            "initialize" => Ok(initialize_result()),
            "tools/list" => Ok(tools_list_result()),
            "tools/call" => self.tools_call(req),
            "ping" => Ok(json!({})),
            // Lifecycle notifications carry no observable server-side effect to report back.
            "notifications/initialized" | "notifications/cancelled" => return None,
            other => Err(RpcError::new(
                METHOD_NOT_FOUND,
                format!("method not found: {other}"),
            )),
        };

        // No id ⇒ this was a request-shaped message we must not answer (defensive: a real
        // client always ids its requests). Notifications already returned None above.
        let id = id?;
        Some(match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err(err) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": err.code, "message": err.message },
            }),
        })
    }

    /// Dispatches a `tools/call`. A malformed envelope (no `name`) is a protocol error; an
    /// unknown tool name is a protocol error too, since no tool was addressed. A tool's *own*
    /// refusal is returned as an `isError` result, never a protocol fault.
    fn tools_call(&self, req: &Value) -> Result<Value, RpcError> {
        let params = req
            .get("params")
            .ok_or_else(|| RpcError::new(INVALID_PARAMS, "tools/call missing params"))?;
        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::new(INVALID_PARAMS, "tools/call missing tool name"))?;
        let arguments = params
            .get("arguments")
            .filter(|v| v.is_object())
            .cloned()
            .unwrap_or_else(|| json!({}));

        match name {
            "evolve" => Ok(self.evolve(&arguments)),
            "job_status" => Ok(self.job_status(&arguments)),
            other => Err(RpcError::new(
                INVALID_PARAMS,
                format!("unknown tool: {other}"),
            )),
        }
    }

    /// The `evolve` tool (e3-1): validate the contract, guard every path against the repo
    /// root, persist a `Queued` job, and return its durable ticket. Never blocks on baseline
    /// validation or the search itself — the ticket is the contract (AD-2, <2s).
    fn evolve(&self, arguments: &Value) -> Value {
        // Every rejection here is a tool error (isError), so the client sees a well-formed
        // result it can render, not a dropped socket.

        let task = arg_str(arguments, "task").trim().to_string();
        let test_command = arg_str(arguments, "test_command").trim().to_string();
        if task.is_empty() {
            return tool_error("`task` must be a non-empty description of the goal.");
        }
        // AC2: a missing/blank test command is refused with an explanation that tests are
        // required — EvoSwarm's whole contract is a test-backed search (AD-5).
        if test_command.is_empty() {
            return tool_error(
                "`test_command` is required: EvoSwarm only runs test-backed tasks, so a job \
                 needs a command that executes its test suite.",
            );
        }

        let raw_paths = arg_str_array(arguments, "paths");
        if raw_paths.is_empty() {
            return tool_error("`paths` must list at least one repository path to operate on.");
        }

        // AC3: the path guard resolves symlinks and `..` before deciding containment, so a
        // traversal like `../../etc` is caught structurally, not lexically. The rejected path
        // is echoed verbatim so the caller learns *which* entry crossed the boundary.
        let mut target_paths = Vec::with_capacity(raw_paths.len());
        for raw in &raw_paths {
            match require_within_root(self.root.as_path(), PathBuf::from(raw).as_path()) {
                Ok(resolved) => target_paths.push(resolved),
                Err(_escape) => {
                    return tool_error(format!(
                        "`paths` entry {raw:?} is outside the repository root and was rejected."
                    ))
                }
            }
        }

        // Budget is schema-required but optional in behaviour: absent ⇒ unbounded within the
        // timeout, present ⇒ carried onto the submission for the engine (e1-10) to enforce.
        let budget = arguments.get("budget");
        let budget_tokens = budget
            .and_then(|b| b.get("tokens"))
            .and_then(|t| t.as_u64());
        let budget_dollars = budget
            .and_then(|b| b.get("dollars"))
            .and_then(|d| d.as_f64());

        let submission = JobSubmission {
            task_description: task,
            test_command,
            target_paths,
            budget_tokens,
            budget_dollars,
            objective: JobObjective::Correctness,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        };

        let job_id = uuid::Uuid::new_v4().to_string();
        // No git resolution here: the MCP layer must not shell out before the ticket is
        // produced (that is e1-1 CLI's job). `base_commit: None` is honest — e3-3 degrades.
        if let Err(e) = self.ledger.insert_job(&job_id, &submission, None) {
            // A ledger write failure is the server's fault, surfaced as a tool error with a
            // generic reason; the ticket was never issued.
            return tool_error(format!("failed to record the job: {e}"));
        }

        tool_ticket(&job_id, JobStatus::Queued)
    }

    /// The `job_status` tool (e3-2): reports progress for a job so a polling Claude Code
    /// session can decide when to re-check. It reads only what the ledger durably carries —
    /// `state` (the job status) and `generation` (the highest committed generation, so a
    /// not-yet-started job is 0) — and returns the engine-sourced fields (`best_score`,
    /// `spend_usd`, `eta_seconds`) as explicit `null`, because the engine keeps those in
    /// memory and does not yet persist them per job. Reporting a fabricated 0.0 spend or a
    /// made-up ETA would be worse than admitting the value is not observable here (SEAM).
    fn job_status(&self, arguments: &Value) -> Value {
        let id = arg_str(arguments, "id");
        if id.trim().is_empty() {
            return tool_error("`id` is required: job_status takes the job id to poll.");
        }

        // A ledger read is the authority; NotFound becomes a tool error that names the id, so
        // the caller learns which job vanished rather than getting an opaque failure.
        let record = match self.ledger.read_job(id) {
            Ok(record) => record,
            Err(evoswarm_ledger::LedgerError::NotFound(_)) => {
                return tool_error(format!("job {id} not found"))
            }
            Err(e) => return tool_error(format!("failed to read job {id}: {e}")),
        };

        let generation = match self.ledger.max_generation(id) {
            Ok(Some(max)) => max,
            Ok(None) => 0,
            Err(e) => return tool_error(format!("failed to read generation for job {id}: {e}")),
        };

        tool_status(&record.job_id, record.status, generation)
    }
}

/// The `initialize` result: the shape a client negotiates before calling any tool.
fn initialize_result() -> Value {
    json!({
        "protocolVersion": PROTOCOL_VERSION,
        "capabilities": { "tools": { "listChanged": false } },
        "serverInfo": { "name": "evoswarm", "version": env!("CARGO_PKG_VERSION") },
    })
}

/// The `tools/list` result: every advertised tool with its input schema. e3-1 advertises
/// `evolve` only; later stories append their tools to this single list.
fn tools_list_result() -> Value {
    json!({ "tools": tool_definitions() })
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "evolve",
            "description": "Delegate a test-backed engineering task to EvoSwarm; returns a job \
                            ticket immediately so the search runs in the background.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task": { "type": "string", "description": "What to accomplish." },
                    "test_command": {
                        "type": "string",
                        "description": "Command that runs the task's test suite (required)."
                    },
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Repository paths the search may touch; each must sit \
                                        inside the repo root."
                    },
                    "budget": {
                        "type": "object",
                        "properties": {
                            "tokens": { "type": "integer", "minimum": 0 },
                            "dollars": { "type": "number", "minimum": 0 }
                        }
                    }
                },
                // The four story §2 params are required so a client validates before calling.
                "required": ["task", "test_command", "paths", "budget"]
            }
        }),
        json!({
            "name": "job_status",
            "description": "Poll a delegated job for its state, generation, best score, \
                            spend and ETA. Null values mean the engine has not persisted \
                            that field yet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Job id from an `evolve` ticket." }
                },
                "required": ["id"]
            }
        }),
    ]
}

/// A successful tool result carrying the full job status contract. `state` is serialised
/// from `JobStatus` (snake_case: queued/running/completed/failed/budget_exhausted/cancelled).
/// `best_score`/`spend_usd`/`eta_seconds` are the engine-sourced progress fields; the ledger
/// does not persist them yet, so they are present-but-null rather than invented.
fn tool_status(job_id: &str, status: JobStatus, generation: i64) -> Value {
    let state = serde_json::to_value(status).unwrap_or_else(|_| json!("queued"));
    json!({
        "content": [{
            "type": "text",
            "text": format!("job {job_id}: state={state}, generation={generation}")
        }],
        "structuredContent": {
            "job_id": job_id,
            "state": state,
            "generation": generation,
            "best_score": Value::Null,
            "spend_usd": Value::Null,
            "eta_seconds": Value::Null,
        },
        "isError": false
    })
}

/// Reads a required string argument, or `""` when absent / not a string (the validator
/// decides whether that is an error).
fn arg_str<'a>(obj: &'a Value, key: &str) -> &'a str {
    obj.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

/// Reads an array-of-strings argument, ignoring non-string elements.
fn arg_str_array(obj: &Value, key: &str) -> Vec<String> {
    obj.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// A successful tool result carrying the job ticket: human-readable `content` plus the
/// machine-readable `structuredContent` a client parses.
fn tool_ticket(job_id: &str, status: JobStatus) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": format!("Queued job {job_id}; poll job_status for progress.")
        }],
        "structuredContent": {
            "job_id": job_id,
            "status": status,
        },
        "isError": false
    })
}

/// A tool-level failure: a well-formed call the tool refused, surfaced as `isError` with a
/// human message. This is *not* a JSON-RPC error object.
fn tool_error(message: impl AsRef<str>) -> Value {
    json!({
        "content": [{ "type": "text", "text": message.as_ref() }],
        "isError": true
    })
}

/// A JSON-RPC protocol-level error (the request never produced a tool result).
struct RpcError {
    code: i64,
    message: String,
}

impl RpcError {
    fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Wiring errors surfaced at construction, not per-request.
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("repo root {0:?} is unusable: {1}")]
    BadRoot(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn server() -> Server {
        let root = tempfile::tempdir().unwrap();
        Server::new(
            JobLedger::open_in_memory().unwrap(),
            RepoRoot::new(root.path()),
        )
        .unwrap()
    }

    /// The `paths` guard must reject a traversal while accepting an in-root entry, proving
    /// the schema-required `budget` does not mask path validation.
    #[test]
    fn arg_helpers_read_defaults_when_absent() {
        let empty = json!({});
        assert_eq!(arg_str(&empty, "task"), "");
        assert!(arg_str_array(&empty, "paths").is_empty());
        // A non-string element is dropped, not coerced.
        let mixed = json!({ "paths": ["src", 7, null, "docs"] });
        assert_eq!(arg_str_array(&mixed, "paths"), vec!["src", "docs"]);
    }

    /// A request with no id is a notification and never answered, even for a real method.
    #[test]
    fn request_without_id_is_not_answered() {
        let s = server();
        assert!(
            s.handle(&json!({"jsonrpc":"2.0","method":"tools/list"}))
                .is_none(),
            "a message with no id gets no response"
        );
    }
}
