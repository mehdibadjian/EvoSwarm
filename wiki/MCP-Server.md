# MCP Server

`evoswarm-mcp` is the JSON-RPC 2.0 server that lets Claude Code delegate evolutionary jobs to EvoSwarm. All logic lives in `lib.rs` (~628 lines).

## The four tools

`tool_definitions()` advertises exactly these, each with `required: ["id"]`:

| Tool | Purpose | Story |
|---|---|---|
| `evolve` | Enqueue a job (task + test command + paths + budgets + objective + timeout) → returns a `Queued` ticket | `e3-1` review (SEAM) |
| `job_status` | Read the job's state and generation from the ledger | `e3-2` review (SEAM) |
| `job_result` | Fetch the finished job's branch / patch path / report path, plus existence flags | `e3-3` review (SEAM) |
| `cancel_job` | Transition `Queued`/`Running` → `Cancelled` (idempotent re-cancel; refuses terminal states) | `e3-4` review (SEAM) |

## The two-channel error model

MCP errors are split by *what went wrong*, which is deliberate:

1. **Protocol faults** → a JSON-RPC `error` object with a standard code:
   - `-32601` — **method-not-found** (unknown tool name).
   - `-32602` — **invalid-params** (malformed/missing parameters).
2. **Tool refusals on a well-formed call** → a *successful* JSON-RPC response whose result carries `isError: true` plus a human-readable `content` string. This covers things like "id required," "job not found," and "this job is already `<state>` and cannot be cancelled." `tool_error` is the helper for these.

**Notifications** (JSON-RPC messages with no `id`) correctly receive **no response** — they aren't requests.

## What each tool returns (and why some fields are null)

- **`job_status`** returns `state` and `generation` (`max_generation`) from the ledger. `best_score`, `spend_usd`, and `eta_seconds` come back as explicit **`null`** because the engine keeps those in memory and never persists a per-job progress row.
- **`job_result`** wires `branch` (`evoswarm/<id>`), `patch_path` (`.evoswarm/patches/<id>.patch`), and `report_path` (`.evoswarm/reports/<id>.md`) to the real CLI artifact layout, and reports whether each **exists on the real filesystem** under a canonicalised repo root. Artifacts are only surfaced for a `Completed` job (or when `partial: true` peeks mid-run). `score` and `tests_passed` are **`null`** because the engine renders them into the markdown report only, never as per-job persisted data.
- **`cancel_job`** returns `structuredContent` `{ job_id, status: "cancelled", best_verified_candidate: null }`. `best_verified_candidate` is `null` because the engine computes the winner in memory and persists only opaque population JSON. Idempotent re-cancel returns the identical payload with `isError: false`; cancelling an already-terminal job is refused **without mutating it**.

## Why these sit at SEAM

Each tool's JSON-RPC layer, schema, ledger reads/writes, and filesystem wiring are verified **in-process** with the real ledger and filesystem, and the tests prove non-vacuity via mutation. The deferred half is the **live Claude Code stdio end-to-end run** (and the in-memory score/spend fields), which needs a running daemon plus LLM provider keys. That distinction is recorded as a `# SEAM:` note per story. See [Sprint Workflow](Sprint-Workflow.md) and [Roadmap & Status](Roadmap-and-Status.md).
