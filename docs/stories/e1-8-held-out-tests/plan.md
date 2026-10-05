# Implementation Plan: Held-out tests

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/fitness/test_held_out.rs::test_stable_partitioning_ratio`](file:///workspace/calm-faraday/tests/fitness/test_held_out.rs): Validates 80/20 partition across test suites of sizes 5, 20, 100.
- [`tests/fitness/test_held_out.rs::test_zero_held_out_prompt_leakage`](file:///workspace/calm-faraday/tests/fitness/test_held_out.rs): Scans prompt history to verify zero held-out test names or assertions.
- [`tests/fitness/test_held_out.rs::test_candidate_fallback_ladder`](file:///workspace/calm-faraday/tests/fitness/test_held_out.rs): Forces candidate 1 to fail held-out tests; verifies candidate 2 chosen.

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
python3 scripts/sprint.py verify --cmd "cargo test --test e1_8_held_out_tests" --anti-cheat
```
