# Architecture

EvoSwarm is organised as a set of cooperating subsystems wired together by a Cargo workspace of 10 crates. This page maps the runtime shape and links each part to its crate and its wiki page.

## Component map

```
Developer ──► Frontend adapters ──► Persistent queue/memory ──► Search & execution engine ──► Model providers
              (CLI, MCP, Gateway)     (SQLite ledger, blob store,   (orchestrator, sandbox,      (Claude/upstream)
                                       FalkorDB graph)               fitness, memory)
```

### Frontends
- **CLI** (`evoswarm-cli`) — `evoswarm run` submits a job; `evoswarm usage` prints gateway spend. See [CLI](CLI.md).
- **MCP server** (`evoswarm-mcp`) — JSON-RPC 2.0 exposing `evolve`, `job_status`, `job_result`, `cancel_job` to Claude Code. See [MCP Server](MCP-Server.md).
- **Gateway** (`evoswarm-gateway`) — optional transparent reverse proxy for token/dollar auditing and exemplar injection. See [Gateway](Gateway.md).

### State & memory
- **Ledger / job queue** (`evoswarm-ledger`) — durable job records backed by SQLite (`rusqlite`, bundled). Tracks the job lifecycle and generations. See [Engine Search Loop](Engine-Search-Loop.md).
- **Core domain types** (`evoswarm-core`) — `JobStatus`, `JobSubmission`, `JobObjective`, execution outcomes, provenance, path guarding, report model. Shared by every crate.
- **Memory primitives** (`evoswarm-memory`) — content-addressed blob store and repo/toolchain fingerprints for replay. See [Memory & Fingerprints](Memory-and-Fingerprints.md).

### Execution
- **The Crucible** (`evoswarm-sandbox`) — sealed `bwrap` namespaces, cgroups v2 limits, seccomp filters, and per-language stacks (Python, Java, C/C++). See [The Crucible](The-Crucible.md).
- **Fitness** (`evoswarm-fitness`) — the four hard gates and the multi-objective score. See [Fitness & Scoring](Fitness-and-Scoring.md).
- **Engine** (`evoswarm-engine`) — seeding, mutation with error feedback, crossover, adversary, budget enforcement, crash recovery, early stop, selection, MAP-Elites. See [Engine Search Loop](Engine-Search-Loop.md).
- **Models** (`evoswarm-models`) — model-role config, prompt construction, context guard, hot reload, idempotency, lineage. This is where provider calls are shaped.

## The three operating modes

| Mode | Trigger | Latency target | Status in code |
|---|---|---|---|
| **Evolve job** | `evolve` MCP tool or `evoswarm run` | Minutes (budgeted) | CLI submit + MCP `evolve` + ledger queue are wired; the multi-generation loop runs against injected `ModelClient`/`SandboxBackend` seams (live provider keys required). |
| **Replay** | Matching spec + repo + toolchain fingerprint | Seconds, 0 model spend | Fingerprint computation is implemented and tested (`memory::fingerprints`); graph-backed replay is Epic 2 (`e2-4`), still backlog. |
| **Gateway** | `ANTHROPIC_BASE_URL` points at EvoSwarm | +< 50 ms TTFT | Transparent pass-through, usage tee, injection opt-out, and TTFT budget are implemented and tested against stub upstreams; live exemplar retrieval (`e4-3`) is backlog. |

## Data flow for one evolve job

1. Client calls `evoswarm run` (CLI) or the `evolve` MCP tool. Submission carries a task description, a sandbox `test_command`, an allowed `--paths` list, budgets, objective, and timeout.
2. The submission is validated (paths must resolve inside the repo root, else exit `PathEscape`) and a durable `Queued` ticket is written to the ledger.
3. The orchestrator takes the job: captures a **baseline** snapshot (test count, names, skips), then runs the search loop.
4. Each candidate: produced by mutation/crossover via model roles → executed in the Crucible → judged by the **gates** → scored by the weighted objective if it passes all gates.
5. The winner is materialised as a git branch `evoswarm/<job-id>`, a `.evoswarm/patches/<id>.patch`, and a `.evoswarm/reports/<id>.md`.
6. Clients poll `job_status` and fetch results via `job_result`; a run can be stopped via `cancel_job` (`Queued`/`Running` → `Cancelled`).

> Evidence note: steps 1–2 and 6 are verified in-process with the real filesystem and ledger. Steps 3–5 depend on a live `ModelClient` (LLM keys) and, for non-Python stacks, real toolchains; those halves are tracked as SEAM stories on the [Roadmap & Status](Roadmap-and-Status.md) page.

## Cross-cutting invariants

- **Gate-first:** a candidate that fails any hard gate scores 0 — performance and parsimony never rescue an unverified patch.
- **Test immutability:** tests and harness files are read-only in the sandbox; a diff touching them trips the tamper gate.
- **Deterministic fitness:** `fitness::gates::evaluate` is pure — identical inputs yield an identical `GateResult`, and no config can override a failure.
- **Budget-first search:** token and dollar budgets are enforced in the engine (`e1-10`, done) before any additional spend.

See [Crates Reference](Crates-Reference.md) for per-crate module detail and [Sprint Workflow](Sprint-Workflow.md) for how features are built and verified.
