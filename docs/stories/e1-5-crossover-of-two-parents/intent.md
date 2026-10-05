# Story Intent: Crossover of two parents

## 1. Metadata
- **Story Key:** `e1-5-crossover-of-two-parents`
- **Epic:** [Epic 1: Evolve CLI and Fitness](../../epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Priority:** Should
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e1-4-mutation-with-error-feedback`
- **Ledger Status:** `backlog` (source of truth: [`sprint-status.yaml`](../../../sprint-status.yaml))

---

## 2. Job-to-be-Done (JTBD)
> When two candidates pass different subsets of the test suite, I want a synthesiser model to recombine their complementary strengths, so that partial fixes can be merged into a comprehensive solution.

---

## 3. Problem Statement & Architectural Context
Governed by AD-6. Crossover pairs are drawn preferentially from candidates that pass disjoint test partitions. Crossover calls use the reasoning synthesiser model and are capped at 25% of total job calls.

---

## 4. Scope Discipline & Boundary
- **In Scope:** The contracts in [`spec.md`](spec.md) and the work order in [`plan.md`](plan.md); nothing beyond them.
- **Out of Scope:** Synthesising candidates without test provenance data.
