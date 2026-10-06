# Sprint Workflow

EvoSwarm development is driven by `scripts/sprint.py` (the "myloop-lean" ledger manager) plus strict Red-Green-Refactor TDD discipline. This page documents how a story moves from backlog to done, and what "SEAM" means.

## The ledger

Progress lives in `sprint-status.yaml` (epics + stories + `execution_tiers`). Every status change goes through `sprint.py`, which enforces a legal state machine:

```
backlog        → ready-for-dev, blocked
ready-for-dev  → in-progress, blocked, backlog
in-progress    → review, blocked, ready-for-dev
review         → done, in-progress, blocked
blocked        → backlog, ready-for-dev, in-progress, review
done           → review            # reopen if review finds a regression
```

```bash
python3 scripts/sprint.py status            # whole board
python3 scripts/sprint.py next              # first ready-for-dev story + tier
python3 scripts/sprint.py update <key> --status <s>   # validated transition
python3 scripts/sprint.py verify --cmd "..." --anti-cheat
```

## Ledger-first ordering

A story's status is updated **at each stage as it happens**, not batched at the end: set `ready-for-dev` *before* coding, `in-progress` *during*, `review` *after* verification. The ledger is a live record of the workflow, not a retroactive summary.

## TDD cycle

1. **Red** — write a failing test first.
2. **Verify the failure** — `sprint.py verify --cmd "<test cmd>"` runs it.
3. **Green** — author the minimal production code to pass.
4. **Refactor** — clean up; keep clippy `-D warnings` and `cargo fmt --check` clean.

Non-vacuity is proven by **mutation testing**: deliberately break a production line and confirm the test fails, then restore green. This is what stops a "passing test" from being a test that asserts nothing.

## The anti-cheat gate

`verify --anti-cheat` runs `detect_test_tampering(diff_text)`, which inspects the git diff for **deleted or weakened assertions** (removed / `-` assert lines). Weakening an existing test to make it pass is flagged. New test files are safe; removing assertions is not.

## Two PRs per story

Each story ships as **two** pull requests:

1. a **code PR** carrying the tests + production change, then
2. a **separate review/ledger PR** carrying only the `sprint.py update` transition(s).

Legal transitions only ever happen via `sprint.py`; the ledger PR keeps state changes auditable and decoupled from code.

## SEAM vs. done

A story is marked `done` only when it's fully verifiable in code. When part of a story can only be checked against a stub/injected seam — because the real thing needs an external dependency that isn't present — the story stays at **`review`** with an inline `# SEAM:` note in `sprint-status.yaml` spelling out exactly what is proven and what is deferred. Examples of the missing dependencies: LLM provider keys, a running FalkorDB, cgroup delegation, or toolchains like `dotnet`. The seam-verified half is real and mutation-tested; the deferred half is honest about why it can't run yet.

Roadmap-declared **blocked** stories, by contrast, are kept as plain `backlog` entries (not the lifecycle `blocked` status) until their dependency is actually available. See [Roadmap & Status](Roadmap-and-Status.md) for the current board.
