---
name: prd
description: Author, validate, and refine Product Requirements Documents (PRDs) using Jobs-to-be-Done and testable acceptance criteria. Use when defining new features or user requirements.
---

# Product Requirements Document (PRD)

## Goal
Transform user intent and business needs into an actionable Product Requirements Document grounded in user value and verifiable acceptance criteria.

## Workflow

### 1. Requirements Discovery
- Frame the feature around **Jobs-to-be-Done (JTBD)**: "When [situation], I want to [motivation], so I can [expected outcome]."
- Identify user personas, key workflows, edge cases, and non-functional requirements (latency, security, accessibility).

### 2. PRD Structure
Write the PRD to `docs/prd/<feature>-prd.md` following this structure:
- **Executive Summary & Value Proposition**
- **User Personas & Problem Statement**
- **User Journeys & Functional Requirements**
- **Acceptance Criteria (Given / When / Then)**: Every requirement must have at least one testable Gherkin scenario.
- **Out of Scope & Future Work**

### 3. Validation
- Check completeness: Are there any placeholders or TBDs?
- Check testability: Can an automated or manual test verify every acceptance criterion?
- Present the PRD to the user or save as an artifact.
