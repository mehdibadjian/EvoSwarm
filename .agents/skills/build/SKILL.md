---
name: build
description: Autonomous TDD implementation engine. Writes failing tests, implements minimal working code, refactors cleanly, and verifies acceptance criteria. Use when implementing a story, fixing a bug, or building a feature.
---

# Build: Test-Driven Implementation

## Goal
Implement approved stories with test-first discipline (Red, Green, Refactor), producing verified code that meets every acceptance criterion without AI comment slop.

## Execution Workflow

### 1. Story Ingestion & Pre-Flight Plan
- Read the story specification from `docs/stories/<story-key>.md` (or `intent.md` / `spec.md`).
- Inspect existing codebase patterns, architecture invariants, and past mistakes in `.agents/rules/lessons-learned.md`.
- **Pre-Flight Plan (`plan.md`):** Author `plan.md` detailing the test files to write and code files to edit. **Do NOT touch production code until `plan.md` is committed.**
- Update sprint status to `in-progress` via `python3 scripts/sprint.py update <story-key> --status in-progress`.

### 2. Red Phase (Failing Test)
- Author tests mapping to the story's Acceptance Criteria and Input/Output matrix.
- Run the test suite using `python3 scripts/sprint.py verify --cmd "<test-cmd>" --anti-cheat`.
- **Verify that the test fails** specifically due to the absence of the feature.

### 3. Green Phase (Minimal Implementation)
- Write the minimal code required to satisfy the failing test.
- Run the test suite again.
- **Verify that all tests pass cleanly.**

### 4. Refactor & Verification
- Clean up code structure, ensure idiomatic patterns, eliminate duplication.
- Ensure no AI metadata comments or temporary scaffolding remains.
- Run `python3 scripts/sprint.py verify --cmd "<test-cmd>" --anti-cheat` to confirm clean pass and zero test tampering.

### 5. Atomic Commit & Hand-off
- Commit changes: `git commit -m "feat(<scope>): implement <story-key>"`.
- Advance sprint status to `review`: `python3 scripts/sprint.py update <story-key> --status review`.
