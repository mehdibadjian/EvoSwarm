# Story: Lineage CLI

## Metadata
- **Story Key:** `e2-8-lineage-cli` (Short: `e2-8`)
- **Epic:** [System 1 Memory and Replay](../epics/epic-2-system-1-memory-and-replay.md)
- **Persona:** Developer
- **Priority:** Could
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e2-2-lineage-written-to-falkordb`

---

## 1. User Story
As a developer, I want to see how a winner was reached, so that I can trust or debug the result.

---

## 2. Architectural Context & Invariants
CLI command `evoswarm lineage <job> [--json]` traverses graph ancestry back to Gen 0 drafts and prints formatted ASCII tree or JSON.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Job ID` | `ASCII lineage tree (generation, model, score, failure reasons)` | `JSON object if --json specified` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a job ID**, **when I run `evoswarm lineage <job>`**, **then it prints the winner's ancestry with generation, model, score and gate failure reasons.**.
- **Given `--json`**, **when run**, **then the same data prints as JSON.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- `tests/cli/test_lineage_cli.rs::test_ascii_tree_rendering`: Validates formatted output contains all ancestors.
- `tests/cli/test_lineage_cli.rs::test_json_lineage_export`: Validates JSON schema matches expected properties.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e2_8_lineage_cli" --anti-cheat
```
