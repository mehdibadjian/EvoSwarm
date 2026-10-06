# Crates Reference

The workspace (`Cargo.toml`: `members = ["crates/*"]`, `resolver = "2"`) contains 10 crates. Shared dependency versions come from `[workspace.dependencies]`: `serde`, `serde_json`, `toml`, `arc-swap`, `thiserror`, `async-trait`, `sha2`, `tempfile`, `uuid` (v4), `clap`, `rusqlite` (bundled), `git2`, `tokio`.

Dependency shape: `evoswarm-core` is the leaf domain crate everything depends on. `sandbox`, `fitness`, `memory`, `models`, and `ledger` build on core. `engine` composes those; `cli` and `mcp` are the user-facing binaries/servers; `gateway` is the optional proxy.

## evoswarm-core — domain types (no heavy deps)

Shared vocabulary used by every crate.

| Module | Contents |
|---|---|
| `job.rs` | `JobSubmission` (task, test_command, target_paths, budgets, objective, timeout), `JobObjective` (`correctness`/`perf`), `JobStatus` (`queued`/`running`/`completed`/`failed`/`budget_exhausted`/`cancelled`), `JobTicket` (fast ≤2 s receipt) |
| `execution.rs` | `ExecutionResult` + `RunStatus` (success / failed / timeout / oom / tamper); a candidate's own failure is data, not a sandbox error |
| `outcome.rs`, `candidate.rs` | Candidate representation and outcome types |
| `path_guard.rs` | Resolves `--paths` / diff paths against the repo root; rejects escapes |
| `provenance.rs` | Test provenance model (trusted / held-out / adversary) |
| `adversary.rs`, `holdout.rs` | Adversary and held-out policy types |
| `usage.rs`, `report_model.rs` | Token/dollar usage accounting and the report structure |

Tests: `path_guard.rs`.

## evoswarm-sandbox — The Crucible

Isolation boundary for untrusted candidate code. See [The Crucible](The-Crucible.md).

- `lib.rs` — the `SandboxBackend` trait: `prepare(profile, candidate_patch) -> workdir`, `run(workdir, test_command) -> ExecutionResult`, `collect(workdir)`. `SandboxError` covers isolation-layer failures (`Prepare`, `Execution`, `Cleanup`, `Unavailable`), deliberately distinct from a candidate's non-zero exit.
- `bwrap.rs` (~392 lines) — `BwrapBackend`: `--unshare-all` (no network), read-only toolchain/test mounts, tmpfs workdirs, wall-clock timeouts via SIGKILL.
- `seccomp.rs` — syscall filter (story `e2-7`, done).
- `tamper.rs` — `detect_harness_override`, `hash_tests_dir`, `PROHIBITED_HARNESS_FILES` (conftest.py, .props, build scripts).
- `stacks/` — `python.rs` (venv cache, JUnit XML parsing: `parse_junit_xml`, `parse_junit_outcomes`, `JunitSummary`, `TestOutcome`, `TestStatus`, `lockfile_hash`), `java.rs` (pom hash, offline mvn), `cpp.rs` (cmake, gtest/catch2 parsers).
- `calibration.rs` — resource-limit profiling math.
- `red_team.rs` (~417 lines) — escape/egress/DNS/fork-bomb denial suite.

Tests: `e0_1_sandbox_runner`, `e0_5_python_stack`, `e0_7_tamper_proof`, `e0_8_red_team_suite`, `e0_9_limit_calibration`, `e2_7_seccomp_filter`, `e5_3_java_stack`, `e5_4_c_and_cpp_stack`.

## evoswarm-fitness — gates & scoring

See [Fitness & Scoring](Fitness-and-Scoring.md). Modules: `gates.rs` (four hard gates, pure `evaluate`), `scoring.rs` (weighted score + `Weights::validate`), `holdout.rs`, `tamper.rs`, `leak_scan.rs`, `baseline.rs`. Tests: `e1_6_hard_gates`, `e1_7_weighted_score`, `e1_8_held_out_tests`.

## evoswarm-engine — search loop

See [Engine Search Loop](Engine-Search-Loop.md). Modules: `seeding.rs`, `mutation.rs`, `crossover.rs`, `adversary.rs`, `feedback.rs` (error feedback into prompts), `budget.rs` (token/dollar enforcement, 258 lines), `selection.rs`, `early_stop.rs`, `recovery.rs` (crash resume), `dedup.rs`, `dispatch.rs`, `map_elites.rs`, `archive_report.rs`. Tests: `e1_3_seed`, `e1_4_mutation`, `e1_5_crossover`, `e1_9_adversary`, `e1_10_budget`, `e1_12_resume`, `e5_1_map_elites_grid`, `e5_6_archive_health_report`.

## evoswarm-models — provider shaping

`config.rs` (model roles: mutator / synthesiser / adversary), `prompt.rs`, `context_guard.rs` (348 lines; `e3-5` done), `hot_reload.rs` (arc-swap), `idempotency.rs`, `lineage.rs`, `error.rs`. Tests: `e1_2_model_roles_in_config`, `e3_5_context_guard` (+ `fixtures`).

## evoswarm-ledger — durable queue

`ledger.rs` (385 lines) — job records + lifecycle transitions backed by bundled SQLite; `RepoRoot`; `LedgerError::IllegalTransition` for illegal status moves. `generations.rs` (max_generation tracking), `cache.rs`. No crate-level test files (tested via engine/cli/mcp consumers).

## evoswarm-memory — blobs & fingerprints

See [Memory & Fingerprints](Memory-and-Fingerprints.md). `blob_store.rs` (SHA-256 content-addressed store + `.lastref` GC), `fingerprints.rs` (repo + toolchain SHA-256). Tests: `e2_1_blob_store`, `e2_3_fingerprints`.

## evoswarm-gateway — optional proxy

See [Gateway](Gateway.md). `proxy.rs` (352 lines, Axum/Tokio SSE pass-through), `usage.rs` (327 lines, tee + sqlite), `injection.rs`, `latency.rs`, `config.rs`. Tests: `e4_1_transparent_pass_through`, `e4_2_usage_logging`, `e4_4_injection_opt_out`, `e4_5_latency_budget_in_ci` (+ `common`).

## evoswarm-mcp — Claude Code server

`lib.rs` (628 lines) — JSON-RPC 2.0 server exposing `evolve`, `job_status`, `job_result`, `cancel_job`. See [MCP Server](MCP-Server.md). Tests: `e3_1_evolve_tool`, `e3_2_job_status_tool`, `e3_3_job_result_tool`, `e3_4_cancel_job_tool` (+ `common`).

## evoswarm-cli — binary `evoswarm`

`main.rs` (clap: `run`, `usage`), `run.rs` (`submit`), `exit_codes.rs` (contract), `baseline.rs`, `artifacts.rs` (patch/report/branch layout), `git_writer.rs` (`git2` branch + commit), `report.rs`, `usage.rs`, `host_check.rs`. Tests: `e0_2_host_self_check`, `e1_1_start_a_job_from_the_cli`, `e1_11_patch_and_report`, `e4_2_usage_cli`.
