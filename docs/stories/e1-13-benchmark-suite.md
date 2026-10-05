# Story: Benchmark suite

## Metadata
- **Story Key:** `e1-13-benchmark-suite` (Short: `e1-13`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Team Lead
- **Priority:** Must
- **Sizing:** L
- **Execution Tier:** `pro`
- **Dependencies:** `e1-11-patch-and-report`

---

## 1. User Story
As a team lead, I want a fixed benchmark, so that we decide on evidence whether evolution pays off.

---

## 2. Architectural Context & Invariants
Defines 30 tasks (at least 10 Python, 10 C#) with trusted tests and held-out slices. Automated comparison script runs EvoSwarm vs single-shot generation at equal token budget; reports solve rate delta.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Benchmark suite tasks` | `Comparative report: solve rate delta (target >=15%), tokens per solve, wall time` | `Exit gate fails if delta < 15%` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given the benchmark**, **when assembled**, **then it has 30 tasks, at least 10 Python and 10 C#, each with trusted tests and a held-out split.**.
- **Given the comparison script**, **when run**, **then EvoSwarm and single-shot-with-feedback run at equal token budget.**.
- **Given a run completes**, **when results print**, **then they show solve rate, tokens per solved task and wall time, stored for comparison across versions.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/benchmark/test_benchmark_runner.py::test_benchmark_loading`](file:///workspace/calm-faraday/tests/benchmark/test_benchmark_runner.py): Asserts 30 benchmark tasks valid.
- [`tests/benchmark/test_benchmark_runner.py::test_single_shot_comparison`](file:///workspace/calm-faraday/tests/benchmark/test_benchmark_runner.py): Executes single-shot harness at equal token cap.
- [`tests/benchmark/test_benchmark_runner.py::test_solve_rate_delta_calculation`](file:///workspace/calm-faraday/tests/benchmark/test_benchmark_runner.py): Computes and prints solve rate metric.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_13_benchmark_suite" --anti-cheat
```
