# Story: Setup guide

## Metadata
- **Story Key:** `e3-6-setup-guide` (Short: `e3-6`)
- **Epic:** [Claude Code Integration (MCP)](../epics/epic-3-claude-code-integration-mcp.md)
- **Persona:** Developer
- **Priority:** Should
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e3-1-evolve-tool`, `e3-2-job-status-tool`, `e3-3-job-result-tool`, `e3-4-cancel-job-tool`

---

## 1. User Story
As a developer, I want a short setup guide, so that I can run my first job within 15 minutes.

---

## 2. Architectural Context & Invariants
Documents step-by-step setup in README and `docs/guides/mcp-setup.md`, including `claude mcp add` config snippet and troubleshooting fixes for E0-2 self-check failures.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Developer follows guide on clean machine` | `EvoSwarm running, MCP configured, sample job completes in <15 minutes` | `Documented fixes for common AppArmor / linger issues` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a fresh machine**, **when I follow the README**, **then the sample job completes from Claude Code.**.
- **Given the README**, **when read**, **then it includes the Claude Code MCP config snippet and a fix for each E0-2 self-check failure.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/docs/test_quickstart_doc.py::test_claude_config_json_syntax`: Validates MCP config snippet parses as valid JSON.
- `tests/docs/test_quickstart_doc.py::test_commands_executable`: Dry-runs CLI commands described in guide.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e3_6_setup_guide" --anti-cheat
```
