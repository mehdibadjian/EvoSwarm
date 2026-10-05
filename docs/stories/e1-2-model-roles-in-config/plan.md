# Implementation Plan: Model roles in config

**Story:** `e1-2-model-roles-in-config` · **Sizing:** S · **Tier:** `flash` · **Target crate:** `evoswarm-models`
**Depends on:** None — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

## 1. Pre-Flight Architecture & Rule Check
Complete these before writing code; tick each only when it actually holds.

- [ ] Reviewed [`ARCHITECTURE-SPINE.md`](../../architecture/ARCHITECTURE-SPINE.md) and confirmed this story upholds its invariants: Governed by AD-6 (Model Role Segregation).
- [ ] Confirmed compliance with [`architecture-rules.md`](../../../.agents/rules/architecture-rules.md), [`tdd-discipline.md`](../../../.agents/rules/tdd-discipline.md) and [`security-hygiene.md`](../../../.agents/rules/security-hygiene.md).
- [ ] Confirmed every dependency listed above is `done`; otherwise stop and report the blocker.
- [ ] Confirmed scope matches [`spec.md`](spec.md) section 2 and nothing outside it is being built.
- [ ] Committed to zero AI comments in production code: comments explain **WHY**, never **WHAT**.

---

## 2. Red Phase (Failing Tests to Author First)
Author these tests before any production code, then run the gate command in section 5 and
confirm every test fails for its intended reason. A test that passes before implementation
is a false-positive test and must be rewritten.

- `crates/evoswarm-models/tests/e1_2_model_roles_in_config.rs::test_parse_valid_role_configuration` — Verifies all three roles deserialize with correct token caps, temperatures and cost fields.
- `crates/evoswarm-models/tests/e1_2_model_roles_in_config.rs::test_startup_failure_on_invalid_role` — Asserts a startup error naming the role when a mandatory role is omitted or its `model_id` is empty.
- `crates/evoswarm-models/tests/e1_2_model_roles_in_config.rs::test_sighup_atomic_swap` — Sends SIGHUP after editing temperature and asserts the next read returns the new value.
- `crates/evoswarm-models/tests/e1_2_model_roles_in_config.rs::test_sighup_rejects_invalid_config` — Sends SIGHUP with malformed TOML and asserts the prior config is still served with no panic.

**Target file(s):** `crates/evoswarm-models/tests/e1_2_model_roles_in_config.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | each dispatch uses its configured role settings | `test_parse_valid_role_configuration` |
| AC2 | it fails loudly with the offending role named | `test_startup_failure_on_invalid_role` |
| AC3 | the new settings apply seamlessly without daemon restart | `test_sighup_atomic_swap` |
| AC4 | the previous config stays active and the daemon remains healthy | `test_sighup_rejects_invalid_config` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `crates/evoswarm-models/src/config.rs` — `ModelConfig`, `RoleConfig`, `Role` enum (`Mutator | Synthesiser | Adversary`).
- `crates/evoswarm-models/src/config.rs` — `pub fn load(path: &Path) -> Result<ModelConfig, ConfigError>` and `ModelConfig::validate(&self)`.
- `crates/evoswarm-models/src/hot_reload.rs` — `pub struct ConfigHandle(ArcSwap<ModelConfig>)` with `current()` and `reload(path)`.
- `crates/evoswarm-models/src/error.rs` — `ConfigError::{MissingRole, InvalidField, Parse}` carrying role and field names.

### Work order
1. Define `RoleConfig` with serde rename to snake_case TOML keys and `deny_unknown_fields` so typos fail loudly.
2. Implement `validate()` returning the first error with the role name interpolated; call it from `load()` so no invalid config can reach the handle.
3. Introduce `arc-swap` and wrap the config in `ConfigHandle`; `reload()` parses into a temp, validates, then swaps.
4. Register a `tokio::signal::unix::SignalKind::hangup()` task that calls `reload()` and logs the error without propagating it.
5. Assert in tests that `current()` returns an owned `Arc` snapshot, so a concurrent swap cannot tear a call in progress.

### Implementation note
Validation runs on every reload, not only at startup — an unvalidated swap is how a typo silently disables budget accounting downstream.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-models` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-models --test e1_2_model_roles_in_config" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-2-model-roles-in-config --status done
```
