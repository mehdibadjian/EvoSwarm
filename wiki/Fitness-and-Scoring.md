# Fitness & Scoring

`evoswarm-fitness` decides whether a candidate is acceptable and, among acceptable candidates, how good it is. It is intentionally **pure and gate-first**: nothing about config, weights, or performance can rescue a candidate that fails a hard gate.

## The four hard gates (`gates.rs`)

`evaluate(input: &GateInput) -> GateResult` runs the gates **in order and short-circuits on the first failure**, so the reported `reason` is always the earliest gate. `GateFailure` ∈ { `Build`, `TestFailure`, `Tamper`, `Skipped` }.

The input is bundled into a struct (`GateInput`) rather than a long positional argument list, because several fields share types (`&[String]`, `usize`) and positional args would transpose silently.

| Gate | Rule | Failure code |
|---|---|---|
| 1. **Build** | `run.exit_code == 0` **and** `run.status == Success` | `Build` |
| 2. **Trusted tests pass** | every baseline trusted test name must be in `passed_tests` | `TestFailure` (lists `failed_tests`) |
| 3. **No tampering** | `tamper::detect(patch_paths, allowed_paths)` must be empty — the diff touches no test/harness/build file and stays inside the declared `--paths` | `Tamper` |
| 4. **No skips** | `executed_count >= baseline.test_count` **and** `skipped_count <= baseline.skipped` | `Skipped` |

The gate reason is a fail-any → score-0 signal; a `GateResult` with `passed: true` is required before scoring matters. Story `e1-6` (**done**).

## Multi-objective score (`scoring.rs`)

For candidates that pass every gate, the score is a normalised weighted sum:

```
S = w_a·A + w_p·P + w_s·Z
```

Default weights (README §4): `w_a = 0.5` adversary pass rate, `w_p = 0.3` runtime, `w_s = 0.2` parsimony.

`Weights::validate` enforces `w_a + w_p + w_s == 1.0` (within `1e-9`) and returns `ScoreError::WeightsNotNormalised` otherwise — the weights must sum to 1, no exceptions.

### The terms

- **Adversary pass rate `A`** — `adversary_pass_rate(passed, total)` returns `None` when `total == 0`. When `A` is present, the score is the plain weighted sum.
- **Runtime `P`** — `runtime_term(baseline_ms, candidate_ms)` = `clamp(baseline/candidate, 0, 1)`; a `candidate_ms == 0` short-circuits to `1.0`.
- **Parsimony `Z`** — `parsimony_term(diff_lines)` = `exp(-diff_lines / 100)`; rewards shorter diffs.

### Zero-adversary redistribution

When `adversary_pass_rate` is `None` (no adversary tests ran), the adversary weight can't contribute, so `w_p` and `w_s` are **renormalised** over their own sum: `wp = w_p/(w_p+w_s)`, `ws = w_s/(w_p+w_s)`. If that denominator is ~0 the function returns `ScoreError::DivisionByZero`. Story `e1-7` (**done**).

## Held-out & tamper support

- `holdout.rs` — held-out test slice policy used for final winner selection (`e1-8`, done).
- `tamper.rs` — the diff-path detector that feeds Gate 3.
- `leak_scan.rs` — scans for secret leakage in prompts/diffs.
- `baseline.rs` — the `Baseline` snapshot (trusted test names, count, skips) captured once at job start; the gate references it rather than re-deriving from the sandbox.

## Why this matters for verification

Because the gate/score pipeline is pure and side-effect-free, the `e1-6/e1-7/e1-8` stories could be fully verified with deterministic unit tests — no LLM or sandbox needed — which is why they are at **done** rather than SEAM.

See [Engine Search Loop](Engine-Search-Loop.md) for how fitness is called during the search, and [The Crucible](The-Crucible.md) for how the read-only mounts make the tamper gate enforceable.
