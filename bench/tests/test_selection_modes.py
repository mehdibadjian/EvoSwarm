"""Tests for selection mode comparison (e5-2)."""

import pytest

from bench.compare_selection import (
    SelectionComparison,
    SelectionRunResult,
    compare_selection,
    parse_selection_mode,
)


def test_mode_dispatch():
    assert parse_selection_mode("topk") == "topk"
    assert parse_selection_mode("top-k") == "topk"
    assert parse_selection_mode("Top_K") == "topk"
    assert parse_selection_mode("mapelites") == "mapelites"
    assert parse_selection_mode("map-elites") == "mapelites"
    assert parse_selection_mode("MAP_Elites") == "mapelites"

    with pytest.raises(ValueError):
        parse_selection_mode("random_walk")


def test_side_by_side_reporting():
    topk = SelectionRunResult(
        mode="topk",
        solved=18,
        total=30,
        tokens=3_600_000,
        coverage_pct=25.0,
    )
    mapelites = SelectionRunResult(
        mode="mapelites",
        solved=22,
        total=30,
        tokens=4_100_000,
        coverage_pct=62.5,
    )

    comparison = compare_selection(topk, mapelites)

    assert comparison.topk.solve_rate == 60.0
    assert pytest.approx(comparison.mapelites.solve_rate, rel=1e-3) == 73.333
    assert pytest.approx(comparison.solve_rate_delta, rel=1e-3) == 13.333
    assert comparison.topk.tokens_per_solved == 200_000.0
    assert pytest.approx(comparison.mapelites.tokens_per_solved, rel=1e-3) == 186_363.636
    assert comparison.coverage_delta == 37.5

    summary = comparison.summary_text()
    assert "Selection Comparison" in summary
    assert "MAP-Elites 73.33%" in summary
    assert "Top-K 60.00%" in summary
    assert "Coverage: 62.5% vs 25.0%" in summary
