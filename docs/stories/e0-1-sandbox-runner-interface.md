# Story: Sandbox runner interface

## Metadata
- **Story Key:** `e0-1-sandbox-runner-interface` (Short: `e0-1`)
- **Epic:** [The Crucible (Sandbox)](file:///workspace/calm-faraday/docs/epics/epic-0.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** None

---

## 1. User Story
As a developer, I want every candidate to run through one sandbox interface, so that adding a language stack never touches the search loop.

---

## 2. Architectural Context & Invariants
Governed by AD-1. The search loop must remain completely decoupled from OS-level isolation mechanics and stack specifics. A unified Rust trait `SandboxBackend` must define the lifecycle contract (`prepare`, `run`, `collect`).

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `profile: SandboxProfile, patch: Vec<u8>` | `Ephemeral workdir PathBuf` | `Invalid mount or tmpfs allocation error` |
| `workdir: PathBuf, test_cmd: String` | `ExecutionResult(exit_code, stdout, stderr, wall_ms, peak_mem_bytes, status)` | `Timeout, OOM, or ProcessExecutionError` |
| `workdir: PathBuf` | `Result<(), SandboxError>` | `Workdir cleanup / unmount error` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a stack profile and a work directory**, **when `run` is called**, **then it returns exit code, stdout, stderr, wall time and peak memory.**.
- **Given a run exceeds its time or memory limit**, **when it is killed**, **then the result is marked `timeout` or `oom`, distinct from `failed`.**.
- **Given two runs execute in parallel**, **when both finish**, **then neither could read or write the other's files.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/test_sandbox_runner.rs::test_sandbox_runner_lifecycle_success`](file:///workspace/calm-faraday/tests/test_sandbox_runner.rs): Executes an echo command inside bwrap and verifies exit code 0, captured stdout, wall time > 0.
- [`tests/test_sandbox_runner.rs::test_sandbox_runner_timeout_marking`](file:///workspace/calm-faraday/tests/test_sandbox_runner.rs): Executes `sleep 10` with a 1s limit and asserts status is `RunStatus::Timeout`.
- [`tests/test_sandbox_runner.rs::test_sandbox_runner_concurrent_isolation`](file:///workspace/calm-faraday/tests/test_sandbox_runner.rs): Executes two concurrent runners writing to `/work/test.txt` and verifies distinct contents.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e0_1_sandbox_runner_interface" --anti-cheat
```
