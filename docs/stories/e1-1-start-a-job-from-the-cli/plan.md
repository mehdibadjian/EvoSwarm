# Implementation Plan: Start a job from the CLI

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/cli/test_run.rs::test_submit_valid_job_fast_return`](file:///workspace/calm-faraday/tests/cli/test_run.rs): Submits valid job and verifies JSON ticket emitted in <2.0s with SQLite job state 'queued'.
- [`tests/cli/test_run.rs::test_reject_broken_baseline_command`](file:///workspace/calm-faraday/tests/cli/test_run.rs): Supplies invalid binary in test command and verifies immediate rejection with stderr diagnostic.
- [`tests/cli/test_run.rs::test_reject_flaky_baseline_suite`](file:///workspace/calm-faraday/tests/cli/test_run.rs): Mocks a non-deterministic test runner and asserts rejection with list of flaky tests.
- [`tests/cli/test_run.rs::test_reject_already_passing_suite_without_perf_flag`](file:///workspace/calm-faraday/tests/cli/test_run.rs): Supplies clean passing suite without --objective perf and verifies guidance error.

---

## 3. Green Phase (Minimal Production Code)
Implement the minimal logic in target crates/modules to satisfy tests:
- Define core structs and traits.
- Implement error handling and bounds checking.
- Connect persistence / CLI / sandbox dispatch.

---

## 4. Refactor Phase & Verification Gate
- Remove any redundant allocations or temporary scaffolding.
- Ensure all comments explain **WHY**, not **WHAT**.
- Execute verification gate:
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_1_start_a_job_from_the_cli" --anti-cheat
```
