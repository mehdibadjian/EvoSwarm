# Story Intent: Crossover of two parents

## 1. Metadata
- **Story Key:** `e1-5-crossover-of-two-parents`
- **Epic:** [Epic 1: Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Status:** ready-for-dev

---

## 2. Job-to-be-Done (JTBD)
> When two candidates pass different subsets of the test suite, I want a synthesiser model to recombine their complementary strengths, so that partial fixes can be merged into a comprehensive solution.

---

## 3. Problem Statement & Architectural Context
Governed by AD-6. Crossover pairs are drawn preferentially from candidates that pass disjoint test partitions. Crossover calls use the reasoning synthesiser model and are capped at 25% of total job calls.

---

## 4. Scope Discipline & Boundary
- **In Scope:** Core execution and contracts defined in specification.
- **Out of Scope:** Synthesising candidates without test provenance data.
