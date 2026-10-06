"""EvoSwarm evolutionary runner: invokes the `evoswarm` CLI (the population
search from Epic 1) under the SAME shared fairness config as the single-shot
baseline.

FAIRNESS: budget, model roles, sandbox profile and held-out policy are
re-exported by reference from `bench.config`, binding the identical objects the
single-shot runner uses. The two methods therefore cannot diverge on the knobs
the spec fixes (§2) — there is a single source of each.

SEAM: `run()` is the live path that shells out to the `evoswarm` CLI and drives
a real search, which needs LLM provider keys and the sandbox toolchain (roadmap
§6). The tested half is the shared fairness contract (identical config objects,
the cap helper), not the provider round-trip; `run()` fails loudly rather than
returning a fabricated solve.
"""

from bench.config import (
    HELD_OUT_POLICY,
    MODEL_ROLES,
    SANDBOX_PROFILE,
    TOKEN_CAP,
)


def enforce_cap(spent_tokens: int, cap: int = TOKEN_CAP) -> None:
    """Same budget ceiling as the single-shot runner: the search must terminate
    at the shared cap so a run that drifts is invalidated (§2)."""
    if spent_tokens > cap:
        raise ValueError(f"spent {spent_tokens} tokens exceeds shared cap {cap}")


def run(_task):
    """Live EvoSwarm search. Requires a model provider and the sandbox; not
    available in the deterministic test environment."""
    raise NotImplementedError(
        "run_evoswarm.run needs LLM provider keys and the evoswarm CLI sandbox "
        "(roadmap §6); this is the SEAM half of e1-13. The shared fairness "
        "config above is the tested contract."
    )
