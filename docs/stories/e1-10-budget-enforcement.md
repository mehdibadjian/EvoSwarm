# Story: Budget enforcement

## Metadata
- **Story Key:** `e1-10-budget-enforcement` (Short: `e1-10`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** `e1-2-model-roles-in-config`

---

## 1. User Story
As a developer, I want a hard spend cap per job, so that a search never costs more than I agreed.

---

## 2. Architectural Context & Invariants
Governed by AD-6. Checks projected token and dollar cost before each LLM call. Stops early if no score improvement across 2 generations. Halts with `budget_exhausted` when cap hit.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Current spend + projected call cost vs cap` | `Execute call OR abort loop with status 'budget_exhausted'` | `Best partial patch returned` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a token and dollar cap**, **when a model call is about to be sent**, **then its projected cost is checked against the remaining budget.**.
- **Given the cap would be exceeded**, **when checked**, **then the job stops as `budget_exhausted` and returns the best verified result so far.**.
- **Given no score improvement over 2 generations**, **when the generation ends**, **then the job stops early.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/engine/test_budget.rs::test_hard_token_cap_stop`](file:///workspace/calm-faraday/tests/engine/test_budget.rs): Sets low token cap and confirms loop stops before exceeding.
- [`tests/engine/test_budget.rs::test_early_stop_plateau`](file:///workspace/calm-faraday/tests/engine/test_budget.rs): Simulates 2 generations with identical score and asserts early stop.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_10_budget_enforcement" --anti-cheat
```
