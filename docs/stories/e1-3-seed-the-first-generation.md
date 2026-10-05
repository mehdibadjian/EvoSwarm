# Story: Seed the first generation

## Metadata
- **Story Key:** `e1-3-seed-the-first-generation` (Short: `e1-3`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** `e1-1-start-a-job-from-the-cli`, `e1-2-model-roles-in-config`

---

## 1. User Story
As a developer, I want the search to start from several different attempts, so that it is not stuck refining one idea.

---

## 2. Architectural Context & Invariants
Populates Generation 0 with: current code baseline + N-1 fresh candidate drafts from mutator model. Deduplicates identical drafts by content hash.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Population size N (e.g. 6), task description` | `List of N distinct candidate patches with model ID & prompt hash` | `Fails if model calls fail` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given population size N**, **when generation 0 is built**, **then it holds the current code plus N-1 fresh drafts.**.
- **Given each draft**, **when it is created**, **then it records model, prompt hash and an empty parent.**.
- **Given two drafts have identical content hashes**, **when the generation is finalised**, **then the duplicate is dropped and replaced.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/engine/test_seeding.rs::test_gen0_population_count`](file:///workspace/calm-faraday/tests/engine/test_seeding.rs): Verifies exactly N candidates produced.
- [`tests/engine/test_seeding.rs::test_gen0_deduplication`](file:///workspace/calm-faraday/tests/engine/test_seeding.rs): Simulates two identical drafts and verifies one is dropped and regenerated.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_3_seed_the_first_generation" --anti-cheat
```
