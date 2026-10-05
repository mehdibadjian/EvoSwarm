# Story: Host self-check

## Metadata
- **Story Key:** `e0-2-host-self-check` (Short: `e0-2`)
- **Epic:** [The Crucible (Sandbox)](file:///workspace/calm-faraday/docs/epics/epic-0.md)
- **Persona:** Operator
- **Priority:** Must
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e0-3-apparmor-profile-for-bwrap`

---

## 1. User Story
As an operator, I want the service to verify host isolation features at start, so that jobs never run on a box where the sandbox silently degrades.

---

## 2. Architectural Context & Invariants
Governed by AD-1. Unprivileged namespaces, cgroups v2, and AppArmor can be silently broken by host kernel updates. EvoSwarm must perform deterministic health checks at startup and via `evoswarm status`.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `CLI flag: evoswarm status / daemon startup` | `Exit code 0, status 'sandbox: ready'` | `Exit code 1, diagnostic remediation messages detailing failed check` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given unprivileged user namespaces are blocked**, **when the service starts**, **then it logs which check failed with the fix and refuses new jobs.**.
- **Given all checks pass**, **when I run `evoswarm status`**, **then it reports `sandbox: ready`.**.
- **Given a check starts failing after a kernel update**, **when the service restarts**, **then queued jobs stay queued rather than failing.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/test_host_check.rs::test_check_user_namespaces`](file:///workspace/calm-faraday/tests/test_host_check.rs): Validates unshare(CLONE_NEWUSER) syscall availability.
- [`tests/test_host_check.rs::test_check_cgroup_v2_delegation`](file:///workspace/calm-faraday/tests/test_host_check.rs): Checks `/sys/fs/cgroup/user.slice` controllers for `memory` and `pids`.
- [`tests/test_host_check.rs::test_check_linger_enabled`](file:///workspace/calm-faraday/tests/test_host_check.rs): Verifies systemd user lingering status for the runtime user.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e0_2_host_self_check" --anti-cheat
```
