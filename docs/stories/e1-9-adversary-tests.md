# Story: Adversary tests

## Metadata
- **Story Key:** `e1-9-adversary-tests` (Short: `e1-9`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Developer
- **Priority:** Should
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e1-6-hard-gates`

---

## 1. User Story
As a developer, I want edge cases I missed probed automatically, so that weak solutions rank lower.

---

## 2. Architectural Context & Invariants
Governed by AD-5. Uses adversary model role to generate K edge-case tests. Discards non-compiling tests. Flags tests that fail on baseline and all candidates as suspect. Never gates candidates.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Spec + best candidate patch` | `List of candidate test cases` | `Suspect filter discards broken tests` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given the spec and the current best candidate**, **when the adversary runs**, **then it writes up to K tests, with K set in config.**.
- **Given an adversary test does not compile**, **when collected**, **then it is discarded.**.
- **Given an adversary test fails on every candidate and the baseline**, **when collected**, **then it is flagged `suspect` and excluded from scoring.**.
- **Given any adversary test**, **when gates run**, **then it never affects a gate.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/engine/test_adversary.rs::test_adversary_test_generation`](file:///workspace/calm-faraday/tests/engine/test_adversary.rs): Asserts K tests drafted.
- [`tests/engine/test_adversary.rs::test_uncompilable_test_discard`](file:///workspace/calm-faraday/tests/engine/test_adversary.rs): Verifies compiler error drops test.
- [`tests/engine/test_adversary.rs::test_suspect_adversary_test_exclusion`](file:///workspace/calm-faraday/tests/engine/test_adversary.rs): Confirms universally failing test is tagged suspect.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_9_adversary_tests" --anti-cheat
```
