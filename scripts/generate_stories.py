#!/usr/bin/env python3
"""Generates structured story markdown documents for EvoSwarm Epics 0 through 5."""

import os
from pathlib import Path

STORIES = [
    # ----------------------------------------------------
    # Epic 0: The Crucible (Sandbox)
    # ----------------------------------------------------
    {
        "key": "e0-1-sandbox-runner-interface",
        "short_key": "e0-1",
        "epic": "epic-0",
        "epic_title": "The Crucible (Sandbox)",
        "title": "Sandbox runner interface",
        "persona": "developer",
        "priority": "Must",
        "size": "M",
        "tier": "flash",
        "depends_on": [],
        "user_story": "As a developer, I want every candidate to run through one sandbox interface, so that adding a language stack never touches the search loop.",
        "context": "Governed by AD-1. The search loop must remain completely decoupled from OS-level isolation mechanics and stack specifics. A unified Rust trait `SandboxBackend` must define the lifecycle contract (`prepare`, `run`, `collect`).",
        "io_matrix": [
            ("profile: SandboxProfile, patch: Vec<u8>", "Ephemeral workdir PathBuf", "Invalid mount or tmpfs allocation error"),
            ("workdir: PathBuf, test_cmd: String", "ExecutionResult(exit_code, stdout, stderr, wall_ms, peak_mem_bytes, status)", "Timeout, OOM, or ProcessExecutionError"),
            ("workdir: PathBuf", "Result<(), SandboxError>", "Workdir cleanup / unmount error")
        ],
        "scenarios": [
            ("Given a stack profile and a work directory", "when `run` is called", "then it returns exit code, stdout, stderr, wall time and peak memory."),
            ("Given a run exceeds its time or memory limit", "when it is killed", "then the result is marked `timeout` or `oom`, distinct from `failed`."),
            ("Given two runs execute in parallel", "when both finish", "then neither could read or write the other's files.")
        ],
        "test_plan": [
            ("tests/test_sandbox_runner.rs::test_sandbox_runner_lifecycle_success", "Executes an echo command inside bwrap and verifies exit code 0, captured stdout, wall time > 0."),
            ("tests/test_sandbox_runner.rs::test_sandbox_runner_timeout_marking", "Executes `sleep 10` with a 1s limit and asserts status is `RunStatus::Timeout`."),
            ("tests/test_sandbox_runner.rs::test_sandbox_runner_concurrent_isolation", "Executes two concurrent runners writing to `/work/test.txt` and verifies distinct contents.")
        ]
    },
    {
        "key": "e0-2-host-self-check",
        "short_key": "e0-2",
        "epic": "epic-0",
        "epic_title": "The Crucible (Sandbox)",
        "title": "Host self-check",
        "persona": "operator",
        "priority": "Must",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e0-3-apparmor-profile-for-bwrap"],
        "user_story": "As an operator, I want the service to verify host isolation features at start, so that jobs never run on a box where the sandbox silently degrades.",
        "context": "Governed by AD-1. Unprivileged namespaces, cgroups v2, and AppArmor can be silently broken by host kernel updates. EvoSwarm must perform deterministic health checks at startup and via `evoswarm status`.",
        "io_matrix": [
            ("CLI flag: evoswarm status / daemon startup", "Exit code 0, status 'sandbox: ready'", "Exit code 1, diagnostic remediation messages detailing failed check")
        ],
        "scenarios": [
            ("Given unprivileged user namespaces are blocked", "when the service starts", "then it logs which check failed with the fix and refuses new jobs."),
            ("Given all checks pass", "when I run `evoswarm status`", "then it reports `sandbox: ready`."),
            ("Given a check starts failing after a kernel update", "when the service restarts", "then queued jobs stay queued rather than failing.")
        ],
        "test_plan": [
            ("tests/test_host_check.rs::test_check_user_namespaces", "Validates unshare(CLONE_NEWUSER) syscall availability."),
            ("tests/test_host_check.rs::test_check_cgroup_v2_delegation", "Checks `/sys/fs/cgroup/user.slice` controllers for `memory` and `pids`."),
            ("tests/test_host_check.rs::test_check_linger_enabled", "Verifies systemd user lingering status for the runtime user.")
        ]
    },
    {
        "key": "e0-3-apparmor-profile-for-bwrap",
        "short_key": "e0-3",
        "epic": "epic-0",
        "epic_title": "The Crucible (Sandbox)",
        "title": "AppArmor profile for bwrap",
        "persona": "operator",
        "priority": "Must",
        "size": "S",
        "tier": "flash",
        "depends_on": [],
        "user_story": "As an operator, I want a ready-made AppArmor profile for bwrap, so that unprivileged sandboxes work on a stock Ubuntu Server.",
        "context": "Recent Ubuntu releases (23.10+) restrict unprivileged user namespaces via AppArmor. An explicit profile allowing bwrap unshare without root privileges must be shipped and installed.",
        "io_matrix": [
            ("installer script: scripts/install_apparmor.sh", "Loaded AppArmor profile in /etc/apparmor.d/bwrap", "PermissionDenied (if not sudo)")
        ],
        "scenarios": [
            ("Given a fresh Ubuntu Server LTS install", "when I run the install script", "then the profile loads and E0-2 passes without a reboot."),
            ("Given the profile is removed", "when the service starts", "then E0-2 fails with a reinstall hint."),
            ("Given the script is run twice", "when it finishes", "then the system state is identical to one run.")
        ],
        "test_plan": [
            ("tests/test_apparmor.py::test_profile_idempotency", "Runs install script twice and checks aa-status."),
            ("tests/test_apparmor.py::test_bwrap_unprivileged_execution", "Spawns unprivileged bwrap container under the loaded profile.")
        ]
    },
    {
        "key": "e0-4-resource-limits",
        "short_key": "e0-4",
        "epic": "epic-0",
        "epic_title": "The Crucible (Sandbox)",
        "title": "Resource limits",
        "persona": "security reviewer",
        "priority": "Must",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e0-1-sandbox-runner-interface"],
        "user_story": "As a security reviewer, I want memory, process and time caps on every run, so that a runaway candidate cannot starve the host.",
        "context": "Governed by AD-1. Uses `systemd-run --user --scope` with `MemoryMax`, `TasksMax`, and `CPUQuota=100%`, plus an outer process timeout with 5s SIGKILL grace.",
        "io_matrix": [
            ("Fork-bomb script inside sandbox", "Status: RunStatus::Oom or RunStatus::Failed, host load normalized <10s", "Host freeze or kernel panic (forbidden)"),
            ("Memory-leak script (>2GB)", "Status: RunStatus::Oom, killed by cgroup OOM killer", "Host process killed (forbidden)"),
            ("Infinite loop script", "Status: RunStatus::Timeout, killed after wall limit + 5s", "Process leak (forbidden)")
        ],
        "scenarios": [
            ("Given a candidate runs a fork bomb", "when TasksMax is hit", "then the run ends as `oom` or `failed` and host load returns to normal within 10 s."),
            ("Given a candidate allocates 4 GB under a 2 GB cap", "when the cgroup limit is reached", "then the run ends as `oom` and no host process is killed."),
            ("Given a candidate loops forever", "when the wall limit plus 5 s grace passes", "then the process tree is killed and marked `timeout`.")
        ],
        "test_plan": [
            ("tests/test_resource_limits.rs::test_fork_bomb_contained", "Spawns a bash fork bomb `:(){ :|:& };:` and verifies termination in <10s."),
            ("tests/test_resource_limits.rs::test_memory_cap_enforcement", "Allocates 4GB via Python in a 512MB limit sandbox; verifies RunStatus::Oom."),
            ("tests/test_resource_limits.rs::test_wall_time_sigkill", "Executes `sleep 60` with a 2s timeout and confirms kill within 7s.")
        ]
    },
    {
        "key": "e0-5-python-stack",
        "short_key": "e0-5",
        "epic": "epic-0",
        "epic_title": "The Crucible (Sandbox)",
        "title": "Python stack",
        "persona": "developer",
        "priority": "Must",
        "size": "M",
        "tier": "flash",
        "depends_on": ["e0-1-sandbox-runner-interface", "e0-4-resource-limits"],
        "user_story": "As a developer, I want Python tasks tested with pytest in the sandbox, so that I can evolve Python code without exposing my machine.",
        "context": "Governed by AD-1. Creates an offline read-only venv keyed by lockfile SHA-256 (`uv.lock` or `requirements.txt`). pytest runs with JUnit XML output.",
        "io_matrix": [
            ("Python project + candidate patch + pytest command", "JUnit XML parsed: passed, failed, skipped counts", "Pip network error or build failure")
        ],
        "scenarios": [
            ("Given a repo lockfile", "when the first job runs", "then a venv is built outside the sandbox keyed by the lockfile hash and reused by later jobs."),
            ("Given a candidate calls `pip install`", "when it runs", "then the install fails because there is no network and the venv is read-only."),
            ("Given the sample suite", "when it runs", "then per-test results are parsed from JUnit XML into pass, fail and skip counts.")
        ],
        "test_plan": [
            ("tests/test_python_stack.rs::test_venv_cache_reuse", "Verifies venv is created once and reused for identical lockfile hashes."),
            ("tests/test_python_stack.rs::test_pip_install_egress_fails", "Attempts `pip install requests` inside sandbox and confirms failure."),
            ("tests/test_python_stack.rs::test_junit_xml_parsing", "Parses pytest junitxml output into structured test summary.")
        ]
    },
    {
        "key": "e0-6-csharp-stack",
        "short_key": "e0-6",
        "epic": "epic-0",
        "epic_title": "The Crucible (Sandbox)",
        "title": "C# stack",
        "persona": "developer",
        "priority": "Must",
        "size": "L",
        "tier": "pro",
        "depends_on": ["e0-1-sandbox-runner-interface", "e0-4-resource-limits"],
        "user_story": "As a developer, I want C# tasks tested with dotnet test, so that cTrader bots and enterprise code can be evolved safely.",
        "context": "Governed by AD-1. Pre-restores NuGet dependencies outside the sandbox to a hash-keyed cache. Executes `dotnet test --no-restore` in bwrap with xUnit and Reqnroll support.",
        "io_matrix": [
            ("C# csproj + packages.lock.json + test suite", "Parsed test summary (pass, fail, skip)", "Missing package error -> 'restore cache stale'")
        ],
        "scenarios": [
            ("Given a project's lock or package references", "when the first job runs", "then NuGet restore runs once outside the sandbox into a cache keyed by their hash."),
            ("Given the cache exists", "when `dotnet test --no-restore` runs with no network", "then xUnit and Reqnroll sample suites pass."),
            ("Given a candidate references a package missing from the cache", "when it builds", "then the result says `restore cache stale` instead of a generic build failure.")
        ],
        "test_plan": [
            ("tests/test_csharp_stack.rs::test_nuget_cache_restore", "Restores NuGet packages to isolated cache directory."),
            ("tests/test_csharp_stack.rs::test_dotnet_test_offline", "Executes `dotnet test --no-restore` in offline bwrap container."),
            ("tests/test_csharp_stack.rs::test_missing_package_stale_cache_error", "Injects unknown PackageReference and confirms diagnostic.")
        ]
    },
    {
        "key": "e0-7-tamper-proof-tests",
        "short_key": "e0-7",
        "epic": "epic-0",
        "epic_title": "The Crucible (Sandbox)",
        "title": "Tamper-proof tests",
        "persona": "developer",
        "priority": "Must",
        "size": "S",
        "tier": "pro",
        "depends_on": ["e0-1-sandbox-runner-interface"],
        "user_story": "As a developer, I want tests mounted read-only and harness overrides detected, so that a candidate can never pass by changing how tests run.",
        "context": "Governed by AD-1 and AD-5. Candidate diffs must never touch `/tests`. Harness configuration files (`conftest.py`, `Directory.Build.props`, `pytest.ini`) are prohibited in the candidate patch.",
        "io_matrix": [
            ("Candidate patch attempting to write to /work/tests", "Write error (EROFS: Read-only file system)", "Tampering detected, score 0"),
            ("Candidate patch adding conftest.py in /work root", "Gate check failure: Status 'tamper'", "Score 0, flagged tampering")
        ],
        "scenarios": [
            ("Given a candidate writes into the tests directory", "when it runs", "then the write fails and the result is unchanged."),
            ("Given a candidate adds a harness override file such as `conftest.py` or `Directory.Build.props` in the work root", "when gates run", "then it is flagged as tampering."),
            ("Given any run", "when it ends", "then the tests directory hash matches the hash taken before it started.")
        ],
        "test_plan": [
            ("tests/test_tamper_detection.rs::test_readonly_test_mount", "Candidate writes to `tests/test_foo.py` and asserts EROFS."),
            ("tests/test_tamper_detection.rs::test_harness_override_rejection", "Validates diff rejector flags `conftest.py` in patch root.")
        ]
    },
    {
        "key": "e0-8-red-team-suite",
        "short_key": "e0-8",
        "epic": "epic-0",
        "epic_title": "The Crucible (Sandbox)",
        "title": "Red-team suite",
        "persona": "security reviewer",
        "priority": "Must",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e0-4-resource-limits", "e0-7-tamper-proof-tests"],
        "user_story": "As a security reviewer, I want automated escape attempts, so that isolation regressions are caught before release.",
        "context": "Red-team test harness that deliberately attempts: TCP/DNS egress, reading `/home` and `/etc/shadow`, writing outside `/work`, `ptrace` on host PIDs, reading `/proc/1`.",
        "io_matrix": [
            ("Red-team exploit payloads", "All exploits fail cleanly (EACCES, ENETUNREACH, EPERM)", "Any successful exploit blocks build and release")
        ],
        "scenarios": [
            ("Given the suite runs", "when it attempts TCP and DNS egress, reading `/home` and `/etc/shadow`, writing outside the work dir, ptrace on a host process and access to `/proc/1`", "then every attempt fails."),
            ("Given one attempt succeeds", "when CI runs", "then the build fails and release is blocked."),
            ("Given the service starts", "when E0-2 runs", "then a quick subset of the suite runs as part of the self-check.")
        ],
        "test_plan": [
            ("tests/red_team/test_network_egress.rs", "Attempts raw TCP socket connect and DNS resolve; expects network unreachable."),
            ("tests/red_team/test_filesystem_traversal.rs", "Attempts reading `/etc/shadow` and `/home`; expects EACCES/ENOENT."),
            ("tests/red_team/test_ptrace_denial.rs", "Attempts ptrace(PTRACE_ATTACH, 1); expects EPERM.")
        ]
    },
    {
        "key": "e0-9-limit-calibration",
        "short_key": "e0-9",
        "epic": "epic-0",
        "epic_title": "The Crucible (Sandbox)",
        "title": "Limit calibration",
        "persona": "operator",
        "priority": "Should",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e0-5-python-stack", "e0-6-csharp-stack"],
        "user_story": "As an operator, I want limits measured on my own box, so that timeouts fit my hardware instead of guesses.",
        "context": "Measures 100 baseline runs on the host box; computes p50 and p95 wall-clock and peak memory; generates tailored profile recommending worker concurrency based on available RAM.",
        "io_matrix": [
            ("CLI: evoswarm calibrate --stack <python|csharp>", "Calibration report + updated config.toml (wall=p95*2, mem=peak*1.5)", "Error if baseline suite fails")
        ],
        "scenarios": [
            ("Given the calibration script", "when it runs 100 baseline builds per stack", "then it records p50 and p95 wall time and peak memory."),
            ("Given the measurements", "when calibration finishes", "then the stack profile sets wall limit to p95 x 2 and memory to peak x 1.5."),
            ("Given the available RAM", "when the report prints", "then it recommends a worker count per stack.")
        ],
        "test_plan": [
            ("tests/test_calibration.py::test_calibration_statistics", "Feeds mock run times and verifies p50/p95 calculations."),
            ("tests/test_calibration.py::test_profile_generation", "Asserts generated config applies 2x wall and 1.5x memory safety factors.")
        ]
    },

    # ----------------------------------------------------
    # Epic 1: Evolve CLI and Fitness
    # ----------------------------------------------------
    {
        "key": "e1-1-start-a-job-from-the-cli",
        "short_key": "e1-1",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Start a job from the CLI",
        "persona": "developer",
        "priority": "Must",
        "size": "M",
        "tier": "flash",
        "depends_on": ["e0-5-python-stack", "e0-6-csharp-stack"],
        "user_story": "As a developer, I want to hand off a test-backed task with one command, so that I can keep working while EvoSwarm searches.",
        "context": "CLI command `evoswarm run --task <desc> --cmd <test_cmd> --paths <paths> --budget <budget>` validates inputs, runs baseline test 3 times for flakiness, and enqueues job in SQLite.",
        "io_matrix": [
            ("Valid task, tests, paths, budget", "Prints Job ID within 2s, job state 'queued'", "ValidationError if paths outside repo or budget invalid"),
            ("Baseline tests fail to execute", "Job rejected with command output", "Non-zero exit diagnostic"),
            ("Flaky tests across 3 baseline runs", "Job rejected, flaky tests listed", "Flakiness diagnostic"),
            ("All baseline tests pass, no --objective perf", "Job rejected with message suggesting --objective perf", "Zero-diff rejection")
        ],
        "scenarios": [
            ("Given valid task, tests, paths and budget", "when I run `evoswarm run`", "then a job ID prints within 2 s and the job is `queued`."),
            ("Given the test command cannot execute on the baseline", "when I submit", "then the job is rejected with the command's output shown."),
            ("Given baseline results differ across 3 runs", "when I submit", "then the job is rejected and the flaky tests are listed."),
            ("Given every baseline test already passes and no performance objective is set", "when I submit", "then I am told there is nothing to improve and asked for `--objective perf`.")
        ],
        "test_plan": [
            ("tests/cli/test_run_command.rs::test_submit_valid_job", "Verifies job enqueued in SQLite within 2s."),
            ("tests/cli/test_run_command.rs::test_reject_broken_baseline", "Supplies invalid test command and verifies immediate rejection."),
            ("tests/cli/test_run_command.rs::test_detect_flaky_baseline", "Simulates alternating test results and checks flakiness detection.")
        ]
    },
    {
        "key": "e1-2-model-roles-in-config",
        "short_key": "e1-2",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Model roles in config",
        "persona": "operator",
        "priority": "Must",
        "size": "S",
        "tier": "flash",
        "depends_on": [],
        "user_story": "As an operator, I want model roles set in config, so that I can swap models or cut cost without a rebuild.",
        "context": "Governed by AD-6. Configuration file `config.toml` binds roles `mutator`, `synthesiser`, and `adversary` to model IDs, max_tokens, and temperatures. Supports SIGHUP hot-reload.",
        "io_matrix": [
            ("config.toml with valid roles", "Service runs with mapped models", "Error on unknown model ID or invalid temperature"),
            ("SIGHUP sent to daemon", "Next generation reads updated config", "Config reload log entry")
        ],
        "scenarios": [
            ("Given config maps `mutator`, `synthesiser` and `adversary` to a model ID, max tokens and temperature", "when a job runs", "then each call uses its role's settings."),
            ("Given an unknown model ID", "when the service starts", "then it fails with the offending role named."),
            ("Given I edit config and send SIGHUP", "when the next generation starts", "then the new settings apply.")
        ],
        "test_plan": [
            ("tests/config/test_model_roles.rs::test_load_valid_config", "Verifies config parser deserializes roles."),
            ("tests/config/test_model_roles.rs::test_reject_unknown_model", "Injects invalid model name and expects startup panic with role name."),
            ("tests/config/test_model_roles.rs::test_sighup_reload", "Sends SIGHUP and asserts modified temperature takes effect.")
        ]
    },
    {
        "key": "e1-3-seed-the-first-generation",
        "short_key": "e1-3",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Seed the first generation",
        "persona": "developer",
        "priority": "Must",
        "size": "M",
        "tier": "flash",
        "depends_on": ["e1-1-start-a-job-from-the-cli", "e1-2-model-roles-in-config"],
        "user_story": "As a developer, I want the search to start from several different attempts, so that it is not stuck refining one idea.",
        "context": "Populates Generation 0 with: current code baseline + N-1 fresh candidate drafts from mutator model. Deduplicates identical drafts by content hash.",
        "io_matrix": [
            ("Population size N (e.g. 6), task description", "List of N distinct candidate patches with model ID & prompt hash", "Fails if model calls fail")
        ],
        "scenarios": [
            ("Given population size N", "when generation 0 is built", "then it holds the current code plus N-1 fresh drafts."),
            ("Given each draft", "when it is created", "then it records model, prompt hash and an empty parent."),
            ("Given two drafts have identical content hashes", "when the generation is finalised", "then the duplicate is dropped and replaced.")
        ],
        "test_plan": [
            ("tests/engine/test_seeding.rs::test_gen0_population_count", "Verifies exactly N candidates produced."),
            ("tests/engine/test_seeding.rs::test_gen0_deduplication", "Simulates two identical drafts and verifies one is dropped and regenerated.")
        ]
    },
    {
        "key": "e1-4-mutation-with-error-feedback",
        "short_key": "e1-4",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Mutation with error feedback",
        "persona": "developer",
        "priority": "Must",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e1-3-seed-the-first-generation", "e1-6-hard-gates"],
        "user_story": "As a developer, I want each new attempt to learn from the last one's failures, so that the search converges instead of guessing.",
        "context": "Passes compiler output and first failing assertion (trimmed to 4,000 tokens) to the mutator model. Maintains stable system prompt prefix to maximize prompt caching.",
        "io_matrix": [
            ("Failed candidate + error text", "New candidate patch linked via MUTATED_FROM", "BudgetExceeded if token cap reached")
        ],
        "scenarios": [
            ("Given a failed parent", "when its child is drafted", "then the prompt includes the first failing assertion and error text trimmed to 4k tokens."),
            ("Given any child", "when it is stored", "then it is linked to its parent with `MUTATED_FROM`."),
            ("Given repeated calls in one job", "when they are sent", "then repo context sits in a stable prefix and the prompt-cache hit rate is logged.")
        ],
        "test_plan": [
            ("tests/engine/test_mutation.rs::test_error_feedback_prompt_formatting", "Checks failure message truncation to 4k tokens."),
            ("tests/engine/test_mutation.rs::test_prompt_cache_prefix_stability", "Verifies system prompt bytes remain identical across mutations.")
        ]
    },
    {
        "key": "e1-5-crossover-of-two-parents",
        "short_key": "e1-5",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Crossover of two parents",
        "persona": "developer",
        "priority": "Should",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e1-4-mutation-with-error-feedback"],
        "user_story": "As a developer, I want strengths of two partial solutions combined, so that complementary fixes are not lost.",
        "context": "Selects pairs of parents passing disjoint subsets of test cases; calls synthesiser model (max 25% of total calls); links candidate with MERGED_FROM edge containing synthesized traits.",
        "io_matrix": [
            ("Parent A (passes tests 1, 2) + Parent B (passes tests 3, 4)", "Child C candidate patch with traits description", "Fallback to mutation if no disjoint parents")
        ],
        "scenarios": [
            ("Given two parents pass different subsets of tests", "when parents are selected", "then such pairs are preferred for crossover."),
            ("Given a crossover child", "when it is stored", "then it links to both parents with the traits the model reports merging."),
            ("Given default settings", "when a job runs", "then crossover is at most 25% of model calls.")
        ],
        "test_plan": [
            ("tests/engine/test_crossover.rs::test_disjoint_parent_selection", "Asserts parents with complementary test passes are prioritized."),
            ("tests/engine/test_crossover.rs::test_crossover_call_cap", "Asserts crossover calls never exceed 25% of total generation calls.")
        ]
    },
    {
        "key": "e1-6-hard-gates",
        "short_key": "e1-6",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Hard gates",
        "persona": "reviewer",
        "priority": "Must",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e0-7-tamper-proof-tests"],
        "user_story": "As a reviewer, I want every candidate checked against hard gates, so that nothing returned to me has gamed the tests.",
        "context": "Governed by AD-5. Evaluates 4 sequential gates: (1) Builds with no new errors, (2) All visible trusted tests pass, (3) Diff touches no tests/harness/build scripts, (4) No tests skipped/deleted relative to baseline.",
        "io_matrix": [
            ("Candidate build & test result", "GateResult(passed: bool, failure_reason: Option<String>)", "Immediate score=0 upon any gate failure")
        ],
        "scenarios": [
            ("Given the candidate fails to build", "when gated", "then its score is 0 with reason `build`."),
            ("Given any visible trusted test fails", "when gated", "then its score is 0 with the failing tests listed."),
            ("Given the diff touches tests, harness files, build scripts or CI config", "when gated", "then its score is 0 with reason `tamper`."),
            ("Given fewer tests ran than in the baseline, or new skips appear", "when gated", "then its score is 0 with reason `skipped`.")
        ],
        "test_plan": [
            ("tests/fitness/test_hard_gates.rs::test_gate_build_failure", "Simulates syntax error and asserts reason 'build'."),
            ("tests/fitness/test_hard_gates.rs::test_gate_test_failure", "Simulates test assertion failure and asserts failing tests listed."),
            ("tests/fitness/test_hard_gates.rs::test_gate_tamper_failure", "Includes diff in `tests/` and asserts reason 'tamper'."),
            ("tests/fitness/test_hard_gates.rs::test_gate_skipped_tests", "Decrements total run count and asserts reason 'skipped'.")
        ]
    },
    {
        "key": "e1-7-weighted-score",
        "short_key": "e1-7",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Weighted score",
        "persona": "developer",
        "priority": "Must",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e1-6-hard-gates"],
        "user_story": "As a developer, I want passing candidates ranked by a clear score, so that the best of several valid solutions wins.",
        "context": "Computes S = w_a * A + w_p * P + w_s * Z. Weights configurable in config.toml (defaults: 0.5, 0.3, 0.2, summing to 1.0). If no adversary tests, redistributes w_a proportionally.",
        "io_matrix": [
            ("A: f64, P: f64, Z: f64, weights: Weights", "Score S: f64", "Error if weights do not sum to 1.0")
        ],
        "scenarios": [
            ("Given a candidate passes all gates", "when scored", "then S is computed from adversary pass rate, runtime versus baseline and diff size, using config weights."),
            ("Given weights that do not sum to 1", "when the service starts", "then it fails with the weights shown."),
            ("Given no adversary tests exist", "when scoring", "then the adversary weight is redistributed proportionally to the other terms."),
            ("Given identical inputs", "when scored twice", "then the score is identical.")
        ],
        "test_plan": [
            ("tests/fitness/test_scoring.rs::test_score_formula_calculation", "Verifies exact arithmetic for given weights and inputs."),
            ("tests/fitness/test_scoring.rs::test_weight_redistribution_without_adversary", "Checks proportional redistribution when A is absent."),
            ("tests/fitness/test_scoring.rs::test_weight_validation_on_startup", "Rejects config with weights summing to 0.9.")
        ]
    },
    {
        "key": "e1-8-held-out-tests",
        "short_key": "e1-8",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Held-out tests",
        "persona": "reviewer",
        "priority": "Must",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e1-6-hard-gates"],
        "user_story": "As a reviewer, I want the winner checked on tests the models never saw, so that I know it generalises.",
        "context": "Governed by AD-5. Splits trusted user test suite into 80% visible and 20% held-out. Held-out tests are never provided in mutation prompts. The top-scoring candidate must pass 100% of held-out tests.",
        "io_matrix": [
            ("Full test suite", "(VisibleTests, HeldOutTests)", "HeldOutTests empty if suite has fewer than 5 tests"),
            ("Top candidate + HeldOutTests", "Verified winner or fallback to next candidate", "Status: 'no verified winner' if all fail")
        ],
        "scenarios": [
            ("Given a job starts", "when tests are split", "then about 20% are held out using a split stable for that job."),
            ("Given any model call", "when the prompt log is scanned", "then no held-out test content appears in it."),
            ("Given the top candidate fails held-out tests", "when selecting a winner", "then the next candidate is tried."),
            ("Given no candidate passes held-out tests", "when the job ends", "then it reports `no verified winner` and flags the best-effort patch.")
        ],
        "test_plan": [
            ("tests/fitness/test_held_out.rs::test_stable_test_split", "Verifies deterministic 80/20 partition keyed by job ID."),
            ("tests/fitness/test_held_out.rs::test_prompt_leakage_audit", "Scans generated prompts to assert zero held-out test text."),
            ("tests/fitness/test_held_out.rs::test_runner_fallback_on_held_out_failure", "Forces rank 1 candidate to fail held-out tests and asserts rank 2 chosen.")
        ]
    },
    {
        "key": "e1-9-adversary-tests",
        "short_key": "e1-9",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Adversary tests",
        "persona": "developer",
        "priority": "Should",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e1-6-hard-gates"],
        "user_story": "As a developer, I want edge cases I missed probed automatically, so that weak solutions rank lower.",
        "context": "Governed by AD-5. Uses adversary model role to generate K edge-case tests. Discards non-compiling tests. Flags tests that fail on baseline and all candidates as suspect. Never gates candidates.",
        "io_matrix": [
            ("Spec + best candidate patch", "List of candidate test cases", "Suspect filter discards broken tests")
        ],
        "scenarios": [
            ("Given the spec and the current best candidate", "when the adversary runs", "then it writes up to K tests, with K set in config."),
            ("Given an adversary test does not compile", "when collected", "then it is discarded."),
            ("Given an adversary test fails on every candidate and the baseline", "when collected", "then it is flagged `suspect` and excluded from scoring."),
            ("Given any adversary test", "when gates run", "then it never affects a gate.")
        ],
        "test_plan": [
            ("tests/engine/test_adversary.rs::test_adversary_test_generation", "Asserts K tests drafted."),
            ("tests/engine/test_adversary.rs::test_uncompilable_test_discard", "Verifies compiler error drops test."),
            ("tests/engine/test_adversary.rs::test_suspect_adversary_test_exclusion", "Confirms universally failing test is tagged suspect.")
        ]
    },
    {
        "key": "e1-10-budget-enforcement",
        "short_key": "e1-10",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Budget enforcement",
        "persona": "developer",
        "priority": "Must",
        "size": "M",
        "tier": "flash",
        "depends_on": ["e1-2-model-roles-in-config"],
        "user_story": "As a developer, I want a hard spend cap per job, so that a search never costs more than I agreed.",
        "context": "Governed by AD-6. Checks projected token and dollar cost before each LLM call. Stops early if no score improvement across 2 generations. Halts with `budget_exhausted` when cap hit.",
        "io_matrix": [
            ("Current spend + projected call cost vs cap", "Execute call OR abort loop with status 'budget_exhausted'", "Best partial patch returned")
        ],
        "scenarios": [
            ("Given a token and dollar cap", "when a model call is about to be sent", "then its projected cost is checked against the remaining budget."),
            ("Given the cap would be exceeded", "when checked", "then the job stops as `budget_exhausted` and returns the best verified result so far."),
            ("Given no score improvement over 2 generations", "when the generation ends", "then the job stops early.")
        ],
        "test_plan": [
            ("tests/engine/test_budget.rs::test_hard_token_cap_stop", "Sets low token cap and confirms loop stops before exceeding."),
            ("tests/engine/test_budget.rs::test_early_stop_plateau", "Simulates 2 generations with identical score and asserts early stop.")
        ]
    },
    {
        "key": "e1-11-patch-and-report",
        "short_key": "e1-11",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Patch and report",
        "persona": "developer",
        "priority": "Must",
        "size": "M",
        "tier": "flash",
        "depends_on": ["e1-6-hard-gates", "e1-7-weighted-score"],
        "user_story": "As a developer, I want the result as a branch with a report, so that I can review and merge with confidence.",
        "context": "Creates git branch `evoswarm/<job-id>` from base commit, emits `.evoswarm/patches/<job-id>.patch`, and writes markdown report with lineage, scores, token/dollar costs, and adversary test proposals.",
        "io_matrix": [
            ("Completed job with winning candidate", "Git branch, .patch file, Markdown report", "Base branch remains untouched")
        ],
        "scenarios": [
            ("Given a winner", "when the job completes", "then branch `evoswarm/<job-id>` is created from the base commit and a `.patch` file is written."),
            ("Given the report", "when opened", "then it shows score breakdown, tests passed including held-out, model calls, tokens, cost, lineage and proposed candidate tests."),
            ("Given any job", "when it completes", "then the base branch is untouched.")
        ],
        "test_plan": [
            ("tests/cli/test_output.rs::test_git_branch_creation", "Verifies git branch exists and base branch HEAD is unchanged."),
            ("tests/cli/test_output.rs::test_patch_file_applicability", "Applies generated patch to clean checkout and verifies tests pass."),
            ("tests/cli/test_output.rs::test_report_structure", "Validates markdown report sections (scores, costs, lineage).")
        ]
    },
    {
        "key": "e1-12-resume-after-crash",
        "short_key": "e1-12",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Resume after crash",
        "persona": "operator",
        "priority": "Must",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e1-1-start-a-job-from-the-cli"],
        "user_story": "As an operator, I want jobs to survive a reboot, so that a restart of the 24/7 box does not waste spend.",
        "context": "Governed by AD-7. Commits state to SQLite job ledger. On reboot/crash, resumes from last completed generation. Uses idempotency hashes to avoid re-running completed model calls.",
        "io_matrix": [
            ("Abrupt SIGKILL during generation G", "Daemon restarts: resumes generation G without repeating completed LLM calls", "Corrupted state recovered via SQLite WAL")
        ],
        "scenarios": [
            ("Given the process is killed mid-generation", "when the service restarts", "then the job resumes from the last completed generation."),
            ("Given a model call completed before the crash", "when resuming", "then it is not repeated, using a stored idempotency key."),
            ("Given sandbox runs were in flight", "when resuming", "then they are re-run.")
        ],
        "test_plan": [
            ("tests/engine/test_recovery.rs::test_resume_mid_generation", "Kills process during Gen 2; restarts and verifies Gen 2 completes without re-drafting."),
            ("tests/engine/test_recovery.rs::test_idempotency_key_dedup", "Verifies stored model responses reused on restart.")
        ]
    },
    {
        "key": "e1-13-benchmark-suite",
        "short_key": "e1-13",
        "epic": "epic-1",
        "epic_title": "Evolve CLI and Fitness",
        "title": "Benchmark suite",
        "persona": "team lead",
        "priority": "Must",
        "size": "L",
        "tier": "pro",
        "depends_on": ["e1-11-patch-and-report"],
        "user_story": "As a team lead, I want a fixed benchmark, so that we decide on evidence whether evolution pays off.",
        "context": "Defines 30 tasks (at least 10 Python, 10 C#) with trusted tests and held-out slices. Automated comparison script runs EvoSwarm vs single-shot generation at equal token budget; reports solve rate delta.",
        "io_matrix": [
            ("Benchmark suite tasks", "Comparative report: solve rate delta (target >=15%), tokens per solve, wall time", "Exit gate fails if delta < 15%")
        ],
        "scenarios": [
            ("Given the benchmark", "when assembled", "then it has 30 tasks, at least 10 Python and 10 C#, each with trusted tests and a held-out split."),
            ("Given the comparison script", "when run", "then EvoSwarm and single-shot-with-feedback run at equal token budget."),
            ("Given a run completes", "when results print", "then they show solve rate, tokens per solved task and wall time, stored for comparison across versions.")
        ],
        "test_plan": [
            ("tests/benchmark/test_benchmark_runner.py::test_benchmark_loading", "Asserts 30 benchmark tasks valid."),
            ("tests/benchmark/test_benchmark_runner.py::test_single_shot_comparison", "Executes single-shot harness at equal token cap."),
            ("tests/benchmark/test_benchmark_runner.py::test_solve_rate_delta_calculation", "Computes and prints solve rate metric.")
        ]
    },

    # ----------------------------------------------------
    # Epic 2: System 1 Memory and Replay
    # ----------------------------------------------------
    {
        "key": "e2-1-content-addressed-blob-store",
        "short_key": "e2-1",
        "epic": "epic-2",
        "epic_title": "System 1 Memory and Replay",
        "title": "Content-addressed blob store",
        "persona": "operator",
        "priority": "Must",
        "size": "M",
        "tier": "flash",
        "depends_on": [],
        "user_story": "As an operator, I want code and logs stored on disk by hash, so that the graph stays small enough to live in RAM.",
        "context": "Governed by AD-3. Stores immutable files under `.evoswarm/blobs/<sha256>`. Provides write, read, and 7-day unreferenced garbage collection sweep.",
        "io_matrix": [
            ("Raw bytes (code/log)", "SHA-256 hash string", "Disk I/O error"),
            ("SHA-256 hash", "Exact original bytes", "NotFound error"),
            ("GC trigger", "Count and byte size of deleted unreferenced blobs", "Zero deletion if all blobs referenced")
        ],
        "scenarios": [
            ("Given the same content is written twice", "when stored", "then it exists once on disk."),
            ("Given a hash", "when read", "then the exact original bytes are returned."),
            ("Given blobs no node references for 7 days", "when garbage collection runs", "then they are deleted and the freed space is reported.")
        ],
        "test_plan": [
            ("tests/storage/test_blob_store.rs::test_deduplication", "Writes identical payload twice and verifies single file."),
            ("tests/storage/test_blob_store.rs::test_byte_exact_read", "Verifies read matches original write byte-for-byte."),
            ("tests/storage/test_blob_store.rs::test_gc_sweep", "Sweeps unreferenced blobs older than 7 days.")
        ]
    },
    {
        "key": "e2-2-lineage-written-to-falkordb",
        "short_key": "e2-2",
        "epic": "epic-2",
        "epic_title": "System 1 Memory and Replay",
        "title": "Lineage written to FalkorDB",
        "persona": "developer",
        "priority": "Must",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e2-1-content-addressed-blob-store", "e1-11-patch-and-report"],
        "user_story": "As a developer, I want every job's history recorded in the graph, so that past work can be queried and reused.",
        "context": "Governed by AD-3. Writes Task, Implementation, Evaluation, TestCase nodes and edges. Queries indexed by spec_hash and fingerprints. Queues retry if FalkorDB temporarily down.",
        "io_matrix": [
            ("Completed job record", "Graph nodes and edges committed in FalkorDB", "Queued in retry spool if DB unavailable")
        ],
        "scenarios": [
            ("Given a job completes", "when it is recorded", "then Task, Implementation, Evaluation and TestCase nodes and their edges match the schema."),
            ("Given the graph", "when queried by spec_hash, repo_fingerprint or toolchain_fingerprint", "then the lookup uses an index."),
            ("Given FalkorDB is unavailable", "when a job completes", "then the job still succeeds and the write is queued for retry.")
        ],
        "test_plan": [
            ("tests/memory/test_falkordb_lineage.rs::test_graph_schema_conformance", "Validates created Cypher nodes and relationship types."),
            ("tests/memory/test_falkordb_lineage.rs::test_index_usage", "Checks EXPLAIN output for indexed lookups."),
            ("tests/memory/test_falkordb_lineage.rs::test_db_downtime_retry_queue", "Mocks DB outage and verifies spool file.")
        ]
    },
    {
        "key": "e2-3-repo-and-toolchain-fingerprints",
        "short_key": "e2-3",
        "epic": "epic-2",
        "epic_title": "System 1 Memory and Replay",
        "title": "Repo and toolchain fingerprints",
        "persona": "developer",
        "priority": "Must",
        "size": "S",
        "tier": "flash",
        "depends_on": [],
        "user_story": "As a developer, I want context changes detected, so that a stored winner is never trusted after the code around it changed.",
        "context": "Governed by AD-4. Computes `repo_fingerprint` over touched files + lockfile. Computes `toolchain_fingerprint` over compiler, runtime, and harness versions.",
        "io_matrix": [
            ("Repository files + touched path list + lockfile", "repo_fingerprint: String (SHA-256)", "Error if file unreadable"),
            ("Stack name (e.g. python, csharp)", "toolchain_fingerprint: String (SHA-256)", "Error if toolchain binary missing")
        ],
        "scenarios": [
            ("Given a file the task touches changes", "when fingerprinted", "then the repo fingerprint changes."),
            ("Given the lockfile changes", "when fingerprinted", "then the repo fingerprint changes."),
            ("Given an untouched file changes", "when fingerprinted", "then the repo fingerprint is unchanged."),
            ("Given a compiler, runtime or test runner version changes", "when fingerprinted", "then the toolchain fingerprint changes.")
        ],
        "test_plan": [
            ("tests/memory/test_fingerprints.rs::test_touched_file_invalidation", "Mutates touched file and checks hash diff."),
            ("tests/memory/test_fingerprints.rs::test_untouched_file_insensitivity", "Mutates sibling file outside paths and asserts identical hash."),
            ("tests/memory/test_fingerprints.rs::test_toolchain_version_hash", "Mocks compiler version change and asserts fingerprint diff.")
        ]
    },
    {
        "key": "e2-4-verified-replay",
        "short_key": "e2-4",
        "epic": "epic-2",
        "epic_title": "System 1 Memory and Replay",
        "title": "Verified replay",
        "persona": "developer",
        "priority": "Must",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e2-2-lineage-written-to-falkordb", "e2-3-repo-and-toolchain-fingerprints"],
        "user_story": "As a developer, I want a repeat task answered from memory after re-verification, so that it finishes in seconds with no model spend.",
        "context": "Governed by AD-4. Matches triple fingerprints in FalkorDB; re-runs stored winner on full suite (including held-out) in Crucible. Returns in seconds with 0 LLM calls if passed.",
        "io_matrix": [
            ("Task matching spec_hash + repo_fingerprint + toolchain_fingerprint", "Status 'replayed', 0 model calls, verified patch", "Fallback to seeded search if verification fails")
        ],
        "scenarios": [
            ("Given spec hash and both fingerprints match a stored winner", "when the job starts", "then the winner runs against the full suite including held-out tests."),
            ("Given it passes", "when returned", "then the job is marked `replayed` and made 0 model calls."),
            ("Given it fails", "when checked", "then a normal job starts with it as a seed and the mismatch is logged.")
        ],
        "test_plan": [
            ("tests/memory/test_replay.rs::test_instant_replay_success", "Executes repeat task, asserts status 'replayed', 0 LLM calls, <5s wall time."),
            ("tests/memory/test_replay.rs::test_replay_verification_failure_fallback", "Induces test failure and asserts fallback to normal search.")
        ]
    },
    {
        "key": "e2-5-seeding-from-similar-winners",
        "short_key": "e2-5",
        "epic": "epic-2",
        "epic_title": "System 1 Memory and Replay",
        "title": "Seeding from similar winners",
        "persona": "developer",
        "priority": "Should",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e2-2-lineage-written-to-falkordb"],
        "user_story": "As a developer, I want new tasks to start from proven approaches, so that they solve faster and cheaper.",
        "context": "Uses vector embeddings of task descriptions in FalkorDB to find similar winners. Injects up to 2 past winners into Gen 0 population as seeds.",
        "io_matrix": [
            ("Task description embedding", "Up to 2 past winning patches joined to Gen 0", "Empty list if no task above cosine similarity threshold")
        ],
        "scenarios": [
            ("Given a past task above the similarity threshold", "when generation 0 is built", "then up to 2 of its winners join as seeds."),
            ("Given a seed", "when evaluated", "then it passes through the same gates as any candidate and is returned only if it wins."),
            ("Given the benchmark", "when run with and without seeding", "then tokens per solved task are reported for both.")
        ],
        "test_plan": [
            ("tests/memory/test_seeding.rs::test_vector_similarity_lookup", "Retrieves nearest past task by embedding cosine similarity."),
            ("tests/memory/test_seeding.rs::test_seed_candidate_gating", "Ensures injected seeds are evaluated through standard gates.")
        ]
    },
    {
        "key": "e2-6-approve-candidate-tests",
        "short_key": "e2-6",
        "epic": "epic-2",
        "epic_title": "System 1 Memory and Replay",
        "title": "Approve candidate tests",
        "persona": "reviewer",
        "priority": "Should",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e1-9-adversary-tests", "e2-2-lineage-written-to-falkordb"],
        "user_story": "As a reviewer, I want to promote good adversary tests, so that future jobs on this repo are held to a higher bar.",
        "context": "Governed by AD-5. CLI command `evoswarm approve-tests <job> --ids <ids>` promotes chosen candidate tests to trusted status, committing them on a git branch.",
        "io_matrix": [
            ("Job ID + test case IDs to approve", "Promoted status in FalkorDB + git branch with committed tests", "Error if test ID not found")
        ],
        "scenarios": [
            ("Given a job report", "when I run `evoswarm approve-tests <job> --ids`", "then the chosen tests become trusted for that repo and are added on a branch for me to merge."),
            ("Given I reject a test", "when later jobs run", "then it is not proposed again.")
        ],
        "test_plan": [
            ("tests/cli/test_approve_tests.rs::test_promote_adversary_test", "Checks status updated to 'trusted' in FalkorDB."),
            ("tests/cli/test_approve_tests.rs::test_rejected_test_suppression", "Verifies rejected test marked and omitted from future proposals.")
        ]
    },
    {
        "key": "e2-7-seccomp-filter",
        "short_key": "e2-7",
        "epic": "epic-2",
        "epic_title": "System 1 Memory and Replay",
        "title": "Seccomp filter",
        "persona": "security reviewer",
        "priority": "Should",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e0-8-red-team-suite"],
        "user_story": "As a security reviewer, I want syscalls the harnesses never need blocked, so that the kernel attack surface is minimal.",
        "context": "Attaches a seccomp BPF filter to bwrap invocation blocking: `ptrace`, `mount`, `umount`, `keyctl`, `bpf`, `perf_event_open`, and `unshare`.",
        "io_matrix": [
            ("Attempt to invoke ptrace / bpf inside sandbox", "Syscall returns EPERM / process killed with SIGSYS", "Normal stack suites pass without error")
        ],
        "scenarios": [
            ("Given the filter is active", "when a candidate calls ptrace, mount, umount, keyctl, bpf, perf_event_open or unshare", "then the call fails."),
            ("Given the filter is active", "when all stack sample suites run", "then they still pass."),
            ("Given the red-team suite", "when extended with these syscalls", "then every attempt fails.")
        ],
        "test_plan": [
            ("tests/security/test_seccomp.rs::test_blocked_syscalls_sigsys", "Calls ptrace and keyctl and asserts EPERM/SIGSYS."),
            ("tests/security/test_seccomp.rs::test_harness_compatibility", "Runs full pytest and dotnet suites under seccomp filter.")
        ]
    },
    {
        "key": "e2-8-lineage-cli",
        "short_key": "e2-8",
        "epic": "epic-2",
        "epic_title": "System 1 Memory and Replay",
        "title": "Lineage CLI",
        "persona": "developer",
        "priority": "Could",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e2-2-lineage-written-to-falkordb"],
        "user_story": "As a developer, I want to see how a winner was reached, so that I can trust or debug the result.",
        "context": "CLI command `evoswarm lineage <job> [--json]` traverses graph ancestry back to Gen 0 drafts and prints formatted ASCII tree or JSON.",
        "io_matrix": [
            ("Job ID", "ASCII lineage tree (generation, model, score, failure reasons)", "JSON object if --json specified")
        ],
        "scenarios": [
            ("Given a job ID", "when I run `evoswarm lineage <job>`", "then it prints the winner's ancestry with generation, model, score and gate failure reasons."),
            ("Given `--json`", "when run", "then the same data prints as JSON.")
        ],
        "test_plan": [
            ("tests/cli/test_lineage_cli.rs::test_ascii_tree_rendering", "Validates formatted output contains all ancestors."),
            ("tests/cli/test_lineage_cli.rs::test_json_lineage_export", "Validates JSON schema matches expected properties.")
        ]
    },

    # ----------------------------------------------------
    # Epic 3: Claude Code integration (MCP)
    # ----------------------------------------------------
    {
        "key": "e3-1-evolve-tool",
        "short_key": "e3-1",
        "epic": "epic-3",
        "epic_title": "Claude Code Integration (MCP)",
        "title": "evolve tool",
        "persona": "developer",
        "priority": "Must",
        "size": "M",
        "tier": "flash",
        "depends_on": ["e1-1-start-a-job-from-the-cli"],
        "user_story": "As a developer, I want Claude Code to delegate a test-backed task to EvoSwarm, so that long searches run in the background while my session continues.",
        "context": "Governed by AD-2. Exposes MCP tool `evolve(task, test_command, paths, budget)` over stdio JSON-RPC. Returns job ID within 2 seconds. Rejects missing tests or paths outside repo root.",
        "io_matrix": [
            ("MCP params: task, test_command, paths, budget", "JSON-RPC response: { job_id: '...', status: 'queued' }", "Error if test_command empty or path outside root")
        ],
        "scenarios": [
            ("Given a valid call with task, test command, paths and budget", "when Claude Code invokes `evolve`", "then a job ID returns within 2 s."),
            ("Given no test command", "when invoked", "then the tool returns an error explaining that a test suite is required."),
            ("Given a path outside the repo root", "when invoked", "then the call is rejected and the path named.")
        ],
        "test_plan": [
            ("tests/mcp/test_evolve_tool.rs::test_evolve_submission_fast_return", "Asserts job created and ticket returned in <2s."),
            ("tests/mcp/test_evolve_tool.rs::test_evolve_requires_test_command", "Asserts validation error if test command omitted."),
            ("tests/mcp/test_evolve_tool.rs::test_evolve_path_traversal_rejection", "Rejects path pointing to `../../etc`.")
        ]
    },
    {
        "key": "e3-2-job-status-tool",
        "short_key": "e3-2",
        "epic": "epic-3",
        "epic_title": "Claude Code Integration (MCP)",
        "title": "job_status tool",
        "persona": "developer",
        "priority": "Must",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e3-1-evolve-tool"],
        "user_story": "As a developer, I want Claude Code to see job progress, so that it can report back and decide when to check again.",
        "context": "Exposes MCP tool `job_status(id)` returning state, current generation, best score, spend so far, and estimated completion time based on calibration data.",
        "io_matrix": [
            ("job_id: String", "{ state: 'running', generation: 2, best_score: 0.85, spend_usd: 0.42, eta_seconds: 90 }", "NotFound error if invalid job ID")
        ],
        "scenarios": [
            ("Given a running job", "when `job_status` is called", "then it returns state, current generation, best score, spend so far and an ETA from calibrated timings."),
            ("Given an unknown job ID", "when called", "then it returns a not-found error.")
        ],
        "test_plan": [
            ("tests/mcp/test_job_status.rs::test_status_query", "Queries running job and asserts JSON schema."),
            ("tests/mcp/test_job_status.rs::test_status_not_found", "Queries nonexistent ID and checks error response.")
        ]
    },
    {
        "key": "e3-3-job-result-tool",
        "short_key": "e3-3",
        "epic": "epic-3",
        "epic_title": "Claude Code Integration (MCP)",
        "title": "job_result tool",
        "persona": "developer",
        "priority": "Must",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e3-1-evolve-tool", "e1-11-patch-and-report"],
        "user_story": "As a developer, I want Claude Code to fetch the finished result, so that it can apply and verify the patch for me.",
        "context": "Exposes MCP tool `job_result(id)` returning git branch, patch file path, score, test pass counts, and report path. Verified by end-to-end test where Claude Code applies the patch locally.",
        "io_matrix": [
            ("job_id: String", "{ branch: '...', patch_path: '...', score: 0.92, tests_passed: 18, report_path: '...' }", "Status returned with no patch if job still running (unless partial=true)")
        ],
        "scenarios": [
            ("Given a completed job", "when `job_result` is called", "then it returns branch name, patch path, score, tests passed and report path."),
            ("Given an unfinished job", "when called", "then it returns the current state and no patch unless `partial` is set."),
            ("Given the end-to-end test", "when Claude Code applies the patch and runs the tests locally", "then they pass.")
        ],
        "test_plan": [
            ("tests/mcp/test_job_result.rs::test_fetch_completed_result", "Fetches completed job result and checks file paths."),
            ("tests/mcp/test_job_result.rs::test_local_patch_verification", "Applies returned patch locally and runs test command.")
        ]
    },
    {
        "key": "e3-4-cancel-job-tool",
        "short_key": "e3-4",
        "epic": "epic-3",
        "epic_title": "Claude Code Integration (MCP)",
        "title": "cancel_job tool",
        "persona": "developer",
        "priority": "Must",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e3-1-evolve-tool"],
        "user_story": "As a developer, I want to stop a job from Claude Code, so that spend stops when my plans change.",
        "context": "Exposes MCP tool `cancel_job(id)`. Halts further model dispatch immediately after in-flight calls conclude; marks job `cancelled` and preserves best verified candidate so far.",
        "io_matrix": [
            ("job_id: String", "{ status: 'cancelled', best_verified_candidate: Option<String> }", "Idempotent if already cancelled")
        ],
        "scenarios": [
            ("Given a running job", "when `cancel_job` is called", "then no new model calls start after those in flight finish."),
            ("Given the job had a verified candidate", "when cancelled", "then that result is returned and the state is `cancelled`."),
            ("Given an already cancelled job", "when called again", "then it returns the same result with no error.")
        ],
        "test_plan": [
            ("tests/mcp/test_cancel_job.rs::test_cancel_running_job", "Cancels job and verifies no subsequent generations run."),
            ("tests/mcp/test_cancel_job.rs::test_cancel_idempotency", "Calls cancel twice and expects identical response.")
        ]
    },
    {
        "key": "e3-5-context-guard",
        "short_key": "e3-5",
        "epic": "epic-3",
        "epic_title": "Claude Code Integration (MCP)",
        "title": "Context guard",
        "persona": "security reviewer",
        "priority": "Must",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e1-3-seed-the-first-generation"],
        "user_story": "As a security reviewer, I want secrets kept out of every prompt, so that nothing sensitive leaves the box.",
        "context": "Filters outgoing prompt context against per-job path allowlist. Scans contents with regex for private keys (`BEGIN PRIVATE KEY`), AWS/OpenAI tokens, and excludes `.env` and `*.pem`.",
        "io_matrix": [
            ("Raw repository context", "Sanitized prompt payload with redacted secrets logged", "Rejection if critical credentials detected")
        ],
        "scenarios": [
            ("Given a per-job path allowlist", "when prompts are built", "then no file outside it is included."),
            ("Given content matching key patterns such as API keys or private keys", "when a prompt is built", "then the match is redacted and the redaction logged."),
            ("Given default settings", "when a job runs", "then `.env` and `*.pem` files are excluded.")
        ],
        "test_plan": [
            ("tests/security/test_context_guard.rs::test_path_allowlist_enforcement", "Ensures files outside allowlist excluded."),
            ("tests/security/test_context_guard.rs::test_secret_redaction", "Injects mock API key and verifies replacement with `[REDACTED_SECRET]`."),
            ("tests/security/test_context_guard.rs::test_env_pem_exclusion", "Places `.env` and `cert.pem` in repo; asserts absent from prompt.")
        ]
    },
    {
        "key": "e3-6-setup-guide",
        "short_key": "e3-6",
        "epic": "epic-3",
        "epic_title": "Claude Code Integration (MCP)",
        "title": "Setup guide",
        "persona": "developer",
        "priority": "Should",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e3-1-evolve-tool", "e3-2-job-status-tool", "e3-3-job-result-tool", "e3-4-cancel-job-tool"],
        "user_story": "As a developer, I want a short setup guide, so that I can run my first job within 15 minutes.",
        "context": "Documents step-by-step setup in README and `docs/guides/mcp-setup.md`, including `claude mcp add` config snippet and troubleshooting fixes for E0-2 self-check failures.",
        "io_matrix": [
            ("Developer follows guide on clean machine", "EvoSwarm running, MCP configured, sample job completes in <15 minutes", "Documented fixes for common AppArmor / linger issues")
        ],
        "scenarios": [
            ("Given a fresh machine", "when I follow the README", "then the sample job completes from Claude Code."),
            ("Given the README", "when read", "then it includes the Claude Code MCP config snippet and a fix for each E0-2 self-check failure.")
        ],
        "test_plan": [
            ("tests/docs/test_quickstart_doc.py::test_claude_config_json_syntax", "Validates MCP config snippet parses as valid JSON."),
            ("tests/docs/test_quickstart_doc.py::test_commands_executable", "Dry-runs CLI commands described in guide.")
        ]
    },

    # ----------------------------------------------------
    # Epic 4: Optional gateway
    # ----------------------------------------------------
    {
        "key": "e4-1-transparent-pass-through",
        "short_key": "e4-1",
        "epic": "epic-4",
        "epic_title": "Optional Gateway",
        "title": "Transparent pass-through",
        "persona": "developer",
        "priority": "Should",
        "size": "L",
        "tier": "pro",
        "depends_on": [],
        "user_story": "As a developer, I want Claude Code to behave identically through the gateway, so that adding it never breaks my workflow.",
        "context": "Governed by AD-2. Built on Axum and Tokio. Forwards SSE events byte-for-byte; preserves `tool_use` and `thinking` blocks; passes upstream 4xx/5xx status and headers unmodified; handles client/upstream aborts.",
        "io_matrix": [
            ("Claude Code HTTP POST /v1/messages", "Exact SSE stream forwarded with byte-for-byte fidelity", "Connection aborted cleanly within 1s on client disconnect")
        ],
        "scenarios": [
            ("Given recorded Claude Code sessions", "when replayed through the gateway", "then the SSE event sequence matches the direct response byte for byte."),
            ("Given responses with tool_use and thinking blocks", "when streamed", "then every block arrives intact."),
            ("Given an upstream 4xx or 5xx", "when forwarded", "then status, body and retry-after headers are unchanged."),
            ("Given either side drops the connection", "when detected", "then the other side is closed within 1 s.")
        ],
        "test_plan": [
            ("tests/gateway/test_sse_passthrough.rs::test_byte_for_byte_fidelity", "Streams recorded Anthropic SSE stream and compares SHA-256 hash."),
            ("tests/gateway/test_sse_passthrough.rs::test_thinking_blocks_intact", "Verifies thinking and tool_use blocks preserved."),
            ("tests/gateway/test_sse_passthrough.rs::test_upstream_error_propagation", "Mocks upstream 429 and asserts retry-after header forwarded.")
        ]
    },
    {
        "key": "e4-2-usage-logging",
        "short_key": "e4-2",
        "epic": "epic-4",
        "epic_title": "Optional Gateway",
        "title": "Usage logging",
        "persona": "operator",
        "priority": "Should",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e4-1-transparent-pass-through"],
        "user_story": "As an operator, I want token usage logged per session and day, so that I know where spend goes.",
        "context": "Extracts `usage` block from completion SSE event; records input, output, and cached tokens to local SQLite table; surfaces via `evoswarm usage --since <date>` without storing prompt contents.",
        "io_matrix": [
            ("SSE stream usage metadata", "Logged usage record in SQLite (session_id, timestamp, in_tokens, out_tokens, cached_tokens)", "No prompt text stored")
        ],
        "scenarios": [
            ("Given any request", "when it completes", "then input, output and cached token counts are logged locally with no prompt content by default."),
            ("Given `evoswarm usage --since <date>`", "when run", "then it prints totals and cost by day.")
        ],
        "test_plan": [
            ("tests/gateway/test_usage_logging.rs::test_usage_record_persistence", "Verifies tokens logged to SQLite."),
            ("tests/gateway/test_usage_logging.rs::test_evoswarm_usage_cli", "Executes `evoswarm usage` and verifies daily cost summary.")
        ]
    },
    {
        "key": "e4-3-exemplar-injection",
        "short_key": "e4-3",
        "epic": "epic-4",
        "epic_title": "Optional Gateway",
        "title": "Exemplar injection",
        "persona": "developer",
        "priority": "Could",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e2-5-seeding-from-similar-winners", "e4-1-transparent-pass-through"],
        "user_story": "As a developer, I want relevant past winners offered to the model in ordinary chats, so that everyday answers benefit from what EvoSwarm has proven.",
        "context": "Governed by AD-2. Intercepts incoming messages; queries FalkorDB for similar winners with a strict 20 ms timeout. Prepends up to 2 exemplars (max 2,000 tokens) to the system prompt.",
        "io_matrix": [
            ("Incoming prompt + FalkorDB query (<20ms)", "Modified prompt with prepended exemplars", "If timeout (>20ms), forward unmodified prompt")
        ],
        "scenarios": [
            ("Given a request similar to stored winners above the threshold", "when forwarded", "then up to 2 exemplars totalling at most 2k tokens are prepended to the system prompt."),
            ("Given the lookup takes over 20 ms", "when it times out", "then the request is forwarded unchanged."),
            ("Given an injection happens", "when logged", "then the exemplar IDs are recorded with the request.")
        ],
        "test_plan": [
            ("tests/gateway/test_exemplar_injection.rs::test_successful_injection", "Verifies system prompt contains retrieved exemplar."),
            ("tests/gateway/test_exemplar_injection.rs::test_timeout_fallback", "Simulates 50ms memory delay; asserts unmodified prompt forwarded in <25ms.")
        ]
    },
    {
        "key": "e4-4-injection-opt-out",
        "short_key": "e4-4",
        "epic": "epic-4",
        "epic_title": "Optional Gateway",
        "title": "Injection opt-out",
        "persona": "developer",
        "priority": "Should",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e4-3-exemplar-injection"],
        "user_story": "As a developer, I want to switch injection off, so that I control when extra context is added.",
        "context": "Supports header `x-evoswarm-inject: off` or config flag `gateway.inject = false`. Bypasses memory lookup entirely while retaining token usage logging.",
        "io_matrix": [
            ("Request with header x-evoswarm-inject: off", "Forwarded unmodified, usage logged, 0ms memory lookup", "Config flag achieves identical result")
        ],
        "scenarios": [
            ("Given header `x-evoswarm-inject: off` or the config flag", "when a request passes through", "then nothing is injected."),
            ("Given injection is off", "when the request completes", "then usage is still logged.")
        ],
        "test_plan": [
            ("tests/gateway/test_opt_out.rs::test_header_opt_out", "Sends `x-evoswarm-inject: off` and confirms zero injection."),
            ("tests/gateway/test_opt_out.rs::test_config_flag_opt_out", "Sets `inject = false` in config and confirms bypass.")
        ]
    },
    {
        "key": "e4-5-latency-budget-in-ci",
        "short_key": "e4-5",
        "epic": "epic-4",
        "epic_title": "Optional Gateway",
        "title": "Latency budget in CI",
        "persona": "operator",
        "priority": "Should",
        "size": "M",
        "tier": "flash",
        "depends_on": ["e4-1-transparent-pass-through"],
        "user_story": "As an operator, I want gateway latency tested continuously, so that it never slows the IDE.",
        "context": "Automated load test running 20 concurrent streams measuring added TTFT. Blocks CI if p95 added latency exceeds 50 ms.",
        "io_matrix": [
            ("20 concurrent stream load test", "p50 and p95 added TTFT reported", "CI failure if p95 > 50ms")
        ],
        "scenarios": [
            ("Given a load test with 20 concurrent streams", "when run", "then p50 and p95 added time-to-first-token are reported."),
            ("Given p95 exceeds 50 ms", "when CI runs", "then the build fails.")
        ],
        "test_plan": [
            ("tests/gateway/test_latency_benchmark.rs::test_20_stream_ttft", "Executes 20 concurrent SSE streams and asserts p95 added TTFT < 50ms.")
        ]
    },

    # ----------------------------------------------------
    # Epic 5: MAP-Elites and more stacks
    # ----------------------------------------------------
    {
        "key": "e5-1-map-elites-grid",
        "short_key": "e5-1",
        "epic": "epic-5",
        "epic_title": "MAP-Elites and Multi-Stack Expansion",
        "title": "MAP-Elites grid",
        "persona": "developer",
        "priority": "Could",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e1-7-weighted-score", "e2-2-lineage-written-to-falkordb"],
        "user_story": "As a developer, I want the archive to keep the best candidate of each kind, so that the search does not collapse onto one approach.",
        "context": "Governed by AD-8. 4x4 phenotypic grid: runtime relative to baseline (4 bins) x diff size in changed lines (4 bins). Replaces cell occupant if incoming candidate has higher score. Writes Cell nodes to FalkorDB.",
        "io_matrix": [
            ("Scored candidate (score, runtime, diff_lines)", "Grid cell (x, y) assigned; occupant replaced if score is higher", "Cell nodes created in FalkorDB")
        ],
        "scenarios": [
            ("Given a scored candidate", "when archived", "then it is placed in a 4 x 4 grid cell by runtime-versus-baseline bin and diff-size bin."),
            ("Given a cell already holds a candidate", "when a higher-scoring one arrives", "then it replaces the old one."),
            ("Given bin edges in config", "when changed", "then the next job uses them, and Cell nodes are written to the graph.")
        ],
        "test_plan": [
            ("tests/engine/test_map_elites.rs::test_cell_binning", "Calculates grid coordinates from runtime and diff lines."),
            ("tests/engine/test_map_elites.rs::test_cell_replacement", "Replaces lower-scoring occupant with higher-scoring candidate.")
        ]
    },
    {
        "key": "e5-2-selection-mode-comparison",
        "short_key": "e5-2",
        "epic": "epic-5",
        "epic_title": "MAP-Elites and Multi-Stack Expansion",
        "title": "Selection mode comparison",
        "persona": "team lead",
        "priority": "Could",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e5-1-map-elites-grid", "e1-13-benchmark-suite"],
        "user_story": "As a team lead, I want MAP-Elites compared against top-k, so that we keep it only if it earns its complexity.",
        "context": "Allows setting `selection = 'topk'` or `'mapelites'` in config. Runs benchmark suite under both modes and compares solve rate, token spend, and candidate diversity.",
        "io_matrix": [
            ("Selection mode flag", "Comparative report: MAP-Elites vs Top-K on benchmark suite", "Summary printed with diversity metrics")
        ],
        "scenarios": [
            ("Given `selection: topk` or `selection: mapelites`", "when a job runs", "then parents are drawn by that mode."),
            ("Given the benchmark", "when run in both modes", "then solve rate and tokens per solved task are reported side by side.")
        ],
        "test_plan": [
            ("tests/benchmark/test_selection_modes.py::test_mode_dispatch", "Verifies parent selection reflects config setting."),
            ("tests/benchmark/test_selection_modes.py::test_side_by_side_reporting", "Generates comparative solve rate report.")
        ]
    },
    {
        "key": "e5-3-java-stack",
        "short_key": "e5-3",
        "epic": "epic-5",
        "epic_title": "MAP-Elites and Multi-Stack Expansion",
        "title": "Java stack",
        "persona": "developer",
        "priority": "Could",
        "size": "L",
        "tier": "pro",
        "depends_on": ["e0-1-sandbox-runner-interface", "e0-4-resource-limits"],
        "user_story": "As a developer, I want Java tasks tested with JUnit or Cucumber, so that legacy monoliths can be evolved safely.",
        "context": "Governed by AD-1. Prepares `.m2` repository snapshot keyed by pom.xml hash. Runs `mvn -o test` inside bwrap with no network, read-only JVM mounts (`/usr/lib/jvm`), and 2GB memory cap.",
        "io_matrix": [
            ("Maven project + candidate patch", "Parsed JUnit / Cucumber test summary", "Fails loudly if network requested or snapshot missing")
        ],
        "scenarios": [
            ("Given a pom.xml", "when the first job runs", "then a `.m2` snapshot is built outside the sandbox keyed by its hash."),
            ("Given the snapshot", "when `mvn -o test` runs with no network", "then JUnit and Cucumber samples pass."),
            ("Given the Java profile", "when the red-team suite and E0-9 calibration run", "then all attempts fail and limits are recorded.")
        ],
        "test_plan": [
            ("tests/stacks/test_java_stack.rs::test_m2_snapshot_caching", "Caches m2 snapshot by pom.xml hash."),
            ("tests/stacks/test_java_stack.rs::test_mvn_offline_execution", "Runs `mvn -o test` in bwrap with JUnit test parsing."),
            ("tests/stacks/test_java_stack.rs::test_java_red_team_containment", "Executes red-team suite in Java sandbox.")
        ]
    },
    {
        "key": "e5-4-c-and-cpp-stack",
        "short_key": "e5-4",
        "epic": "epic-5",
        "epic_title": "MAP-Elites and Multi-Stack Expansion",
        "title": "C and C++ stack",
        "persona": "developer",
        "priority": "Could",
        "size": "M",
        "tier": "pro",
        "depends_on": ["e0-1-sandbox-runner-interface", "e0-4-resource-limits"],
        "user_story": "As a developer, I want C and C++ tasks tested with GoogleTest or Catch2, so that native code can be evolved safely.",
        "context": "Governed by AD-1. Configures CMake in tmpfs against read-only prebuilt test libraries. Runs ctest in bwrap with 1GB memory limit and 60s timeout.",
        "io_matrix": [
            ("CMake C/C++ project + patch", "Parsed GoogleTest / Catch2 test summary", "Compilation or linkage error captured in feedback")
        ],
        "scenarios": [
            ("Given a CMake project", "when a candidate runs", "then configure and build happen in tmpfs against read-only prebuilt test libraries."),
            ("Given the samples", "when run", "then GoogleTest and Catch2 suites pass."),
            ("Given the C/C++ profile", "when the red-team suite and E0-9 calibration run", "then all attempts fail and limits are recorded.")
        ],
        "test_plan": [
            ("tests/stacks/test_cpp_stack.rs::test_cmake_tmpfs_build", "Builds CMake candidate in ephemeral tmpfs."),
            ("tests/stacks/test_cpp_stack.rs::test_googletest_execution", "Executes GoogleTest test suite offline."),
            ("tests/stacks/test_cpp_stack.rs::test_cpp_red_team_containment", "Verifies red team attempts blocked in native sandbox.")
        ]
    },
    {
        "key": "e5-5-test-triage-page",
        "short_key": "e5-5",
        "epic": "epic-5",
        "epic_title": "MAP-Elites and Multi-Stack Expansion",
        "title": "Test triage page",
        "persona": "reviewer",
        "priority": "Could",
        "size": "M",
        "tier": "flash",
        "depends_on": ["e2-6-approve-candidate-tests"],
        "user_story": "As a reviewer, I want one page to triage candidate tests, so that approving them is not a CLI chore.",
        "context": "Lightweight web UI binding strictly to 127.0.0.1. Displays candidate adversary tests for a repository, which candidates failed them, and provides 1-click Approve / Reject buttons.",
        "io_matrix": [
            ("HTTP GET /triage on 127.0.0.1", "HTML interface with candidate tests & diffs", "Refuses connections outside 127.0.0.1"),
            ("HTTP POST /triage/approve?id=...", "Promotes test to trusted via E2-6 workflow", "Status: 200 OK")
        ],
        "scenarios": [
            ("Given the page", "when started", "then it binds to 127.0.0.1 only."),
            ("Given candidate tests for a repo", "when listed", "then each shows its code and which candidates it failed."),
            ("Given I approve or reject a test", "when saved", "then the same path as E2-6 is used.")
        ],
        "test_plan": [
            ("tests/ui/test_triage_page.rs::test_binds_localhost_only", "Asserts server socket rejects non-loopback bind."),
            ("tests/ui/test_triage_page.rs::test_triage_approval_action", "Submits test approval and asserts promotion in DB.")
        ]
    },
    {
        "key": "e5-6-archive-health-report",
        "short_key": "e5-6",
        "epic": "epic-5",
        "epic_title": "MAP-Elites and Multi-Stack Expansion",
        "title": "Archive health report",
        "persona": "developer",
        "priority": "Could",
        "size": "S",
        "tier": "flash",
        "depends_on": ["e5-1-map-elites-grid"],
        "user_story": "As a developer, I want to see archive diversity per job, so that I can tell whether the search stayed broad.",
        "context": "Renders an ASCII or HTML visualization of the 4x4 grid showing cell occupancy (out of 16) and highest score per cell across generations.",
        "io_matrix": [
            ("Completed job with MAP-Elites archive", "Occupancy count (e.g. 11/16 cells) + 4x4 matrix heatmap in job report", "Diversity score logged")
        ],
        "scenarios": [
            ("Given a completed job", "when the report renders", "then it shows occupied cells out of 16 and the best score per cell for each generation.")
        ],
        "test_plan": [
            ("tests/engine/test_archive_report.rs::test_grid_heatmap_rendering", "Validates 4x4 ASCII grid output."),
            ("tests/engine/test_archive_report.rs::test_occupancy_ratio_calculation", "Calculates occupied cell count.")
        ]
    }
]


