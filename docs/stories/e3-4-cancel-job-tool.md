# Story: cancel_job tool

## Metadata
- **Story Key:** `e3-4-cancel-job-tool` (Short: `e3-4`)
- **Epic:** [Claude Code Integration (MCP)](../epics/epic-3-claude-code-integration-mcp.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e3-1-evolve-tool`

---

## 1. User Story
As a developer, I want to stop a job from Claude Code, so that spend stops when my plans change.

---

## 2. Architectural Context & Invariants
Exposes MCP tool `cancel_job(id)`. Halts further model dispatch immediately after in-flight calls conclude; marks job `cancelled` and preserves best verified candidate so far.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `job_id: String` | `{ status: 'cancelled', best_verified_candidate: Option<String> }` | `Idempotent if already cancelled` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a running job**, **when `cancel_job` is called**, **then no new model calls start after those in flight finish.**.
- **Given the job had a verified candidate**, **when cancelled**, **then that result is returned and the state is `cancelled`.**.
- **Given an already cancelled job**, **when called again**, **then it returns the same result with no error.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/mcp/test_cancel_job.rs::test_cancel_running_job`: Cancels job and verifies no subsequent generations run.
- `tests/mcp/test_cancel_job.rs::test_cancel_idempotency`: Calls cancel twice and expects identical response.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e3_4_cancel_job_tool" --anti-cheat
```
