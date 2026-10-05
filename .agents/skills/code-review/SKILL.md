---
name: code-review
description: Multi-lens adversarial code review covering architectural invariants, logic flaws, test coverage, and security. Use when reviewing pull requests, feature branches, or diffs before merging.
---

# Code Review: Multi-Lens Adversarial Evaluation

## Goal
Adversarially evaluate code changes before merge, ensuring high architectural consistency, zero regressions, and robust error handling.

## The 4 Review Lenses

### 1. Architectural Integrity Lens
- Does this change violate any invariants in `.agents/rules/architecture-rules.md` or ADRs?
- Are component boundaries respected, or does domain logic leak across layers?

### 2. Logic & Edge Case Hunter
- Are boundary conditions, null/empty states, and unexpected inputs handled?
- Are there off-by-one errors, race conditions, or unhandled promise rejections?

### 3. Test Completeness & Verification Lens
- Does a dedicated test cover every newly added branch or logic path?
- Were tests executed and observed passing? Does the test suite assert outcomes rather than implementation details?

### 4. Security & Hygiene Lens
- Are any hardcoded credentials, API keys, or private tokens present?
- Is all user-supplied input sanitized against injection attacks?

## Verdicts & Output
Present findings structured as:
- **Blockers (Must Fix)**: Hard issues that prevent merge.
- **Recommendations (Should Fix)**: Cleanliness, performance, or readability suggestions.
- **Verdict**: `APPROVE` or `REQUEST_CHANGES`.
- On approval, transition story to `done` via `python3 scripts/sprint.py update <story-key> --status done`.
