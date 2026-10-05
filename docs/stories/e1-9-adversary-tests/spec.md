# Story Specification: Adversary tests

## 1. Functional Specification
### 1. Generation & Filtering Pipeline
1. Adversary model generates up to $K$ test functions in the target harness.
2. **Compilation Filter:** Test functions are compiled in the sandbox. If compilation fails, discard.
3. **Suspect Filter:** Execute against the baseline. If a test fails on the baseline AND on all Gen 0 candidates, tag it `suspect` and exclude it from scoring term $A$.
4. **Scoring Invariant:** Surviving valid adversary tests contribute to term $A$ in score $S$ (e1-7).

### 2. Gate Isolation Invariant
Adversary tests are structurally excluded from hard-gate evaluation (e1-6). A candidate
can never fail a gate because of an adversary-authored test, and an adversary test can
never be counted in the Gate 2 trusted-test total or the Gate 4 baseline count.

### 3. Provenance Tagging
Every adversary test is tagged with its origin so downstream stages can distinguish it
from user-authored trusted tests. Untagged tests are rejected at ingest.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| Spec + best candidate patch | Up to $K$ adversary test cases | Zero valid tests yields `A` absent, triggering e1-7 redistribution |
| Test that does not compile in the sandbox | Discarded before execution | Compiler diagnostic logged, not surfaced to scoring |
| Test failing on baseline and all candidates | Tagged `suspect`, excluded from $A$ | Suspect list included in the e1-11 report |
| Any adversary test outcome | Never affects a hard gate result | Gate evaluation receives trusted tests only |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given the spec and current best candidate**, **when the adversary role runs**, **then it generates up to K candidate tests**.
- **Given an adversary test does not compile**, **when collected**, **then it is discarded immediately**.
- **Given an adversary test fails on all candidates and the baseline**, **when collected**, **then it is flagged `suspect` and excluded from scoring**.
- **Given any adversary test fails**, **when hard gates run on a candidate**, **then it never causes that candidate to fail a gate**.
