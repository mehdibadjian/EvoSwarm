# Story: Content-addressed blob store

## Metadata
- **Story Key:** `e2-1-content-addressed-blob-store` (Short: `e2-1`)
- **Epic:** [System 1 Memory and Replay](file:///workspace/calm-faraday/docs/epics/epic-2.md)
- **Persona:** Operator
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** None

---

## 1. User Story
As an operator, I want code and logs stored on disk by hash, so that the graph stays small enough to live in RAM.

---

## 2. Architectural Context & Invariants
Governed by AD-3. Stores immutable files under `.evoswarm/blobs/<sha256>`. Provides write, read, and 7-day unreferenced garbage collection sweep.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Raw bytes (code/log)` | `SHA-256 hash string` | `Disk I/O error` |
| `SHA-256 hash` | `Exact original bytes` | `NotFound error` |
| `GC trigger` | `Count and byte size of deleted unreferenced blobs` | `Zero deletion if all blobs referenced` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given the same content is written twice**, **when stored**, **then it exists once on disk.**.
- **Given a hash**, **when read**, **then the exact original bytes are returned.**.
- **Given blobs no node references for 7 days**, **when garbage collection runs**, **then they are deleted and the freed space is reported.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/storage/test_blob_store.rs::test_deduplication`](file:///workspace/calm-faraday/tests/storage/test_blob_store.rs): Writes identical payload twice and verifies single file.
- [`tests/storage/test_blob_store.rs::test_byte_exact_read`](file:///workspace/calm-faraday/tests/storage/test_blob_store.rs): Verifies read matches original write byte-for-byte.
- [`tests/storage/test_blob_store.rs::test_gc_sweep`](file:///workspace/calm-faraday/tests/storage/test_blob_store.rs): Sweeps unreferenced blobs older than 7 days.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e2_1_content_addressed_blob_store" --anti-cheat
```
