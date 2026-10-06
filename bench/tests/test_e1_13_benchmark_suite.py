"""e1-13 benchmark-suite acceptance tests (story §5 Red phase).

Gate: `python3 -m pytest bench/tests/test_e1_13_benchmark_suite.py -v`

SEAM boundary
-------------
All four tests validate deterministic shape and math: fixture integrity, structural
config-sharing across the two runners, delta arithmetic and the gate predicate. The
live halves the story also names — actually executing the two runners against a
provider, and building the C# fixtures under `dotnet` — need LLM keys and a
dotnet SDK that this sandbox does not have (roadmap §6); they are the SEAM and
deferred to the review note rather than stubbed or faked here.
"""

import shutil
import tempfile
from pathlib import Path

import pytest

from bench import compare, config, harness


REPO_ROOT = Path(__file__).resolve().parents[1]
TASKS_DIR = REPO_ROOT / "tasks"


# --- AC1: fixture integrity -------------------------------------------------

def test_benchmark_task_integrity():
    """30 tasks, >=10 Python and >=10 C#, each with task.md, baseline/, a trusted
    test file and a held-out file."""
    suite = harness.load_suite(TASKS_DIR)
    assert len(suite.tasks) == 30, f"expected 30 tasks, got {len(suite.tasks)}"
    langs = [t.language for t in suite.tasks]
    assert langs.count("python") >= 10, f"need >=10 python, got {langs.count('python')}"
    assert langs.count("csharp") >= 10, f"need >=10 csharp, got {langs.count('csharp')}"

    # Non-vacuity: prove the loader really checks each invariant, not just the
    # happy path. A missing held-out file must name the offending task.
    broken = Path(tempfile.mkdtemp())
    try:
        for tid in ("py-01", "py-02", "py-03", "py-04"):
            p = broken / tid
            shutil.copytree(TASKS_DIR / tid, p)
        shutil.rmtree(broken / "py-03" / "tests" / "held_out")  # no held_out => error
        (broken / "py-04" / "tests" / "trusted" / "test_trusted.py").write_text("")
        # trusted dir now contains only an empty file: harness must treat
        # "trusted tests present" as "non-empty test content".
        with pytest.raises(harness.SuiteIntegrityError) as exc:
            harness.load_suite(broken)
        msg = str(exc.value)
        assert "py-03" in msg, f"loader must name offending task, got {msg}"
        assert "py-04" in msg, f"loader must reject empty trusted test, got {msg}"
    finally:
        shutil.rmtree(broken, ignore_errors=True)


# --- AC2: structural budget / config equality -------------------------------

def test_token_budget_equality():
    """Both runners share the token cap, model roles, sandbox profile and
    held-out policy by object identity, not by re-typed equality. That is the
    fairness invariant the spec mandates (§2)."""
    assert config.TOKEN_CAP == 500_000, "spec fixes the shared cap at 500k tokens"

    # Both runners import the SAME shared config values. Identity (not ==) is
    # the check: a mutant that copies the number into one runner to keep the
    # runtime equal but not the shared object must still be caught, because
    # only one object can be the same.
    from bench import run_evoswarm, run_single_shot

    assert run_single_shot.TOKEN_CAP is config.TOKEN_CAP, (
        "single-shot must read the shared cap, not a copy"
    )
    assert run_evoswarm.TOKEN_CAP is config.TOKEN_CAP, (
        "evoswarm must read the shared cap, not a copy"
    )
    assert run_single_shot.MODEL_ROLES is config.MODEL_ROLES
    assert run_evoswarm.MODEL_ROLES is config.MODEL_ROLES
    assert run_single_shot.SANDBOX_PROFILE is config.SANDBOX_PROFILE
    assert run_evoswarm.SANDBOX_PROFILE is config.SANDBOX_PROFILE
    assert run_single_shot.HELD_OUT_POLICY is config.HELD_OUT_POLICY
    assert run_evoswarm.HELD_OUT_POLICY is config.HELD_OUT_POLICY


# --- AC3: delta math + reporting shape --------------------------------------

