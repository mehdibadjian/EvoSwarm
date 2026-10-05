# Story Intent: Hard gates

## 1. Metadata
- **Story Key:** `e1-6-hard-gates`
- **Epic:** [Epic 1: Evolve CLI and Fitness](../../epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Reviewer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e0-7-tamper-proof-tests`
- **Ledger Status:** `backlog` (source of truth: [`sprint-status.yaml`](../../../sprint-status.yaml))

---

## 2. Job-to-be-Done (JTBD)
> When evaluating any candidate code patch, I want 4 non-negotiable hard gates enforced, so that no candidate that games tests or tampers with harnesses can ever be scored or returned.

---

## 3. Problem Statement & Architectural Context
Governed by AD-5. Gating is evaluated prior to scoring: (1) Clean build, (2) Visible trusted tests pass, (3) Zero diffs on tests/harnesses, (4) Zero skips/deletions relative to baseline. Any failure sets score $S = 0$.

---

## 4. Scope Discipline & Boundary
- **In Scope:** The contracts in [`spec.md`](spec.md) and the work order in [`plan.md`](plan.md); nothing beyond them.
- **Out of Scope:** Subjective aesthetic code formatting gating.
