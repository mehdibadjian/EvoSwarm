# Story: Usage logging

## Metadata
- **Story Key:** `e4-2-usage-logging` (Short: `e4-2`)
- **Epic:** [Optional Gateway](../epics/epic-4-optional-gateway.md)
- **Persona:** Operator
- **Priority:** Should
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e4-1-transparent-pass-through`

---

## 1. User Story
As an operator, I want token usage logged per session and day, so that I know where spend goes.

---

## 2. Architectural Context & Invariants
Extracts `usage` block from completion SSE event; records input, output, and cached tokens to local SQLite table; surfaces via `evoswarm usage --since <date>` without storing prompt contents.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `SSE stream usage metadata` | `Logged usage record in SQLite (session_id, timestamp, in_tokens, out_tokens, cached_tokens)` | `No prompt text stored` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given any request**, **when it completes**, **then input, output and cached token counts are logged locally with no prompt content by default.**.
- **Given `evoswarm usage --since <date>`**, **when run**, **then it prints totals and cost by day.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/gateway/test_usage_logging.rs::test_usage_record_persistence`: Verifies tokens logged to SQLite.
- `tests/gateway/test_usage_logging.rs::test_evoswarm_usage_cli`: Executes `evoswarm usage` and verifies daily cost summary.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e4_2_usage_logging" --anti-cheat
```
