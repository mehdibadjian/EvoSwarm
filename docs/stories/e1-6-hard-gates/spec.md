# Story Specification: Hard gates

## 1. Functional Specification
### 1. Gate Definitions & Priority Order
Gates run in this order and short-circuit: the first failure is the reported reason.
1. **Gate 1 (Build):** Exit code of the build step $= 0$. Failure reason: `build`.
2. **Gate 2 (Trusted Pass):** Count of passed visible trusted tests $=$ total visible trusted tests. Failure reason: `test_failure`.
3. **Gate 3 (Tamper):** Diff touches only paths in `--paths`. Any touch of `tests/`, `conftest.py`, `Directory.Build.props`, or build files fails immediately with reason: `tamper`.
4. **Gate 4 (No Skips):** Count of executed tests $\ge$ baseline count; zero newly skipped or ignored tests. Failure reason: `skipped`.

Gate 3 is evaluated against the patch file list, not against filesystem state, so a
candidate cannot pass by reverting a tampered file after the run.

### 2. Failure Outcome
- Composite score $S = 0.0$.
- Candidate excluded from winning selection.
- Detailed failure reason logged for mutator feedback (consumed by e1-4).

### 3. Non-Negotiability
No configuration flag, model output or later stage may override a failed gate. Gate
evaluation is pure: identical inputs always yield an identical `GateResult`.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Candidate` patch + `SandboxProfile` | `GateResult { passed: bool, reason: Option<GateFailure> }` | First failing gate short-circuits the rest |
| Patch touching `tests/`, `conftest.py`, `Directory.Build.props` or build files | `GateResult { passed: false, reason: Tamper }` | Offending paths listed in the report |
| Build or trusted-test failure | Score forced to `0.0`, candidate excluded from selection | Failure reason fed back to the mutator via e1-4 |
| Fewer tests executed than baseline, or new skips | `GateResult { passed: false, reason: Skipped }` | Baseline and candidate counts reported |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given a candidate fails to compile**, **when evaluated**, **then its score is 0 and the reason is `build`**.
- **Given any visible trusted test fails**, **when evaluated**, **then its score is 0 and the failing test names are recorded**.
- **Given the diff touches tests, harness files or build scripts**, **when evaluated**, **then its score is 0 and the reason is `tamper`**.
- **Given fewer tests ran than in the baseline or new skips appear**, **when evaluated**, **then its score is 0 and the reason is `skipped`**.
- **Given a candidate that fails more than one gate at once**, **when evaluated**, **then the reported reason is the earliest gate in the defined order**.
