# Implementation Plan: Crossover of two parents

**Story:** `e1-5-crossover-of-two-parents` · **Sizing:** M · **Tier:** `pro` · **Target crate:** `evoswarm-engine`
**Depends on:** `e1-4-mutation-with-error-feedback` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

## 1. Pre-Flight Architecture & Rule Check
Complete these before writing code; tick each only when it actually holds.

- [ ] Reviewed [`ARCHITECTURE-SPINE.md`](../../architecture/ARCHITECTURE-SPINE.md) and confirmed this story upholds its invariants: Governed by AD-6.
- [ ] Confirmed compliance with [`architecture-rules.md`](../../../.agents/rules/architecture-rules.md), [`tdd-discipline.md`](../../../.agents/rules/tdd-discipline.md) and [`security-hygiene.md`](../../../.agents/rules/security-hygiene.md).
- [ ] Confirmed every dependency listed above is `done`; otherwise stop and report the blocker.
- [ ] Confirmed scope matches the contract matrix and acceptance criteria in [`spec.md`](spec.md) and nothing outside them is being built.
- [ ] Committed to zero AI comments in production code: comments explain **WHY**, never **WHAT**.

---

## 2. Red Phase (Failing Tests to Author First)
Author these tests before any production code, then run the gate command in section 5 and
confirm every test fails for its intended reason. A test that passes before implementation
is a false-positive test and must be rewritten.

- `crates/evoswarm-engine/tests/e1_5_crossover_of_two_parents.rs::test_disjoint_test_pair_selection` — Asserts pairs are ranked by descending Hamming distance and the top disjoint pair is chosen.
- `crates/evoswarm-engine/tests/e1_5_crossover_of_two_parents.rs::test_no_disjoint_pair_falls_back_to_mutation` — Gives all candidates identical pass vectors and asserts zero crossover calls are scheduled.
- `crates/evoswarm-engine/tests/e1_5_crossover_of_two_parents.rs::test_merged_from_dual_lineage` — Verifies the child connects to both parents with merged traits metadata on each edge.
- `crates/evoswarm-engine/tests/e1_5_crossover_of_two_parents.rs::test_crossover_call_cap_budget` — Simulates generation dispatch and asserts crossover calls stay at or below 25% of total calls.

**Target file(s):** `crates/evoswarm-engine/tests/e1_5_crossover_of_two_parents.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | the pair with the greatest Hamming distance is prioritised | `test_disjoint_test_pair_selection` |
| AC2 | no crossover is scheduled and the generation falls back to mutation | `test_no_disjoint_pair_falls_back_to_mutation` |
| AC3 | it links to both parents with the traits reported by the synthesiser model | `test_merged_from_dual_lineage` |
| AC4 | crossover calls constitute at most 25% of total model calls | `test_crossover_call_cap_budget` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-engine/src/crossover.rs` — `pub fn rank_pairs(pop: &[Candidate]) -> Vec<(usize, usize, usize)>` returning `(a, b, distance)` sorted descending.
- `crates/evoswarm-engine/src/crossover.rs` — `pub async fn crossover(a: &Candidate, b: &Candidate, deps: &CrossoverDeps) -> Result<Candidate, CrossoverError>`.
- `crates/evoswarm-core/src/provenance.rs` — `PassVector` bitset keyed by trusted test name.
- `crates/evoswarm-engine/src/budget.rs` — extend the e1-10 counter with `crossover_calls` and the 25% cap check.

### Work order
1. Represent each candidate's trusted-test outcomes as a `PassVector` bitset so distance is a XOR plus `count_ones`.
2. Implement `rank_pairs`, skipping any candidate lacking provenance and returning early when all distances are 0.
3. Build the synthesiser prompt from both diffs with their passing test lists, reusing the e1-4 static prefix.
4. Parse the returned patch and traits summary; on parse failure, discard the child rather than retrying silently.
5. Record both `MERGED_FROM` edges with the traits payload.
6. Check the 25% cap before dispatch and degrade to mutation when it would be exceeded.

### Implementation note
The cap is checked pre-dispatch, not reconciled afterwards: an over-cap call has already spent the tokens it was meant to save.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-engine` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-engine --test e1_5_crossover_of_two_parents" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-5-crossover-of-two-parents --status done
```
