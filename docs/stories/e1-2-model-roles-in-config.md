# Story: Model roles in config

## Metadata
- **Story Key:** `e1-2-model-roles-in-config` (Short: `e1-2`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Operator
- **Priority:** Must
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** None

---

## 1. User Story
As an operator, I want model roles set in config, so that I can swap models or cut cost without a rebuild.

---

## 2. Architectural Context & Invariants
Governed by AD-6. Configuration file `config.toml` binds roles `mutator`, `synthesiser`, and `adversary` to model IDs, max_tokens, and temperatures. Supports SIGHUP hot-reload.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `config.toml with valid roles` | `Service runs with mapped models` | `Error on unknown model ID or invalid temperature` |
| `SIGHUP sent to daemon` | `Next generation reads updated config` | `Config reload log entry` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given config maps `mutator`, `synthesiser` and `adversary` to a model ID, max tokens and temperature**, **when a job runs**, **then each call uses its role's settings.**.
- **Given an unknown model ID**, **when the service starts**, **then it fails with the offending role named.**.
- **Given I edit config and send SIGHUP**, **when the next generation starts**, **then the new settings apply.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/config/test_model_roles.rs::test_load_valid_config`](file:///workspace/calm-faraday/tests/config/test_model_roles.rs): Verifies config parser deserializes roles.
- [`tests/config/test_model_roles.rs::test_reject_unknown_model`](file:///workspace/calm-faraday/tests/config/test_model_roles.rs): Injects invalid model name and expects startup panic with role name.
- [`tests/config/test_model_roles.rs::test_sighup_reload`](file:///workspace/calm-faraday/tests/config/test_model_roles.rs): Sends SIGHUP and asserts modified temperature takes effect.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_2_model_roles_in_config" --anti-cheat
```
