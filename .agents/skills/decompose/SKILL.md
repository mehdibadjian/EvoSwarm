---
name: decompose
description: Decompose PRDs and architecture specifications into sequential, testable epics and user stories. Use when planning sprints or breaking large requirements into bite-sized tasks.
---

# Decompose: Epics & Story Breakdown

## Goal
Transform PRDs and architecture invariants into vertically sliced, independently shippable user stories sequenced by technical dependency.

## Workflow

### 1. Epic Formulation
- Group requirements into cohesive milestones (Epics) representing user-valuable increments.
- Assign an Epic ID (e.g. `epic-1`, `epic-2`).

### 2. Story Slicing Standards
- **Vertical Slice**: Each story must touch all necessary layers (UI/API/Data) to deliver a verifiable outcome.
- **Single User-Facing Goal**: Target 900–1600 tokens of specification. Avoid monolithic mega-stories.
- **Input/Output Matrix**: Explicitly define input contracts, expected outputs, and error handling.
- **Test Plan**: Map every acceptance criterion to a named test case that will be written before code.

### 3. Story Artifact Creation
- Write each story to `docs/stories/<epic_id>-<story_number>-<slug>.md`.
- Register the story in `sprint-status.yaml` under `development_status` with initial state `backlog` or `ready-for-dev`.
- Assign an execution tier (`flash` or `pro` / `standard` or `frontier`) under `execution_tiers`.
