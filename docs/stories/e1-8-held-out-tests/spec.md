# Story Specification: Held-out tests

## 1. Functional Specification
### 1. Partitioning Invariant
- Total trusted tests $M$. If $M \ge 5$, held-out count $H = \max(1, \lfloor 0.20 \times M \rfloor)$.
- If $M < 5$, $H = 0$: holding out from a tiny suite would leave too few visible tests to drive search.
- Test names are assigned using SHA-256(JobId + TestName); the $H$ lowest hashes form the held-out set.
- The split is stable for a job: recomputing it always yields the same partition, which is what makes e1-12 resume safe.

### 2. Leakage Barrier
Held-out test names, bodies and assertion text never appear in any prompt payload.
Leakage scanning runs over the assembled prompt before dispatch, not over the raw
template, so interpolated diffs and error dumps are covered too.

### 3. Selection Loop
1. Rank candidates passing hard gates by score $S$ descending.
2. For the top candidate, run held-out tests in the sandbox.
3. If 100% pass $\rightarrow$ candidate is the **Verified Winner**.
4. If any held-out test fails $\rightarrow$ evaluate the next candidate.
5. If all candidates fail $\rightarrow$ emit status `no verified winner` and flag the best-effort patch.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| Full trusted test suite, `job_id` | `(VisibleTests, HeldOutTests)` stable for that job | Held-out empty when $M < 5$ |
| Any outgoing model prompt | Zero held-out test names or assertions present | Prompt rejected before dispatch on leak detection |
| Top candidate + held-out tests | Verified winner, or fallback to next candidate | Status `no verified winner` when all fail |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given a job starts**, **when tests are partitioned**, **then about 20% are held out using a stable split keyed by job ID**.
- **Given any model call**, **when prompt payloads are scanned**, **then no held-out test content appears in any prompt**.
- **Given the top candidate fails held-out tests**, **when selecting the winner**, **then the runner falls back to evaluate the next candidate**.
- **Given no candidate passes held-out tests**, **when the job ends**, **then it reports `no verified winner` and flags the best-effort patch**.
- **Given a suite with fewer than 5 trusted tests**, **when it is partitioned**, **then the held-out set is empty and the reason is recorded**.
