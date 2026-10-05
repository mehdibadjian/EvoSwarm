# Execution Roadmap — Epics 0–5

Status: proposed · Owner: Qoder · Environment probe date: 2026-10-05

This document turns the six epics into a single dependency-ordered execution plan and
states honestly which stories can be **executed and exit-gate-certified in this
environment**, which can be **implemented against a stub seam** (deterministic logic
verified, live dependency absent), and which are **blocked** until infrastructure or
credentials arrive. It is the plan-of-record referenced by every implementation PR.

## 1. Method (unchanged from e1-2)

Every story is delivered the same way, with no shortcuts:

1. Three-artifact chain exists under `docs/stories/<key>/` (Epic 1) or the flat
   `docs/stories/<key>.md` (Epics 0, 2–5). Validate with
   `python3 scripts/sprint.py validate-chain <dir>` where a chain dir exists.
2. **Red:** author the failing integration test(s) named in the story plan, at
   `crates/<crate>/tests/<target>.rs` (subdirectory test paths are Cargo helper modules
   and are *never* compiled as `--test` targets).
3. **Green:** implement the minimum production code to pass.
4. **Refactor:** remove dead code, tighten visibility, keep clippy `-D warnings` clean.
5. **Verify:** `python3 scripts/sprint.py verify --cmd "<test cmd>" --anti-cheat` must
   print `VERIFICATION PASSED`.
6. **Ledger:** advance status through the legal transitions
   (`ready-for-dev → in-progress → review → done`), then PR → review → merge to `main`.

No story is marked `done` on a stub. Where a live dependency is absent, the story stops
at `review` with a `SEAM` note, and the ledger keeps it out of `done` until the gate can
be certified. This is the anti-fake-progress rule.

## 2. Target workspace layout

The Cargo workspace rooted at `Cargo.toml` (created by e1-2, already on `main`):

```
crates/evoswarm-core/       domain types: Job, Candidate, ExecutionResult, SandboxProfile, path_guard
crates/evoswarm-sandbox/    SandboxBackend trait + bwrap impl (Epic 0)
crates/evoswarm-models/     role config (done), dispatch, prompt caching, lineage seam (AD-6)
crates/evoswarm-ledger/     SQLite job ledger, WAL (AD-7)
crates/evoswarm-fitness/    hard gates, scoring, held-out split (AD-5)
crates/evoswarm-engine/     seeding, mutation, crossover, adversary, budget, recovery
crates/evoswarm-cli/        `evoswarm run|status|lineage|approve-tests|usage`
crates/evoswarm-memory/     blob store, fingerprints, FalkorDB lineage (AD-3, AD-4)
crates/evoswarm-mcp/        MCP tools: evolve, job_status, job_result, cancel_job
crates/evoswarm-gateway/    Axum SSE reverse proxy (AD-2)
bench/                      Python benchmark harness (Gate 1, Gate 5)
```

Shared types (`ExecutionResult`, `SandboxProfile`, `RunStatus`, `Candidate`) live in
`evoswarm-core` and are re-exported by the crates that need them, so the search loop stays
decoupled from isolation mechanics (AD-1).

## 3. Environment probe (2026-10-05) — what actually runs here

| Capability | Present? | Evidence | Consequence |
|---|---|---|---|
| `bwrap --unshare-all` | **Yes** | `bwrap --unshare-all --ro-bind / / --tmpfs /tmp echo` → exit 0 | Sandbox isolation, net/filesystem/ptrace denial enforceable |
| Rust 1.99 + crates.io | **Yes** | `cargo build --workspace` green; registry reachable | Real compilation & tests |
| python3 / venv / pytest | **Yes** | pytest 9.1.1 installed in a fresh venv | Python stack runnable offline |
| java / mvn / cmake / gcc / g++ / make | **Yes** | all on PATH | Java and C/C++ stacks buildable |
| sqlite3 | **Yes** | `/usr/bin/sqlite3`; rusqlite fetchable | Ledger & resume testable |
| git / gh | **Yes** | PRs #5, #6 merged this session | Branch/patch/report testable |
| cgroup v2 **write** (memory.max/pids.max) | **No** | `mkdir` under own cgroup OK but `memory.max` write → Permission denied; no delegation | Hard memory/pids caps **cannot be enforced or verified** |
| `systemd-run --user --scope` | **No** | fails under `dbus-run-session`: `systemd1 exited with status 1`, no session bus | AD-1 resource-scope mechanism unavailable |
| AppArmor | **No** | `aa-status` missing | Cannot load/verify the bwrap profile |
| `dotnet` | **No** | not on PATH | C# stack unbuildable |
| FalkorDB / redis | **No** | neither on PATH | Graph lineage & replay unverifiable |
| LLM API keys | **No** | env has no `*_API_KEY` / provider keys | Real model dispatch impossible |
| seccomp | **Yes** | `/proc/self/status` `Seccomp: 0` (filterable); bwrap `--seccomp` available | BPF filter applicable |

