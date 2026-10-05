# Story Specification: Mutation with error feedback

## 1. Functional Specification
### 1. Feedback Truncation Invariant
- Compiler errors: captured up to first 2,000 tokens.
- Test failures: first failing assertion, test name, and stack trace captured up to 2,000 tokens.
- Total error context hard-clamped to 4,000 tokens.
- Truncation always appends an explicit marker so the model is not misled into believing the dump is complete.
- Token counting uses the same estimator as e1-10 so the clamp and the budget agree.

### 2. Prompt Caching Structure
- **Static Prefix (Cached):** System instructions + repository overview + API contracts + immutable test rules.
- **Dynamic Suffix:** Current parent diff + sandbox execution failure diagnostics.
- The prefix is assembled once per job and reused verbatim; any per-call variation (timestamps, candidate IDs, job IDs) must live in the suffix.

### 3. Lineage
Every child records a `MUTATED_FROM` edge to its parent. Mutation never orphans a candidate.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| Failed parent `Candidate` + `ExecutionResult` | Child `Candidate` with `MUTATED_FROM` lineage | `BudgetExhausted` if the token cap would be crossed |
| Error text larger than the clamp | Feedback trimmed to 4,000 tokens total | Truncation marker appended, never a silent cut |
| Two sequential mutation calls | Byte-identical static prompt prefix | Cache-miss logged when prefix bytes diverge |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given a failed parent candidate**, **when its child is drafted**, **then the prompt includes the first failing assertion and error text trimmed to 4k tokens**.
- **Given an error dump far larger than the clamp**, **when feedback is assembled**, **then the total is hard-clamped to 4,000 tokens and carries a truncation marker**.
- **Given any child candidate**, **when it is stored**, **then it is linked to its parent with a `MUTATED_FROM` edge**.
- **Given repeated mutation calls in one job**, **when they are sent**, **then repository context sits in a byte-identical static prefix and the cache hit rate is logged**.
