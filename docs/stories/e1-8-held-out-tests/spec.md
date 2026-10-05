# Story Specification: Held-out tests

## 1. Functional Specification
### 1. Partitioning Invariant
- Total trusted tests $M$. If $M \ge 5$, held-out count $H = \max(1, \lfloor 0.20 	imes M floor)$.
- Test indices partitioned using SHA-256(JobId + TestName).
- Visible tests: $M - H$. Held-out tests: $H$.

### 2. Selection Loop
1. Rank candidates passing hard gates by score $S$ descending.
2. For top candidate, run held-out tests in sandbox.
3. If 100% pass $ightarrow$ candidate is **Verified Winner**.
4. If any held-out test fails $ightarrow$ evaluate next candidate.
5. If all candidates fail $ightarrow$ emit status `no verified winner` and flag best-effort patch.


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given a job starts**, **when tests are partitioned**, **then about 20% are held out using a stable split keyed by job ID.**.
- **Given any model call**, **when prompt payloads are scanned**, **then no held-out test content appears in any prompt.**.
- **Given the top candidate fails held-out tests**, **when selecting the winner**, **then the runner falls back to evaluate the next candidate.**.
- **Given no candidate passes held-out tests**, **when the job ends**, **then it reports `no verified winner` and flags the best-effort patch.**.
