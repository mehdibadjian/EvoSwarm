# Story: Seeding from similar winners

## Metadata
- **Story Key:** `e2-5-seeding-from-similar-winners` (Short: `e2-5`)
- **Epic:** [System 1 Memory and Replay](../epics/epic-2-system-1-memory-and-replay.md)
- **Persona:** Developer
- **Priority:** Should
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e2-2-lineage-written-to-falkordb`

---

## 1. User Story
As a developer, I want new tasks to start from proven approaches, so that they solve faster and cheaper.

---

## 2. Architectural Context & Invariants
Uses vector embeddings of task descriptions in FalkorDB to find similar winners. Injects up to 2 past winners into Gen 0 population as seeds.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Task description embedding` | `Up to 2 past winning patches joined to Gen 0` | `Empty list if no task above cosine similarity threshold` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a past task above the similarity threshold**, **when generation 0 is built**, **then up to 2 of its winners join as seeds.**.
- **Given a seed**, **when evaluated**, **then it passes through the same gates as any candidate and is returned only if it wins.**.
- **Given the benchmark**, **when run with and without seeding**, **then tokens per solved task are reported for both.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/memory/test_seeding.rs::test_vector_similarity_lookup`: Retrieves nearest past task by embedding cosine similarity.
- `tests/memory/test_seeding.rs::test_seed_candidate_gating`: Ensures injected seeds are evaluated through standard gates.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e2_5_seeding_from_similar_winners" --anti-cheat
```
