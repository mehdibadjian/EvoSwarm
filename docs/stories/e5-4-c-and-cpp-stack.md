# Story: C and C++ stack

## Metadata
- **Story Key:** `e5-4-c-and-cpp-stack` (Short: `e5-4`)
- **Epic:** [MAP-Elites and Multi-Stack Expansion](../epics/epic-5-map-elites-and-more-stacks.md)
- **Persona:** Developer
- **Priority:** Could
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e0-1-sandbox-runner-interface`, `e0-4-resource-limits`

---

## 1. User Story
As a developer, I want C and C++ tasks tested with GoogleTest or Catch2, so that native code can be evolved safely.

---

## 2. Architectural Context & Invariants
Governed by AD-1. Configures CMake in tmpfs against read-only prebuilt test libraries. Runs ctest in bwrap with 1GB memory limit and 60s timeout.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `CMake C/C++ project + patch` | `Parsed GoogleTest / Catch2 test summary` | `Compilation or linkage error captured in feedback` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a CMake project**, **when a candidate runs**, **then configure and build happen in tmpfs against read-only prebuilt test libraries.**.
- **Given the samples**, **when run**, **then GoogleTest and Catch2 suites pass.**.
- **Given the C/C++ profile**, **when the red-team suite and E0-9 calibration run**, **then all attempts fail and limits are recorded.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/stacks/test_cpp_stack.rs::test_cmake_tmpfs_build`: Builds CMake candidate in ephemeral tmpfs.
- `tests/stacks/test_cpp_stack.rs::test_googletest_execution`: Executes GoogleTest test suite offline.
- `tests/stacks/test_cpp_stack.rs::test_cpp_red_team_containment`: Verifies red team attempts blocked in native sandbox.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e5_4_c_and_cpp_stack" --anti-cheat
```
