# Roadmap & Status

EvoSwarm ships as six epics (0–5), decomposed into **47 vertical-slice stories** tracked in `sprint-status.yaml`. The board below reflects the actual ledger state as committed on `main`.

## Epic exit gates

| Epic | Theme | Exit gate |
|---|---|---|
| 0 — The Crucible | Sandbox & isolation | 100 baseline runs, 0 escapes; network + fork-bomb contained; limits calibrated |
| 1 — Evolve CLI & Fitness | Search + scoring + benchmark | Beats single-shot by **≥ 15 points** on a 30-task benchmark at equal token budget |
| 2 — System 1 Memory & Replay | Graph memory | Replay never serves a stale winner; seeding cuts spend ≥ 30% |
| 3 — Claude Code Integration | MCP | Claude delegates via `evolve`, fetches patch, verifies locally with 0 intervention |
| 4 — Optional Gateway | Streaming proxy | < 50 ms TTFT at p95 under 20 streams, byte-for-byte fidelity |
| 5 — MAP-Elites & More Stacks | Diversity + languages | 4×4 archive matches/beats Top-K; Java & C/C++ pass red-team containment |

## Status rollup

| State | Count | Meaning |
|---|---|---|
| **done** | 15 | Fully verified in code (deterministic) |
| **review** (SEAM) | 20 | Seam-verified + mutation-tested half; a live half deferred on an external dependency |
| **backlog** | 12 | Not started; most are roadmap-declared **blocked** pending FalkorDB / kernel features / toolchains |

### Done (15)
`e0-1` sandbox runner · `e0-5` python stack · `e0-7` tamper-proof tests · `e1-1` CLI job · `e1-2` model roles · `e1-6` hard gates · `e1-7` weighted score · `e1-8` held-out · `e1-10` budget · `e1-11` patch+report · `e1-12` resume · `e2-1` blob store · `e2-3` fingerprints · `e2-7` seccomp · `e3-5` context guard.

### Review with SEAM (20)
- **Epic 0:** `e0-2` host self-check (unshare probe real; cgroup-delegation + linger deferred), `e0-8` red-team (bwrap escape/egress/seccomp subset asserted; shadow-read + cgroup deferred), `e0-9` calibration (math only; live 100-run needs dotnet).
- **Epic 1:** `e1-3` seed, `e1-4` mutation, `e1-5` crossover, `e1-9` adversary — all verified against stub `ModelClient`/`SandboxBackend`; live half needs provider keys. `e1-13` benchmark suite — integrity/fairness/delta/gate math verified; live runs need LLM keys + dotnet.
- **Epic 3:** `e3-1` evolve, `e3-2` job_status, `e3-3` job_result, `e3-4` cancel_job — JSON-RPC + ledger + filesystem wiring verified in-process; live Claude Code e2e needs daemon + keys.
- **Epic 4:** `e4-1` pass-through, `e4-2` usage, `e4-4` injection opt-out, `e4-5` latency — verified against stub upstream; live provider needed for real accounting/load.
- **Epic 5:** `e5-1` MAP-Elites, `e5-3` java, `e5-4` c/cpp, `e5-6` archive health — binning/cmd/parsers verified against in-process sinks; live build/`mvn -o`/FalkorDB needed.

### Backlog / blocked (12)
`e0-3` apparmor profile · `e0-4` resource limits · `e0-6` csharp stack · `e2-2` lineage→FalkorDB · `e2-4` verified replay · `e2-5` seeding from winners · `e2-6` approve candidate tests · `e2-8` lineage CLI · `e3-6` setup guide · `e4-3` exemplar injection · `e5-2` selection-mode comparison · `e5-5` test-triage page.

> By convention, roadmap-declared **blocked** stories are recorded as plain `backlog` (not the lifecycle `blocked` status) until their dependency exists. The recurring blockers are: **FalkorDB** (Epic 2 graph + `e4-3` + `e5-1`/`e5-6` live halves), **cgroup v2 delegation / AppArmor** (kernel features, not installable — host-dependent, blocks `e0-4`), **`.NET` / `dotnet` SDK** (`e0-6` C# stack, C# fixtures in `e0-9`/`e1-13`), and **LLM provider keys** (every Epic 1/3/4 live half).

## Where the blockers are

The seam work is exhausted: what remains to move `review`→`done` or `backlog`→shipped requires the external dependencies above, principally a **working LLM endpoint** and a **running FalkorDB**. See [Sprint Workflow](Sprint-Workflow.md) for how those transitions are recorded.

## Per-story specs

Detailed story slices live under `docs/stories/` (with `spec.md`/`plan.md` for those authored so far); epic-level specs are in `docs/epics/` and the sequencing plan in `docs/plans/ROADMAP-EPICS-0-5.md`.
