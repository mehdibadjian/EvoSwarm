# Story Intent: Resume after crash

## 1. Metadata
- **Story Key:** `e1-12-resume-after-crash`
- **Epic:** [Epic 1: Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Operator
- **Status:** ready-for-dev

---

## 2. Job-to-be-Done (JTBD)
> When the host box experiences an unexpected reboot or crash mid-search, I want running jobs to resume from their last completed generation without re-spending tokens, so that 24/7 autonomous operations are resilient.

---

## 3. Problem Statement & Architectural Context
Governed by AD-7 (SQLite Job Ledger). SQLite in WAL mode commits completed generations and model call responses with idempotency hashes. On reboot, incomplete sandbox runs are rescheduled; completed model calls are reused.

---

## 4. Scope Discipline & Boundary
- **In Scope:** Core execution and contracts defined in specification.
- **Out of Scope:** Live process memory checkpointing via CRIU.
