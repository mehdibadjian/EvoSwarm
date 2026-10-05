# Implementation Plan: Weighted score

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/fitness/test_scoring.rs::test_exact_weighted_score_math`](file:///workspace/calm-faraday/tests/fitness/test_scoring.rs): Validates exact floating point computation for known test vector.
- [`tests/fitness/test_scoring.rs::test_weight_validation_at_startup`](file:///workspace/calm-faraday/tests/fitness/test_scoring.rs): Rejects config with weights summing to 1.05.
- [`tests/fitness/test_scoring.rs::test_proportional_redistribution_without_adversary`](file:///workspace/calm-faraday/tests/fitness/test_scoring.rs): Checks score calculation when adversary tests = 0.
- [`tests/fitness/test_scoring.rs::test_deterministic_scoring`](file:///workspace/calm-faraday/tests/fitness/test_scoring.rs): Scores identical candidate 1,000 times and checks for bitwise identical float.

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
python3 scripts/sprint.py verify --cmd "cargo test --test e1_7_weighted_score" --anti-cheat
```
