# Story: Crossover of two parents

## Metadata
- **Story Key:** `e1-5-crossover-of-two-parents` (Short: `e1-5`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Developer
- **Priority:** Should
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e1-4-mutation-with-error-feedback`

---

## 1. User Story
As a developer, I want strengths of two partial solutions combined, so that complementary fixes are not lost.

---

## 2. Architectural Context & Invariants
Selects pairs of parents passing disjoint subsets of test cases; calls synthesiser model (max 25% of total calls); links candidate with MERGED_FROM edge containing synthesized traits.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Parent A (passes tests 1, 2) + Parent B (passes tests 3, 4)` | `Child C candidate patch with traits description` | `Fallback to mutation if no disjoint parents` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given two parents pass different subsets of tests**, **when parents are selected**, **then such pairs are preferred for crossover.**.
- **Given a crossover child**, **when it is stored**, **then it links to both parents with the traits the model reports merging.**.
- **Given default settings**, **when a job runs**, **then crossover is at most 25% of model calls.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/engine/test_crossover.rs::test_disjoint_parent_selection`](file:///workspace/calm-faraday/tests/engine/test_crossover.rs): Asserts parents with complementary test passes are prioritized.
- [`tests/engine/test_crossover.rs::test_crossover_call_cap`](file:///workspace/calm-faraday/tests/engine/test_crossover.rs): Asserts crossover calls never exceed 25% of total generation calls.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_5_crossover_of_two_parents" --anti-cheat
```
