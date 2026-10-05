# Story Specification: Weighted score

## 1. Functional Specification
### 1. Scoring Terms
- $A \in [0.0, 1.0]$: Pass rate on candidate adversary tests: $\frac{\text{Passed Adversary Tests}}{\text{Total Valid Adversary Tests}}$.
- $P \in [0.0, 1.0]$: Relative runtime score: $\text{clamp}\left(\frac{\text{Baseline Runtime}}{\text{Candidate Runtime}}, 0.0, 1.0\right)$.
- $Z \in [0.0, 1.0]$: Size parsimony score: $e^{-\frac{\text{diff\_lines}}{100}}$.

### 2. Composite Score
$$S = w_a A + w_p P + w_s Z$$
with defaults $w_a = 0.5$, $w_p = 0.3$, $w_s = 0.2$. Weights are read from `config.toml`
(e1-2) and must sum to $1.0$ within $\epsilon = 10^{-9}$.

### 3. Redistribution Rule (No Adversary Tests)
When adversary tests are absent, $w_a$ is redistributed proportionally to the remaining terms:
$$w'_p = \frac{w_p}{w_p + w_s} = \frac{0.3}{0.5} = 0.6$$
$$w'_s = \frac{w_s}{w_p + w_s} = \frac{0.2}{0.5} = 0.4$$
$$S = w'_p \cdot P + w'_s \cdot Z$$
If $w_p + w_s = 0$, redistribution is undefined and scoring fails loudly rather than
defaulting silently.

### 4. Numerical Determinism
Identical inputs must produce bitwise identical `f64` output across processes and runs.
Terms are therefore combined in a fixed order with no parallel or reassociated
floating-point reduction, and no input-dependent branching that changes operation order.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `A`, `P`, `Z` in $[0.0, 1.0]$ plus `Weights` | Composite score `S: f64` | `ScoreError::WeightsNotNormalised` naming the offending sum |
| Weights summing to $1.0 \pm \epsilon$ | Service starts, scoring proceeds | Startup abort listing configured weights |
| Zero valid adversary tests | `S` from redistributed $w'_p$, $w'_s$ only | `ScoreError::DivisionByZero` if $w_p + w_s = 0$ |
| Candidate runtime $= 0$ or diff of 0 lines | Clamped $P = 1.0$, $Z = 1.0$ | No NaN or infinity may escape |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given a candidate passes all gates**, **when scored**, **then S is computed using adversary pass rate, runtime versus baseline, and diff size**.
- **Given weights that do not sum to 1.0 in config**, **when the service starts**, **then it fails with an explicit configuration error showing the weights**.
- **Given no adversary tests exist**, **when scoring**, **then the adversary weight is redistributed proportionally to the runtime and diff size terms**.
- **Given identical inputs**, **when scored 1,000 times**, **then every returned score is bitwise identical**.
- **Given a candidate runtime of zero or a zero-line diff**, **when scored**, **then the result is clamped and contains no NaN or infinity**.
