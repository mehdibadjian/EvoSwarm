# Implementation Plan: Adversary tests

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/engine/test_adversary.rs::test_adversary_generation_quota`](file:///workspace/calm-faraday/tests/engine/test_adversary.rs): Asserts adversary generates up to K tests.
- [`tests/engine/test_adversary.rs::test_syntax_error_discard`](file:///workspace/calm-faraday/tests/engine/test_adversary.rs): Feeds uncompilable test and checks discard.
- [`tests/engine/test_adversary.rs::test_suspect_test_filtering`](file:///workspace/calm-faraday/tests/engine/test_adversary.rs): Simulates universally failing test and verifies suspect tag.

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
python3 scripts/sprint.py verify --cmd "cargo test --test e1_9_adversary_tests" --anti-cheat
```
