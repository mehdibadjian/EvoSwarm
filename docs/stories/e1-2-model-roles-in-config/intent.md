# Story Intent: Model roles in config

## 1. Metadata
- **Story Key:** `e1-2-model-roles-in-config`
- **Epic:** [Epic 1: Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** Operator
- **Status:** ready-for-dev

---

## 2. Job-to-be-Done (JTBD)
> When operating EvoSwarm on changing budget or API tier conditions, I want to map distinct model roles (`mutator`, `synthesiser`, `adversary`) in a TOML config file with hot-reload, so that I can optimize token costs without restarting the daemon or recompiling.

---

## 3. Problem Statement & Architectural Context
Governed by AD-6 (Model Role Segregation). EvoSwarm allocates 75% of calls to fast/cheap mutators and reserves reasoning models for crossover and adversary generation. Dynamic SIGHUP reload prevents interrupting running jobs.

---

## 4. Scope Discipline & Boundary
- **In Scope:** Core execution and contracts defined in specification.
- **Out of Scope:** Automated bidding on spot LLM auctions, dynamically switching cloud providers mid-generation.
