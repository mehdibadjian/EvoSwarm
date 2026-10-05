# Story Specification: Crossover of two parents

## 1. Functional Specification
### 1. Parent Selection for Recombination
- Candidate pairs $(P_A, P_B)$ are scored by Hamming distance of test pass vectors:
  $$D(P_A, P_B) = |(Pass_A \setminus Pass_B) \cup (Pass_B \setminus Pass_A)|$$
- Pairs with highest $D > 0$ are scheduled for crossover.

### 2. Synthesiser Dispatch & Lineage
- Prompt presents Parent A diff and tests it passes, Parent B diff and tests it passes.
- Synthesiser emits unified patch and summary of merged traits.
- Graph edge: `(Child)-[:MERGED_FROM {traits: "..."}]->(Parent_A)` and `(Child)-[:MERGED_FROM {traits: "..."}]->(Parent_B)`.
- Job call budget enforces crossover calls $\le 0.25 	imes 	ext{TotalCalls}$.


---

## 2. Acceptance Criteria (Gherkin Scenarios)
- **Given two parents pass different subsets of tests**, **when parents are selected for crossover**, **then such pairs are prioritized.**.
- **Given a crossover child is generated**, **when it is stored**, **then it links to both parents with the traits reported by the synthesiser model.**.
- **Given default budget settings**, **when a job runs**, **then crossover calls constitute at most 25% of total model calls.**.
