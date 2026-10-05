# Implementation Plan: Held-out tests

**Story:** `e1-8-held-out-tests` · **Sizing:** M · **Tier:** `pro` · **Target crate:** `evoswarm-fitness`
**Depends on:** `e1-6-hard-gates` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

## 1. Pre-Flight Architecture & Rule Check
Complete these before writing code; tick each only when it actually holds.

- [ ] Reviewed [`ARCHITECTURE-SPINE.md`](../../architecture/ARCHITECTURE-SPINE.md) and confirmed this story upholds its invariants: Governed by AD-5.
- [ ] Confirmed compliance with [`architecture-rules.md`](../../../.agents/rules/architecture-rules.md), [`tdd-discipline.md`](../../../.agents/rules/tdd-discipline.md) and [`security-hygiene.md`](../../../.agents/rules/security-hygiene.md).
- [ ] Confirmed every dependency listed above is `done`; otherwise stop and report the blocker.
- [ ] Confirmed scope matches the contract matrix and acceptance criteria in [`spec.md`](spec.md) and nothing outside them is being built.
- [ ] Committed to zero AI comments in production code: comments explain **WHY**, never **WHAT**.

---

## 2. Red Phase (Failing Tests to Author First)
Author these tests before any production code, then run the gate command in section 5 and
confirm every test fails for its intended reason. A test that passes before implementation
is a false-positive test and must be rewritten.

- `crates/evoswarm-fitness/tests/e1_8_held_out_tests.rs::test_stable_partitioning_ratio` — Validates the 80/20 partition and its stability across suites of size 5, 20 and 100.
- `crates/evoswarm-fitness/tests/e1_8_held_out_tests.rs::test_zero_held_out_prompt_leakage` — Scans assembled prompt payloads and asserts zero held-out test names or assertion text.
- `crates/evoswarm-fitness/tests/e1_8_held_out_tests.rs::test_candidate_fallback_ladder` — Forces candidate 1 to fail held-out tests and verifies candidate 2 is selected.
- `crates/evoswarm-fitness/tests/e1_8_held_out_tests.rs::test_no_verified_winner_status` — Fails held-out tests for every candidate and asserts status `no verified winner` with the best-effort patch flagged.
- `crates/evoswarm-fitness/tests/e1_8_held_out_tests.rs::test_small_suite_yields_empty_holdout` — Partitions a 3-test suite and asserts $H = 0$ with the reason recorded.

**Target file(s):** `crates/evoswarm-fitness/tests/e1_8_held_out_tests.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | about 20% are held out using a stable split keyed by job ID | `test_stable_partitioning_ratio` |
| AC2 | no held-out test content appears in any prompt | `test_zero_held_out_prompt_leakage` |
| AC3 | the runner falls back to evaluate the next candidate | `test_candidate_fallback_ladder` |
| AC4 | it reports `no verified winner` and flags the best-effort patch | `test_no_verified_winner_status` |
| AC5 | the held-out set is empty and the reason is recorded | `test_small_suite_yields_empty_holdout` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-fitness/src/holdout.rs` — `pub fn partition(job_id: &str, trusted: &[String]) -> TestSplit` returning `{ visible, held_out }`.
- `crates/evoswarm-fitness/src/holdout.rs` — `pub fn assignment_hash(job_id: &str, test_name: &str) -> [u8; 32]`.
- `crates/evoswarm-fitness/src/leak_scan.rs` — `pub fn assert_no_leakage(prompt: &[u8], held_out: &[String]) -> Result<(), LeakDetected>`.
- `crates/evoswarm-engine/src/selection.rs` — `pub async fn select_verified_winner(ranked: &[Candidate], split: &TestSplit, deps: &SelectionDeps) -> SelectionOutcome`.
- `crates/evoswarm-core/src/outcome.rs` — `SelectionOutcome::{Verified(Candidate), NoVerifiedWinner { best_effort: Candidate } }`.

### Work order
1. Compute `assignment_hash` per trusted test name, sort ascending, take the lowest $H$ as held-out.
2. Apply the $M < 5$ rule before hashing so a tiny suite never loses a test.
3. Persist `TestSplit` with the job row so resume (e1-12) reuses the identical partition instead of recomputing from a possibly changed suite.
4. Call `assert_no_leakage` at the single dispatch chokepoint in `evoswarm-models`, covering every role.
5. Walk the ranked candidate list, running held-out tests in the sandbox per candidate, stopping at the first 100% pass.
6. Emit `NoVerifiedWinner` with the highest-scoring gated candidate flagged best-effort when the list is exhausted.

### Implementation note
Leak scanning belongs at the dispatch chokepoint, not in each prompt builder: one unwired builder is enough to invalidate the entire held-out guarantee.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-fitness` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-fitness --test e1_8_held_out_tests" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-8-held-out-tests --status done
```
