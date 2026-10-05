# Implementation Plan: Mutation with error feedback

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/engine/test_mutation_feedback.rs::test_feedback_prompt_formatting`](file:///workspace/calm-faraday/tests/engine/test_mutation_feedback.rs): Verifies truncation of large error dumps to 4k tokens.
- [`tests/engine/test_mutation_feedback.rs::test_mutated_from_lineage_edge`](file:///workspace/calm-faraday/tests/engine/test_mutation_feedback.rs): Asserts MUTATED_FROM relationship is correctly attached to child node.
- [`tests/engine/test_mutation_feedback.rs::test_cache_prefix_byte_identity`](file:///workspace/calm-faraday/tests/engine/test_mutation_feedback.rs): Asserts byte-identical static prompt prefix across sequential mutations.

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
python3 scripts/sprint.py verify --cmd "cargo test --test e1_4_mutation_with_error_feedback" --anti-cheat
```
