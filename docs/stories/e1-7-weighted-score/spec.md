# Story Specification: Weighted score

## 1. Functional Specification
### 1. Scoring Terms
- $A \in [0.0, 1.0]$: Pass rate on candidate adversary tests: $rac{	ext{Passed Adversary Tests}}{	ext{Total Valid Adversary Tests}}$.
- $P \in [0.0, 1.0]$: Relative runtime score: $	ext{clamp}\left(rac{	ext{Baseline Runtime}}{	ext{Candidate Runtime}}, 0.0, 1.0ight)$.
- $Z \in [0.0, 1.0]$: Size parsimony score: $e^{-rac{	ext{diff\_lines}}{100}}$.

### 2. Redistribution Rule (No Adversary Tests)
When adversary tests are absent:
$$w'_p = rac{w_p}{w_p + w_s} = rac{0.3}{0.5} = 0.6$$
$$w'_s = rac{w_s}{w_p + w_s} = rac{0.2}{0.5} = 0.4$$
$$S = w'_p \cdot P + w'_s \cdot Z$$


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given a candidate passes all gates**, **when scored**, **then S is computed using adversary pass rate, runtime vs baseline, and diff size.**.
- **Given weights that do not sum to 1.0 in config**, **when the service starts**, **then it fails with an explicit configuration error.**.
- **Given no adversary tests exist**, **when scoring**, **then adversary weight is redistributed proportionally to runtime and diff size terms.**.
- **Given identical inputs**, **when scored twice**, **then the returned score is strictly identical.**.
