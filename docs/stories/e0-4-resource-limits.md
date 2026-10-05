# Story: Resource limits

## Metadata
- **Story Key:** `e0-4-resource-limits` (Short: `e0-4`)
- **Epic:** [The Crucible (Sandbox)](../epics/epic-0-the-crucible.md)
- **Persona:** Security Reviewer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e0-1-sandbox-runner-interface`

---

## 1. User Story
As a security reviewer, I want memory, process and time caps on every run, so that a runaway candidate cannot starve the host.

---

## 2. Architectural Context & Invariants
Governed by AD-1. Uses `systemd-run --user --scope` with `MemoryMax`, `TasksMax`, and `CPUQuota=100%`, plus an outer process timeout with 5s SIGKILL grace.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Fork-bomb script inside sandbox` | `Status: RunStatus::Oom or RunStatus::Failed, host load normalized <10s` | `Host freeze or kernel panic (forbidden)` |
| `Memory-leak script (>2GB)` | `Status: RunStatus::Oom, killed by cgroup OOM killer` | `Host process killed (forbidden)` |
| `Infinite loop script` | `Status: RunStatus::Timeout, killed after wall limit + 5s` | `Process leak (forbidden)` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a candidate runs a fork bomb**, **when TasksMax is hit**, **then the run ends as `oom` or `failed` and host load returns to normal within 10 s.**.
- **Given a candidate allocates 4 GB under a 2 GB cap**, **when the cgroup limit is reached**, **then the run ends as `oom` and no host process is killed.**.
- **Given a candidate loops forever**, **when the wall limit plus 5 s grace passes**, **then the process tree is killed and marked `timeout`.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/test_resource_limits.rs::test_fork_bomb_contained`: Spawns a bash fork bomb `:(){ :|:& };:` and verifies termination in <10s.
- `tests/test_resource_limits.rs::test_memory_cap_enforcement`: Allocates 4GB via Python in a 512MB limit sandbox; verifies RunStatus::Oom.
- `tests/test_resource_limits.rs::test_wall_time_sigkill`: Executes `sleep 60` with a 2s timeout and confirms kill within 7s.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e0_4_resource_limits" --anti-cheat
```
