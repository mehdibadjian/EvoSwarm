---
name: architecture
description: Produce the architecture spine of non-negotiable invariants, component boundaries, and Architecture Decision Records (ADRs). Use when designing systems, planning technical infrastructure, or setting up architectural standards.
---

# Architecture Spine & System Design

## Goal
Establish the **architecture spine**: a durable consistency contract that fixes the invariants keeping independently built modules from diverging.

## The Divergence Test
> If two developers or subagents built two units independently, could they make incompatible choices? Fix it here only if the answer is **yes**, the decision is **non-obvious**, and it involves a **real trade-off**.

## Workflow

### 1. Context & Invariant Discovery
- Identify the core design paradigm (e.g. event-driven, modular monolith, hexagonal architecture).
- Define module boundaries, data ownership, and state mutation models.
- Determine external interfaces and dependency inversion rules.

### 2. Architecture Decisions (ADRs)
Document each critical decision in `docs/architecture/ARCHITECTURE-SPINE.md` with stable IDs:
- **`AD-n: Title`**
  - **Context**: Problem statement and alternatives considered.
  - **Decision**: The selected pattern or constraint.
  - **Binds**: What downstream builders must obey.
  - **Prevents**: The divergence this prevents.
  - **Status**: Proposed | Adopted | Superseded

### 3. Diagramming
- Express system topologies, data flows, and state machines using Mermaid diagrams.
- Avoid vague descriptive prose where a crisp diagram provides definitive structure.
