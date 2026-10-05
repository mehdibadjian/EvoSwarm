# Story: Archive health report

## Metadata
- **Story Key:** `e5-6-archive-health-report` (Short: `e5-6`)
- **Epic:** [MAP-Elites and Multi-Stack Expansion](../epics/epic-5-map-elites-and-more-stacks.md)
- **Persona:** Developer
- **Priority:** Could
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e5-1-map-elites-grid`

---

## 1. User Story
As a developer, I want to see archive diversity per job, so that I can tell whether the search stayed broad.

---

## 2. Architectural Context & Invariants
Renders an ASCII or HTML visualization of the 4x4 grid showing cell occupancy (out of 16) and highest score per cell across generations.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Completed job with MAP-Elites archive` | `Occupancy count (e.g. 11/16 cells) + 4x4 matrix heatmap in job report` | `Diversity score logged` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a completed job**, **when the report renders**, **then it shows occupied cells out of 16 and the best score per cell for each generation.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/engine/test_archive_report.rs::test_grid_heatmap_rendering`: Validates 4x4 ASCII grid output.
- `tests/engine/test_archive_report.rs::test_occupancy_ratio_calculation`: Calculates occupied cell count.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e5_6_archive_health_report" --anti-cheat
```
