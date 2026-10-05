# Story: Patch and report

## Metadata
- **Story Key:** `e1-11-patch-and-report` (Short: `e1-11`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** `e1-6-hard-gates`, `e1-7-weighted-score`

---

## 1. User Story
As a developer, I want the result as a branch with a report, so that I can review and merge with confidence.

---

## 2. Architectural Context & Invariants
Creates git branch `evoswarm/<job-id>` from base commit, emits `.evoswarm/patches/<job-id>.patch`, and writes markdown report with lineage, scores, token/dollar costs, and adversary test proposals.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Completed job with winning candidate` | `Git branch, .patch file, Markdown report` | `Base branch remains untouched` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a winner**, **when the job completes**, **then branch `evoswarm/<job-id>` is created from the base commit and a `.patch` file is written.**.
- **Given the report**, **when opened**, **then it shows score breakdown, tests passed including held-out, model calls, tokens, cost, lineage and proposed candidate tests.**.
- **Given any job**, **when it completes**, **then the base branch is untouched.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/cli/test_output.rs::test_git_branch_creation`](file:///workspace/calm-faraday/tests/cli/test_output.rs): Verifies git branch exists and base branch HEAD is unchanged.
- [`tests/cli/test_output.rs::test_patch_file_applicability`](file:///workspace/calm-faraday/tests/cli/test_output.rs): Applies generated patch to clean checkout and verifies tests pass.
- [`tests/cli/test_output.rs::test_report_structure`](file:///workspace/calm-faraday/tests/cli/test_output.rs): Validates markdown report sections (scores, costs, lineage).

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_11_patch_and_report" --anti-cheat
```