**Net effect:** isolation *shape* (namespaces, ro mounts, net denial, ptrace denial) is
enforceable, but isolation *resource caps* (cgroup memory/pids, systemd scope) and all
external services (LLM, FalkorDB, dotnet, AppArmor) are not. Plans below respect this.

## 4. Story classification (47 stories; e1-2 already `done`)

Classes: **EXEC** = fully executable and exit-gate-certifiable here · **SEAM** =
deterministic logic executable against a stub/trait seam, live gate not certifiable ·
**BLOCKED** = cannot be meaningfully executed here.

### Epic 0 — The Crucible (AD-1)

| Story | Class | Rationale |
|---|---|---|
| e0-1 sandbox-runner-interface | **EXEC** | bwrap + wall-clock timeout kill; lifecycle/isolation tests run for real |
| e0-2 host-self-check | **SEAM** | check functions testable with injectable probes; "ready" path not achievable (cgroup delegation genuinely absent → correctly reports not-ready) |
| e0-3 apparmor-profile-for-bwrap | **BLOCKED** | no `aa-status`, no profile load; ship installer script only |
| e0-4 resource-limits | **BLOCKED** | cgroup writes denied, systemd scope fails → caps unenforceable/unverifiable |
| e0-5 python-stack | **EXEC** | venv+pytest offline; net-egress denial via `--unshare-net`; JUnit parse |
| e0-6 csharp-stack | **BLOCKED** | no `dotnet` |
| e0-7 tamper-proof-tests | **EXEC** | ro-bind `/work/tests` → EROFS; harness-override rejection is pure path logic |
| e0-8 red-team-suite | **SEAM** | net/`/etc/shadow`/ptrace/`/proc/1` denial enforceable under bwrap; memory-starvation cases need e0-4 (blocked) |
| e0-9 limit-calibration | **SEAM** | p50/p95 statistics & profile generation pure & testable; 100-run C# baseline blocked |

### Epic 1 — Evolve CLI & Fitness (AD-5/6/7)

| Story | Class | Rationale |
|---|---|---|
| e1-1 start-a-job-from-the-cli | **EXEC** | SQLite ledger + path guard + baseline run through the python stack backend |
| e1-2 model-roles-in-config | **done** | merged PR #5 |
| e1-3 seed-the-first-generation | **SEAM** | quota/dedup/metadata deterministic via stub `ModelClient`; real drafts need keys |
| e1-4 mutation-with-error-feedback | **SEAM** | 4k-token clamp, prefix byte-identity, `MUTATED_FROM` edge testable via stub |
| e1-5 crossover-of-two-parents | **SEAM** | Hamming ranking, 25% cap, dual `MERGED_FROM` testable via stub |
| e1-6 hard-gates | **EXEC** | pure gate evaluation over constructed `ExecutionResult` + patch paths |
| e1-7 weighted-score | **EXEC** | pure deterministic math |
| e1-8 held-out-tests | **EXEC** | stable 80/20 partition + leak scan pure; held-out run uses e0-1 sandbox |
| e1-9 adversary-tests | **SEAM** | K-count/uncompilable-discard/suspect-filter testable via stub |
| e1-10 budget-enforcement | **EXEC** | projection & early-stop are pure counter logic against config (no live call needed) |
| e1-11 patch-and-report | **EXEC** | git branch + `.patch` + markdown report; base untouched |
| e1-12 resume-after-crash | **EXEC** | SQLite resume + idempotency dedup (stub dispatch for the model-call reuse) |
| e1-13 benchmark-suite | **SEAM** | suite loading/validation & solve-rate math testable; real EvoSwarm-vs-single-shot needs keys |

