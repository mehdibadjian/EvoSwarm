# Story: Selection mode comparison

## Metadata
- **Story Key:** `e5-2-selection-mode-comparison` (Short: `e5-2`)
- **Epic:** [MAP-Elites and Multi-Stack Expansion](file:///workspace/calm-faraday/docs/epics/epic-5.md)
- **Persona:** Team Lead
- **Priority:** Could
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e5-1-map-elites-grid`, `e1-13-benchmark-suite`

---

## 1. User Story
As a team lead, I want MAP-Elites compared against top-k, so that we keep it only if it earns its complexity.

---

## 2. Architectural Context & Invariants
Allows setting `selection = 'topk'` or `'mapelites'` in config. Runs benchmark suite under both modes and compares solve rate, token spend, and candidate diversity.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Selection mode flag` | `Comparative report: MAP-Elites vs Top-K on benchmark suite` | `Summary printed with diversity metrics` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given `selection: topk` or `selection: mapelites`**, **when a job runs**, **then parents are drawn by that mode.**.
- **Given the benchmark**, **when run in both modes**, **then solve rate and tokens per solved task are reported side by side.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/benchmark/test_selection_modes.py::test_mode_dispatch`](file:///workspace/calm-faraday/tests/benchmark/test_selection_modes.py): Verifies parent selection reflects config setting.
- [`tests/benchmark/test_selection_modes.py::test_side_by_side_reporting`](file:///workspace/calm-faraday/tests/benchmark/test_selection_modes.py): Generates comparative solve rate report.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e5_2_selection_mode_comparison" --anti-cheat
```
