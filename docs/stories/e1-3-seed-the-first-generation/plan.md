# Implementation Plan: Seed the first generation

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/engine/test_generation_seeding.rs::test_gen0_population_quota`](file:///workspace/calm-faraday/tests/engine/test_generation_seeding.rs): Verifies exact N candidates in Gen 0.
- [`tests/engine/test_generation_seeding.rs::test_candidate_metadata_recording`](file:///workspace/calm-faraday/tests/engine/test_generation_seeding.rs): Verifies generation=0, empty parent IDs, and valid prompt hash.
- [`tests/engine/test_generation_seeding.rs::test_diff_hash_deduplication`](file:///workspace/calm-faraday/tests/engine/test_generation_seeding.rs): Injects mock duplicate draft and confirms replacement.

---

## 3. Green Phase (Minimal Production Code)
Implement the minimal logic in target crates/modules to satisfy tests:
- Define core structs and traits.
- Implement error handling and bounds checking.
- Connect persistence / CLI / sandbox dispatch.

---

## 4. Refactor Phase & Verification Gate
- Remove any redundant allocations or temporary scaffolding.
- Ensure all comments explain **WHY**, not **WHAT**.
- Execute verification gate:
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_3_seed_the_first_generation" --anti-cheat
```
