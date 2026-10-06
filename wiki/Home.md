# EvoSwarm Wiki

> Self-hosted evolutionary code synthesis with persistent graph memory, sealed bubblewrap sandboxes, and native Claude Code MCP integration.

EvoSwarm is an autonomous Rust service that evolves code against automated test suites and remembers what worked. Given an engineering task from the CLI or Claude Code, it breeds candidate patches with specialized LLM roles, validates them inside sealed `bwrap` sandboxes, and emits verified patches backed by execution evidence and lineage.

This wiki documents the repository **as it exists in the code today**. Each page is grounded in source under `crates/`, `bench/`, `docs/`, and `scripts/` — not in design intent that hasn't shipped. Where a capability is only partially wired, the page says so explicitly.

## What's actually built

The workspace is a Cargo workspace (`members = ["crates/*"]`) of **10 crates**. The core substrate — sandbox isolation, fitness gating and scoring, the search loop, ledger/queue, MCP JSON-RPC surface, CLI, gateway, and content-addressed memory primitives — exists and is unit-tested. Live end-to-end execution is gated behind LLM provider keys and host kernel features that are described on the [Roadmap & Status](Roadmap-and-Status.md) page.

## Pages

| Page | What it covers |
|---|---|
| [Architecture](Architecture.md) | Subsystems, data flow, the three operating modes |
| [Crates Reference](Crates-Reference.md) | All 10 crates, key modules, tests, dependencies |
| [The Crucible (Sandbox)](The-Crucible.md) | `bwrap` + cgroups, seccomp, per-language stacks, red-team |
| [Fitness & Scoring](Fitness-and-Scoring.md) | The four hard gates and the multi-objective score |
| [Engine Search Loop](Engine-Search-Loop.md) | Seeding, mutation, crossover, adversary, budget, recovery, MAP-Elites |
| [Memory & Fingerprints](Memory-and-Fingerprints.md) | Content-addressed blob store and replay fingerprints |
| [MCP Server](MCP-Server.md) | The four JSON-RPC tools and the two-channel error model |
| [CLI](CLI.md) | `evoswarm run` / `evoswarm usage` and the exit-code contract |
| [Gateway](Gateway.md) | Transparent pass-through, usage logging, injection opt-out, latency |
| [Benchmark Suite](Benchmark-Suite.md) | The Python `bench/` harness and the ≥15-point gate |
| [Sprint Workflow](Sprint-Workflow.md) | `sprint.py` ledger, TDD discipline, SEAM classification |
| [Roadmap & Status](Roadmap-and-Status.md) | Epics 0–5 and the live ledger state |
| [Security & Anti-Cheat](Security-and-Anti-Cheat.md) | Test immutability, tamper detection, secret hygiene |

## Quick orientation

- **Product spec:** [`docs/prd/evoswarm-prd.md`](../docs/prd/evoswarm-prd.md)
- **Architecture decisions:** [`docs/architecture/ARCHITECTURE-SPINE.md`](../docs/architecture/ARCHITECTURE-SPINE.md)
- **Roadmap:** [`docs/plans/ROADMAP-EPICS-0-5.md`](../docs/plans/ROADMAP-EPICS-0-5.md)
- **Sprint ledger:** [`sprint-status.yaml`](../sprint-status.yaml) (`python3 scripts/sprint.py status`)
- **Entry points:** binary `crates/evoswarm-cli/src/main.rs`, MCP server `crates/evoswarm-mcp/src/lib.rs`

## Build & test

```bash
cargo build --workspace
cargo test  --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all --check

# Benchmark suite (Python)
python3 -m pytest bench/tests -v

# Sprint ledger
python3 scripts/sprint.py status
python3 scripts/sprint.py next
```

Target platform: a single headless Linux machine (down to repurposed 2012-era hardware, 16 GB RAM). No VMs or container daemons required.
