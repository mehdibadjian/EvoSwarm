---
name: e2e-tests
description: Generate comprehensive automated integration and end-to-end regression tests verifying full user journeys. Use when adding test coverage for existing APIs, services, or UI components.
---

# E2E Tests: Integration & Regression Test Suite

## Goal
Generate robust, automated integration and end-to-end tests that validate full user journeys and verify system behavior against PRD acceptance criteria.

## Workflow

### 1. Test Matrix Formulation
- Map all user stories and acceptance criteria from `docs/stories/` or PRD.
- Identify the necessary mock fixtures, database seed states, and API endpoints.

### 2. Implementation Standards
- Tests must be black-box oriented: test through public interfaces and APIs, not internal private state.
- Ensure test isolation: each test must clean up its own state or run against ephemeral test containers/sandboxes.
- Cover positive flows, negative error responses (4xx/5xx), and timeout/cancellation scenarios.

### 3. Execution & Verification
- Run tests via the project test runner (e.g. `pytest`, `playwright`, `jest`).
- Verify tests run predictably without flakes.
- Commit tests and verify passing exit code.
