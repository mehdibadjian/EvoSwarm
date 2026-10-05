# Story: Test triage page

## Metadata
- **Story Key:** `e5-5-test-triage-page` (Short: `e5-5`)
- **Epic:** [MAP-Elites and Multi-Stack Expansion](../epics/epic-5-map-elites-and-more-stacks.md)
- **Persona:** Reviewer
- **Priority:** Could
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** `e2-6-approve-candidate-tests`

---

## 1. User Story
As a reviewer, I want one page to triage candidate tests, so that approving them is not a CLI chore.

---

## 2. Architectural Context & Invariants
Lightweight web UI binding strictly to 127.0.0.1. Displays candidate adversary tests for a repository, which candidates failed them, and provides 1-click Approve / Reject buttons.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `HTTP GET /triage on 127.0.0.1` | `HTML interface with candidate tests & diffs` | `Refuses connections outside 127.0.0.1` |
| `HTTP POST /triage/approve?id=...` | `Promotes test to trusted via E2-6 workflow` | `Status: 200 OK` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given the page**, **when started**, **then it binds to 127.0.0.1 only.**.
- **Given candidate tests for a repo**, **when listed**, **then each shows its code and which candidates it failed.**.
- **Given I approve or reject a test**, **when saved**, **then the same path as E2-6 is used.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/ui/test_triage_page.rs::test_binds_localhost_only`: Asserts server socket rejects non-loopback bind.
- `tests/ui/test_triage_page.rs::test_triage_approval_action`: Submits test approval and asserts promotion in DB.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e5_5_test_triage_page" --anti-cheat
```
