# Story: Python stack

## Metadata
- **Story Key:** `e0-5-python-stack` (Short: `e0-5`)
- **Epic:** [The Crucible (Sandbox)](../epics/epic-0-the-crucible.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** `e0-1-sandbox-runner-interface`, `e0-4-resource-limits`

---

## 1. User Story
As a developer, I want Python tasks tested with pytest in the sandbox, so that I can evolve Python code without exposing my machine.

---

## 2. Architectural Context & Invariants
Governed by AD-1. Creates an offline read-only venv keyed by lockfile SHA-256 (`uv.lock` or `requirements.txt`). pytest runs with JUnit XML output.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Python project + candidate patch + pytest command` | `JUnit XML parsed: passed, failed, skipped counts` | `Pip network error or build failure` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a repo lockfile**, **when the first job runs**, **then a venv is built outside the sandbox keyed by the lockfile hash and reused by later jobs.**.
- **Given a candidate calls `pip install`**, **when it runs**, **then the install fails because there is no network and the venv is read-only.**.
- **Given the sample suite**, **when it runs**, **then per-test results are parsed from JUnit XML into pass, fail and skip counts.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/test_python_stack.rs::test_venv_cache_reuse`: Verifies venv is created once and reused for identical lockfile hashes.
- `tests/test_python_stack.rs::test_pip_install_egress_fails`: Attempts `pip install requests` inside sandbox and confirms failure.
- `tests/test_python_stack.rs::test_junit_xml_parsing`: Parses pytest junitxml output into structured test summary.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e0_5_python_stack" --anti-cheat
```
