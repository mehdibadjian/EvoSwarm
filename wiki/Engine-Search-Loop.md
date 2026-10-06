# Engine Search Loop

`evoswarm-engine` is the multi-generation evolutionary search that turns a task into a verified patch. It composes the other subsystems: it asks `models` for candidate code, runs it through the `SandboxBackend` ([The Crucible](The-Crucible.md)), judges it with `fitness` ([Fitness & Scoring](Fitness-and-Scoring.md)), persists via `ledger`/`memory`, and stops on budget or early convergence.

## The loop, end to end

1. **Seed** (`seeding.rs`) — build generation 0. Either from a fresh synthesiser call, or (once memory lands) from prior similar winners. `e1-3` is review (SEAM): seeding is verified against a stub `ModelClient`/`MemorySeeder`; the live half needs provider keys.
2. **Mutate** (`mutation.rs` + `feedback.rs`) — take a parent and ask the `mutator` role for an improved diff, feeding prior error output back into the prompt. `e1-4` review (SEAM).
3. **Crossover** (`crossover.rs`) — recombine two parents into a child. `e1-5` review (SEAM).
4. **Adversary** (`adversary.rs`) — generate candidate-breaking tests; their pass rate feeds the score. `e1-9` review (SEAM): verified against a stub `ModelClient`/`SandboxBackend`.
5. **Execute & judge** — each candidate runs in the Crucible and passes through the four gates; survivors get a weighted score.
6. **Select** (`selection.rs`) — choose survivors/parents for the next generation.
7. **Dedup** (`dedup.rs`) — drop duplicate candidates to avoid wasted spend.
8. **Early stop** (`early_stop.rs`) — terminate once the objective is met / converged.

## Budget enforcement (`budget.rs`, ~258 lines)

`e1-10` is **done** and fully deterministic. The engine tracks cumulative token and dollar spend against the job's `budget_tokens` / `budget_dollars` and stops the search with a `BudgetExhausted` job status before overrunning the cap. Because this is arithmetic over accumulated usage, it needs no live provider to verify.

## Crash recovery (`recovery.rs`)

`e1-12` is **done**. Search state (population, current generation) is persisted opaquely through the ledger, so a job interrupted mid-run can resume from the last durable generation rather than restarting. `JobStatus` is the ledger-tracked lifecycle: `Queued → Running → {Completed | Failed | BudgetExhausted | Cancelled}`; `Queued → Running → Cancelled` is also a legal move via `cancel_job`.

## Dispatch (`dispatch.rs`)

Routes work to the right model role and provider configuration; the boundary where an injected `ModelClient` seam plugs in. This is what SEAM tests substitute with a stub to keep the loop verifiable without live keys.

## Diversity & archives

- **MAP-Elites grid** (`map_elites.rs`, 235 lines) — `e5-1` review (SEAM): binning and replacement are verified against an in-process `RecordingCellSink`; writing `Cell` nodes to FalkorDB needs a redis-compatible FalkorDB instance.
- **Archive health report** (`archive_report.rs`) — `e5-6` review (SEAM): occupancy/heatmap math verified against an in-process archive; per-generation `Cell`-node history needs FalkorDB.

## Which parts are proven without a live provider

The engine is deliberately built so its *control logic* — budget, dedup, selection math, recovery, MAP-Elites binning, and each operator's interaction with an injected seam — is deterministically testable. The `review`-with-SEAM status on `e1-3/e1-4/e1-5/e1-9` reflects exactly that: the seam-verified half is green, and the live-model half is honest-but-deferred pending LLM keys. See [Roadmap & Status](Roadmap-and-Status.md).
