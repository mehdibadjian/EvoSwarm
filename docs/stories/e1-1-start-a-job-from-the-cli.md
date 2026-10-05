# Story: Start a job from the CLI

## Metadata
- **Story Key:** `e1-1-start-a-job-from-the-cli` (Short: `e1-1`)
- **Epic:** [Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1.md)
- **Persona:** Developer
- **Priority:** Must
- **Sizing:** M
- **Execution Tier:** `flash`
- **Dependencies:** `e0-5-python-stack`, `e0-6-csharp-stack`

---

## 1. User Story
As a developer, I want to hand off a test-backed task with one command, so that I can keep working while EvoSwarm searches.

---

## 2. Architectural Context & Invariants
CLI command `evoswarm run --task <desc> --cmd <test_cmd> --paths <paths> --budget <budget>` validates inputs, runs baseline test 3 times for flakiness, and enqueues job in SQLite.

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
| `Valid task, tests, paths, budget` | `Prints Job ID within 2s, job state 'queued'` | `ValidationError if paths outside repo or budget invalid` |
| `Baseline tests fail to execute` | `Job rejected with command output` | `Non-zero exit diagnostic` |
| `Flaky tests across 3 baseline runs` | `Job rejected, flaky tests listed` | `Flakiness diagnostic` |
| `All baseline tests pass, no --objective perf` | `Job rejected with message suggesting --objective perf` | `Zero-diff rejection` |

---

## 4. Acceptance Criteria (Given / When / Then)
- **Given valid task, tests, paths and budget**, **when I run `evoswarm run`**, **then a job ID prints within 2 s and the job is `queued`.**.
- **Given the test command cannot execute on the baseline**, **when I submit**, **then the job is rejected with the command's output shown.**.
- **Given baseline results differ across 3 runs**, **when I submit**, **then the job is rejected and the flaky tests are listed.**.
- **Given every baseline test already passes and no performance objective is set**, **when I submit**, **then I am told there is nothing to improve and asked for `--objective perf`.**.

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
- [`tests/cli/test_run_command.rs::test_submit_valid_job`](file:///workspace/calm-faraday/tests/cli/test_run_command.rs): Verifies job enqueued in SQLite within 2s.
- [`tests/cli/test_run_command.rs::test_reject_broken_baseline`](file:///workspace/calm-faraday/tests/cli/test_run_command.rs): Supplies invalid test command and verifies immediate rejection.
- [`tests/cli/test_run_command.rs::test_detect_flaky_baseline`](file:///workspace/calm-faraday/tests/cli/test_run_command.rs): Simulates alternating test results and checks flakiness detection.

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test e1_1_start_a_job_from_the_cli" --anti-cheat
```
