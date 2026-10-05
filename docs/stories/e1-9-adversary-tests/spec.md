# Story Specification: Adversary tests

## 1. Functional Specification
### 1. Generation & Filtering Pipeline
1. Adversary model generates $K$ test functions in the target harness.
2. **Compilation Filter:** Test functions compiled in sandbox. If compilation fails, discard.
3. **Suspect Filter:** Execute against baseline. If test fails on baseline AND all Gen 0 candidates, tag `suspect` and exclude from scoring $A$.
4. **Scoring Invariant:** Surviving valid adversary tests contribute to term $A$ in score $S$.


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given the spec and current best candidate**, **when the adversary role runs**, **then it generates up to K candidate tests.**.
- **Given an adversary test does not compile**, **when collected**, **then it is discarded immediately.**.
- **Given an adversary test fails on all candidates and the baseline**, **when collected**, **then it is flagged `suspect` and excluded from scoring.**.
- **Given any adversary test**, **when hard gates run**, **then it never causes a candidate to fail a gate.**.
