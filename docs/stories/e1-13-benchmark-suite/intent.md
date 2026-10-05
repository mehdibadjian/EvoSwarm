# Story Intent: Benchmark suite

## 1. Metadata
- **Story Key:** `e1-13-benchmark-suite`
- **Epic:** [Epic 1: Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Team Lead
- **Status:** ready-for-dev

---

## 2. Job-to-be-Done (JTBD)
> When evaluating whether evolutionary code search pays off, I want a standardized 30-task benchmark comparing EvoSwarm against single-shot generation at equal token budget, so that investment is justified by empirical data.

---

## 3. Problem Statement & Architectural Context
Mandatory Exit Gate (Gate 1). The benchmark consists of 30 tasks (at least 10 Python, 10 C#) with trusted tests and held-out slices. EvoSwarm must achieve a solve rate $\ge 15$ points higher than single-shot with test feedback.

---

## 4. Scope Discipline & Boundary
- **In Scope:** Core execution and contracts defined in specification.
- **Out of Scope:** Continuous benchmarking on every minor commit.
