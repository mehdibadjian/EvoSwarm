# Implementation Plan: Weighted score

**Story:** `e1-7-weighted-score` · **Sizing:** S · **Tier:** `flash` · **Target crate:** `evoswarm-fitness`
**Depends on:** `e1-6-hard-gates` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

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

- `crates/evoswarm-fitness/tests/e1_7_weighted_score.rs::test_exact_weighted_score_math` — Validates exact arithmetic against a hand-computed vector with default weights.
- `crates/evoswarm-fitness/tests/e1_7_weighted_score.rs::test_weight_validation_at_startup` — Rejects a config with weights summing to 1.05 and asserts the error prints the configured weights.
- `crates/evoswarm-fitness/tests/e1_7_weighted_score.rs::test_proportional_redistribution_without_adversary` — Checks $w'_p = 0.6$, $w'_s = 0.4$ when the adversary count is 0.
- `crates/evoswarm-fitness/tests/e1_7_weighted_score.rs::test_deterministic_scoring` — Scores an identical candidate 1,000 times and asserts bitwise equality via `f64::to_bits`.
- `crates/evoswarm-fitness/tests/e1_7_weighted_score.rs::test_edge_inputs_are_clamped` — Passes zero runtime and zero diff lines and asserts a finite score in $[0.0, 1.0]$.

**Target file(s):** `crates/evoswarm-fitness/tests/e1_7_weighted_score.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | S is computed using adversary pass rate, runtime versus baseline, and diff size | `test_exact_weighted_score_math` |
| AC2 | it fails with an explicit configuration error showing the weights | `test_weight_validation_at_startup` |
| AC3 | the adversary weight is redistributed proportionally to the runtime and diff size terms | `test_proportional_redistribution_without_adversary` |
| AC4 | every returned score is bitwise identical | `test_deterministic_scoring` |
| AC5 | the result is clamped and contains no NaN or infinity | `test_edge_inputs_are_clamped` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-fitness/src/scoring.rs` — `pub struct Weights { pub w_a: f64, pub w_p: f64, pub w_s: f64 }` with `Weights::validate(&self)`.
- `crates/evoswarm-fitness/src/scoring.rs` — `pub fn score(terms: &ScoreTerms, weights: &Weights) -> Result<f64, ScoreError>`.
- `crates/evoswarm-fitness/src/scoring.rs` — `pub fn adversary_pass_rate(passed: usize, total: usize) -> Option<f64>` returning `None` when `total == 0`.
- `crates/evoswarm-fitness/src/scoring.rs` — `pub fn runtime_term(baseline_ms: u64, candidate_ms: u64) -> f64` and `pub fn parsimony_term(diff_lines: usize) -> f64`.

### Work order
1. Validate weights at config load (e1-2 hook) and again in `score()` so a hand-built `Weights` cannot bypass the sum check.
2. Return `Option::None` from `adversary_pass_rate` when the denominator is 0; `score()` branches to the redistribution path on `None`.
3. Guard `candidate_ms == 0` before dividing, returning the clamped maximum rather than infinity.
4. Compute `w_a * A + w_p * P + w_s * Z` in that fixed source order; do not refactor into a fold over a dynamically ordered slice.
5. Reject $w_p + w_s = 0$ explicitly in the redistribution path.

### Implementation note
Bitwise determinism is why the term order is fixed in code. A fold over an unordered map would pass every functional test and still reorder floating-point additions.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-fitness` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-fitness --test e1_7_weighted_score" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-7-weighted-score --status done
```
