# Story Intent: Held-out tests

## 1. Metadata
- **Story Key:** `e1-8-held-out-tests`
- **Epic:** [Epic 1: Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Reviewer
- **Status:** ready-for-dev

---

## 2. Job-to-be-Done (JTBD)
> When selecting a winning implementation, I want it evaluated against a hidden 20% slice of trusted tests that models never saw, so that overfitting and prompt memorization are prevented.

---

## 3. Problem Statement & Architectural Context
Governed by AD-5. The user's trusted test suite is partitioned 80/20 at job start using a stable hash seed. Held-out tests are never provided in mutation prompts. The top-scoring candidate must pass 100% of held-out tests.

---

## 4. Scope Discipline & Boundary
- **In Scope:** Core execution and contracts defined in specification.
- **Out of Scope:** Generating artificial held-out tests with LLMs.
