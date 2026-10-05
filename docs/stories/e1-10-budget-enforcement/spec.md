# Story Specification: Budget enforcement

## 1. Functional Specification
### 1. Budget Checks
- Before dispatch: $	ext{Spent} + 	ext{ProjectedCallCost} \le 	ext{DollarCap}$.
- Token check: $	ext{TokensUsed} + 	ext{MaxTokens} \le 	ext{TokenCap}$.
- If exceeded: halt generation, mark job `budget_exhausted`, return best verified candidate.

### 2. Early-Stop Invariant
- If $\max(S_{G}) \le \max(S_{G-1}) \le \max(S_{G-2})$, terminate search with reason `plateau_early_stop`.


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given a token and dollar cap**, **when a model call is about to be sent**, **then projected cost is checked against remaining budget.**.
- **Given the cap would be exceeded**, **when checked**, **then the job halts as `budget_exhausted` and returns the best verified candidate so far.**.
- **Given no score improvement over 2 generations**, **when generation ends**, **then the job stops early with partial result.**.
