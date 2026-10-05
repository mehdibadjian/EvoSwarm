# Story: Lineage written to FalkorDB

## Metadata
- **Story Key:** `e2-2-lineage-written-to-falkordb` (Short: `e2-2`)
- **Epic:** [System 1 Memory and Replay](file:///workspace/calm-faraday/docs/epics/epic-2.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e2-1-content-addressed-blob-store`, `e1-11-patch-and-report`

---

## 1. User Story
As a developer, I want every job's history recorded in the graph, so that past work can be queried and reused.

---

## 2. Architectural Context & Invariants
Governed by AD-3. Writes Task, Implementation, Evaluation, TestCase nodes and edges. Queries indexed by spec_hash and fingerprints. Queues retry if FalkorDB temporarily down.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Completed job record` | `Graph nodes and edges committed in FalkorDB` | `Queued in retry spool if DB unavailable` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a job completes**, **when it is recorded**, **then Task, Implementation, Evaluation and TestCase nodes and their edges match the schema.**.
- **Given the graph**, **when queried by spec_hash, repo_fingerprint or toolchain_fingerprint**, **then the lookup uses an index.**.
- **Given FalkorDB is unavailable**, **when a job completes**, **then the job still succeeds and the write is queued for retry.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/memory/test_falkordb_lineage.rs::test_graph_schema_conformance`](file:///workspace/calm-faraday/tests/memory/test_falkordb_lineage.rs): Validates created Cypher nodes and relationship types.
- [`tests/memory/test_falkordb_lineage.rs::test_index_usage`](file:///workspace/calm-faraday/tests/memory/test_falkordb_lineage.rs): Checks EXPLAIN output for indexed lookups.
- [`tests/memory/test_falkordb_lineage.rs::test_db_downtime_retry_queue`](file:///workspace/calm-faraday/tests/memory/test_falkordb_lineage.rs): Mocks DB outage and verifies spool file.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e2_2_lineage_written_to_falkordb" --anti-cheat
```
