# Story: Hard gates

## Metadata
- **Story Key:** `e1-6-hard-gates` (Short: `e1-6`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Reviewer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e0-7-tamper-proof-tests`

---

## 1. User Story
As a reviewer, I want every candidate checked against hard gates, so that nothing returned to me has gamed the tests.

---

## 2. Architectural Context & Invariants
Governed by AD-5. Evaluates 4 sequential gates: (1) Builds with no new errors, (2) All visible trusted tests pass, (3) Diff touches no tests/harness/build scripts, (4) No tests skipped/deleted relative to baseline.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Candidate build & test result` | `GateResult(passed: bool, failure_reason: Option<String>)` | `Immediate score=0 upon any gate failure` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given the candidate fails to build**, **when gated**, **then its score is 0 with reason `build`.**.
- **Given any visible trusted test fails**, **when gated**, **then its score is 0 with the failing tests listed.**.
- **Given the diff touches tests, harness files, build scripts or CI config**, **when gated**, **then its score is 0 with reason `tamper`.**.
- **Given fewer tests ran than in the baseline, or new skips appear**, **when gated**, **then its score is 0 with reason `skipped`.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/fitness/test_hard_gates.rs::test_gate_build_failure`](file:///workspace/calm-faraday/tests/fitness/test_hard_gates.rs): Simulates syntax error and asserts reason 'build'.
- [`tests/fitness/test_hard_gates.rs::test_gate_test_failure`](file:///workspace/calm-faraday/tests/fitness/test_hard_gates.rs): Simulates test assertion failure and asserts failing tests listed.
- [`tests/fitness/test_hard_gates.rs::test_gate_tamper_failure`](file:///workspace/calm-faraday/tests/fitness/test_hard_gates.rs): Includes diff in `tests/` and asserts reason 'tamper'.
- [`tests/fitness/test_hard_gates.rs::test_gate_skipped_tests`](file:///workspace/calm-faraday/tests/fitness/test_hard_gates.rs): Decrements total run count and asserts reason 'skipped'.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_6_hard_gates" --anti-cheat
```
