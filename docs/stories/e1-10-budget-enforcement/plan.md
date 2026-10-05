# Implementation Plan: Budget enforcement

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/engine/test_budget_enforcement.rs::test_pre_call_cost_check_halt`](file:///workspace/calm-faraday/tests/engine/test_budget_enforcement.rs): Asserts loop halts before exceeding dollar cap.
- [`tests/engine/test_budget_enforcement.rs::test_token_cap_exhaustion`](file:///workspace/calm-faraday/tests/engine/test_budget_enforcement.rs): Asserts status 'budget_exhausted' when token limit reached.
- [`tests/engine/test_budget_enforcement.rs::test_early_stop_score_plateau`](file:///workspace/calm-faraday/tests/engine/test_budget_enforcement.rs): Simulates 2 generations with identical score and verifies early stop.

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
python3 scripts/sprint.py verify --cmd "cargo test --test e1_10_budget_enforcement" --anti-cheat
```
