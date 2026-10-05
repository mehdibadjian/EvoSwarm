# Story Intent: Patch and report

## 1. Metadata
- **Story Key:** `e1-11-patch-and-report`
- **Epic:** [Epic 1: Evolve CLI and Fitness](../../epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** `e1-6-hard-gates`, `e1-7-weighted-score`
- **Ledger Status:** `backlog` (source of truth: [`sprint-status.yaml`](../../../sprint-status.yaml))

---

## 2. Job-to-be-Done (JTBD)
> When a job finishes, I want the winning patch delivered on a clean git branch alongside a `.patch` file and audit report, so that I can review, inspect lineage, and merge with confidence.

---

## 3. Problem Statement & Architectural Context
Governed by AD-1 and AD-5. Creates branch `evoswarm/<job-id>` from the base commit, writes `.evoswarm/patches/<job-id>.patch`, and generates a markdown audit report. The base branch remains untouched.

---

## 4. Scope Discipline & Boundary
- **In Scope:** The contracts in [`spec.md`](spec.md) and the work order in [`plan.md`](plan.md); nothing beyond them.
- **Out of Scope:** Auto-pushing branches to remote git hosts without user confirmation.
