# Story Intent: Start a job from the CLI

## 1. Metadata
- **Story Key:** `e1-1-start-a-job-from-the-cli`
- **Epic:** [Epic 1: Evolve CLI and Fitness](../../epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** `e0-5-python-stack`, `e0-6-csharp-stack`
- **Ledger Status:** `backlog` (source of truth: [`sprint-status.yaml`](../../../sprint-status.yaml))

---

## 2. Job-to-be-Done (JTBD)
> When I have a task backed by failing or benchmark tests, I want to submit it via `evoswarm run` in a single command, so that EvoSwarm validates the task baseline and searches for a verified passing patch in the background.

---

## 3. Problem Statement & Architectural Context
Governed by AD-7 (SQLite Job Ledger) and AD-1 (Sandbox Isolation). The CLI must validate inputs, verify baseline reproducibility across 3 runs to prevent flaky tests, check that tests aren't already passing without a perf objective, and register the job in SQLite.

---

## 4. Scope Discipline & Boundary
- **In Scope:** The contracts in [`spec.md`](spec.md) and the work order in [`plan.md`](plan.md); nothing beyond them.
- **Out of Scope:** Interactive TUI dashboards, distributed cluster orchestration, or multi-repo workspaces.
