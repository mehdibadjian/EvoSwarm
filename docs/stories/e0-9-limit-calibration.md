# Story: Limit calibration

## Metadata
- **Story Key:** `e0-9-limit-calibration` (Short: `e0-9`)
- **Epic:** [The Crucible (Sandbox)](file:///workspace/calm-faraday/docs/epics/epic-0.md)
- **Persona:** Operator
- **Priority:** Should
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e0-5-python-stack`, `e0-6-csharp-stack`

---

## 1. User Story
As an operator, I want limits measured on my own box, so that timeouts fit my hardware instead of guesses.

---

## 2. Architectural Context & Invariants
Measures 100 baseline runs on the host box; computes p50 and p95 wall-clock and peak memory; generates tailored profile recommending worker concurrency based on available RAM.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `CLI: evoswarm calibrate --stack <python|csharp>` | `Calibration report + updated config.toml (wall=p95*2, mem=peak*1.5)` | `Error if baseline suite fails` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given the calibration script**, **when it runs 100 baseline builds per stack**, **then it records p50 and p95 wall time and peak memory.**.
- **Given the measurements**, **when calibration finishes**, **then the stack profile sets wall limit to p95 x 2 and memory to peak x 1.5.**.
- **Given the available RAM**, **when the report prints**, **then it recommends a worker count per stack.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/test_calibration.py::test_calibration_statistics`](file:///workspace/calm-faraday/tests/test_calibration.py): Feeds mock run times and verifies p50/p95 calculations.
- [`tests/test_calibration.py::test_profile_generation`](file:///workspace/calm-faraday/tests/test_calibration.py): Asserts generated config applies 2x wall and 1.5x memory safety factors.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e0_9_limit_calibration" --anti-cheat
```
