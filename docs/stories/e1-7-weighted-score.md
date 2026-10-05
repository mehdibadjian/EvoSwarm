# Story: Weighted score

## Metadata
- **Story Key:** `e1-7-weighted-score` (Short: `e1-7`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** S
- **Execution Tier:** `flash`
- **Dependencies:** `e1-6-hard-gates`

---

## 1. User Story
As a developer, I want passing candidates ranked by a clear score, so that the best of several valid solutions wins.

---

## 2. Architectural Context & Invariants
Computes S = w_a * A + w_p * P + w_s * Z. Weights configurable in config.toml (defaults: 0.5, 0.3, 0.2, summing to 1.0). If no adversary tests, redistributes w_a proportionally.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `A: f64, P: f64, Z: f64, weights: Weights` | `Score S: f64` | `Error if weights do not sum to 1.0` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given a candidate passes all gates**, **when scored**, **then S is computed from adversary pass rate, runtime versus baseline and diff size, using config weights.**.
- **Given weights that do not sum to 1**, **when the service starts**, **then it fails with the weights shown.**.
- **Given no adversary tests exist**, **when scoring**, **then the adversary weight is redistributed proportionally to the other terms.**.
- **Given identical inputs**, **when scored twice**, **then the score is identical.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/fitness/test_scoring.rs::test_score_formula_calculation`](file:///workspace/calm-faraday/tests/fitness/test_scoring.rs): Verifies exact arithmetic for given weights and inputs.
- [`tests/fitness/test_scoring.rs::test_weight_redistribution_without_adversary`](file:///workspace/calm-faraday/tests/fitness/test_scoring.rs): Checks proportional redistribution when A is absent.
- [`tests/fitness/test_scoring.rs::test_weight_validation_on_startup`](file:///workspace/calm-faraday/tests/fitness/test_scoring.rs): Rejects config with weights summing to 0.9.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_7_weighted_score" --anti-cheat
```
