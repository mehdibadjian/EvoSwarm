# Story Intent: Mutation with error feedback

## 1. Metadata
- **Story Key:** `e1-4-mutation-with-error-feedback`
- **Epic:** [Epic 1: Evolve CLI and Fitness](../../epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `pro`
- **Dependencies:** `e1-3-seed-the-first-generation`, `e1-6-hard-gates`
- **Ledger Status:** `backlog` (source of truth: [`sprint-status.yaml`](../../../sprint-status.yaml))

---

## 2. Job-to-be-Done (JTBD)
> When a candidate fails compilation or tests, I want the mutator model provided with the exact compiler errors and failing assertions, so that subsequent mutations converge on passing solutions rather than guessing randomly.

---

## 3. Problem Statement & Architectural Context
Governed by AD-5. Failed candidates contain high-value signal. The compiler output and first failing test assertion (truncated to 4,000 tokens) are packaged into a structured prompt. Prompt caching prefixes are strictly preserved.

---

## 4. Scope Discipline & Boundary
- **In Scope:** The contracts in [`spec.md`](spec.md) and the work order in [`plan.md`](plan.md); nothing beyond them.
- **Out of Scope:** Multi-turn conversational debates with the model during a single mutation.
