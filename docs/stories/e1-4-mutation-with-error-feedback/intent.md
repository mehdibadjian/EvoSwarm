# Story Intent: Mutation with error feedback

## 1. Metadata
- **Story Key:** `e1-4-mutation-with-error-feedback`
- **Epic:** [Epic 1: Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Status:** ready-for-dev

---

## 2. Job-to-be-Done (JTBD)
> When a candidate fails compilation or tests, I want the mutator model provided with the exact compiler errors and failing assertions, so that subsequent mutations converge on passing solutions rather than guessing randomly.

---

## 3. Problem Statement & Architectural Context
Governed by AD-5. Failed candidates contain high-value signal. The compiler output and first failing test assertion (truncated to 4,000 tokens) are packaged into a structured prompt. Prompt caching prefixes are strictly preserved.

---

## 4. Scope Discipline & Boundary
- **In Scope:** Core execution and contracts defined in specification.
- **Out of Scope:** Multi-turn conversational debates with the model during a single mutation.
