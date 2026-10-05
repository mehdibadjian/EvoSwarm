# Story: Red-team suite

## Metadata
- **Story Key:** `e0-8-red-team-suite` (Short: `e0-8`)
- **Epic:** [The Crucible (Sandbox)](../epics/epic-0-the-crucible.md)
- **Persona:** Security Reviewer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e0-4-resource-limits`, `e0-7-tamper-proof-tests`

---

## 1. User Story
As a security reviewer, I want automated escape attempts, so that isolation regressions are caught before release.

---

## 2. Architectural Context & Invariants
Red-team test harness that deliberately attempts: TCP/DNS egress, reading `/home` and `/etc/shadow`, writing outside `/work`, `ptrace` on host PIDs, reading `/proc/1`.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Red-team exploit payloads` | `All exploits fail cleanly (EACCES, ENETUNREACH, EPERM)` | `Any successful exploit blocks build and release` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given the suite runs**, **when it attempts TCP and DNS egress, reading `/home` and `/etc/shadow`, writing outside the work dir, ptrace on a host process and access to `/proc/1`**, **then every attempt fails.**.
- **Given one attempt succeeds**, **when CI runs**, **then the build fails and release is blocked.**.
- **Given the service starts**, **when E0-2 runs**, **then a quick subset of the suite runs as part of the self-check.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/red_team/test_network_egress.rs`: Attempts raw TCP socket connect and DNS resolve; expects network unreachable.
- `tests/red_team/test_filesystem_traversal.rs`: Attempts reading `/etc/shadow` and `/home`; expects EACCES/ENOENT.
- `tests/red_team/test_ptrace_denial.rs`: Attempts ptrace(PTRACE_ATTACH, 1); expects EPERM.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e0_8_red_team_suite" --anti-cheat
```
