# Story: job_status tool

## Metadata
- **Story Key:** `e3-2-job-status-tool` (Short: `e3-2`)
- **Epic:** [Claude Code Integration (MCP)](file:///workspace/calm-faraday/docs/epics/epic-3.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e3-1-evolve-tool`

---

## 1. User Story
As a developer, I want Claude Code to see job progress, so that it can report back and decide when to check again.

---

## 2. Architectural Context & Invariants
Exposes MCP tool `job_status(id)` returning state, current generation, best score, spend so far, and estimated completion time based on calibration data.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `job_id: String` | `{ state: 'running', generation: 2, best_score: 0.85, spend_usd: 0.42, eta_seconds: 90 }` | `NotFound error if invalid job ID` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a running job**, **when `job_status` is called**, **then it returns state, current generation, best score, spend so far and an ETA from calibrated timings.**.
- **Given an unknown job ID**, **when called**, **then it returns a not-found error.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/mcp/test_job_status.rs::test_status_query`](file:///workspace/calm-faraday/tests/mcp/test_job_status.rs): Queries running job and asserts JSON schema.
- [`tests/mcp/test_job_status.rs::test_status_not_found`](file:///workspace/calm-faraday/tests/mcp/test_job_status.rs): Queries nonexistent ID and checks error response.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e3_2_job_status_tool" --anti-cheat
```
