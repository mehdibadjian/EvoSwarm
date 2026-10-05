# Story Intent: Budget enforcement

## 1. Metadata
- **Story Key:** `e1-10-budget-enforcement`
- **Epic:** [Epic 1: Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Status:** ready-for-dev

---

## 2. Job-to-be-Done (JTBD)
> When running evolutionary jobs, I want strict token, dollar, and generation early-stopping caps enforced before every model dispatch, so that long-running searches never exceed agreed costs.

---

## 3. Problem Statement & Architectural Context
Governed by AD-6. Checks projected costs against budget caps prior to network dispatch. Early stop terminates if no score improvement across 2 consecutive generations. Emits status `budget_exhausted` and returns best verified candidate.

---

## 4. Scope Discipline & Boundary
- **In Scope:** Core execution and contracts defined in specification.
- **Out of Scope:** Dynamic credit card charging or billing API integrations.
