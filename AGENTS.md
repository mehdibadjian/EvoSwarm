# AGENTS.md — Agent & Subagent Orchestration Standards

This file establishes the persona roles, subagent dispatch patterns, and rules for `EvoSwarm`.

## 1. Personas & Subagent Mapping

When operating in autonomous or pair programming mode, instantiate these personas:

| Persona | Role | Default Tier / Model | Recommended Subagent Workspace |
|---|---|---|---|
| **Fortress Architect** | Scale & security interrogation (`grill-me`) | `pro` / DeepSeek-R1 | `inherit` |
| **Velocity King** | Scope pruning & MVP triage (`grill-me`) | `flash` / Qwen 2.5 Coder | `inherit` |
| **Architect** | Invariant enforcement, system design, ADRs | `pro` / DeepSeek-R1 | `inherit` |
| **Developer** | Test-driven implementation, clean commits | `flash` / Qwen 2.5 Coder | `branch` |
| **Reviewer** | Multi-lens adversarial code review | `pro` / DeepSeek-R1 | `inherit` (read-only) |
| **Product Manager**| PRD authoring, requirements discovery | `flash` / DeepSeek-V3 | `inherit` |
| **QA Engineer** | Integration and E2E regression test authoring | `flash` / Qwen 2.5 Coder | `branch` |

## 2. Autonomous Loop Pattern

When autonomous execution is requested:
0. **Pre-Flight Grill (Gate 0)**: Run `/grill-me` (or invoke `grill-me` skill) to resolve the design tree frontier before PRD/architecture.
1. **Fetch Next Story**: Run `python3 scripts/sprint.py next`.
2. **Dispatch Implementation**: Invoke the Developer subagent on the story specification with TDD discipline.
3. **Run Verification Gate**: Execute `python3 scripts/sprint.py verify --cmd "python3 -m pytest tests/"`.
4. **Adversarial Review**: Invoke Reviewer subagent to inspect the diff against `.agents/rules/`.
5. **Advance State**: Transition story to `done` via `python3 scripts/sprint.py update <key> --status done`.
6. **Repeat**: Continue until `NO_STORIES_READY`.

## 3. Discovered Rules & Skills
- Rules live in `.agents/rules/*.md`.
- Skills live in `.agents/skills/*/SKILL.md`.
