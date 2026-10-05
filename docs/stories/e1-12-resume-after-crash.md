# Story: Resume after crash

## Metadata
- **Story Key:** `e1-12-resume-after-crash` (Short: `e1-12`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Operator
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e1-1-start-a-job-from-the-cli`

---

## 1. User Story
As an operator, I want jobs to survive a reboot, so that a restart of the 24/7 box does not waste spend.

---

## 2. Architectural Context & Invariants
Governed by AD-7. Commits state to SQLite job ledger. On reboot/crash, resumes from last completed generation. Uses idempotency hashes to avoid re-running completed model calls.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Abrupt SIGKILL during generation G` | `Daemon restarts: resumes generation G without repeating completed LLM calls` | `Corrupted state recovered via SQLite WAL` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given the process is killed mid-generation**, **when the service restarts**, **then the job resumes from the last completed generation.**.
- **Given a model call completed before the crash**, **when resuming**, **then it is not repeated, using a stored idempotency key.**.
- **Given sandbox runs were in flight**, **when resuming**, **then they are re-run.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/engine/test_recovery.rs::test_resume_mid_generation`](file:///workspace/calm-faraday/tests/engine/test_recovery.rs): Kills process during Gen 2; restarts and verifies Gen 2 completes without re-drafting.
- [`tests/engine/test_recovery.rs::test_idempotency_key_dedup`](file:///workspace/calm-faraday/tests/engine/test_recovery.rs): Verifies stored model responses reused on restart.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_12_resume_after_crash" --anti-cheat
```
