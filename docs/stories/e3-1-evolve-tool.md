# Story: evolve tool

## Metadata
- **Story Key:** `e3-1-evolve-tool` (Short: `e3-1`)
- **Epic:** [Claude Code Integration (MCP)](file:///workspace/calm-faraday/docs/epics/epic-3.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** `e1-1-start-a-job-from-the-cli`

---

## 1. User Story
As a developer, I want Claude Code to delegate a test-backed task to EvoSwarm, so that long searches run in the background while my session continues.

---

## 2. Architectural Context & Invariants
Governed by AD-2. Exposes MCP tool `evolve(task, test_command, paths, budget)` over stdio JSON-RPC. Returns job ID within 2 seconds. Rejects missing tests or paths outside repo root.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `MCP params: task, test_command, paths, budget` | `JSON-RPC response: { job_id: '...', status: 'queued' }` | `Error if test_command empty or path outside root` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a valid call with task, test command, paths and budget**, **when Claude Code invokes `evolve`**, **then a job ID returns within 2 s.**.
- **Given no test command**, **when invoked**, **then the tool returns an error explaining that a test suite is required.**.
- **Given a path outside the repo root**, **when invoked**, **then the call is rejected and the path named.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/mcp/test_evolve_tool.rs::test_evolve_submission_fast_return`](file:///workspace/calm-faraday/tests/mcp/test_evolve_tool.rs): Asserts job created and ticket returned in <2s.
- [`tests/mcp/test_evolve_tool.rs::test_evolve_requires_test_command`](file:///workspace/calm-faraday/tests/mcp/test_evolve_tool.rs): Asserts validation error if test command omitted.
- [`tests/mcp/test_evolve_tool.rs::test_evolve_path_traversal_rejection`](file:///workspace/calm-faraday/tests/mcp/test_evolve_tool.rs): Rejects path pointing to `../../etc`.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e3_1_evolve_tool" --anti-cheat
```
