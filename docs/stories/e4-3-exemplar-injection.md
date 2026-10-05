# Story: Exemplar injection

## Metadata
- **Story Key:** `e4-3-exemplar-injection` (Short: `e4-3`)
- **Epic:** [Optional Gateway](file:///workspace/calm-faraday/docs/epics/epic-4.md)
- **Persona:** Developer
- **Priority:** Could
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e2-5-seeding-from-similar-winners`, `e4-1-transparent-pass-through`

---

## 1. User Story
As a developer, I want relevant past winners offered to the model in ordinary chats, so that everyday answers benefit from what EvoSwarm has proven.

---

## 2. Architectural Context & Invariants
Governed by AD-2. Intercepts incoming messages; queries FalkorDB for similar winners with a strict 20 ms timeout. Prepends up to 2 exemplars (max 2,000 tokens) to the system prompt.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Incoming prompt + FalkorDB query (<20ms)` | `Modified prompt with prepended exemplars` | `If timeout (>20ms), forward unmodified prompt` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a request similar to stored winners above the threshold**, **when forwarded**, **then up to 2 exemplars totalling at most 2k tokens are prepended to the system prompt.**.
- **Given the lookup takes over 20 ms**, **when it times out**, **then the request is forwarded unchanged.**.
- **Given an injection happens**, **when logged**, **then the exemplar IDs are recorded with the request.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/gateway/test_exemplar_injection.rs::test_successful_injection`](file:///workspace/calm-faraday/tests/gateway/test_exemplar_injection.rs): Verifies system prompt contains retrieved exemplar.
- [`tests/gateway/test_exemplar_injection.rs::test_timeout_fallback`](file:///workspace/calm-faraday/tests/gateway/test_exemplar_injection.rs): Simulates 50ms memory delay; asserts unmodified prompt forwarded in <25ms.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e4_3_exemplar_injection" --anti-cheat
```
