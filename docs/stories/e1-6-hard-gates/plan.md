# Implementation Plan: Hard gates

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/fitness/test_hard_gates.rs::test_build_error_gate`](file:///workspace/calm-faraday/tests/fitness/test_hard_gates.rs): Simulates compiler failure; asserts score 0 and reason 'build'.
- [`tests/fitness/test_hard_gates.rs::test_assertion_failure_gate`](file:///workspace/calm-faraday/tests/fitness/test_hard_gates.rs): Simulates test assertion failure; asserts score 0 and list of failed tests.
- [`tests/fitness/test_hard_gates.rs::test_harness_tamper_gate`](file:///workspace/calm-faraday/tests/fitness/test_hard_gates.rs): Injects conftest.py in patch; asserts score 0 and reason 'tamper'.
- [`tests/fitness/test_hard_gates.rs::test_skipped_test_detection`](file:///workspace/calm-faraday/tests/fitness/test_hard_gates.rs): Skips one test in runner; asserts score 0 and reason 'skipped'.

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
python3 scripts/sprint.py verify --cmd "cargo test --test e1_6_hard_gates" --anti-cheat
```
