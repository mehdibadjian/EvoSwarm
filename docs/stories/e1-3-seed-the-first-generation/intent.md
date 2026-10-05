# Story Intent: Seed the first generation

## 1. Metadata
- **Story Key:** `e1-3-seed-the-first-generation`
- **Epic:** [Epic 1: Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Status:** ready-for-dev

---

## 2. Job-to-be-Done (JTBD)
> When starting a search, I want Generation 0 populated with diverse initial attempts alongside the current code baseline, so that the evolutionary loop explores multiple distinct conceptual paths.

---

## 3. Problem Statement & Architectural Context
Governed by AD-5 and AD-6. Population size N (default 6) is seeded by: (1) baseline code, (2) retrieved memory winners if available (up to 2), and (3) N-1 (or N-3) fresh drafts from the mutator model.

---

## 4. Scope Discipline & Boundary
- **In Scope:** Core execution and contracts defined in specification.
- **Out of Scope:** Synthesising candidates using third-party web search or unverified external snippets.
