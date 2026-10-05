# Implementation Plan: Hard gates

**Story:** `e1-6-hard-gates` · **Sizing:** M · **Tier:** `pro` · **Target crate:** `evoswarm-fitness`
**Depends on:** `e0-7-tamper-proof-tests` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

## 1. Pre-Flight Architecture & Rule Check
Complete these before writing code; tick each only when it actually holds.

- [ ] Reviewed [`ARCHITECTURE-SPINE.md`](../../architecture/ARCHITECTURE-SPINE.md) and confirmed this story upholds its invariants: Governed by AD-5.
- [ ] Confirmed compliance with [`architecture-rules.md`](../../../.agents/rules/architecture-rules.md), [`tdd-discipline.md`](../../../.agents/rules/tdd-discipline.md) and [`security-hygiene.md`](../../../.agents/rules/security-hygiene.md).
- [ ] Confirmed every dependency listed above is `done`; otherwise stop and report the blocker.
- [ ] Confirmed scope matches [`spec.md`](spec.md) section 2 and nothing outside it is being built.
- [ ] Committed to zero AI comments in production code: comments explain **WHY**, never **WHAT**.

---

## 2. Red Phase (Failing Tests to Author First)
Author these tests before any production code, then run the gate command in section 5 and
confirm every test fails for its intended reason. A test that passes before implementation
is a false-positive test and must be rewritten.

- `crates/evoswarm-fitness/tests/e1_6_hard_gates.rs::test_build_error_gate` — Simulates a compiler failure and asserts `passed = false`, reason `build`, score 0.
- `crates/evoswarm-fitness/tests/e1_6_hard_gates.rs::test_assertion_failure_gate` — Simulates a test assertion failure and asserts reason `test_failure` with the failing test names listed.
- `crates/evoswarm-fitness/tests/e1_6_hard_gates.rs::test_harness_tamper_gate` — Injects a `conftest.py` change into the patch and asserts reason `tamper` naming the path.
- `crates/evoswarm-fitness/tests/e1_6_hard_gates.rs::test_skipped_test_detection` — Marks one test skipped relative to baseline and asserts reason `skipped` with both counts reported.
- `crates/evoswarm-fitness/tests/e1_6_hard_gates.rs::test_gate_short_circuit_order` — Fails build and tampers simultaneously and asserts the reported reason is `build`, proving gate order.

**Target file(s):** `crates/evoswarm-fitness/tests/e1_6_hard_gates.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | its score is 0 and the reason is `build` | `test_build_error_gate` |
| AC2 | its score is 0 and the failing test names are recorded | `test_assertion_failure_gate` |
| AC3 | its score is 0 and the reason is `tamper` | `test_harness_tamper_gate` |
| AC4 | its score is 0 and the reason is `skipped` | `test_skipped_test_detection` |
| AC5 | the reported reason is the earliest gate in the defined order | `test_gate_short_circuit_order` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-fitness/src/gates.rs` — `pub fn evaluate(candidate: &Candidate, run: &ExecutionResult, baseline: &Baseline) -> GateResult`.
- `crates/evoswarm-fitness/src/gates.rs` — `GateResult { passed: bool, reason: Option<GateFailure>, failed_tests: Vec<String> }` and `GateFailure::{Build, TestFailure, Tamper, Skipped}`.
- `crates/evoswarm-fitness/src/tamper.rs` — `pub fn detect(patch_paths: &[PathBuf], allowed: &[PathBuf]) -> Vec<PathBuf>` returning offending paths.
- `crates/evoswarm-fitness/src/baseline.rs` — `Baseline { test_count, skipped, trusted_names }` captured at job start.

### Work order
1. Capture `Baseline` once at job start through `SandboxBackend::run` and reuse it for every candidate.
2. Implement the four gates as separate private functions returning `Option<GateFailure>`, evaluated in order with early return.
3. Implement tamper detection over the patch's declared path list, matching `tests/`, `conftest.py`, `Directory.Build.props` and build files; compare canonicalised paths, never substrings, so `src/tests_helper.rs` is not a false positive.
4. Compare executed and skipped counts against `Baseline` for Gate 4.
5. Force `S = 0.0` and exclusion at the single call site so no later stage can resurrect a gated candidate.

### Implementation note
Substring matching on `tests/` is the classic false positive here: `mytests/` and `latests.rs` both match. Anchor on path components.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-fitness` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-fitness --test e1_6_hard_gates" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-6-hard-gates --status done
```
