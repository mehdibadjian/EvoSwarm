# Story Specification: Crossover of two parents

## 1. Functional Specification
### 1. Parent Selection for Recombination
- Candidate pairs $(P_A, P_B)$ are scored by Hamming distance of test pass vectors:
  $$D(P_A, P_B) = |(Pass_A \setminus Pass_B) \cup (Pass_B \setminus Pass_A)|$$
- Pairs with highest $D > 0$ are scheduled for crossover.
- If every pair has $D = 0$, the generation falls back to mutation only.

### 2. Synthesiser Dispatch & Lineage
- Prompt presents Parent A diff and tests it passes, Parent B diff and tests it passes.
- Synthesiser emits a unified patch and a summary of merged traits.
- Graph edges: `(Child)-[:MERGED_FROM {traits: "..."}]->(Parent_A)` and `(Child)-[:MERGED_FROM {traits: "..."}]->(Parent_B)`.
- Job call budget enforces crossover calls $\le 0.25 \times \text{TotalCalls}$.

### 3. Provenance Requirement
A candidate without recorded test provenance is never eligible as a parent: crossover of
unmeasured code cannot be shown to combine complementary strengths.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| Parent A (passes tests 1, 2) + Parent B (passes tests 3, 4) | Child `Candidate` with synthesiser-reported traits | Fall back to mutation when no pair has $D > 0$ |
| Pass vectors for the whole population | Pairs ranked by descending Hamming distance | Empty ranking yields mutation-only generation |
| Crossover dispatch request | Synthesiser call, two `MERGED_FROM` edges | `BudgetExhausted` once calls reach 25% of total |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given two parents pass different subsets of tests**, **when parents are selected for crossover**, **then the pair with the greatest Hamming distance is prioritised**.
- **Given every candidate passes exactly the same tests**, **when a generation is planned**, **then no crossover is scheduled and the generation falls back to mutation**.
- **Given a crossover child is generated**, **when it is stored**, **then it links to both parents with the traits reported by the synthesiser model**.
- **Given default budget settings**, **when a job runs**, **then crossover calls constitute at most 25% of total model calls**.
