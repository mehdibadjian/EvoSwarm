# Story: Held-out tests

## Metadata
- **Story Key:** `e1-8-held-out-tests` (Short: `e1-8`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Reviewer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e1-6-hard-gates`

---

## 1. User Story
As a reviewer, I want the winner checked on tests the models never saw, so that I know it generalises.

---

## 2. Architectural Context & Invariants
Governed by AD-5. Splits trusted user test suite into 80% visible and 20% held-out. Held-out tests are never provided in mutation prompts. The top-scoring candidate must pass 100% of held-out tests.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Full test suite` | `(VisibleTests, HeldOutTests)` | `HeldOutTests empty if suite has fewer than 5 tests` |
| `Top candidate + HeldOutTests` | `Verified winner or fallback to next candidate` | `Status: 'no verified winner' if all fail` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a job starts**, **when tests are split**, **then about 20% are held out using a split stable for that job.**.
- **Given any model call**, **when the prompt log is scanned**, **then no held-out test content appears in it.**.
- **Given the top candidate fails held-out tests**, **when selecting a winner**, **then the next candidate is tried.**.
- **Given no candidate passes held-out tests**, **when the job ends**, **then it reports `no verified winner` and flags the best-effort patch.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/fitness/test_held_out.rs::test_stable_test_split`](file:///workspace/calm-faraday/tests/fitness/test_held_out.rs): Verifies deterministic 80/20 partition keyed by job ID.
- [`tests/fitness/test_held_out.rs::test_prompt_leakage_audit`](file:///workspace/calm-faraday/tests/fitness/test_held_out.rs): Scans generated prompts to assert zero held-out test text.
- [`tests/fitness/test_held_out.rs::test_runner_fallback_on_held_out_failure`](file:///workspace/calm-faraday/tests/fitness/test_held_out.rs): Forces rank 1 candidate to fail held-out tests and asserts rank 2 chosen.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_8_held_out_tests" --anti-cheat
```
