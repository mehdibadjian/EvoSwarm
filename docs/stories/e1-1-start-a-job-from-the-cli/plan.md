# Implementation Plan: Start a job from the CLI

**Story:** `e1-1-start-a-job-from-the-cli` · **Sizing:** M · **Tier:** `flash` · **Target crate:** `evoswarm-cli`
**Depends on:** `e0-5-python-stack`, `e0-6-csharp-stack` — confirm each is `done` in [`sprint-status.yaml`](../../../sprint-status.yaml) before starting.

## 1. Pre-Flight Architecture & Rule Check
Complete these before writing code; tick each only when it actually holds.

- [ ] Reviewed [`ARCHITECTURE-SPINE.md`](../../architecture/ARCHITECTURE-SPINE.md) and confirmed this story upholds its invariants: Governed by AD-7 (SQLite Job Ledger) and AD-1 (Sandbox Isolation).
- [ ] Confirmed compliance with [`architecture-rules.md`](../../../.agents/rules/architecture-rules.md), [`tdd-discipline.md`](../../../.agents/rules/tdd-discipline.md) and [`security-hygiene.md`](../../../.agents/rules/security-hygiene.md).
- [ ] Confirmed every dependency listed above is `done`; otherwise stop and report the blocker.
- [ ] Confirmed scope matches [`spec.md`](spec.md) section 2 and nothing outside it is being built.
- [ ] Committed to zero AI comments in production code: comments explain **WHY**, never **WHAT**.

---

## 2. Red Phase (Failing Tests to Author First)
Author these tests before any production code, then run the gate command in section 5 and
confirm every test fails for its intended reason. A test that passes before implementation
is a false-positive test and must be rewritten.

- `crates/evoswarm-cli/tests/e1_1_start_a_job_from_the_cli.rs::test_submit_valid_job_fast_return` — Submits a valid job and asserts the JSON ticket is emitted in under 2.0 s with SQLite job state `queued`.
- `crates/evoswarm-cli/tests/e1_1_start_a_job_from_the_cli.rs::test_reject_path_traversal_outside_repo` — Passes `--paths ../outside` and asserts `ExitCode::PathEscape` with the resolved path in stderr.
- `crates/evoswarm-cli/tests/e1_1_start_a_job_from_the_cli.rs::test_reject_broken_baseline_command` — Supplies a non-existent binary as the test command and asserts immediate rejection carrying the child exit code.
- `crates/evoswarm-cli/tests/e1_1_start_a_job_from_the_cli.rs::test_reject_flaky_baseline_suite` — Points at a scripted runner that alternates pass/fail and asserts rejection plus the list of varying test names.
- `crates/evoswarm-cli/tests/e1_1_start_a_job_from_the_cli.rs::test_reject_already_passing_suite_without_perf_flag` — Supplies a clean passing suite without `--objective perf` and asserts `ExitCode::NothingToImprove` naming the flag.

**Target file(s):** `crates/evoswarm-cli/tests/e1_1_start_a_job_from_the_cli.rs`
All tests for this story live in the single crate-scoped target above. It does not exist yet: author it in the Red phase before any production code. Tests must sit at `crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested path as a helper module and never compiles it as a `--test` target.

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
| AC1 | a job ID prints within 2 s and the job status is `queued` in SQLite | `test_submit_valid_job_fast_return` |
| AC2 | the job is rejected with `ExitCode::PathEscape` and the resolved path shown | `test_reject_path_traversal_outside_repo` |
| AC3 | the job is rejected with the command's non-zero output and exit code shown | `test_reject_broken_baseline_command` |
| AC4 | the job is rejected and the flaky tests are listed | `test_reject_flaky_baseline_suite` |
| AC5 | I am told there is nothing to improve and asked for `--objective perf` | `test_reject_already_passing_suite_without_perf_flag` |

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
- `Cargo.toml` — virtual workspace manifest with `members = ["crates/*"]` and shared `[workspace.dependencies]`.
- `crates/evoswarm-cli/src/main.rs` — clap parser for `run`, dispatching to `submit()`.
- `crates/evoswarm-cli/src/run.rs` — `pub async fn submit(sub: JobSubmission, repo: &RepoRoot, ledger: &JobLedger) -> Result<JobTicket, SubmitError>`.
- `crates/evoswarm-cli/src/exit_codes.rs` — `ExitCode` enum mapped to distinct process codes.
- `crates/evoswarm-core/src/job.rs` — `JobSubmission`, `JobObjective`, `JobStatus`, `JobTicket`.
- `crates/evoswarm-core/src/path_guard.rs` — `pub fn require_within_root(root: &Path, candidate: &Path) -> Result<PathBuf, PathEscape>`.
- `crates/evoswarm-ledger/src/lib.rs` — `JobLedger::open`, `insert_job`, `transition`, `read_status` over SQLite in WAL mode.

### Work order
1. Scaffold the workspace and confirm `cargo test --workspace` runs with zero tests.
2. Implement `require_within_root` using `std::fs::canonicalize` on the joined path, then `strip_prefix(root)`; reject on `Err`. Never compare raw strings, which `..%2f` and symlink forms defeat.
3. Add `JobLedger` with `PRAGMA journal_mode=WAL` and the `jobs` table from AD-7.
4. Wire `submit()`: parse, validate paths, insert `Queued`, print ticket, then run baseline validation and transition state on failure.
5. Run the baseline command 3 times through `SandboxBackend::run`, diff the parsed test-name to status map, and emit `FlakyTestDetected` with the varying names.
6. Gate on all-green plus missing `perf` objective returning `NothingToImprove`.

### Implementation note
The 2 s ticket deadline is met by persisting and printing before baseline validation, not by making validation fast. `SandboxBackend` is a trait so tests inject a scripted fake instead of bwrap.

---

## 4. Refactor Phase
- Remove redundant allocations and any temporary scaffolding introduced to reach green.
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- Re-run the full `evoswarm-cli` suite, not just this story's target, to catch regressions.
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "cargo test -p evoswarm-cli --test e1_1_start_a_job_from_the_cli" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update e1-1-start-a-job-from-the-cli --status done
```
