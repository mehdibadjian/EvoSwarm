# Story: Context guard

## Metadata
- **Story Key:** `e3-5-context-guard` (Short: `e3-5`)
- **Epic:** [Claude Code Integration (MCP)](../epics/epic-3-claude-code-integration-mcp.md)
- **Persona:** Security Reviewer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e1-3-seed-the-first-generation`

---

## 1. User Story
As a security reviewer, I want secrets kept out of every prompt, so that nothing sensitive leaves the box.

---

## 2. Architectural Context & Invariants
Filters outgoing prompt context against per-job path allowlist. Scans contents with regex for private keys (`BEGIN PRIVATE KEY`), AWS/OpenAI tokens, and excludes `.env` and `*.pem`.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Raw repository context` | `Sanitized prompt payload with redacted secrets logged` | `Rejection if critical credentials detected` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a per-job path allowlist**, **when prompts are built**, **then no file outside it is included.**.
- **Given content matching key patterns such as API keys or private keys**, **when a prompt is built**, **then the match is redacted and the redaction logged.**.
- **Given default settings**, **when a job runs**, **then `.env` and `*.pem` files are excluded.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/security/test_context_guard.rs::test_path_allowlist_enforcement`: Ensures files outside allowlist excluded.
- `tests/security/test_context_guard.rs::test_secret_redaction`: Injects mock API key and verifies replacement with `[REDACTED_SECRET]`.
- `tests/security/test_context_guard.rs::test_env_pem_exclusion`: Places `.env` and `cert.pem` in repo; asserts absent from prompt.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e3_5_context_guard" --anti-cheat
```
