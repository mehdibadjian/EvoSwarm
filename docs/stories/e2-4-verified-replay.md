# Story: Verified replay

## Metadata
- **Story Key:** `e2-4-verified-replay` (Short: `e2-4`)
- **Epic:** [System 1 Memory and Replay](file:///workspace/calm-faraday/docs/epics/epic-2.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e2-2-lineage-written-to-falkordb`, `e2-3-repo-and-toolchain-fingerprints`

---

## 1. User Story
As a developer, I want a repeat task answered from memory after re-verification, so that it finishes in seconds with no model spend.

---

## 2. Architectural Context & Invariants
Governed by AD-4. Matches triple fingerprints in FalkorDB; re-runs stored winner on full suite (including held-out) in Crucible. Returns in seconds with 0 LLM calls if passed.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Task matching spec_hash + repo_fingerprint + toolchain_fingerprint` | `Status 'replayed', 0 model calls, verified patch` | `Fallback to seeded search if verification fails` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given spec hash and both fingerprints match a stored winner**, **when the job starts**, **then the winner runs against the full suite including held-out tests.**.
- **Given it passes**, **when returned**, **then the job is marked `replayed` and made 0 model calls.**.
- **Given it fails**, **when checked**, **then a normal job starts with it as a seed and the mismatch is logged.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/memory/test_replay.rs::test_instant_replay_success`](file:///workspace/calm-faraday/tests/memory/test_replay.rs): Executes repeat task, asserts status 'replayed', 0 LLM calls, <5s wall time.
- [`tests/memory/test_replay.rs::test_replay_verification_failure_fallback`](file:///workspace/calm-faraday/tests/memory/test_replay.rs): Induces test failure and asserts fallback to normal search.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e2_4_verified_replay" --anti-cheat
```
