# Implementation Plan: Seed the first generation

**Story:** `e1-3-seed-the-first-generation` · **Sizing:** M · **Tier:** `flash` · **Target crate:** `evoswarm-engine`
**Depends on:** `e1-1-start-a-job-from-the-cli`, `e1-2-model-roles-in-config` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

## 1. Pre-Flight Architecture & Rule Check
Complete these before writing code; tick each only when it actually holds.

- [ ] Reviewed [`ARCHITECTURE-SPINE.md`](../../architecture/ARCHITECTURE-SPINE.md) and confirmed this story upholds its invariants: Governed by AD-5 and AD-6.
- [ ] Confirmed compliance with [`architecture-rules.md`](../../../.agents/rules/architecture-rules.md), [`tdd-discipline.md`](../../../.agents/rules/tdd-discipline.md) and [`security-hygiene.md`](../../../.agents/rules/security-hygiene.md).
- [ ] Confirmed every dependency listed above is `done`; otherwise stop and report the blocker.
- [ ] Confirmed scope matches [`spec.md`](spec.md) section 2 and nothing outside it is being built.
- [ ] Committed to zero AI comments in production code: comments explain **WHY**, never **WHAT**.

---

## 2. Red Phase (Failing Tests to Author First)
Author these tests before any production code, then run the gate command in section 5 and
confirm every test fails for its intended reason. A test that passes before implementation
is a false-positive test and must be rewritten.

- `crates/evoswarm-engine/tests/e1_3_seed_the_first_generation.rs::test_gen0_population_quota` — Verifies exactly `N` candidates in Gen 0, including the baseline at index 0.
- `crates/evoswarm-engine/tests/e1_3_seed_the_first_generation.rs::test_candidate_metadata_recording` — Verifies `generation = 0`, empty `parent_ids`, non-empty `prompt_hash` and the configured `model_id`.
- `crates/evoswarm-engine/tests/e1_3_seed_the_first_generation.rs::test_diff_hash_deduplication` — Injects a duplicate draft and asserts it is replaced by a distinct draft at jittered temperature.
- `crates/evoswarm-engine/tests/e1_3_seed_the_first_generation.rs::test_dedup_retry_budget_exhaustion` — Forces every draft to collide and asserts `SeedingError::DedupExhausted` rather than a short population.

**Target file(s):** `crates/evoswarm-engine/tests/e1_3_seed_the_first_generation.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | it holds the current code plus N-1 fresh drafts | `test_gen0_population_quota` |
| AC2 | it records model ID, prompt hash, generation 0 and an empty parent list | `test_candidate_metadata_recording` |
| AC3 | the duplicate is dropped and replaced with a unique draft | `test_diff_hash_deduplication` |
| AC4 | seeding fails loudly instead of returning a short population | `test_dedup_retry_budget_exhaustion` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-core/src/candidate.rs` — `Candidate { id, patch, diff_hash, generation, parent_ids, model_id, prompt_hash }`.
- `crates/evoswarm-engine/src/seeding.rs` — `pub async fn seed_generation_zero(cfg: &SeedConfig, deps: &SeedingDeps) -> Result<Vec<Candidate>, SeedingError>`.
- `crates/evoswarm-engine/src/seeding.rs` — `trait MemorySeeder` returning `Vec<Candidate>` (empty impl in Epic 1; FalkorDB impl arrives in e2-5).
- `crates/evoswarm-engine/src/dedup.rs` — `pub fn diff_hash(patch: &[u8]) -> [u8; 32]` and the replacement retry loop.

### Work order
1. Add `Candidate` to `evoswarm-core` with `Serialize`/`Deserialize` so e1-12 can persist populations as JSON.
2. Build the baseline candidate from the working tree and pin it at index 0.
3. Call the mutator role concurrently with `futures::join_all`, one distinct seed per slot, honouring the e1-10 budget check.
4. Hash each diff, and on collision re-invoke at `temperature + 0.1`, counting attempts and erroring at 3.
5. Assert population length equals `N` as the final line of `seed_generation_zero`.

### Implementation note
`MemorySeeder` as a trait is the dependency-inversion seam mandated by architecture-rules section 3: Epic 2 supplies the graph-backed impl without touching this loop.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-engine` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-engine --test e1_3_seed_the_first_generation" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-3-seed-the-first-generation --status done
```
