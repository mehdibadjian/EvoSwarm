# Story: Approve candidate tests

## Metadata
- **Story Key:** `e2-6-approve-candidate-tests` (Short: `e2-6`)
- **Epic:** [System 1 Memory and Replay](../epics/epic-2-system-1-memory-and-replay.md)
- **Persona:** Reviewer
- **Priority:** Should
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e1-9-adversary-tests`, `e2-2-lineage-written-to-falkordb`

---

## 1. User Story
As a reviewer, I want to promote good adversary tests, so that future jobs on this repo are held to a higher bar.

---

## 2. Architectural Context & Invariants
Governed by AD-5. CLI command `evoswarm approve-tests <job> --ids <ids>` promotes chosen candidate tests to trusted status, committing them on a git branch.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Job ID + test case IDs to approve` | `Promoted status in FalkorDB + git branch with committed tests` | `Error if test ID not found` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a job report**, **when I run `evoswarm approve-tests <job> --ids`**, **then the chosen tests become trusted for that repo and are added on a branch for me to merge.**.
- **Given I reject a test**, **when later jobs run**, **then it is not proposed again.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/cli/test_approve_tests.rs::test_promote_adversary_test`: Checks status updated to 'trusted' in FalkorDB.
- `tests/cli/test_approve_tests.rs::test_rejected_test_suppression`: Verifies rejected test marked and omitted from future proposals.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e2_6_approve_candidate_tests" --anti-cheat
```
