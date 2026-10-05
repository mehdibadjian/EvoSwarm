# Story: Injection opt-out

## Metadata
- **Story Key:** `e4-4-injection-opt-out` (Short: `e4-4`)
- **Epic:** [Optional Gateway](file:///workspace/calm-faraday/docs/epics/epic-4.md)
- **Persona:** Developer
- **Priority:** Should
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e4-3-exemplar-injection`

---

## 1. User Story
As a developer, I want to switch injection off, so that I control when extra context is added.

---

## 2. Architectural Context & Invariants
Supports header `x-evoswarm-inject: off` or config flag `gateway.inject = false`. Bypasses memory lookup entirely while retaining token usage logging.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Request with header x-evoswarm-inject: off` | `Forwarded unmodified, usage logged, 0ms memory lookup` | `Config flag achieves identical result` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given header `x-evoswarm-inject: off` or the config flag**, **when a request passes through**, **then nothing is injected.**.
- **Given injection is off**, **when the request completes**, **then usage is still logged.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/gateway/test_opt_out.rs::test_header_opt_out`](file:///workspace/calm-faraday/tests/gateway/test_opt_out.rs): Sends `x-evoswarm-inject: off` and confirms zero injection.
- [`tests/gateway/test_opt_out.rs::test_config_flag_opt_out`](file:///workspace/calm-faraday/tests/gateway/test_opt_out.rs): Sets `inject = false` in config and confirms bypass.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e4_4_injection_opt_out" --anti-cheat
```
