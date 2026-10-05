# Story Specification: Patch and report

## 1. Functional Specification
### 1. Artifact Outputs
1. **Git Branch:** `evoswarm/<job-id>` branched from the base commit with a single atomic commit.
2. **Patch File:** `.evoswarm/patches/<job-id>.patch`.
3. **Audit Report:** `.evoswarm/reports/<job-id>.md` detailing:
   - Score breakdown ($S$, $A$, $P$, $Z$) and the weights used
   - Visible and held-out test pass lists
   - Model calls, tokens, and dollar cost per role
   - Lineage tree
   - Proposed adversary candidate tests for review

### 2. Isolation Invariant
The user's checked-out branch and working tree are never modified. Branch creation happens
against the object store without switching the working tree, so a dirty checkout cannot
block or corrupt artifact emission.

### 3. Patch Integrity
The emitted `.patch` must apply cleanly to the base commit with `git apply --check`.
Integrity is verified before the job is reported complete; an unappliable patch is a job
failure, not a warning.

### 4. No- Winner Path
When selection returns `no verified winner`, the best-effort patch and report are still
emitted and the report states the outcome explicitly rather than implying success.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| Verified winner + base commit | Branch `evoswarm/<job-id>` with one atomic commit | Base branch and working tree left untouched |
| Winning candidate diff | `.evoswarm/patches/<job-id>.patch` | `ReportError::PatchUnappliable` if it fails on a clean tree |
| Job metrics and lineage | `.evoswarm/reports/<job-id>.md` | Missing section fails report generation |
| No verified winner | Best-effort patch plus report flagging the status | Report states `no verified winner` explicitly |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given a verified winner**, **when the job completes**, **then branch `evoswarm/<job-id>` is created from the base commit and the `.patch` file is emitted**.
- **Given the emitted patch file**, **when applied to a clean checkout of the base commit**, **then it applies without conflict**.
- **Given the markdown report**, **when opened**, **then it displays score breakdown, held-out passes, model calls, cost, lineage, and proposed tests**.
- **Given any job**, **when it completes**, **then the base working tree and active branch are untouched**.
