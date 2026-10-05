# Story Specification: Resume after crash

## 1. Functional Specification
### 1. SQLite Ledger Schema
```sql
CREATE TABLE IF NOT EXISTS job_generations (
    job_id TEXT NOT NULL,
    generation INTEGER NOT NULL,
    population_json TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(job_id, generation)
);

CREATE TABLE IF NOT EXISTS model_call_cache (
    idempotency_hash TEXT PRIMARY KEY,
    model_id TEXT NOT NULL,
    response_text TEXT NOT NULL,
    tokens_in INTEGER NOT NULL,
    tokens_out INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 2. Recovery Protocol
- On service restart: scan `jobs` table for `Running` state.
- Locate highest completed generation in `job_generations`.
- Resume evolutionary loop from generation $G + 1$.
- Any model call whose prompt hash exists in `model_call_cache` is read from disk with 0 API tokens spent.


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given process killed mid-generation**, **when service restarts**, **then job resumes from the last completed generation.**.
- **Given a model call completed before the crash**, **when resuming**, **then it is not repeated, using its stored idempotency key.**.
- **Given sandbox runs were in flight during crash**, **when resuming**, **then they are re-run cleanly.**.
