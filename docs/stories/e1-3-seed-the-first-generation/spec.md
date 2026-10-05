# Story Specification: Seed the first generation

## 1. Functional Specification
### 1. Seeding Algorithm
1. Candidate 0 is initialized from current baseline files.
2. If similar past winners exist in FalkorDB (Epic 2), inject up to 2 past winning patches.
3. Mutator role is invoked concurrently with different random seeds to generate remaining drafts up to $N$.
4. **Deduplication:** Compute SHA-256 of candidate diffs. If two drafts produce identical diffs, drop the duplicate and invoke mutator again with temperature jitter ($+0.1$).
5. Each draft records `model_id`, `prompt_hash`, `generation = 0`, and `parent_ids = []`.


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given population size N**, **when generation 0 is built**, **then it holds the current code plus N-1 fresh drafts.**.
- **Given each draft**, **when it is created**, **then it records model ID, prompt hash, and an empty parent list.**.
- **Given two drafts produce identical content hashes**, **when generation 0 is finalised**, **then the duplicate is dropped and replaced with a unique draft.**.
