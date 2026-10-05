# Story: Transparent pass-through

## Metadata
- **Story Key:** `e4-1-transparent-pass-through` (Short: `e4-1`)
- **Epic:** [Optional Gateway](../epics/epic-4-optional-gateway.md)
- **Persona:** Developer
- **Priority:** Should
- **Sizing:** L
- **Execution Tier:** `pro`
- **Dependencies:** None

---

## 1. User Story
As a developer, I want Claude Code to behave identically through the gateway, so that adding it never breaks my workflow.

---

## 2. Architectural Context & Invariants
Governed by AD-2. Built on Axum and Tokio. Forwards SSE events byte-for-byte; preserves `tool_use` and `thinking` blocks; passes upstream 4xx/5xx status and headers unmodified; handles client/upstream aborts.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Claude Code HTTP POST /v1/messages` | `Exact SSE stream forwarded with byte-for-byte fidelity` | `Connection aborted cleanly within 1s on client disconnect` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given recorded Claude Code sessions**, **when replayed through the gateway**, **then the SSE event sequence matches the direct response byte for byte.**.
- **Given responses with tool_use and thinking blocks**, **when streamed**, **then every block arrives intact.**.
- **Given an upstream 4xx or 5xx**, **when forwarded**, **then status, body and retry-after headers are unchanged.**.
- **Given either side drops the connection**, **when detected**, **then the other side is closed within 1 s.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/gateway/test_sse_passthrough.rs::test_byte_for_byte_fidelity`: Streams recorded Anthropic SSE stream and compares SHA-256 hash.
- `tests/gateway/test_sse_passthrough.rs::test_thinking_blocks_intact`: Verifies thinking and tool_use blocks preserved.
- `tests/gateway/test_sse_passthrough.rs::test_upstream_error_propagation`: Mocks upstream 429 and asserts retry-after header forwarded.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e4_1_transparent_pass_through" --anti-cheat
```
