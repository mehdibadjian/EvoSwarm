---
name: grill-me
description: Relentless adversarial grill session and design-tree interview to stress-test ideas before committing to PRD or code. Combines the Fortress Architect vs. Velocity King duel with frontier round-based interrogation.
---

# 🎭 The Grill Session: The Duel of Agents & Design-Tree Frontier

## Overview
A relentless pre-flight interrogation to stress-test plans, feature proposals, or project concepts before committing to PRD, Architecture, or implementation.

It maps the concept as a **design tree** where every decision branches into the decisions that hang off it, and subjects it to an adversarial dialectic between two opposing philosophies:
- **Agent A (The Fortress Builder):** The Scale & Security Architect. Motto: *"If it doesn’t scale gracefully under a 10x load, it’s broken before it’s even coded."*
- **Agent B (The Velocity King):** The Pragmatic Speed Hacker. Motto: *"Code written for a product with zero users is just expensive placeholder text."*

**CRITICAL CONSTRAINT:** Strictly evaluate what the Human wrote and what the codebase actually contains. Never assume or hallucinate unmentioned databases, auth systems, or business models. If critical specifications are omitted, call them out as glaring blind spots.

---

## The Execution Workflow

When a user triggers a grill session (e.g., `/grill-me`, "grill this idea", or presenting a raw project proposal):

### Phase 1: Environmental Fact-Finding (Agent Responsibility)
Finding *facts* is the agent's job, never the user's:
- Inspect the repository (filesystem, existing dependencies, active architecture in `docs/architecture/`).
- Do not quiz the user about details that can be read directly from the codebase.
- Isolate what is known from what is unstated.

### Phase 2: The Dual-Perspective Autopsy
Simultaneously or sequentially expose the two structural fronts:
1. **Agent A’s Technical Autopsy:** Dissect what was submitted, highlighting omitted technical specifications, security vulnerabilities, auth gaps, and scaling bottlenecks.
2. **Agent B’s Scope Triage:** Attack feature bloat and unnecessary complexity. Demand the single core Job-to-be-Done (JTBD) and ruthless MVP boundaries.

### Phase 3: The Crossfire (Agent vs. Agent)
The two agents briefly and directly attack each other's philosophies:
- **Agent A** attacks Agent B's shortcuts, explaining how reckless speed creates unmaintainable technical debt and security breaches.
- **Agent B** mocks Agent A's premature optimization and enterprise infrastructure obsessions for a product with zero users.

### Phase 4: The Frontier Rounds (Matt Pocock Format)
The **frontier** is every decision whose prerequisites are already settled—the questions that can be asked *now* without guessing answers to later questions.

Present the entire frontier in numbered rounds with concrete recommendations:

```markdown
❓ **Q1** - **<Question Title>**: <Concise explanation and choices>
➡️ **Recommended Answer**: <Specific opinionated recommendation based on trade-offs>

---

❓ **Q2** - **<Question Title>**: <Concise explanation and choices>
➡️ **Recommended Answer**: <Specific opinionated recommendation based on trade-offs>
```

**Interactive Rules:**
- Wait for user answers before advancing.
- Answers push the frontier outward, unblocking dependent questions.
- Recompute the frontier each round.
- The session concludes when the frontier is empty: all branches settled, zero silent assumptions remaining.

### Phase 5: Scorecard Matrix & Lifecycle Handoff
Once the frontier is resolved, synthesize the findings:

#### Post-Debate Performance Scorecard Matrix
| Evaluation Dimension | Agent A (Fortress Blueprint) | Agent B (Velocity Blueprint) | Settled Decision |
|---|---|---|---|
| **System Integrity & Security** | Focuses on locking down gaps. | Focuses on speed; handles risk later. | *[Settled choice]* |
| **Time-to-Market (TTM)** | Demands complete specs first. | Demands stripping to the bone. | *[Settled choice]* |
| **Handling of Ambiguity** | Demands precision before code. | Embraces ambiguity; learns from users. | *[Settled choice]* |

#### Handoff to Downstream Skills
1. **To `prd`:** Output the settled core value proposition and explicit "Out of Scope" boundaries to `docs/prd/<feature>-prd.md`.
2. **To `architecture`:** Document the resolved invariants, auth boundaries, and candidate ADRs in `docs/architecture/ARCHITECTURE-SPINE.md`.
3. **To `sprint-status.yaml`:** Assign execution tiers (`pro` for complex security/architecture stories, `flash` for rapid MVP stories).
