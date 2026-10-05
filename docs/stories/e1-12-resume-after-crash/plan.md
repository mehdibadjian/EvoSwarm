# Implementation Plan: Resume after crash

**Story:** `e1-12-resume-after-crash` · **Sizing:** M · **Tier:** `pro` · **Target crate:** `evoswarm-ledger`
**Depends on:** `e1-1-start-a-job-from-the-cli` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

## 1. Pre-Flight Architecture & Rule Check
Complete these before writing code; tick each only when it actually holds.

- [ ] Reviewed [`ARCHITECTURE-SPINE.md`](../../architecture/ARCHITECTURE-SPINE.md) and confirmed this story upholds its invariants: Governed by AD-7 (SQLite Job Ledger).
- [ ] Confirmed compliance with [`architecture-rules.md`](../../../.agents/rules/architecture-rules.md), [`tdd-discipline.md`](../../../.agents/rules/tdd-discipline.md) and [`security-hygiene.md`](../../../.agents/rules/security-hygiene.md).
- [ ] Confirmed every dependency listed above is `done`; otherwise stop and report the blocker.
- [ ] Confirmed scope matches the contract matrix and acceptance criteria in [`spec.md`](spec.md) and nothing outside them is being built.
- [ ] Committed to zero AI comments in production code: comments explain **WHY**, never **WHAT**.

---

## 2. Red Phase (Failing Tests to Author First)
Author these tests before any production code, then run the gate command in section 5 and
confirm every test fails for its intended reason. A test that passes before implementation
is a false-positive test and must be rewritten.

- `crates/evoswarm-engine/tests/e1_12_resume_after_crash.rs::test_resume_from_last_generation` — Simulates a crash during Gen 2 and asserts resume starts at Gen 3.
- `crates/evoswarm-engine/tests/e1_12_resume_after_crash.rs::test_model_call_idempotency_cache` — Verifies the cached response is reused on replay with zero network calls.
- `crates/evoswarm-engine/tests/e1_12_resume_after_crash.rs::test_interrupted_sandbox_rerun` — Verifies an in-flight sandbox run is re-evaluated on restart rather than cached.
- `crates/evoswarm-engine/tests/e1_12_resume_after_crash.rs::test_changed_prompt_invalidates_cache` — Alters one suffix byte and asserts a cache miss triggers a fresh dispatch.

**Target file(s):** `crates/evoswarm-engine/tests/e1_12_resume_after_crash.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | the job resumes from the last completed generation | `test_resume_from_last_generation` |
| AC2 | it is not repeated, using its stored idempotency key | `test_model_call_idempotency_cache` |
| AC3 | they are re-run cleanly rather than read from cache | `test_interrupted_sandbox_rerun` |
| AC4 | the cache misses and a fresh dispatch occurs | `test_changed_prompt_invalidates_cache` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-ledger/src/generations.rs` — `pub fn commit_generation(job_id, generation, population, status)` in a single transaction.
- `crates/evoswarm-ledger/src/cache.rs` — `pub fn lookup(hash: &[u8; 32]) -> Option<CachedCall>` and `pub fn store(call: CachedCall)`.
- `crates/evoswarm-models/src/idempotency.rs` — `pub fn call_hash(role: Role, model_id: &str, prompt: &[u8]) -> [u8; 32]`.
- `crates/evoswarm-engine/src/recovery.rs` — `pub async fn recover(ledger: &JobLedger) -> Vec<ResumableJob>` and the resume entry point in the search loop.

### Work order
1. Enable WAL and `synchronous=FULL` on the ledger connection; durability here is the whole feature.
2. Wrap `commit_generation` in one transaction so a crash cannot expose a partial population.
3. Compute `call_hash` over role, model ID and exact prompt bytes; consult the cache before dispatch and store after.
4. Never route `SandboxBackend::run` through the cache; keep sandbox execution explicitly uncached in the dispatch path.
5. On startup, scan for `Running` jobs, read the max committed generation, and re-enter the loop at $G + 1$.
6. Reconcile e1-10 spend counters from `model_call_cache` so resumed jobs cannot re-spend already-recorded budget.

### Implementation note
Caching sandbox results would be the tempting optimisation and the wrong one: a killed run's partial output looks exactly like a genuine failure and would poison scoring.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-ledger` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-engine --test e1_12_resume_after_crash" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-12-resume-after-crash --status done
```
