# Story Specification: Hard gates

## 1. Functional Specification
### 1. Gate Definitions & Priority Order
1. **Gate 1 (Build):** Exit code of build step == 0. Failure reason: `build`.
2. **Gate 2 (Trusted Pass):** Count of passed visible trusted tests == Total visible trusted tests. Failure reason: `test_failure`.
3. **Gate 3 (Tamper):** Diff touches only paths in `--paths`. Any touch of `tests/`, `conftest.py`, `Directory.Build.props`, or build files fails immediately with reason: `tamper`.
4. **Gate 4 (No Skips):** Count of executed tests $\ge$ baseline count; zero newly skipped/ignored tests. Failure reason: `skipped`.

### 2. Failure Outcome
- Composite score $S = 0.0$.
- Candidate excluded from winning selection.
- Detailed failure reason logged for mutator feedback.


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given a candidate fails to compile**, **when evaluated**, **then its score is 0 and status is marked `build`.**.
- **Given any visible trusted test fails**, **when evaluated**, **then its score is 0 and failing test names are recorded.**.
- **Given the diff touches tests, harness files or build scripts**, **when evaluated**, **then its score is 0 and status is marked `tamper`.**.
- **Given fewer tests ran than in the baseline or new skips appear**, **when evaluated**, **then its score is 0 and status is marked `skipped`.**.
