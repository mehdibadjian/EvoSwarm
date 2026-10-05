# Story: C# stack

## Metadata
- **Story Key:** `e0-6-csharp-stack` (Short: `e0-6`)
- **Epic:** [The Crucible (Sandbox)](file:///workspace/calm-faraday/docs/epics/epic-0.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** L
- **Execution Tier:** `pro`
- **Dependencies:** `e0-1-sandbox-runner-interface`, `e0-4-resource-limits`

---

## 1. User Story
As a developer, I want C# tasks tested with dotnet test, so that cTrader bots and enterprise code can be evolved safely.

---

## 2. Architectural Context & Invariants
Governed by AD-1. Pre-restores NuGet dependencies outside the sandbox to a hash-keyed cache. Executes `dotnet test --no-restore` in bwrap with xUnit and Reqnroll support.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `C# csproj + packages.lock.json + test suite` | `Parsed test summary (pass, fail, skip)` | `Missing package error -> 'restore cache stale'` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a project's lock or package references**, **when the first job runs**, **then NuGet restore runs once outside the sandbox into a cache keyed by their hash.**.
- **Given the cache exists**, **when `dotnet test --no-restore` runs with no network**, **then xUnit and Reqnroll sample suites pass.**.
- **Given a candidate references a package missing from the cache**, **when it builds**, **then the result says `restore cache stale` instead of a generic build failure.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/test_csharp_stack.rs::test_nuget_cache_restore`](file:///workspace/calm-faraday/tests/test_csharp_stack.rs): Restores NuGet packages to isolated cache directory.
- [`tests/test_csharp_stack.rs::test_dotnet_test_offline`](file:///workspace/calm-faraday/tests/test_csharp_stack.rs): Executes `dotnet test --no-restore` in offline bwrap container.
- [`tests/test_csharp_stack.rs::test_missing_package_stale_cache_error`](file:///workspace/calm-faraday/tests/test_csharp_stack.rs): Injects unknown PackageReference and confirms diagnostic.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e0_6_csharp_stack" --anti-cheat
```
