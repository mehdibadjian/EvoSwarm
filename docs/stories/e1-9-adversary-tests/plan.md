# Implementation Plan: Adversary tests

**Story:** `e1-9-adversary-tests` · **Sizing:** M · **Tier:** `pro` · **Target crate:** `evoswarm-engine`
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

- `crates/evoswarm-engine/tests/e1_9_adversary_tests.rs::test_adversary_generation_quota` — Asserts the adversary produces at most `K` tests and each carries an origin tag.
- `crates/evoswarm-engine/tests/e1_9_adversary_tests.rs::test_syntax_error_discard` — Feeds an uncompilable test and asserts it is discarded before execution.
- `crates/evoswarm-engine/tests/e1_9_adversary_tests.rs::test_suspect_test_filtering` — Simulates a universally failing test and verifies the `suspect` tag and exclusion from $A$.
- `crates/evoswarm-engine/tests/e1_9_adversary_tests.rs::test_adversary_never_gates_candidate` — Fails an adversary test against a candidate and asserts the e1-6 `GateResult` is unchanged.

**Target file(s):** `crates/evoswarm-engine/tests/e1_9_adversary_tests.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | it generates up to K candidate tests | `test_adversary_generation_quota` |
| AC2 | it is discarded immediately | `test_syntax_error_discard` |
| AC3 | it is flagged `suspect` and excluded from scoring | `test_suspect_test_filtering` |
| AC4 | it never causes that candidate to fail a gate | `test_adversary_never_gates_candidate` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-engine/src/adversary.rs` — `pub async fn generate(spec: &str, best: &Candidate, k: usize, deps: &AdversaryDeps) -> Vec<AdversaryTest>`.
- `crates/evoswarm-core/src/adversary.rs` — `AdversaryTest { name, body, origin: TestOrigin, status: AdversaryStatus }` with `AdversaryStatus::{Valid, Discarded, Suspect}`.
- `crates/evoswarm-core/src/provenance.rs` — `TestOrigin::{Trusted, Adversary}` so gates and scoring can filter by origin.
- `crates/evoswarm-engine/src/adversary.rs` — `pub fn filter(tests: Vec<AdversaryTest>, baseline: &Baseline) -> Vec<AdversaryTest>` applying compile and suspect filters.

### Work order
1. Dispatch the adversary role with $K$ and reject any returned test lacking an origin tag.
2. Compile each test in the sandbox via `SandboxBackend`, discarding failures with the diagnostic logged.
3. Execute survivors against the baseline and all Gen 0 candidates; tag `suspect` when every run fails.
4. Pass only `AdversaryStatus::Valid` tests into the e1-7 term $A$ denominator.
5. Enforce gate isolation in `evoswarm-fitness`: `gates::evaluate` accepts a trusted-only slice, making adversary inclusion a type error rather than a runtime check.

### Implementation note
Filtering gates by the `TestOrigin` type is deliberate: a boolean flag on a shared list is one forgotten filter away from letting generated tests veto user code.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-engine` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-engine --test e1_9_adversary_tests" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-9-adversary-tests --status done
```
