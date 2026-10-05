# Story Specification: Benchmark suite

## 1. Functional Specification
### 1. Benchmark Harness Contract
- 30 tasks with pre-verified failing baselines and human-verified test suites.
- Language mix: at least 10 Python and 10 C# tasks; the remainder may be either.
- Runner executes both methods at equal token caps ($T = 500{,}000$ tokens per task):
  - **Single-shot with feedback:** 1 draft plus sequential retries up to the token budget.
  - **EvoSwarm:** the population evolutionary loop.
- Metric: $\Delta = \text{SolveRate}_{\text{EvoSwarm}} - \text{SolveRate}_{\text{SingleShot}}$.
- Exit gate requires: $\Delta \ge 15.0\%$.

### 2. Fairness Invariants
- Identical token cap, identical model roles and pricing, identical sandbox profile, and
  identical held-out verification for both methods.
- A task counts as solved only when the held-out slice passes 100%, matching e1-8.
- Neither method may see held-out tests; the e1-8 leak scan applies to the single-shot
  runner as well.

### 3. Reporting
The report prints per-task solve status, aggregate solve rate for both methods, the delta,
tokens per solved task, and wall time. Raw per-task results are written to JSON so the
gate decision is auditable after the fact.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| 30 benchmark task directories | Loaded suite with trusted tests and held-out split per task | Load error naming any task missing tests or a split |
| Equal token cap $T = 500{,}000$ per task | Both runners terminate at the same budget | Run invalidated if caps diverge |
| Completed runs for both methods | Solve rate delta, tokens per solved task, wall time | Gate fails when $\Delta < 15.0\%$ |
| Single-shot-with-feedback runner | 1 draft plus sequential retries within budget | Retry loop must not exceed the cap |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given the benchmark suite**, **when loaded**, **then it contains 30 tasks, at least 10 Python and 10 C#, each with trusted tests and a held-out split**.
- **Given the comparison script**, **when run**, **then EvoSwarm and single-shot-with-feedback execute at an equal token budget**.
- **Given benchmark execution completes**, **when results print**, **then they show solve rate, tokens per solved task, and wall time**.
- **Given a delta below the exit gate**, **when results are evaluated**, **then the gate is reported as failed with the delta shown**.
