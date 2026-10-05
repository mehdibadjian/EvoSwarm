---
description: System architectural invariants, boundary enforcement, and ADR guidelines
trigger: always_on
---

# System Architecture Invariants & Boundaries

## 1. Architectural Spine Standard
- An architecture defines **invariants** that prevent independently built components from diverging.
- Use the **Divergence Test**: If two subagents built two parts of the system independently, could they make incompatible choices? Fix it in the architecture only if the answer is yes, non-obvious, and represents a real trade-off.

## 2. Invariant Tracking (ADRs)
- Decisions must be recorded as stable Architecture Decisions (`AD-n`) with clear Binds, Prevents, and Rule specifications.
- Once adopted, architectural invariants are non-negotiable constraints unless explicitly revisited through an architectural review.

## 3. Dependency Discipline
- High-level domain logic must never depend directly on volatile external infrastructure details.
- Use dependency inversion and modular interfaces to decouple core logic from third-party APIs or storage backends.
