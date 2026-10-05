# Implementation Plan: Crossover of two parents

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/engine/test_crossover.rs::test_disjoint_test_pair_selection`](file:///workspace/calm-faraday/tests/engine/test_crossover.rs): Asserts selection logic prefers pairs with complementary test passes.
- [`tests/engine/test_crossover.rs::test_merged_from_dual_lineage`](file:///workspace/calm-faraday/tests/engine/test_crossover.rs): Verifies child connects to both parents with merged traits metadata.
- [`tests/engine/test_crossover.rs::test_crossover_call_cap_budget`](file:///workspace/calm-faraday/tests/engine/test_crossover.rs): Simulates generation dispatch and asserts crossover <= 25% of calls.

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
python3 scripts/sprint.py verify --cmd "cargo test --test e1_5_crossover_of_two_parents" --anti-cheat
```
