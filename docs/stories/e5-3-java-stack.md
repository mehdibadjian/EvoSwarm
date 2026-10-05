# Story: Java stack

## Metadata
- **Story Key:** `e5-3-java-stack` (Short: `e5-3`)
- **Epic:** [MAP-Elites and Multi-Stack Expansion](../epics/epic-5-map-elites-and-more-stacks.md)
- **Persona:** Developer
- **Priority:** Could
- **Sizing:** L
- **Execution Tier:** `pro`
- **Dependencies:** `e0-1-sandbox-runner-interface`, `e0-4-resource-limits`

---

## 1. User Story
As a developer, I want Java tasks tested with JUnit or Cucumber, so that legacy monoliths can be evolved safely.

---

## 2. Architectural Context & Invariants
Governed by AD-1. Prepares `.m2` repository snapshot keyed by pom.xml hash. Runs `mvn -o test` inside bwrap with no network, read-only JVM mounts (`/usr/lib/jvm`), and 2GB memory cap.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Maven project + candidate patch` | `Parsed JUnit / Cucumber test summary` | `Fails loudly if network requested or snapshot missing` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a pom.xml**, **when the first job runs**, **then a `.m2` snapshot is built outside the sandbox keyed by its hash.**.
- **Given the snapshot**, **when `mvn -o test` runs with no network**, **then JUnit and Cucumber samples pass.**.
- **Given the Java profile**, **when the red-team suite and E0-9 calibration run**, **then all attempts fail and limits are recorded.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/stacks/test_java_stack.rs::test_m2_snapshot_caching`: Caches m2 snapshot by pom.xml hash.
- `tests/stacks/test_java_stack.rs::test_mvn_offline_execution`: Runs `mvn -o test` in bwrap with JUnit test parsing.
- `tests/stacks/test_java_stack.rs::test_java_red_team_containment`: Executes red-team suite in Java sandbox.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e5_3_java_stack" --anti-cheat
```
