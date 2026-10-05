# Implementation Plan: Patch and report

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/cli/test_report_output.rs::test_git_branch_emission`](file:///workspace/calm-faraday/tests/cli/test_report_output.rs): Verifies branch creation and checks base commit equality.
- [`tests/cli/test_report_output.rs::test_patch_file_integrity`](file:///workspace/calm-faraday/tests/cli/test_report_output.rs): Applies emitted .patch file to clean repo and asserts clean application.
- [`tests/cli/test_report_output.rs::test_markdown_report_sections`](file:///workspace/calm-faraday/tests/cli/test_report_output.rs): Validates presence of score, cost, and lineage sections in report.

---

## 3. Green Phase (Minimal Production Code)
Implement the minimal logic in target crates/modules to satisfy tests:
- Define core structs and traits.
- Implement error handling and bounds checking.
- Connect persistence / CLI / sandbox dispatch.

---

## 4. Refactor Phase & Verification Gate
- Remove any redundant allocations or temporary scaffolding.
- Ensure all comments explain **WHY**, not **WHAT**.
- Execute verification gate:
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_11_patch_and_report" --anti-cheat
```
