# Story Specification: Budget enforcement

## 1. Functional Specification
### 1. Budget Checks
Both checks run **before** dispatch; a call that would cross a cap is never sent.
- Dollar check: $\text{Spent} + \text{ProjectedCallCost} \le \text{DollarCap}$.
- Token check: $\text{TokensUsed} + \text{MaxTokens} \le \text{TokenCap}$.
- $\text{ProjectedCallCost}$ is derived from the role's `cost_per_million_input` / `cost_per_million_output` (e1-2) and the prompt token estimate (e1-4).
- If either check fails: halt generation, mark the job `budget_exhausted`, return the best verified candidate.

### 2. Early-Stop Invariant
- If $\max(S_{G}) \le \max(S_{G-1}) \le \max(S_{G-2})$, terminate the search with reason `plateau_early_stop`.
- Strictly-greater improvement is required to continue; an equal score counts as no improvement.
- Early stop never discards an already-verified winner.

### 3. Accounting Invariant
Spend is accumulated from provider-reported usage, and the projection is reconciled
against it after each call. Accounting is monotonic: recorded spend never decreases.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| Current spend + projected call cost vs dollar cap | Dispatch proceeds, or loop aborts with `budget_exhausted` | Best verified candidate returned with the reason |
| Tokens used + requested `max_tokens` vs token cap | Dispatch proceeds, or loop aborts | Remaining token headroom reported |
| Best score per generation history | Continue, or stop with `plateau_early_stop` | Partial result and generation count reported |
| Cost fields from e1-2 config | Projected cost per call | Zero or negative costs rejected at config load |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given a token and dollar cap**, **when a model call is about to be sent**, **then the projected cost is checked against the remaining budget before dispatch**.
- **Given the cap would be exceeded**, **when checked**, **then the job halts as `budget_exhausted` and returns the best verified candidate so far**.
- **Given no score improvement over 2 consecutive generations**, **when a generation ends**, **then the job stops early with reason `plateau_early_stop` and a partial result**.
- **Given provider-reported usage exceeds the projection**, **when reconciled**, **then recorded spend rises to the actual value and never decreases**.
