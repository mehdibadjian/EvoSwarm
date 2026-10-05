---
name: retrospective
description: Conduct evidence-based epic retrospectives, analyze git churn and verification gaps, and refine project rules and invariants. Use when completing an epic or sprint.
---

# Retrospective: Epic Review & Invariant Refinement

## Goal
Conduct a data-driven retrospective upon completing an epic, measure verification accuracy, identify bottlenecks, and update project rules to prevent recurring defects.

## Workflow

### 1. Evidence Collection
- Collect git statistics over the epic's commit range:
  - Commit count and atomic commit cleanliness.
  - Churn (lines added / lines removed) per module.
  - Review feedback cycles and test failure rates.

### 2. Verification Gap Analysis
- Compare PRD acceptance criteria against the delivered code and tests.
- Identify any gaps where criteria were claimed "done" without automated tests.

### 3. Rule Refinement & Continuous Learning
- Identify recurring issues, verification gaps, or developer friction points.
- Codify lessons learned directly into `.agents/rules/lessons-learned.md` so future agent sessions automatically inherit them.
- Close the epic in `sprint-status.yaml` by marking `<epic-key>: done` and `<epic-key>-retrospective: done`.
