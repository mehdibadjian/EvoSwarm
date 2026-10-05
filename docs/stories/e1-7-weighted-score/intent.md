# Story Intent: Weighted score

## 1. Metadata
- **Story Key:** `e1-7-weighted-score`
- **Epic:** [Epic 1: Evolve CLI and Fitness](../../epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e1-6-hard-gates`
- **Ledger Status:** `backlog` (source of truth: [`sprint-status.yaml`](../../../sprint-status.yaml))

---

## 2. Job-to-be-Done (JTBD)
> When multiple candidates pass all hard gates, I want them ranked by a deterministic, multi-objective score function, so that the highest quality, highest performance, and most concise patch wins.

---

## 3. Problem Statement & Architectural Context
Governed by AD-5. Calculates $S = w_a A + w_p P + w_s Z$ with default weights $w_a = 0.5, w_p = 0.3, w_s = 0.2$. Weights must sum to 1.0. If adversary tests are absent, $w_a$ is redistributed proportionally.

---

## 4. Scope Discipline & Boundary
- **In Scope:** The contracts in [`spec.md`](spec.md) and the work order in [`plan.md`](plan.md); nothing beyond them.
- **Out of Scope:** Dynamic weight rebalancing during a running generation.
