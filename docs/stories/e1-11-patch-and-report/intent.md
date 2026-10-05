# Story Intent: Patch and report

## 1. Metadata
- **Story Key:** `e1-11-patch-and-report`
- **Epic:** [Epic 1: Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Status:** ready-for-dev

---

## 2. Job-to-be-Done (JTBD)
> When a job finishes, I want the winning patch delivered on a clean git branch alongside a `.patch` file and audit report, so that I can review, inspect lineage, and merge with confidence.

---

## 3. Problem Statement & Architectural Context
Governed by AD-1 and AD-5. Creates branch `evoswarm/<job-id>` from base commit, writes `.evoswarm/patches/<job-id>.patch`, and generates markdown audit report. Base branch remains untouched.

---

## 4. Scope Discipline & Boundary
- **In Scope:** Core execution and contracts defined in specification.
- **Out of Scope:** Auto-pushing branches to remote git hosts without user confirmation.
