---
name: deep-recon
description: Decision-grade technical, domain, and competitive research with verified sources and citations. Use when gathering intelligence, exploring technical spikes, or evaluating ecosystem solutions.
---

# Deep Recon: Intelligence & Research

## Goal
Conduct grounded, decision-grade research across technical, domain, or architectural dimensions, citing evidence from codebases and verified documentation.

## Workflow

### 1. Scope Formulation
- Clarify the core technical hypothesis or architectural question.
- Define what constitutes evidence (documentation, source code, benchmarks, specifications).

### 2. Investigation & Retrieval
- **Codebase Exploration**: Search local repositories for existing patterns, conventions, and constraints.
- **Web & Official Documentation**: Retrieve authoritative specifications, release notes, and reference implementations.
- Distinguish between verified facts and assumptions.

### 3. Synthesis & Trade-Off Analysis
- Structure findings into an executive decision brief:
  - **Summary & Recommendation**: The direct, actionable answer.
  - **Trade-Off Matrix**: Options compared by complexity, maintenance cost, and performance.
  - **Evidence & Citations**: Direct links to documentation and code snippets.
  - **Risks & Open Questions**: Unresolved unknowns to de-risk later.

### 4. Artifact Generation
- Output the brief to `docs/research/<topic>-brief.md` or present as an Antigravity markdown artifact.