def test_solve_rate_delta_calculation():
    """Delta = EvoSwarm_solve_rate - single_shot_solve_rate, and the summary
    also carries tokens-per-solved and wall time (spec §1.3)."""
    # 12/30 = 40.0% evo, 6/30 = 20.0% single -> delta 20.0 percentage points.
    result = compare.evaluate(
        evo_solved=12, evo_total=30,
        single_solved=6, single_total=30,
        evo_tokens=3_000_000, single_tokens=1_500_000,
        wall_time_secs=42.0,
    )
    assert result.evo_solve_rate == pytest.approx(40.0)
    assert result.single_solve_rate == pytest.approx(20.0)
    assert result.delta == pytest.approx(20.0), (
        "delta is percentage-point difference, not ratio"
    )
    # tokens per solved task (each method independently).
    assert result.evo_tokens_per_solved == pytest.approx(250_000.0)
    assert result.single_tokens_per_solved == pytest.approx(250_000.0)
    assert result.wall_time_secs == pytest.approx(42.0)
    # Gate: delta >= 15.0% passes.
    assert result.gate_passed is True
    # The auditable raw payload exists BEFORE the summary (spec §3 "raw per-task
    # JSON"), so a failed gate remains diagnosable.
    assert isinstance(result.raw, dict)
    assert {"evo_solve_rate", "single_solve_rate", "delta", "gate_passed"} <= result.raw.keys()


# --- AC4: gate failure wording ----------------------------------------------

def test_gate_failure_below_threshold():
    """When delta < 15.0, gate reports failure and the message contains the
    actual delta value."""
    result = compare.evaluate(
        evo_solved=10, evo_total=30,   # 33.3%
        single_solved=6, single_total=30,  # 20.0%
        evo_tokens=2_500_000, single_tokens=1_500_000,
        wall_time_secs=13.5,
    )
    # Delta = 13.333... percentage points (evo 33.333 - single 20.0), below 15.0.
    assert result.gate_passed is False
    msg = result.summary_text()
    # The failure summary must print the delta so the gate decision is auditable.
    # Assert the numeric value appears in the message, not just the word "delta".
    assert "13.3" in msg, (
        f"the delta value must be printed in the failure summary, got {msg!r}"
    )
    # And the raw JSON carries the same numbers, so a post-hoc audit sees them.
    # 13.333... == 40/3 (percentage-point delta), not a ratio.
    assert result.raw["delta"] == pytest.approx(40.0 / 3.0)


# --- Boundary: gate is INCLUSIVE at delta == 15.0 (spec §1.4: ">= 15.0%") ---

def test_gate_boundary_inclusive_at_threshold():
    """Delta exactly 15.0 must PASS the gate — a strict `>` mutant silently
    fails the boundary and would flip the Gate-1 verdict at the knife-edge the
    spec chose (`>=`)."""
    # evo 50% - single 35% = 15.0 exactly.
    result = compare.evaluate(
        evo_solved=15, evo_total=30,
        single_solved=21, single_total=60,  # 35.0%
        evo_tokens=1_000_000, single_tokens=1_000_000,
        wall_time_secs=1.0,
    )
    assert result.delta == pytest.approx(15.0)
    assert result.gate_passed is True, "spec's >= 15.0 is inclusive"


# --- Cap enforcement: neither runner may exceed the shared cap (§2) ----------

def test_cap_enforcement_boundaries():
    """`enforce_cap` is the guard that terminates a live retry loop exactly at
    the shared budget. It must accept `spent == cap` (still within) and reject
    `spent > cap`; a mutant that doubles the ceiling or accepts overshoot lets
    the fairness invariant drift."""
    from bench import run_single_shot, run_evoswarm

    # At the cap: allowed (both runners agree).
    run_single_shot.enforce_cap(config.TOKEN_CAP)
    run_evoswarm.enforce_cap(config.TOKEN_CAP)

    # One token over: must raise, and the shared cap must be what binds.
    with pytest.raises(run_single_shot.BudgetExceeded):
        run_single_shot.enforce_cap(config.TOKEN_CAP + 1)
    with pytest.raises(ValueError):
        run_evoswarm.enforce_cap(config.TOKEN_CAP + 1)


# --- Language-mix aggregate: the loader enforces the >=10/<10 split ----------

def test_language_mix_enforced():
    """A suite of 30 all-Python tasks must be rejected with a message naming
    `csharp`, proving the loader actually checks the aggregate split and not
    only per-task integrity (non-vacuity for the language-mix invariant)."""
    empty = Path(tempfile.mkdtemp())
    try:
        # 30 valid python task dirs (copy py-01 shape, only rename).
        for i in range(1, 31):
            shutil.copytree(TASKS_DIR / "py-01", empty / f"all-py-{i:02d}")
        with pytest.raises(harness.SuiteIntegrityError) as exc:
            harness.load_suite(empty)
        assert "csharp" in str(exc.value), (
            f"aggregate check must name the missing language, got {exc.value}"
        )
    finally:
        shutil.rmtree(empty, ignore_errors=True)