### Epic 2 — System 1 Memory & Replay (AD-3/4)

| Story | Class | Rationale |
|---|---|---|
| e2-1 content-addressed-blob-store | **EXEC** | pure fs SHA-256 store + dedup + GC sweep |
| e2-2 lineage-written-to-falkordb | **BLOCKED** | no FalkorDB; schema/spool logic only |
| e2-3 repo-and-toolchain-fingerprints | **EXEC** | pure hashing over files/versions |
| e2-4 verified-replay | **BLOCKED** | depends on e2-2 graph |
| e2-5 seeding-from-similar-winners | **BLOCKED** | FalkorDB + LLM |
| e2-6 approve-candidate-tests | **BLOCKED** | depends on e1-9 (live) + e2-2 |
| e2-7 seccomp-filter | **SEAM** | BPF filter appliable under bwrap `--seccomp`; depends on e0-8 (partial) |
| e2-8 lineage-cli | **BLOCKED** | depends on e2-2 graph; JSON export shape only |

### Epic 3 — Claude Code Integration / MCP (AD-2/7)

| Story | Class | Rationale |
|---|---|---|
| e3-1 evolve-tool | **SEAM** | MCP JSON-RPC layer + schema testable against stub daemon; e2e needs LLM |
| e3-2 job-status-tool | **SEAM** | polling/ETA/cost shape testable against stub ledger |
| e3-3 job-result-tool | **SEAM** | retrieval shape testable against stub; needs e1-11 (EXEC) |
| e3-4 cancel-job-tool | **SEAM** | cancellation state transition testable against stub ledger |
| e3-5 context-guard | **EXEC** | path allowlist + secret redaction is pure logic |
| e3-6 setup-guide | **BLOCKED** | docs/quickstart depend on the full stack running |

### Epic 4 — Optional Gateway (AD-2)

| Story | Class | Rationale |
|---|---|---|
| e4-1 transparent-pass-through | **SEAM** | Axum SSE proxy testable against a local stub upstream (no LLM needed) |
| e4-2 usage-logging | **SEAM** | token/cost accounting testable against stub stream |
| e4-3 exemplar-injection | **BLOCKED** | depends on e2-5 (FalkorDB) |
| e4-4 injection-opt-out | **SEAM** | header/config bypass logic testable against stub |
| e4-5 latency-budget-in-ci | **SEAM** | p95 TTFT harness testable against stub upstream |

### Epic 5 — MAP-Elites & More Stacks (AD-1/8)

| Story | Class | Rationale |
|---|---|---|
| e5-1 map-elites-grid | **SEAM** | 4×4 archive grid pure & testable; depends on e1-7 (EXEC) + e2-2 (blocked) |
| e5-2 selection-mode-comparison | **BLOCKED** | benchmark + LLM |
| e5-3 java-stack | **SEAM** | java/mvn present; stack profile runnable under bwrap, but hard caps need e0-4 (blocked) |
| e5-4 c-and-cpp-stack | **SEAM** | gcc/cmake present; same e0-4 cap caveat |
| e5-5 test-triage-page | **BLOCKED** | depends on e2-6 (blocked) |
| e5-6 archive-health-report | **SEAM** | occupancy/diversity report pure & testable; depends on e5-1 (SEAM) |

**Tally:** EXEC = 13 (e0-1, e0-5, e0-7, e1-1, e1-6, e1-7, e1-8, e1-10, e1-11, e1-12, e2-1, e2-3, e3-5) · SEAM = 20 · BLOCKED = 13 · done = 1 (e1-2).

## 5. Execution waves (dependency-ordered)

Within a wave, stories touch disjoint crates/files and may proceed in parallel; across
waves they are sequential. Shared workspace scaffolding (`Cargo.toml` members,
`evoswarm-core` types) is authored **once, first**, to avoid parallel edits to the root
manifest.

