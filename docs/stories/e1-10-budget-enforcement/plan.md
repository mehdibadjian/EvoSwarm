# Implementation Plan: Budget enforcement

**Story:** `e1-10-budget-enforcement` · **Sizing:** M · **Tier:** `flash` · **Target crate:** `evoswarm-engine`
**Depends on:** `e1-2-model-roles-in-config` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

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

- `crates/evoswarm-engine/tests/e1_10_budget_enforcement.rs::test_pre_call_cost_check_halt` — Asserts the loop halts before dispatch when the projected cost would cross the dollar cap.
- `crates/evoswarm-engine/tests/e1_10_budget_enforcement.rs::test_token_cap_exhaustion` — Asserts status `budget_exhausted` when the token limit would be reached, with zero further calls dispatched.
- `crates/evoswarm-engine/tests/e1_10_budget_enforcement.rs::test_early_stop_score_plateau` — Simulates 3 generations with non-improving best scores and verifies `plateau_early_stop`.
- `crates/evoswarm-engine/tests/e1_10_budget_enforcement.rs::test_spend_reconciliation_is_monotonic` — Feeds actual usage above projection and asserts recorded spend rises and never decreases.

**Target file(s):** `crates/evoswarm-engine/tests/e1_10_budget_enforcement.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | the projected cost is checked against the remaining budget before dispatch | `test_pre_call_cost_check_halt` |
| AC2 | the job halts as `budget_exhausted` and returns the best verified candidate so far | `test_token_cap_exhaustion` |
| AC3 | the job stops early with reason `plateau_early_stop` and a partial result | `test_early_stop_score_plateau` |
| AC4 | recorded spend rises to the actual value and never decreases | `test_spend_reconciliation_is_monotonic` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-engine/src/budget.rs` — `pub struct BudgetGuard { spent_usd, tokens_used, caps, crossover_calls }`.
- `crates/evoswarm-engine/src/budget.rs` — `pub fn pre_dispatch(&self, req: &CallRequest) -> Result<(), BudgetExhausted>` and `pub fn record(&mut self, usage: Usage)`.
- `crates/evoswarm-engine/src/early_stop.rs` — `pub fn should_stop(history: &[f64]) -> Option<StopReason>` implementing the 3-generation plateau rule.
- `crates/evoswarm-core/src/usage.rs` — `Usage { tokens_in, tokens_out, cost_usd }` and `CallRequest { role, max_tokens, prompt_tokens }`.

### Work order
1. Project cost from the role's per-million rates and the e1-4 token estimator; reject non-positive rates at config load.
2. Call `pre_dispatch` at the single dispatch chokepoint in `evoswarm-models` so no role can bypass it.
3. Record actual usage after each response with `record()`, taking `max(projected, actual)` so spend is monotonic.
4. Implement `should_stop` over the per-generation best-score history, requiring strictly increasing scores to continue.
5. On halt, transition the job to `BudgetExhausted` and return the best verified candidate from e1-8 selection.
6. Persist counters to the e1-12 ledger after each call so a crash cannot resurrect spent budget.

### Implementation note
One chokepoint for `pre_dispatch` mirrors the e1-8 leak-scan decision: per-call-site budget checks are how a new role silently becomes unbudgeted.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-engine` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-engine --test e1_10_budget_enforcement" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-10-budget-enforcement --status done
```
