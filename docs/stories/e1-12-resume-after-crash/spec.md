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
- On service restart: scan the `jobs` table for `Running` state.
- Locate the highest completed generation in `job_generations`.
- Resume the evolutionary loop from generation $G + 1$.
- Any model call whose idempotency hash exists in `model_call_cache` is read from disk with 0 API tokens spent.
- Sandbox runs are never cached: an interrupted run is re-executed from the start, because a partial execution result is indistinguishable from a failure.

### 3. Idempotency Key
The hash covers role, model ID, and the exact prompt bytes (static prefix plus dynamic
suffix). A changed prompt is a different key and correctly misses the cache.

### 4. Commit Discipline
A generation row is committed only after every candidate in it has been evaluated and
scored. Partial generations are never written, so resume cannot inherit half-scored
population state.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| Abrupt SIGKILL during generation $G$ | Daemon resumes at $G + 1$ with completed calls replayed from cache | Partial generation $G$ state discarded, not merged |
| Prompt hash present in `model_call_cache` | Stored response returned, 0 API tokens spent | Cache miss falls through to a real dispatch |
| Sandbox run in flight at crash time | Run re-executed cleanly from the start | No partial output treated as a result |
| Ledger interrupted mid-write | WAL recovery yields the last committed state | No torn generation row visible to resume |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given the process is killed mid-generation**, **when the service restarts**, **then the job resumes from the last completed generation**.
- **Given a model call completed before the crash**, **when resuming**, **then it is not repeated, using its stored idempotency key**.
- **Given sandbox runs were in flight during the crash**, **when resuming**, **then they are re-run cleanly rather than read from cache**.
- **Given the prompt bytes change after a crash**, **when the call is replayed**, **then the cache misses and a fresh dispatch occurs**.