def render_story_markdown(story: dict) -> str:
    dep_str = ", ".join([f"`{d}`" for d in story["depends_on"]]) if story["depends_on"] else "None"
    
    io_rows = []
    for inp, out, err in story["io_matrix"]:
        io_rows.append(f"| `{inp}` | `{out}` | `{err}` |")
    io_table = "\n".join(io_rows)

    scenario_rows = []
    for sc in story["scenarios"]:
        if len(sc) == 3:
            scenario_rows.append(f"- **{sc[0]}**, **{sc[1]}**, **{sc[2]}**.")
        elif len(sc) == 2:
            scenario_rows.append(f"- **{sc[0]}**, **{sc[1]}**.")
        else:
            scenario_rows.append(f"- {' '.join(sc)}")
    scenarios_text = "\n".join(scenario_rows)

    test_rows = []
    for test_fn, desc in story["test_plan"]:
        test_rows.append(f"- [`{test_fn}`](file:///workspace/calm-faraday/{test_fn.split('::')[0]}): {desc}")
    test_plan_text = "\n".join(test_rows)

    content = f"""# Story: {story['title']}

## Metadata
- **Story Key:** `{story['key']}` (Short: `{story['short_key']}`)
- **Epic:** [{story['epic_title']}](file:///workspace/calm-faraday/docs/epics/{story['epic']}.md)
- **Persona:** {story['persona'].title()}
- **Priority:** {story['priority']}
- **Sizing:** {story['size']}
- **Execution Tier:** `{story['tier']}`
- **Dependencies:** {dep_str}

---

## 1. User Story
{story['user_story']}

---

## 2. Architectural Context & Invariants
{story['context']}

---

## 3. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
{io_table}

---

## 4. Acceptance Criteria (Given / When / Then)
{scenarios_text}

---

## 5. TDD Implementation Plan (Red Phase)
Before any production code is authored, author failing test cases:
{test_plan_text}

### Verification Gate Command
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test {story['key'].replace('-', '_')}" --anti-cheat
```
"""
    return content


def main():
    stories_dir = Path("/workspace/calm-faraday/docs/stories")
    stories_dir.mkdir(parents=True, exist_ok=True)

    print(f"Generating {len(STORIES)} stories in {stories_dir}...")
    for s in STORIES:
        file_path = stories_dir / f"{s['key']}.md"
        content = render_story_markdown(s)
        file_path.write_text(content, encoding="utf-8")
        
        # Also create short key symlink if it doesn't conflict
        short_link = stories_dir / f"{s['short_key']}.md"
        if short_link.exists() or short_link.is_symlink():
            short_link.unlink()
        short_link.symlink_to(f"{s['key']}.md")
        
        print(f"  Created: {file_path.name} (symlinked as {short_link.name})")

    print(f"Successfully generated all {len(STORIES)} stories!")


if __name__ == "__main__":
    main()
