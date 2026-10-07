"""Selection mode comparison: Top-K vs MAP-Elites on the benchmark suite (e5-2).

Compares solve rate, tokens per solved task, and phenotypic grid coverage side by side.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any


VALID_SELECTION_MODES = ("topk", "mapelites")


@dataclass(frozen=True)
class SelectionRunResult:
    mode: str
    solved: int
    total: int
    tokens: int
    coverage_pct: float

    @property
    def solve_rate(self) -> float:
        if self.total == 0:
            return 0.0
        return 100.0 * self.solved / self.total

    @property
    def tokens_per_solved(self) -> float | None:
        if self.solved == 0:
            return None
        return self.tokens / self.solved


@dataclass(frozen=True)
class SelectionComparison:
    topk: SelectionRunResult
    mapelites: SelectionRunResult
    solve_rate_delta: float
    coverage_delta: float

    def summary_text(self) -> str:
        topk_t = (
            f"{self.topk.tokens_per_solved:.0f}"
            if self.topk.tokens_per_solved is not None
            else "N/A"
        )
        map_t = (
            f"{self.mapelites.tokens_per_solved:.0f}"
            if self.mapelites.tokens_per_solved is not None
            else "N/A"
        )
        return (
            f"Selection Comparison: delta {self.solve_rate_delta:+.2f}% "
            f"(MAP-Elites {self.mapelites.solve_rate:.2f}% vs Top-K {self.topk.solve_rate:.2f}%), "
            f"Tokens/Solved: {map_t} vs {topk_t}, "
            f"Coverage: {self.mapelites.coverage_pct:.1f}% vs {self.topk.coverage_pct:.1f}%"
        )


def parse_selection_mode(raw: str) -> str:
    cleaned = raw.strip().lower().replace("-", "").replace("_", "")
    if cleaned in ("topk",):
        return "topk"
    if cleaned in ("mapelites",):
        return "mapelites"
    raise ValueError(
        f"Unknown selection mode '{raw}', expected one of: {', '.join(VALID_SELECTION_MODES)}"
    )


def compare_selection(
    topk: SelectionRunResult,
    mapelites: SelectionRunResult,
) -> SelectionComparison:
    solve_delta = mapelites.solve_rate - topk.solve_rate
    cov_delta = mapelites.coverage_pct - topk.coverage_pct
    return SelectionComparison(
        topk=topk,
        mapelites=mapelites,
        solve_rate_delta=solve_delta,
        coverage_delta=cov_delta,
    )
