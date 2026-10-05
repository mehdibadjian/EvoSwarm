# Implementation Plan: Benchmark suite

**Story:** `e1-13-benchmark-suite` · **Sizing:** L · **Tier:** `pro` · **Target crate:** `bench (Python harness)`
**Depends on:** `e1-11-patch-and-report` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

## 1. Pre-Flight Architecture & Rule Check
Complete these before writing code; tick each only when it actually holds.

- [ ] Reviewed [`ARCHITECTURE-SPINE.md`](../../architecture/ARCHITECTURE-SPINE.md) and confirmed this story upholds its invariants: Mandatory Exit Gate (Gate 1).
- [ ] Confirmed compliance with [`architecture-rules.md`](../../../.agents/rules/architecture-rules.md), [`tdd-discipline.md`](../../../.agents/rules/tdd-discipline.md) and [`security-hygiene.md`](../../../.agents/rules/security-hygiene.md).
- [ ] Confirmed every dependency listed above is `done`; otherwise stop and report the blocker.
- [ ] Confirmed scope matches [`spec.md`](spec.md) section 2 and nothing outside it is being built.
- [ ] Committed to zero AI comments in production code: comments explain **WHY**, never **WHAT**.

---

## 2. Red Phase (Failing Tests to Author First)
Author these tests before any production code, then run the gate command in section 5 and
confirm every test fails for its intended reason. A test that passes before implementation
is a false-positive test and must be rewritten.

- `bench/tests/test_e1_13_benchmark_suite.py::test_benchmark_task_integrity` — Validates all 30 tasks have trusted tests, a held-out split and a failing baseline, with at least 10 Python and 10 C#.
- `bench/tests/test_e1_13_benchmark_suite.py::test_token_budget_equality` — Verifies both runners terminate at identical token budgets and share model and sandbox configuration.
- `bench/tests/test_e1_13_benchmark_suite.py::test_solve_rate_delta_calculation` — Validates delta calculation from known solve counts and the gate assertion logic at the 15% boundary.
- `bench/tests/test_e1_13_benchmark_suite.py::test_gate_failure_below_threshold` — Feeds a delta below 15% and asserts the harness reports failure with the delta printed.

**Target file(s):** `bench/tests/test_e1_13_benchmark_suite.py`
All tests for this story live in the single pytest module above. It does not exist yet: author it in the Red phase before any production code.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | it contains 30 tasks, at least 10 Python and 10 C#, each with trusted tests and a held-out split | `test_benchmark_task_integrity` |
| AC2 | EvoSwarm and single-shot-with-feedback execute at an equal token budget | `test_token_budget_equality` |
| AC3 | they show solve rate, tokens per solved task, and wall time | `test_solve_rate_delta_calculation` |
| AC4 | the gate is reported as failed with the delta shown | `test_gate_failure_below_threshold` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `bench/tasks/<task-id>/` — 30 task directories, each with `task.md`, `baseline/`, `tests/trusted/` and `tests/held_out/`.
- `bench/harness.py` — suite loader validating task integrity and the language mix.
- `bench/run_single_shot.py` — single-shot-with-feedback runner honouring the shared token cap.
- `bench/run_evoswarm.py` — evolutionary runner invoking the `evoswarm` CLI.
- `bench/compare.py` — delta computation, gate assertion, and JSON plus console reporting.

### Work order
1. Author the loader first and let `test_benchmark_task_integrity` drive the required task directory layout.
2. Extract the shared budget, model and sandbox configuration into one module both runners import, so equality is structural rather than a runtime assertion.
3. Implement the single-shot runner as one draft plus sequential retries that stop at the token cap, reusing the e1-8 leak scan.
4. Implement `compare.py` to count a solve only on a 100% held-out pass, compute $\Delta$, and assert $\Delta \ge 15.0$.
5. Emit raw per-task JSON before printing the summary so a failed gate remains auditable.
6. Seed at least 10 Python and 10 C# tasks; task authoring is the bulk of this story's size.

### Implementation note
This is the Gate 1 decision point for the whole project: if the delta does not reach 15 points, the epic exit gate halts swarm search in favour of a sandboxed test runner.

---

## 4. Refactor Phase
- Remove redundant work and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the whole benchmark test suite, not just this story's module, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "python3 -m pytest bench/tests/test_e1_13_benchmark_suite.py -v" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-13-benchmark-suite --status done
```
