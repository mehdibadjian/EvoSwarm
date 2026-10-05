# Story Specification: Start a job from the CLI

## 1. Functional Specification
### 1. Command-Line Interface Contract
```bash
evoswarm run \
  --task "<task description string>" \
  --cmd "<test execution command>" \
  --paths "<relative path 1>,<relative path 2>" \
  [--budget-tokens <tokens>] \
  [--budget-dollars <usd>] \
  [--objective <correctness|perf>] \
  [--timeout-secs <seconds>]
```

### 2. Core Data Models (Rust)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSubmission {
    pub task_description: String,
    pub test_command: String,
    pub target_paths: Vec<PathBuf>,
    pub budget_tokens: Option<u64>,
    pub budget_dollars: Option<f64>,
    pub objective: JobObjective,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobObjective {
    Correctness,
    Performance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    BudgetExhausted,
    Cancelled,
}
```

### 3. Execution & Validation Rules
1. **Repository Boundary Check:** All paths in `--paths` must reside strictly within the repository root. Any traversal (`../`) is rejected immediately.
2. **Baseline Flakiness Check:** EvoSwarm executes the test command 3 consecutive times in a temporary sandbox. If results differ across runs, the job is rejected with `ExitCode::FlakyTestDetected` and the offending tests are printed.
3. **Zero-Diff Prevention:** If all baseline tests pass and `--objective` is not set to `perf`, the job is rejected with instructions to specify `--objective perf`.
4. **Fast Job Ticket:** Outputs `{ "job_id": "<uuid>", "status": "queued" }` within $< 2$ seconds. The ticket is printed before baseline validation completes; validation failure is reported as a job state transition, not a lost ticket.
5. **Exit Codes:** Distinct process exit codes per rejection reason so callers can branch without parsing stderr.

### 4. Workspace Scaffold (Prerequisite)
This story creates the Cargo workspace consumed by every later Epic 1 story: a virtual
manifest listing `crates/*`, plus `evoswarm-core`, `evoswarm-cli`, `evoswarm-ledger` and
`evoswarm-sandbox` with the `SandboxBackend` trait from ARCHITECTURE-SPINE section 3.1.

---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `--task`, `--cmd`, `--paths`, optional budgets | `{"job_id": "<uuid>", "status": "queued"}` on stdout, exit 0 | `ValidationError` naming the offending flag |
| `--paths` entry resolving outside repo root | No job created, exit `ExitCode::PathEscape` | Prints resolved path and repo root |
| Baseline `--cmd` exits non-zero or is not executable | No job created, exit `ExitCode::BaselineCommandFailed` | Child stdout/stderr and exit code echoed |
| Baseline results differ across 3 sandbox runs | No job created, exit `ExitCode::FlakyTestDetected` | Lists test names whose status varied |
| All baseline tests pass and `--objective != perf` | No job created, exit `ExitCode::NothingToImprove` | Suggests `--objective perf` |

---

## 3. Acceptance Criteria (Gherkin Scenarios)
- **Given valid task, tests, paths and budget**, **when I run `evoswarm run`**, **then a job ID prints within 2 s and the job status is `queued` in SQLite**.
- **Given a `--paths` entry containing `../` that escapes the repo root**, **when I submit**, **then the job is rejected with `ExitCode::PathEscape` and the resolved path shown**.
- **Given the test command cannot execute on the baseline**, **when I submit**, **then the job is rejected with the command's non-zero output and exit code shown**.
- **Given baseline results differ across 3 consecutive runs**, **when I submit**, **then the job is rejected and the flaky tests are listed**.
- **Given every baseline test already passes and no performance objective is set**, **when I submit**, **then I am told there is nothing to improve and asked for `--objective perf`**.
