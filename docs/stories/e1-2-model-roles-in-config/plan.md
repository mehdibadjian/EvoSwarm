# Implementation Plan: Model roles in config

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
- [`tests/config/test_model_config.rs::test_parse_valid_role_configuration`](file:///workspace/calm-faraday/tests/config/test_model_config.rs): Verifies all roles deserialized with correct token caps and temperatures.
- [`tests/config/test_model_config.rs::test_startup_failure_on_invalid_role`](file:///workspace/calm-faraday/tests/config/test_model_config.rs): Asserts fatal error when mandatory role is omitted or model ID is empty.
- [`tests/config/test_model_config.rs::test_sighup_atomic_swap`](file:///workspace/calm-faraday/tests/config/test_model_config.rs): Sends SIGHUP with updated temperature and verifies subsequent query returns new value.

---

## 3. Green Phase (Minimal Production Code)
Implement the minimal logic in target crates/modules to satisfy tests:
- Define core structs and traits.
- Implement error handling and bounds checking.
- Connect persistence / CLI / sandbox dispatch.

---

## 4. Refactor Phase & Verification Gate
- Remove any redundant allocations or temporary scaffolding.
- Ensure all comments explain **WHY**, not **WHAT**.
- Execute verification gate:
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_2_model_roles_in_config" --anti-cheat
```
