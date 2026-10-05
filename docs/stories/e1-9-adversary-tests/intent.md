# Story Intent: Adversary tests

## 1. Metadata
- **Story Key:** `e1-9-adversary-tests`
- **Epic:** [Epic 1: Evolve CLI and Fitness](../../epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Priority:** Should
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e1-6-hard-gates`
- **Ledger Status:** `backlog` (source of truth: [`sprint-status.yaml`](../../../sprint-status.yaml))

---

## 2. Job-to-be-Done (JTBD)
> When evolving code, I want an adversary model writing edge-case tests against the task specification, so that fragile candidates that pass minimal suites are probed and ranked lower.

---

## 3. Problem Statement & Architectural Context
Governed by AD-5. Adversary model drafts $K$ candidate tests. Broken tests that do not compile are discarded. Tests failing across baseline and all candidates are tagged `suspect`. Adversary tests never fail gates.

---

## 4. Scope Discipline & Boundary
- **In Scope:** The contracts in [`spec.md`](spec.md) and the work order in [`plan.md`](plan.md); nothing beyond them.
- **Out of Scope:** Adversary tests failing hard gates or blocking build verification.
