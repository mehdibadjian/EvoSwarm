# Story: AppArmor profile for bwrap

## Metadata
- **Story Key:** `e0-3-apparmor-profile-for-bwrap` (Short: `e0-3`)
- **Epic:** [The Crucible (Sandbox)](file:///workspace/calm-faraday/docs/epics/epic-0.md)
- **Persona:** Operator
- **Priority:** Must
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** None

---

## 1. User Story
As an operator, I want a ready-made AppArmor profile for bwrap, so that unprivileged sandboxes work on a stock Ubuntu Server.

---

## 2. Architectural Context & Invariants
Recent Ubuntu releases (23.10+) restrict unprivileged user namespaces via AppArmor. An explicit profile allowing bwrap unshare without root privileges must be shipped and installed.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `installer script: scripts/install_apparmor.sh` | `Loaded AppArmor profile in /etc/apparmor.d/bwrap` | `PermissionDenied (if not sudo)` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a fresh Ubuntu Server LTS install**, **when I run the install script**, **then the profile loads and E0-2 passes without a reboot.**.
- **Given the profile is removed**, **when the service starts**, **then E0-2 fails with a reinstall hint.**.
- **Given the script is run twice**, **when it finishes**, **then the system state is identical to one run.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/test_apparmor.py::test_profile_idempotency`](file:///workspace/calm-faraday/tests/test_apparmor.py): Runs install script twice and checks aa-status.
- [`tests/test_apparmor.py::test_bwrap_unprivileged_execution`](file:///workspace/calm-faraday/tests/test_apparmor.py): Spawns unprivileged bwrap container under the loaded profile.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e0_3_apparmor_profile_for_bwrap" --anti-cheat
```
