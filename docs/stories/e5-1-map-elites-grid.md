# Story: MAP-Elites grid

## Metadata
- **Story Key:** `e5-1-map-elites-grid` (Short: `e5-1`)
- **Epic:** [MAP-Elites and Multi-Stack Expansion](../epics/epic-5-map-elites-and-more-stacks.md)
- **Persona:** Developer
- **Priority:** Could
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e1-7-weighted-score`, `e2-2-lineage-written-to-falkordb`

---

## 1. User Story
As a developer, I want the archive to keep the best candidate of each kind, so that the search does not collapse onto one approach.

---

## 2. Architectural Context & Invariants
Governed by AD-8. 4x4 phenotypic grid: runtime relative to baseline (4 bins) x diff size in changed lines (4 bins). Replaces cell occupant if incoming candidate has higher score. Writes Cell nodes to FalkorDB.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Scored candidate (score, runtime, diff_lines)` | `Grid cell (x, y) assigned; occupant replaced if score is higher` | `Cell nodes created in FalkorDB` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a scored candidate**, **when archived**, **then it is placed in a 4 x 4 grid cell by runtime-versus-baseline bin and diff-size bin.**.
- **Given a cell already holds a candidate**, **when a higher-scoring one arrives**, **then it replaces the old one.**.
- **Given bin edges in config**, **when changed**, **then the next job uses them, and Cell nodes are written to the graph.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/engine/test_map_elites.rs::test_cell_binning`: Calculates grid coordinates from runtime and diff lines.
- `tests/engine/test_map_elites.rs::test_cell_replacement`: Replaces lower-scoring occupant with higher-scoring candidate.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e5_1_map_elites_grid" --anti-cheat
```
