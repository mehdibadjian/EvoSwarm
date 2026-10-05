# Implementation Plan: Benchmark suite

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/benchmark/test_benchmark_harness.py::test_benchmark_task_integrity`](file:///workspace/calm-faraday/tests/benchmark/test_benchmark_harness.py): Validates all 30 benchmark tasks have valid tests and held-out slices.
- [`tests/benchmark/test_benchmark_harness.py::test_token_budget_equality`](file:///workspace/calm-faraday/tests/benchmark/test_benchmark_harness.py): Verifies both runners terminate at identical token budgets.
- [`tests/benchmark/test_benchmark_harness.py::test_solve_rate_delta_calculation`](file:///workspace/calm-faraday/tests/benchmark/test_benchmark_harness.py): Validates delta calculation and gate assertion logic.

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
python3 scripts/sprint.py verify --cmd "cargo test --test e1_13_benchmark_suite" --anti-cheat
```
