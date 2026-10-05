# Story Intent: Benchmark suite

## 1. Metadata
- **Story Key:** `e1-13-benchmark-suite`
- **Epic:** [Epic 1: Evolve CLI and Fitness](../../epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Team Lead
- **Priority:** Must
- **Sizing:** L
- **Execution Tier:** `pro`
- **Dependencies:** `e1-11-patch-and-report`
- **Ledger Status:** `backlog` (source of truth: [`sprint-status.yaml`](../../../sprint-status.yaml))

---

## 2. Job-to-be-Done (JTBD)
> When evaluating whether evolutionary code search pays off, I want a standardized 30-task benchmark comparing EvoSwarm against single-shot generation at equal token budget, so that investment is justified by empirical data.

---

## 3. Problem Statement & Architectural Context
Mandatory Exit Gate (Gate 1). The benchmark consists of 30 tasks (at least 10 Python, 10 C#) with trusted tests and held-out slices. EvoSwarm must achieve a solve rate $\ge 15$ points higher than single-shot with test feedback.

---

## 4. Scope Discipline & Boundary
- **In Scope:** The contracts in [`spec.md`](spec.md) and the work order in [`plan.md`](plan.md); nothing beyond them.
- **Out of Scope:** Continuous benchmarking on every minor commit.
