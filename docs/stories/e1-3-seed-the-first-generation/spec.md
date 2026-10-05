# Story Specification: Seed the first generation

## 1. Functional Specification
### 1. Seeding Algorithm
1. Candidate 0 is initialized from current baseline files.
2. If similar past winners exist in FalkorDB (Epic 2), inject up to 2 past winning patches. In Epic 1 this hook is a trait method returning an empty list.
3. Mutator role is invoked concurrently with different random seeds to generate remaining drafts up to $N$.
4. **Deduplication:** Compute SHA-256 of candidate diffs. If two drafts produce identical diffs, drop the duplicate and invoke mutator again with temperature jitter ($+0.1$), up to 3 attempts.
5. Each draft records `model_id`, `prompt_hash`, `generation = 0`, and `parent_ids = []`.

### 2. Population Invariant
The returned population always has length exactly $N$. Seeding never returns a short
population: an exhausted retry budget is a hard error, because a silently smaller Gen 0
biases every later selection statistic.

### 3. Determinism
Draft ordering is deterministic given the same seeds, so a replayed job reproduces Gen 0
byte for byte (required by e1-12 idempotency).

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `JobSubmission`, population size `N` | `Vec<Candidate>` of length exactly `N` | `SeedingError::ModelFailure` after retry budget exhausted |
| Two drafts with identical diff SHA-256 | Duplicate dropped, replacement drafted at temperature +0.1 | `SeedingError::DedupExhausted` after 3 replacement attempts |
| Each fresh draft | Recorded `model_id`, `prompt_hash`, `generation = 0`, `parent_ids = []` | Draft discarded if any field is absent |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given population size N**, **when generation 0 is built**, **then it holds the current code plus N-1 fresh drafts**.
- **Given each draft**, **when it is created**, **then it records model ID, prompt hash, generation 0 and an empty parent list**.
- **Given two drafts produce identical content hashes**, **when generation 0 is finalised**, **then the duplicate is dropped and replaced with a unique draft**.
- **Given every retry also produces a duplicate**, **when the replacement budget is exhausted**, **then seeding fails loudly instead of returning a short population**.
