# Story Specification: Patch and report

## 1. Functional Specification
### 1. Artifact Outputs
1. **Git Branch:** `evoswarm/<job-id>` branched from base commit with single atomic commit.
2. **Patch File:** `.evoswarm/patches/<job-id>.patch`.
3. **Audit Report:** `.evoswarm/reports/<job-id>.md` detailing:
   - Score breakdown ($S, A, P, Z$)
   - Visible and held-out test pass lists
   - Model calls, tokens, and dollar cost
   - Lineage tree
   - Proposed adversary candidate tests for review


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given a verified winner**, **when the job completes**, **then branch `evoswarm/<job-id>` is created from base commit and `.patch` file is emitted.**.
- **Given the markdown report**, **when opened**, **then it displays score breakdown, held-out passes, model calls, cost, lineage, and proposed tests.**.
- **Given any job**, **when it completes**, **then the base working tree and active branch are untouched.**.
