# Story: job_result tool

## Metadata
- **Story Key:** `e3-3-job-result-tool` (Short: `e3-3`)
- **Epic:** [Claude Code Integration (MCP)](file:///workspace/calm-faraday/docs/epics/epic-3.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e3-1-evolve-tool`, `e1-11-patch-and-report`

---

## 1. User Story
As a developer, I want Claude Code to fetch the finished result, so that it can apply and verify the patch for me.

---

## 2. Architectural Context & Invariants
Exposes MCP tool `job_result(id)` returning git branch, patch file path, score, test pass counts, and report path. Verified by end-to-end test where Claude Code applies the patch locally.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `job_id: String` | `{ branch: '...', patch_path: '...', score: 0.92, tests_passed: 18, report_path: '...' }` | `Status returned with no patch if job still running (unless partial=true)` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a completed job**, **when `job_result` is called**, **then it returns branch name, patch path, score, tests passed and report path.**.
- **Given an unfinished job**, **when called**, **then it returns the current state and no patch unless `partial` is set.**.
- **Given the end-to-end test**, **when Claude Code applies the patch and runs the tests locally**, **then they pass.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/mcp/test_job_result.rs::test_fetch_completed_result`](file:///workspace/calm-faraday/tests/mcp/test_job_result.rs): Fetches completed job result and checks file paths.
- [`tests/mcp/test_job_result.rs::test_local_patch_verification`](file:///workspace/calm-faraday/tests/mcp/test_job_result.rs): Applies returned patch locally and runs test command.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e3_3_job_result_tool" --anti-cheat
```
