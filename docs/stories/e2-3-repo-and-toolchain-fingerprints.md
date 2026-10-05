# Story: Repo and toolchain fingerprints

## Metadata
- **Story Key:** `e2-3-repo-and-toolchain-fingerprints` (Short: `e2-3`)
- **Epic:** [System 1 Memory and Replay](file:///workspace/calm-faraday/docs/epics/epic-2.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** None

---

## 1. User Story
As a developer, I want context changes detected, so that a stored winner is never trusted after the code around it changed.

---

## 2. Architectural Context & Invariants
Governed by AD-4. Computes `repo_fingerprint` over touched files + lockfile. Computes `toolchain_fingerprint` over compiler, runtime, and harness versions.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Repository files + touched path list + lockfile` | `repo_fingerprint: String (SHA-256)` | `Error if file unreadable` |
| `Stack name (e.g. python, csharp)` | `toolchain_fingerprint: String (SHA-256)` | `Error if toolchain binary missing` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a file the task touches changes**, **when fingerprinted**, **then the repo fingerprint changes.**.
- **Given the lockfile changes**, **when fingerprinted**, **then the repo fingerprint changes.**.
- **Given an untouched file changes**, **when fingerprinted**, **then the repo fingerprint is unchanged.**.
- **Given a compiler, runtime or test runner version changes**, **when fingerprinted**, **then the toolchain fingerprint changes.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/memory/test_fingerprints.rs::test_touched_file_invalidation`](file:///workspace/calm-faraday/tests/memory/test_fingerprints.rs): Mutates touched file and checks hash diff.
- [`tests/memory/test_fingerprints.rs::test_untouched_file_insensitivity`](file:///workspace/calm-faraday/tests/memory/test_fingerprints.rs): Mutates sibling file outside paths and asserts identical hash.
- [`tests/memory/test_fingerprints.rs::test_toolchain_version_hash`](file:///workspace/calm-faraday/tests/memory/test_fingerprints.rs): Mocks compiler version change and asserts fingerprint diff.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e2_3_repo_and_toolchain_fingerprints" --anti-cheat
```