**Wave 0 — scaffolding (prerequisite, one PR).** Create `evoswarm-core` with
`ExecutionResult`, `SandboxProfile`, `RunStatus`, `Candidate`, `path_guard`; add the
`evoswarm-sandbox` crate skeleton with the `SandboxBackend` trait from ARCHITECTURE-SPINE
§3.1. `cargo test --workspace` must pass with the trait compiling.

**Wave 1 — isolation + pure fitness/memory (no cross-deps).**
e0-1 → e0-7 → e0-5 (sandbox chain); in parallel: e1-7, e1-6, e2-1, e2-3 (pure logic).

**Wave 2 — ledger, CLI, reporting (needs Wave 1 + rusqlite).**
e1-1 → e1-12 → e1-11; in parallel: e1-8, e1-10.

**Wave 3 — engine seams (needs Waves 1–2; stub `ModelClient`/`LineageSink`).**
e1-3, e1-4, e1-5, e1-9 — deterministic ACs verified against the trait seam; left at
`review` with a `SEAM` note, not `done`.

**Wave 4 — secondary seams & stacks (optional, after the critical path is green).**
e3-5 (EXEC), e4-1/2/4/5 (SEAM, stub upstream), e5-3/e5-4 (SEAM stacks), e2-7/e5-1/e5-6
(SEAM), e0-8/e0-2/e0-9 (SEAM, partial containment).

**Blocked backlog (no work until unblocked).** e0-3, e0-4, e0-6, e2-2, e2-4, e2-5, e2-6,
e2-8, e3-6, e4-3, e5-2, e5-5, plus the live-gate halves of e1-13.

## 6. Unblock conditions (what would move SEAM/BLOCKED → EXEC)

- **LLM provider keys** (`*_API_KEY`): e1-3/4/5/9/13 live gates, e3-1..e3-4 e2e, e2-5/6.
- **cgroup v2 delegation** (or a host with `systemd-run --user` session bus): e0-4, and
  the hard-cap halves of e0-8, e5-3, e5-4.
- **AppArmor toolchain + root**: e0-3, and the "ready" path of e0-2.
- **`dotnet` SDK**: e0-6 and the C# half of e1-13, e0-9.
- **FalkorDB (redis-compatible)**: e2-2, e2-4, e2-5, e2-6, e2-8, e4-3, e5-1(graph half).

## 7. Epic exit-gate certification honesty

Exit gates that require a live LLM, FalkorDB, cgroup caps, or dotnet **cannot be
certified in this environment**, and this plan will not claim otherwise:

- **Gate 0** (100 Python+C# runs, 0 escapes): Python + escape-denial subset certifiable;
  C# and cgroup-containment are **not**. → partial.
- **Gate 1** (≥15pt solve-rate vs single-shot): needs LLM keys → **not certifiable here**.
- **Gate 2** (replay never stale, ≥30% token cut): needs FalkorDB + LLM → **not**.
- **Gate 3** (e2e MCP evolve → patch): needs running daemon + LLM → **not** (e3-5 EXEC).
- **Gate 4** (<50ms p95 TTFT, 20 streams): certifiable against a **stub upstream** →
  SEAM, not against a real provider.
- **Gate 5** (MAP-Elites ≥ Top-K; Java/C++ 100% containment): needs LLM + cgroup caps →
  **not** fully.

Where a gate is not certifiable, the implementation lands behind its seam with passing
deterministic tests, the story is held at `review`, and the PR states exactly which gate
clause is deferred and why. This keeps `main` honest and green without faking coverage.

## 8. Definition of done for this roadmap task

1. This plan is merged to `main`.
2. Wave 0 scaffolding merged; `cargo test --workspace` and `cargo clippy --workspace
   --all-targets -- -D warnings` clean.
3. Waves 1–2 EXEC stories implemented via TDD, each `VERIFICATION PASSED --anti-cheat`,
   each merged to `main` and marked `done` in the ledger.
4. Wave 3+ SEAM stories implemented against their seams, deterministic tests green, held
   at `review` with a `SEAM` note; ledger and PRs name the deferred gate clause.
5. BLOCKED stories untouched, with unblock conditions recorded here.

Progress is reported per wave with the ledger board (`python3 scripts/sprint.py status`)
as the source of truth — never a claim of `done` that the environment cannot back.
