# Story: Mutation with error feedback

## Metadata
- **Story Key:** `e1-4-mutation-with-error-feedback` (Short: `e1-4`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e1-3-seed-the-first-generation`, `e1-6-hard-gates`

---

## 1. User Story
As a developer, I want each new attempt to learn from the last one's failures, so that the search converges instead of guessing.

---

## 2. Architectural Context & Invariants
Passes compiler output and first failing assertion (trimmed to 4,000 tokens) to the mutator model. Maintains stable system prompt prefix to maximize prompt caching.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Failed candidate + error text` | `New candidate patch linked via MUTATED_FROM` | `BudgetExceeded if token cap reached` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a failed parent**, **when its child is drafted**, **then the prompt includes the first failing assertion and error text trimmed to 4k tokens.**.
- **Given any child**, **when it is stored**, **then it is linked to its parent with `MUTATED_FROM`.**.
- **Given repeated calls in one job**, **when they are sent**, **then repo context sits in a stable prefix and the prompt-cache hit rate is logged.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/engine/test_mutation.rs::test_error_feedback_prompt_formatting`](file:///workspace/calm-faraday/tests/engine/test_mutation.rs): Checks failure message truncation to 4k tokens.
- [`tests/engine/test_mutation.rs::test_prompt_cache_prefix_stability`](file:///workspace/calm-faraday/tests/engine/test_mutation.rs): Verifies system prompt bytes remain identical across mutations.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_4_mutation_with_error_feedback" --anti-cheat
```
