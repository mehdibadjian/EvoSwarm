# Implementation Plan: Mutation with error feedback

**Story:** `e1-4-mutation-with-error-feedback` · **Sizing:** M · **Tier:** `pro` · **Target crate:** `evoswarm-engine`
**Depends on:** `e1-3-seed-the-first-generation`, `e1-6-hard-gates` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

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

- `crates/evoswarm-engine/tests/e1_4_mutation_with_error_feedback.rs::test_feedback_prompt_formatting` — Verifies compiler and assertion sections appear in order and the total is clamped to 4k tokens.
- `crates/evoswarm-engine/tests/e1_4_mutation_with_error_feedback.rs::test_feedback_truncation_marker` — Feeds a 50k-token dump and asserts the result is exactly at the clamp and ends with the truncation marker.
- `crates/evoswarm-engine/tests/e1_4_mutation_with_error_feedback.rs::test_mutated_from_lineage_edge` — Asserts the `MUTATED_FROM` edge is attached from child to the correct parent id.
- `crates/evoswarm-engine/tests/e1_4_mutation_with_error_feedback.rs::test_cache_prefix_byte_identity` — Asserts the static prefix bytes are identical across sequential mutations in one job.

**Target file(s):** `crates/evoswarm-engine/tests/e1_4_mutation_with_error_feedback.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | the prompt includes the first failing assertion and error text trimmed to 4k tokens | `test_feedback_prompt_formatting` |
| AC2 | the total is hard-clamped to 4,000 tokens and carries a truncation marker | `test_feedback_truncation_marker` |
| AC3 | it is linked to its parent with a `MUTATED_FROM` edge | `test_mutated_from_lineage_edge` |
| AC4 | repository context sits in a byte-identical static prefix and the cache hit rate is logged | `test_cache_prefix_byte_identity` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-engine/src/mutation.rs` — `pub async fn mutate(parent: &Candidate, feedback: &FailureFeedback, deps: &MutationDeps) -> Result<Candidate, MutationError>`.
- `crates/evoswarm-engine/src/feedback.rs` — `FailureFeedback`, `pub fn build(result: &ExecutionResult) -> FailureFeedback` and `pub fn render_clamped(&self, budget: usize) -> String`.
- `crates/evoswarm-models/src/prompt.rs` — `PromptParts { static_prefix: Vec<u8>, dynamic_suffix: Vec<u8> }` and `pub fn estimate_tokens(bytes: &[u8]) -> usize`.
- `crates/evoswarm-models/src/lineage.rs` — `trait LineageSink` with `record_mutation(child, parent)`; graph impl arrives in Epic 2.

### Work order
1. Implement `estimate_tokens` once here and reuse it in e1-10 so the clamp and budget never disagree.
2. Build `FailureFeedback` from `ExecutionResult`: split compiler output from the first failing assertion and stack trace.
3. Apply the 2k/2k/4k clamps in `render_clamped`, appending the truncation marker whenever a section is cut.
4. Assemble the static prefix once per job and store it; construct each call as `prefix ++ suffix` with no per-call data in the prefix.
5. Record the `MUTATED_FROM` edge through `LineageSink` immediately after the child is accepted.
6. Log cache hit rate as `prefix_bytes_identical` plus provider-reported cache reads.

### Implementation note
Prefix byte-identity is a correctness property, not a cost optimisation: a timestamp in the prefix silently disables caching and inflates every e1-10 projection.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-engine` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-engine --test e1_4_mutation_with_error_feedback" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-4-mutation-with-error-feedback --status done
```
