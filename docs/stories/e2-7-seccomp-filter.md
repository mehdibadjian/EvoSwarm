# Story: Seccomp filter

## Metadata
- **Story Key:** `e2-7-seccomp-filter` (Short: `e2-7`)
- **Epic:** [System 1 Memory and Replay](../epics/epic-2-system-1-memory-and-replay.md)
- **Persona:** Security Reviewer
- **Priority:** Should
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e0-8-red-team-suite`

---

## 1. User Story
As a security reviewer, I want syscalls the harnesses never need blocked, so that the kernel attack surface is minimal.

---

## 2. Architectural Context & Invariants
Attaches a seccomp BPF filter to bwrap invocation blocking: `ptrace`, `mount`, `umount`, `keyctl`, `bpf`, `perf_event_open`, and `unshare`.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Attempt to invoke ptrace / bpf inside sandbox` | `Syscall returns EPERM / process killed with SIGSYS` | `Normal stack suites pass without error` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given the filter is active**, **when a candidate calls ptrace, mount, umount, keyctl, bpf, perf_event_open or unshare**, **then the call fails.**.
- **Given the filter is active**, **when all stack sample suites run**, **then they still pass.**.
- **Given the red-team suite**, **when extended with these syscalls**, **then every attempt fails.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/security/test_seccomp.rs::test_blocked_syscalls_sigsys`: Calls ptrace and keyctl and asserts EPERM/SIGSYS.
- `tests/security/test_seccomp.rs::test_harness_compatibility`: Runs full pytest and dotnet suites under seccomp filter.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e2_7_seccomp_filter" --anti-cheat
```
