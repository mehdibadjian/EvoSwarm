# Story: Latency budget in CI

## Metadata
- **Story Key:** `e4-5-latency-budget-in-ci` (Short: `e4-5`)
- **Epic:** [Optional Gateway](../epics/epic-4-optional-gateway.md)
- **Persona:** Operator
- **Priority:** Should
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** `e4-1-transparent-pass-through`

---

## 1. User Story
As an operator, I want gateway latency tested continuously, so that it never slows the IDE.

---

## 2. Architectural Context & Invariants
Automated load test running 20 concurrent streams measuring added TTFT. Blocks CI if p95 added latency exceeds 50 ms.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `20 concurrent stream load test` | `p50 and p95 added TTFT reported` | `CI failure if p95 > 50ms` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a load test with 20 concurrent streams**, **when run**, **then p50 and p95 added time-to-first-token are reported.**.
- **Given p95 exceeds 50 ms**, **when CI runs**, **then the build fails.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/gateway/test_latency_benchmark.rs::test_20_stream_ttft`: Executes 20 concurrent SSE streams and asserts p95 added TTFT < 50ms.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e4_5_latency_budget_in_ci" --anti-cheat
```
