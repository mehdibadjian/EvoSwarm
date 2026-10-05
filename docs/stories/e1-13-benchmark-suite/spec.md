# Story Specification: Benchmark suite

## 1. Functional Specification
### 1. Benchmark Harness Contract
- 30 tasks with pre-verified failing baselines and human-verified test suites.
- Runner executes both methods at equal token caps ($T = 500,000$ tokens per task):
  - **Single-shot with feedback:** 1 draft + sequential retries up to token budget.
  - **EvoSwarm:** Population evolutionary loop.
- Metric: $\Delta = 	ext{SolveRate}_{	ext{EvoSwarm}} - 	ext{SolveRate}_{	ext{SingleShot}}$.
- Exit gate requires: $\Delta \ge 15.0\%$.


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given the benchmark suite**, **when loaded**, **then it contains 30 tasks, at least 10 Python and 10 C#, each with trusted tests and a held-out split.**.
- **Given the comparison script**, **when run**, **then EvoSwarm and single-shot-with-feedback execute at equal token budget.**.
- **Given benchmark execution completes**, **when results print**, **then they show solve rate, tokens per solved task, and wall time.**.
