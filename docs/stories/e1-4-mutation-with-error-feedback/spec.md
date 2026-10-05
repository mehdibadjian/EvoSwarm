# Story Specification: Mutation with error feedback

## 1. Functional Specification
### 1. Feedback Truncation Invariant
- Compiler errors: captured up to first 2,000 tokens.
- Test failures: first failing assertion, test name, and stack trace captured up to 2,000 tokens.
- Total error context hard-clamped to 4,000 tokens.

### 2. Prompt Caching Structure
- **Static Prefix (Cached):** System instructions + repository overview + API contracts + immutable test rules.
- **Dynamic Suffix:** Current parent diff + sandbox execution failure diagnostics.


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given a failed parent candidate**, **when its child is drafted**, **then the prompt includes the first failing assertion and error text trimmed to 4k tokens.**.
- **Given any child candidate**, **when it is stored**, **then it is linked to its parent with `MUTATED_FROM` edge.**.
- **Given repeated mutation calls in one job**, **when they are sent**, **then repository context sits in a stable prefix and prompt cache hit rate is logged.**.
