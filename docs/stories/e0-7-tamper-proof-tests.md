# Story: Tamper-proof tests

## Metadata
- **Story Key:** `e0-7-tamper-proof-tests` (Short: `e0-7`)
- **Epic:** [The Crucible (Sandbox)](file:///workspace/calm-faraday/docs/epics/epic-0.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** S
- **Execution Tier:** `pro`
- **Dependencies:** `e0-1-sandbox-runner-interface`

---

## 1. User Story
As a developer, I want tests mounted read-only and harness overrides detected, so that a candidate can never pass by changing how tests run.

---

## 2. Architectural Context & Invariants
Governed by AD-1 and AD-5. Candidate diffs must never touch `/tests`. Harness configuration files (`conftest.py`, `Directory.Build.props`, `pytest.ini`) are prohibited in the candidate patch.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Candidate patch attempting to write to /work/tests` | `Write error (EROFS: Read-only file system)` | `Tampering detected, score 0` |
| `Candidate patch adding conftest.py in /work root` | `Gate check failure: Status 'tamper'` | `Score 0, flagged tampering` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a candidate writes into the tests directory**, **when it runs**, **then the write fails and the result is unchanged.**.
- **Given a candidate adds a harness override file such as `conftest.py` or `Directory.Build.props` in the work root**, **when gates run**, **then it is flagged as tampering.**.
- **Given any run**, **when it ends**, **then the tests directory hash matches the hash taken before it started.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/test_tamper_detection.rs::test_readonly_test_mount`](file:///workspace/calm-faraday/tests/test_tamper_detection.rs): Candidate writes to `tests/test_foo.py` and asserts EROFS.
- [`tests/test_tamper_detection.rs::test_harness_override_rejection`](file:///workspace/calm-faraday/tests/test_tamper_detection.rs): Validates diff rejector flags `conftest.py` in patch root.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e0_7_tamper_proof_tests" --anti-cheat
```
