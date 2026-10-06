# Security & Anti-Cheat

EvoSwarm runs **untrusted, model-generated code** and is judged by **automated tests it must not be able to cheat**. Security therefore has two independent axes: containment of the code (The Crucible), and integrity of the evaluation (anti-cheat).

## 1. Containment — the sandbox never trusts the candidate

Enforced in `evoswarm-sandbox` (see [The Crucible](The-Crucible.md)):

- **No network** — `bwrap --unshare-all` tears down the network namespace; egress and DNS escape attempts are covered by the red-team suite (`red_team.rs`).
- **Isolated namespaces + tmpfs workdirs** — each candidate builds in a fresh ephemeral directory, so it can't touch the host or leak state across runs.
- **seccomp syscall filter** (`seccomp.rs`, `e2-7` done) — blocks syscalls a candidate should never need, on top of namespace isolation.
- **cgroups v2 resource limits** — memory/PID/CPU caps applied via host cgroups (wall-clock timeouts via SIGKILL). *Host note:* cgroup **delegation** is a kernel/host feature, not a package; where it isn't delegated, the resource-limit live enforcement (`e0-4`) stays blocked.
- **fork-bomb containment** — asserted in the red-team suite.

## 2. Evaluation integrity — tests are immutable and read-only

The whole premise is that a candidate can't rewrite the tests that judge it:

- **Read-only test mounts** — test directories are mounted `--ro-bind`; a candidate literally cannot modify them at runtime.
- **Harness denylist** — `tamper.rs::PROHIBITED_HARNESS_FILES` includes `conftest.py` and `Directory.Build.props` (build/harness overrides). `detect_harness_override` flags attempts to override them; `hash_tests_dir` fingerprints the test tree.
- **Gate 3 (tamper)** in fitness — `tamper::detect(patch_paths, allowed_paths)` fails the candidate if its diff touches any test/harness/build file **or** strays outside the declared `--paths` allowlist. See [Fitness & Scoring](Fitness-and-Scoring.md).
- **Gate 4 (no skips)** — a candidate can't delete or `skip` tests to pass; executed count must meet baseline and skips must not exceed baseline.

## 3. Path-escape defense at the boundary

`core::path_guard` resolves every `--paths` / diff path against the canonicalised repo root. An entry that resolves outside it is rejected — in the CLI as exit code `PathEscape` (3), and in the MCP `evolve` tool as a tool-level error. See [CLI](CLI.md) and [MCP Server](MCP-Server.md).

## 4. Secret hygiene

- **Leak scanning** — `fitness::leak_scan` inspects prompts/diffs for secret leakage.
- **Redaction before the provider** — outgoing prompts are pattern-scanned; `.env` and `*.pem` files are strictly excluded from context.
- **Repo hygiene** — `.gitignore` excludes `.env`, `.env.*`, runtime state, and scratch, so secrets are less likely to be committed by accident.
- **Credential handling** — provider keys are read from the environment only, never written to tracked files or logged.

## 5. Anti-cheat verification gate

`scripts/sprint.py verify --anti-cheat` runs `detect_test_tampering` over the git diff and flags **deleted or weakened assertions** (removed / `-` assert lines). Combined with mutation testing (break a line → the test must fail), this keeps a green test suite honest. It's part of every story's review step — see [Sprint Workflow](Sprint-Workflow.md).

## Status of these guarantees

Deterministic and fully green today: tamper-proof tests (`e0-7`), seccomp (`e2-7`), the gate pipeline (`e1-6`), path-guard, and the anti-cheat gate. Containment that needs host/kernel features not delegated on this sandbox — cgroup delegation (`e0-4`), the cgroup/shadow-read red-team cases (`e0-8`), and AppArmor (`e0-3`) — remain at `review`/`backlog`. The design and the code paths are present; the live host capability is the gap, recorded honestly per story.
