"""Solve-rate delta, the Gate-1 predicate, and the auditable report.

A task counts as solved only when its held-out slice passes at the shared
`HeldOutPolicy.required_pass_rate` (e1-8's definition), so "solved" means the
same thing for both methods. The delta is the percentage-point difference
`EvoSwarm - SingleShot`; the exit gate requires delta >= 15.0 (§1).

`raw` is produced alongside the summary — not after it — so even a failed gate
leaves the per-task decision auditable (spec §3). Live execution that fills the
solved/token counts is the SEAM half (needs provider keys); this module is the
deterministic arithmetic over those counts.
"""

from dataclasses import dataclass
from typing import Any


GATE_THRESHOLD_PCT = 15.0


@dataclass(frozen=True)
class Result:
    evo_solve_rate: float
    single_solve_rate: float
    delta: float
    evo_tokens_per_solved: float | None
    single_tokens_per_solved: float | None
    wall_time_secs: float
    gate_passed: bool
    raw: dict[str, Any]

    def summary_text(self) -> str:
        verdict = "PASS" if self.gate_passed else "FAIL"
        return (
            f"Gate 1 {verdict}: delta {self.delta:.2f}% "
            f"(EvoSwarm {self.evo_solve_rate:.2f}% - "
            f"SingleShot {self.single_solve_rate:.2f}%), "
            f"threshold {GATE_THRESHOLD_PCT:.1f}%"
        )


def _solve_rate(solved: int, total: int) -> float:
    if total == 0:
        return 0.0
    return 100.0 * solved / total


def _tokens_per_solved(tokens: int, solved: int) -> float | None:
    # No solved task => the metric is undefined, not zero; None keeps that honest.
    if solved == 0:
        return None
    return tokens / solved


def evaluate(
    *,
    evo_solved: int,
    evo_total: int,
    single_solved: int,
    single_total: int,
    evo_tokens: int,
    single_tokens: int,
    wall_time_secs: float,
) -> Result:
    evo_rate = _solve_rate(evo_solved, evo_total)
    single_rate = _solve_rate(single_solved, single_total)
    delta = evo_rate - single_rate
    gate_passed = delta >= GATE_THRESHOLD_PCT

    raw: dict[str, Any] = {
        "evo_solve_rate": evo_rate,
        "single_solve_rate": single_rate,
        "delta": delta,
        "evo_tokens_per_solved": _tokens_per_solved(evo_tokens, evo_solved),
        "single_tokens_per_solved": _tokens_per_solved(single_tokens, single_solved),
        "wall_time_secs": wall_time_secs,
        "gate_threshold_pct": GATE_THRESHOLD_PCT,
        "gate_passed": gate_passed,
    }

    return Result(
        evo_solve_rate=evo_rate,
        single_solve_rate=single_rate,
        delta=delta,
        evo_tokens_per_solved=raw["evo_tokens_per_solved"],
        single_tokens_per_solved=raw["single_tokens_per_solved"],
        wall_time_secs=wall_time_secs,
        gate_passed=gate_passed,
        raw=raw,
    )
