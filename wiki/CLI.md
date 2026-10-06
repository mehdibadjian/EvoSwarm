# CLI

The `evoswarm` binary lives in `crates/evoswarm-cli` (`main.rs`, clap-based). It has two subcommands and a strict, contract-frozen exit-code scheme so callers can branch on the outcome without parsing stderr.

## `evoswarm run` (story `e1-1`, done)

Submits a job for evolutionary search. Flags:

| Flag | Meaning |
|---|---|
| `--task <text>` | Natural-language description of the task |
| `--cmd <text>` | Test command executed inside the sandbox |
| `--paths <a,b,...>` | Comma-separated relative paths the candidate may modify (the allowlist the tamper gate enforces) |
| `--budget-tokens <n>` | Optional token cap |
| `--budget-dollars <n>` | Optional dollar cap |
| `--objective <correctness|perf>` | Optimisation objective (default `correctness`) |
| `--timeout-secs <n>` | Per-run wall-clock timeout (default 30) |
| `--repo <path>` | Repository root (defaults to cwd) |

Flow: parse → validate → write a durable `Queued` ticket via `run::submit` → print a fast `JobTicket` receipt → map the outcome to an exit code.

## `evoswarm usage` (story `e4-2`, done at the CLI seam)

Prints local gateway token/cost usage by day from the gateway's SQLite usage DB (`.evoswarm/usage.db` by default). Flags: `--since <YYYY-MM-DD>` (defaults 7 days back), `--db <path>`, `--repo <path>`. See [Gateway](Gateway.md).

## Exit-code contract (`exit_codes.rs`)

`ExitCode` is `#[repr(u8)]`; the numeric values are part of the CLI contract and **must never be reused across reasons** (a unit test asserts they're distinct and non-zero for rejections):

| Code | Name | When |
|---|---|---|
| 0 | `Ok` | Job accepted and queued |
| 2 | `Validation` | A flag malformed or missing (e.g. empty `--paths`) |
| 3 | `PathEscape` | A `--paths` entry resolves outside the repo root |
| 4 | `BaselineCommandFailed` | The baseline test command couldn't be executed |
| 5 | `FlakyTestDetected` | Baseline results differed across three consecutive runs |
| 6 | `NothingToImprove` | Every baseline test already passes and `--objective perf` wasn't set |

Code `1` is intentionally unused by this enum, so `1` never collides with a general shell/`main` failure.

## Output artifacts (`artifacts.rs`, `git_writer.rs`, story `e1-11` done)

A completed job emits a fixed artifact set, which is also exactly what `job_result` (MCP) reads:

- **Git branch** `evoswarm/<job-id>` (created via `git_writer::emit_branch_from_patch`, commit message `evoswarm(<job-id>): winning candidate <id>`).
- **Patch file** `.evoswarm/patches/<job-id>.patch`.
- **Report** `.evoswarm/reports/<job-id>.md` (`report.rs`, the audit report with score/tests-passed prose).

## Host self-check (`host_check.rs`, story `e0-2`)

Before accepting new work, the CLI can probe the host (`CheckId`): unprivileged user namespaces, cgroup v2 delegation (memory + pids), and systemd user lingering. `decide(results)` yields a `HostStatus` with `accepts_new_jobs()`. The unprivileged-namespace probe (`unshare -U true`) is **real**; the cgroup-delegation and user-linger probes are a **SEAM** on this host (no delegated cgroups / systemd user bus), which is why `e0-2` is at `review`, not `done`.
