# Implementation Plan: Patch and report

**Story:** `e1-11-patch-and-report` · **Sizing:** M · **Tier:** `flash` · **Target crate:** `evoswarm-cli`
**Depends on:** `e1-6-hard-gates`, `e1-7-weighted-score` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

## 1. Pre-Flight Architecture & Rule Check
Complete these before writing code; tick each only when it actually holds.

- [ ] Reviewed [`ARCHITECTURE-SPINE.md`](../../architecture/ARCHITECTURE-SPINE.md) and confirmed this story upholds its invariants: Governed by AD-1 and AD-5.
- [ ] Confirmed compliance with [`architecture-rules.md`](../../../.agents/rules/architecture-rules.md), [`tdd-discipline.md`](../../../.agents/rules/tdd-discipline.md) and [`security-hygiene.md`](../../../.agents/rules/security-hygiene.md).
- [ ] Confirmed every dependency listed above is `done`; otherwise stop and report the blocker.
- [ ] Confirmed scope matches the contract matrix and acceptance criteria in [`spec.md`](spec.md) and nothing outside them is being built.
- [ ] Committed to zero AI comments in production code: comments explain **WHY**, never **WHAT**.

---

## 2. Red Phase (Failing Tests to Author First)
Author these tests before any production code, then run the gate command in section 5 and
confirm every test fails for its intended reason. A test that passes before implementation
is a false-positive test and must be rewritten.

- `crates/evoswarm-cli/tests/e1_11_patch_and_report.rs::test_git_branch_emission` — Verifies branch creation from the base commit and asserts the active branch is unchanged.
- `crates/evoswarm-cli/tests/e1_11_patch_and_report.rs::test_patch_file_integrity` — Applies the emitted `.patch` to a clean checkout and asserts clean application.
- `crates/evoswarm-cli/tests/e1_11_patch_and_report.rs::test_markdown_report_sections` — Validates the presence and non-emptiness of score, cost, lineage and proposed-test sections.
- `crates/evoswarm-cli/tests/e1_11_patch_and_report.rs::test_working_tree_untouched` — Dirties the working tree before a job and asserts the same uncommitted changes survive completion.

**Target file(s):** `crates/evoswarm-cli/tests/e1_11_patch_and_report.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | branch `evoswarm/<job-id>` is created from the base commit and the `.patch` file is emitted | `test_git_branch_emission` |
| AC2 | it applies without conflict | `test_patch_file_integrity` |
| AC3 | it displays score breakdown, held-out passes, model calls, cost, lineage, and proposed tests | `test_markdown_report_sections` |
| AC4 | the base working tree and active branch are untouched | `test_working_tree_untouched` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-cli/src/artifacts.rs` — `pub fn emit(job: &JobRecord, winner: &SelectionOutcome, repo: &RepoRoot) -> Result<Artifacts, ReportError>`.
- `crates/evoswarm-cli/src/git_writer.rs` — branch and patch emission built on `git2`, creating the branch from the base commit without checking it out.
- `crates/evoswarm-cli/src/report.rs` — `pub fn render_markdown(job: &JobRecord, winner: &SelectionOutcome) -> String`.
- `crates/evoswarm-core/src/report_model.rs` — `ScoreBreakdown`, `RoleUsage`, `LineageNode` view types.

### Work order
1. Resolve and record the base commit at job start (e1-1) so artifact emission does not depend on HEAD at completion time.
2. Create the branch from the base commit with `git2`, committing the winning patch as a single atomic commit; never call `checkout` on the user's working tree.
3. Write the `.patch` and verify it with `git apply --check` against a temporary clean worktree before reporting success.
4. Render the report from the view types, failing loudly when a section's data is absent rather than omitting the heading.
5. Handle `NoVerifiedWinner` by emitting the best-effort patch and stating the outcome in the report header.
6. Ensure no remote push occurs; the artifact set is local-only by design.

### Implementation note
Creating the branch without a checkout is the whole isolation guarantee: `git switch` would clobber uncommitted user work and is untestable to undo.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-cli` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-cli --test e1_11_patch_and_report" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-11-patch-and-report --status done
```
