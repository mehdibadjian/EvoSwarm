# Implementation Plan: Resume after crash

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/engine/test_recovery.rs::test_resume_from_last_generation`](file:///workspace/calm-faraday/tests/engine/test_recovery.rs): Simulates crash during Gen 2; asserts resume starts at Gen 3.
- [`tests/engine/test_recovery.rs::test_model_call_idempotency_cache`](file:///workspace/calm-faraday/tests/engine/test_recovery.rs): Verifies cached response used on replay with 0 network calls.
- [`tests/engine/test_recovery.rs::test_interrupted_sandbox_rerun`](file:///workspace/calm-faraday/tests/engine/test_recovery.rs): Verifies in-flight sandbox run re-evaluated on restart.

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
python3 scripts/sprint.py verify --cmd "cargo test --test e1_12_resume_after_crash" --anti-cheat
```
