"""Fairness configuration the two runners share by object identity.

The spec's fairness invariants (§2) require identical token cap, model roles,
sandbox profile and held-out verification for both methods. Sharing them here
makes equality structural: any edit to a value propagates to both runners at
once, so the harness cannot silently diverge them.
"""

from dataclasses import dataclass


TOKEN_CAP = 500_000


@dataclass(frozen=True)
class ModelRoles:
    """Which model each role plays. The same roles apply to both methods."""

    draft: str = "claude-3-5-sonnet"
    critique: str = "claude-3-5-haiku"
    repair: str = "claude-3-5-sonnet"


@dataclass(frozen=True)
class SandboxProfile:
    """Which sandbox the tests execute in. Identical for both methods."""

    backend: str = "bwrap"
    cpu_cap: str = "2 cores"
    memory_cap: str = "1 GiB"


@dataclass(frozen=True)
class HeldOutPolicy:
    """The e1-8 held-out slice pass criterion. Reused here so 'solved' has the
    same definition in both runners and neither sees the held-out tests."""

    fraction: float = 0.20
    required_pass_rate: float = 1.00
    leak_scan: bool = True


MODEL_ROLES = ModelRoles()
SANDBOX_PROFILE = SandboxProfile()
HELD_OUT_POLICY = HeldOutPolicy()
