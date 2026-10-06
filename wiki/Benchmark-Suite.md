# Benchmark Suite

Epic 1's exit gate is a claim: *EvoSwarm beats single-shot generation by ≥ 15 points on a 30-task benchmark at equal token budget.* The `bench/` Python package is the harness that makes that claim measurable and fair.

## Layout

```
bench/
├── config.py            # TOKEN_CAP and shared singletons
├── harness.py           # Task/Suite loading + integrity checks
├── compare.py           # solve-rate delta + the pass/fail gate
├── run_single_shot.py   # baseline runner  (run() → NotImplementedError: SEAM)
├── run_evoswarm.py      # EvoSwarm runner  (run() → NotImplementedError: SEAM)
├── tasks/               # 30 task fixtures
└── tests/               # pytest suite (deterministic)
```

## Determinism & fairness knobs

- **`config.TOKEN_CAP = 500_000`** — the equal token budget both runners must respect. `enforce_cap` bounds a run to it.
- **Shared singletons bound by identity** — `ModelRoles`, `SandboxProfile`, and `HeldOutPolicy` are frozen dataclasses instantiated once as module singletons. Both runners **re-export the same objects**, so structural fairness is by object identity (`is`), not by re-typed equality — the two runners provably use the *same* budget/model-roles/sandbox/held-out config.
- **Suite integrity** — `harness.REQUIRED_TOTAL_TASKS = 30`, `REQUIRED_LANGUAGES = {"python": 10, "csharp": 10}`. `load_suite` collects per-task integrity errors and aggregate checks, raising `SuiteIntegrityError` (e.g. an all-python suite is rejected for missing `csharp`).
- **The gate** — `compare.GATE_THRESHOLD_PCT = 15.0`; `evaluate()` computes the percentage-point solve-rate delta between the two runners. The boundary is **inclusive** at 15.0. `_tokens_per_solved` returns `None` when `solved == 0`.

## The 30 task fixtures

`bench/tasks/` holds 30 generated fixtures: 12 `py-*`, 12 `cs-*`, and 6 `py-x*`. Each carries a `task.md` (with a `language:` header), a `baseline/`, a `tests/trusted/`, and a `tests/held_out/` slice.

## What's verified vs. deferred (story `e1-13`, review / SEAM)

Deterministically proven and green:

- fixture integrity (30 tasks, ≥10 python / ≥10 csharp),
- structural fairness (shared `TOKEN_CAP`/model-roles/sandbox/held-out bound by identity into both runners),
- `enforce_cap` boundary behaviour,
- the solve-rate delta arithmetic and the inclusive `>= 15.0` gate.

Deferred: the actual **live runs**. `run_single_shot.run()` and `run_evoswarm.run()` raise `NotImplementedError` because a real comparison needs **LLM provider keys**, and the C# fixtures need the **`dotnet` SDK** to build. The tests that *would* exercise live behaviour are kept as seams so the moment those dependencies exist, the gate can run end-to-end. See the tests in `bench/tests/test_e1_13_benchmark_suite.py`.

Run it:

```bash
python3 -m pytest bench/tests -v
```
