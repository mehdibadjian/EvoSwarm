"""Single-shot-with-feedback runner: 1 draft plus sequential retries up to the
shared token cap (spec §1).

FAIRNESS: the budget, model roles, sandbox profile and held-out policy below are
imported by reference from `bench.config` and re-exported as module attributes.
The runners do not copy these constants — they bind the very same objects — so
equality across methods is structural: there is exactly one value to change.

SEAM: `run()` is the live loop that draws drafts from a model provider, applies
each candidate patch, and executes the trusted/held-out suites in the sandbox.
It requires LLM provider keys and the `evoswarm` CLI sandbox, which the test
suite cannot exercise in this environment (roadmap §6). The deterministic
fairness contract (shared config objects, cap enforcement helper) is real and
tested; the provider round-trip is deferred, and `run()` fails loudly rather
than returning a fabricated solve.
"""

from __future__ import annotations

from bench.config import (
    HELD_OUT_POLICY,
    MODEL_ROLES,
    SANDBOX_PROFILE,
    TOKEN_CAP,
)


class BudgetExceeded(RuntimeError):
    """Raised when a runner would exceed the shared per-task token cap."""


def enforce_cap(spent_tokens: int, cap: int = TOKEN_CAP) -> None:
    """Stops the retry loop the moment the budget would be overrun (§2: 'neither
    method may exceed the cap'). Pure, so the fairness ceiling is testable even
    though the live loop is not."""
    if spent_tokens > cap:
        raise BudgetExceeded(
            f"spent {spent_tokens} tokens exceeds shared cap {cap}"
        )


def run(_task):
    """Live single-shot-with-feedback loop. Requires a model provider; not
    available in the deterministic test environment."""
    raise NotImplementedError(
        "run_single_shot.run needs LLM provider keys (roadmap §6); this is the "
        "SEAM half of e1-13. The shared config and cap enforcement above are the "
        "tested fairness contract."
    )
