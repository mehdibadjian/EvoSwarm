---
description: Continuous organizational memory of past regressions, anti-patterns, and architectural lessons
trigger: always_on
---

# Lessons Learned: Organizational Memory & Defect Prevention

This document is the living institutional memory of `EvoSwarm`. Every agent session loads these lessons before planning or coding.

## 1. Test-Driven Development & Anti-Cheat Invariants
- **Never Alter Test Assertions to Fix a Failing Build:** If a test fails during implementation, fix the production code, not the test. Weakening test assertions or deleting edge cases is strictly prohibited.
- **Write Failing Tests First (Red Phase):** Verify that the test actually fails for the expected reason before writing any production logic. A test that passes before implementation is a false-positive test.

## 2. Specification & Plan Discipline
- **Three-Artifact Chain:** Never begin writing production code without a committed `plan.md`. Writing code before planning leads to architectural divergence.
- **Scope Discipline:** Cut scope to the single core Job-to-be-Done (JTBD). Refuse feature creep; place all non-essential desires into "Out of Scope".

## 3. Past Regressions & Anti-Patterns
- **YAML Comment Preservation:** When modifying `sprint-status.yaml`, never use naive serializers that strip comments or reorder sections. Use targeted line replacement or roundtrip loaders.
- **Subprocess Shell Injection:** Always use parameter lists (`["git", "status"]`) rather than string interpolation in shells (`f"git status {path}"`).
- **Secret Hygiene:** Never commit tokens, credentials, or private keys. Always use standard environment variables.

## 4. Multi-Model Dispatch & Harness Invariants
- **Deterministic Context Compilation:** Always load rule markdown files and skill instructions in sorted lexical order to ensure deterministic prompt payloads across models.
- **Robust HTTP Handling:** When communicating with OpenAI-compatible endpoints using standard `urllib.request`, always set explicit request timeouts (e.g. 60s) and catch `urllib.error.HTTPError` to inspect the error body.

