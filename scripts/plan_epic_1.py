#!/usr/bin/env python3
"""Plans all 13 stories in Epic 1 (Evolve CLI and Fitness) by creating the three-artifact chain:
intent.md, spec.md, plan.md under docs/stories/<story_key>/

The story directory is canonical for Epic 1. The flat docs/stories/<key>.md files produced by
generate_stories.py are not emitted for Epic 1 because they duplicate these artifacts.

Cargo layout assumed by every plan below (created by e1-1, the first story in dependency order):

    crates/evoswarm-core/src        domain types shared by CLI, engine and fitness
    crates/evoswarm-cli/src         `evoswarm run|status|lineage` binary
    crates/evoswarm-ledger/src      SQLite job ledger (AD-7)
    crates/evoswarm-models/src      role config, dispatch, prompt caching (AD-6)
    crates/evoswarm-engine/src      seeding, mutation, crossover, budget, recovery
    crates/evoswarm-fitness/src     hard gates, scoring, held-out split (AD-5)
    crates/evoswarm-sandbox/src     SandboxBackend impl (Epic 0)
    bench/                          Python benchmark harness (Gate 1)

Integration tests live at crates/<crate>/tests/<target>.rs so that
`cargo test -p <crate> --test <target>` resolves. Subdirectory paths such as
tests/fitness/test_scoring.rs are Cargo helper modules and are never compiled as targets.
"""

import os
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

CRATE_OF = {
    "core": "evoswarm-core",
    "cli": "evoswarm-cli",
    "ledger": "evoswarm-ledger",
    "models": "evoswarm-models",
    "engine": "evoswarm-engine",
    "fitness": "evoswarm-fitness",
    "sandbox": "evoswarm-sandbox",
}

EPIC_1_STORIES = [
    {
        "key": "e1-1-start-a-job-from-the-cli",
        "title": "Start a job from the CLI",
        "persona": "Developer",
        "priority": "Must",
        "sizing": "M",
        "tier": "flash",
        "dependencies": ["e0-5-python-stack", "e0-6-csharp-stack"],
        "jtbd": "When I have a task backed by failing or benchmark tests, I want to submit it via `evoswarm run` in a single command, so that EvoSwarm validates the task baseline and searches for a verified passing patch in the background.",
        "context": "Governed by AD-7 (SQLite Job Ledger) and AD-1 (Sandbox Isolation). The CLI must validate inputs, verify baseline reproducibility across 3 runs to prevent flaky tests, check that tests aren't already passing without a perf objective, and register the job in SQLite.",
        "out_of_scope": "Interactive TUI dashboards, distributed cluster orchestration, or multi-repo workspaces.",
        "contract_matrix": [
            ("`--task`, `--cmd`, `--paths`, optional budgets", "`{\"job_id\": \"<uuid>\", \"status\": \"queued\"}` on stdout, exit 0", "`ValidationError` naming the offending flag"),
            ("`--paths` entry resolving outside repo root", "No job created, exit `ExitCode::PathEscape`", "Prints resolved path and repo root"),
            ("Baseline `--cmd` exits non-zero or is not executable", "No job created, exit `ExitCode::BaselineCommandFailed`", "Child stdout/stderr and exit code echoed"),
            ("Baseline results differ across 3 sandbox runs", "No job created, exit `ExitCode::FlakyTestDetected`", "Lists test names whose status varied"),
            ("All baseline tests pass and `--objective != perf`", "No job created, exit `ExitCode::NothingToImprove`", "Suggests `--objective perf`"),
        ],
        "spec_content": r"""### 1. Command-Line Interface Contract
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
""",
        "scenarios": [
            ("Given valid task, tests, paths and budget", "when I run `evoswarm run`", "then a job ID prints within 2 s and the job status is `queued` in SQLite"),
            ("Given a `--paths` entry containing `../` that escapes the repo root", "when I submit", "then the job is rejected with `ExitCode::PathEscape` and the resolved path shown"),
            ("Given the test command cannot execute on the baseline", "when I submit", "then the job is rejected with the command's non-zero output and exit code shown"),
            ("Given baseline results differ across 3 consecutive runs", "when I submit", "then the job is rejected and the flaky tests are listed"),
            ("Given every baseline test already passes and no performance objective is set", "when I submit", "then I am told there is nothing to improve and asked for `--objective perf`"),
        ],
        "tests": [
            ("cli", "test_submit_valid_job_fast_return", "Submits a valid job and asserts the JSON ticket is emitted in under 2.0 s with SQLite job state `queued`."),
            ("cli", "test_reject_path_traversal_outside_repo", "Passes `--paths ../outside` and asserts `ExitCode::PathEscape` with the resolved path in stderr."),
            ("cli", "test_reject_broken_baseline_command", "Supplies a non-existent binary as the test command and asserts immediate rejection carrying the child exit code."),
            ("cli", "test_reject_flaky_baseline_suite", "Points at a scripted runner that alternates pass/fail and asserts rejection plus the list of varying test names."),
            ("cli", "test_reject_already_passing_suite_without_perf_flag", "Supplies a clean passing suite without `--objective perf` and asserts `ExitCode::NothingToImprove` naming the flag."),
        ],
        "implementation": {
            "crate": "cli",
            "creates": [
                "`Cargo.toml` — virtual workspace manifest with `members = [\"crates/*\"]` and shared `[workspace.dependencies]`.",
                "`crates/evoswarm-cli/src/main.rs` — clap parser for `run`, dispatching to `submit()`.",
                "`crates/evoswarm-cli/src/run.rs` — `pub async fn submit(sub: JobSubmission, repo: &RepoRoot, ledger: &JobLedger) -> Result<JobTicket, SubmitError>`.",
                "`crates/evoswarm-cli/src/exit_codes.rs` — `ExitCode` enum mapped to distinct process codes.",
                "`crates/evoswarm-core/src/job.rs` — `JobSubmission`, `JobObjective`, `JobStatus`, `JobTicket`.",
                "`crates/evoswarm-core/src/path_guard.rs` — `pub fn require_within_root(root: &Path, candidate: &Path) -> Result<PathBuf, PathEscape>`.",
                "`crates/evoswarm-ledger/src/lib.rs` — `JobLedger::open`, `insert_job`, `transition`, `read_status` over SQLite in WAL mode.",
            ],
            "steps": [
                "Scaffold the workspace and confirm `cargo test --workspace` runs with zero tests.",
                "Implement `require_within_root` using `std::fs::canonicalize` on the joined path, then `strip_prefix(root)`; reject on `Err`. Never compare raw strings, which `..%2f` and symlink forms defeat.",
                "Add `JobLedger` with `PRAGMA journal_mode=WAL` and the `jobs` table from AD-7.",
                "Wire `submit()`: parse, validate paths, insert `Queued`, print ticket, then run baseline validation and transition state on failure.",
                "Run the baseline command 3 times through `SandboxBackend::run`, diff the parsed test-name to status map, and emit `FlakyTestDetected` with the varying names.",
                "Gate on all-green plus missing `perf` objective returning `NothingToImprove`.",
            ],
            "notes": "The 2 s ticket deadline is met by persisting and printing before baseline validation, not by making validation fast. `SandboxBackend` is a trait so tests inject a scripted fake instead of bwrap.",
        },
    },
    {
        "key": "e1-2-model-roles-in-config",
        "title": "Model roles in config",
        "persona": "Operator",
        "priority": "Must",
        "sizing": "S",
        "tier": "flash",
        "dependencies": [],
        "jtbd": "When operating EvoSwarm on changing budget or API tier conditions, I want to map distinct model roles (`mutator`, `synthesiser`, `adversary`) in a TOML config file with hot-reload, so that I can optimize token costs without restarting the daemon or recompiling.",
        "context": "Governed by AD-6 (Model Role Segregation). EvoSwarm allocates 75% of calls to fast/cheap mutators and reserves reasoning models for crossover and adversary generation. Dynamic SIGHUP reload prevents interrupting running jobs.",
        "out_of_scope": "Automated bidding on spot LLM auctions, dynamically switching cloud providers mid-generation.",
        "contract_matrix": [
            ("`config.toml` with all three roles populated", "`ModelConfig` usable by dispatch, service starts", "`ConfigError::MissingRole` naming the role"),
            ("Role with empty `model_id` or temperature outside `[0.0, 2.0]`", "Startup abort before any job is accepted", "`ConfigError::InvalidField` naming role and field"),
            ("`SIGHUP` with valid edited config", "Next generation reads new settings, no restart", "Reload logged at info level"),
            ("`SIGHUP` with malformed config", "Previous config stays active, daemon healthy", "Parse error logged to stderr, no swap"),
        ],
        "spec_content": r"""### 1. Configuration Schema (`config.toml`)
```toml
[models.roles.mutator]
provider = "anthropic"
model_id = "claude-3-5-haiku-20241022"
max_tokens = 4096
temperature = 0.4
cost_per_million_input = 0.80
cost_per_million_output = 4.00

[models.roles.synthesiser]
provider = "anthropic"
model_id = "claude-3-5-sonnet-20241022"
max_tokens = 8192
temperature = 0.2
cost_per_million_input = 3.00
cost_per_million_output = 15.00

[models.roles.adversary]
provider = "anthropic"
model_id = "claude-3-5-sonnet-20241022"
max_tokens = 4096
temperature = 0.7
cost_per_million_input = 3.00
cost_per_million_output = 15.00
```

### 2. Validation Rules
- All three roles are mandatory; omitting one aborts startup with the role named.
- `model_id` must be non-empty and must not contain whitespace.
- `temperature` must lie in $[0.0, 2.0]$; `max_tokens` must be $> 0$.
- Cost fields must be $\ge 0.0$ because they feed the AD-6 budget projection in e1-10.

### 3. Hot-Reload via SIGHUP
- Upon receiving `SIGHUP`, the daemon re-parses `config.toml`.
- If valid, the runtime atomic reference `ArcSwap<ModelConfig>` is swapped immediately.
- If invalid, the error is logged to stderr, the existing configuration remains active, and the daemon stays healthy.
- A swap is never observable as a torn read: in-flight calls keep the config snapshot they started with.
""",
        "scenarios": [
            ("Given config maps `mutator`, `synthesiser` and `adversary` to model ID, max tokens and temperature", "when a job runs", "then each dispatch uses its configured role settings"),
            ("Given a mandatory role is omitted or its model ID is empty", "when the service starts", "then it fails loudly with the offending role named"),
            ("Given I edit config.toml and send SIGHUP", "when the next generation starts", "then the new settings apply seamlessly without daemon restart"),
            ("Given I send SIGHUP with malformed TOML", "when the daemon handles the signal", "then the previous config stays active and the daemon remains healthy"),
        ],
        "tests": [
            ("models", "test_parse_valid_role_configuration", "Verifies all three roles deserialize with correct token caps, temperatures and cost fields."),
            ("models", "test_startup_failure_on_invalid_role", "Asserts a startup error naming the role when a mandatory role is omitted or its `model_id` is empty."),
            ("models", "test_sighup_atomic_swap", "Sends SIGHUP after editing temperature and asserts the next read returns the new value."),
            ("models", "test_sighup_rejects_invalid_config", "Sends SIGHUP with malformed TOML and asserts the prior config is still served with no panic."),
        ],
        "implementation": {
            "crate": "models",
            "creates": [
                "`crates/evoswarm-models/src/config.rs` — `ModelConfig`, `RoleConfig`, `Role` enum (`Mutator | Synthesiser | Adversary`).",
                "`crates/evoswarm-models/src/config.rs` — `pub fn load(path: &Path) -> Result<ModelConfig, ConfigError>` and `ModelConfig::validate(&self)`.",
                "`crates/evoswarm-models/src/hot_reload.rs` — `pub struct ConfigHandle(ArcSwap<ModelConfig>)` with `current()` and `reload(path)`.",
                "`crates/evoswarm-models/src/error.rs` — `ConfigError::{MissingRole, InvalidField, Parse}` carrying role and field names.",
            ],
            "steps": [
                "Define `RoleConfig` with serde rename to snake_case TOML keys and `deny_unknown_fields` so typos fail loudly.",
                "Implement `validate()` returning the first error with the role name interpolated; call it from `load()` so no invalid config can reach the handle.",
                "Introduce `arc-swap` and wrap the config in `ConfigHandle`; `reload()` parses into a temp, validates, then swaps.",
                "Register a `tokio::signal::unix::SignalKind::hangup()` task that calls `reload()` and logs the error without propagating it.",
                "Assert in tests that `current()` returns an owned `Arc` snapshot, so a concurrent swap cannot tear a call in progress.",
            ],
            "notes": "Validation runs on every reload, not only at startup — an unvalidated swap is how a typo silently disables budget accounting downstream.",
        },
    },
    {
        "key": "e1-3-seed-the-first-generation",
        "title": "Seed the first generation",
        "persona": "Developer",
        "priority": "Must",
        "sizing": "M",
        "tier": "flash",
        "dependencies": ["e1-1-start-a-job-from-the-cli", "e1-2-model-roles-in-config"],
        "jtbd": "When starting a search, I want Generation 0 populated with diverse initial attempts alongside the current code baseline, so that the evolutionary loop explores multiple distinct conceptual paths.",
        "context": "Governed by AD-5 and AD-6. Population size N (default 6) is seeded by: (1) baseline code, (2) retrieved memory winners if available (up to 2), and (3) N-1 (or N-3) fresh drafts from the mutator model.",
        "out_of_scope": "Synthesising candidates using third-party web search or unverified external snippets.",
        "contract_matrix": [
            ("`JobSubmission`, population size `N`", "`Vec<Candidate>` of length exactly `N`", "`SeedingError::ModelFailure` after retry budget exhausted"),
            ("Two drafts with identical diff SHA-256", "Duplicate dropped, replacement drafted at temperature +0.1", "`SeedingError::DedupExhausted` after 3 replacement attempts"),
            ("Each fresh draft", "Recorded `model_id`, `prompt_hash`, `generation = 0`, `parent_ids = []`", "Draft discarded if any field is absent"),
        ],
        "spec_content": r"""### 1. Seeding Algorithm
1. Candidate 0 is initialized from current baseline files.
2. If similar past winners exist in FalkorDB (Epic 2), inject up to 2 past winning patches. In Epic 1 this hook is a trait method returning an empty list.
3. Mutator role is invoked concurrently with different random seeds to generate remaining drafts up to $N$.
4. **Deduplication:** Compute SHA-256 of candidate diffs. If two drafts produce identical diffs, drop the duplicate and invoke mutator again with temperature jitter ($+0.1$), up to 3 attempts.
5. Each draft records `model_id`, `prompt_hash`, `generation = 0`, and `parent_ids = []`.

### 2. Population Invariant
The returned population always has length exactly $N$. Seeding never returns a short
population: an exhausted retry budget is a hard error, because a silently smaller Gen 0
biases every later selection statistic.

### 3. Determinism
Draft ordering is deterministic given the same seeds, so a replayed job reproduces Gen 0
byte for byte (required by e1-12 idempotency).
""",
        "scenarios": [
            ("Given population size N", "when generation 0 is built", "then it holds the current code plus N-1 fresh drafts"),
            ("Given each draft", "when it is created", "then it records model ID, prompt hash, generation 0 and an empty parent list"),
            ("Given two drafts produce identical content hashes", "when generation 0 is finalised", "then the duplicate is dropped and replaced with a unique draft"),
            ("Given every retry also produces a duplicate", "when the replacement budget is exhausted", "then seeding fails loudly instead of returning a short population"),
        ],
        "tests": [
            ("engine", "test_gen0_population_quota", "Verifies exactly `N` candidates in Gen 0, including the baseline at index 0."),
            ("engine", "test_candidate_metadata_recording", "Verifies `generation = 0`, empty `parent_ids`, non-empty `prompt_hash` and the configured `model_id`."),
            ("engine", "test_diff_hash_deduplication", "Injects a duplicate draft and asserts it is replaced by a distinct draft at jittered temperature."),
            ("engine", "test_dedup_retry_budget_exhaustion", "Forces every draft to collide and asserts `SeedingError::DedupExhausted` rather than a short population."),
        ],
        "implementation": {
            "crate": "engine",
            "creates": [
                "`crates/evoswarm-core/src/candidate.rs` — `Candidate { id, patch, diff_hash, generation, parent_ids, model_id, prompt_hash }`.",
                "`crates/evoswarm-engine/src/seeding.rs` — `pub async fn seed_generation_zero(cfg: &SeedConfig, deps: &SeedingDeps) -> Result<Vec<Candidate>, SeedingError>`.",
                "`crates/evoswarm-engine/src/seeding.rs` — `trait MemorySeeder` returning `Vec<Candidate>` (empty impl in Epic 1; FalkorDB impl arrives in e2-5).",
                "`crates/evoswarm-engine/src/dedup.rs` — `pub fn diff_hash(patch: &[u8]) -> [u8; 32]` and the replacement retry loop.",
            ],
            "steps": [
                "Add `Candidate` to `evoswarm-core` with `Serialize`/`Deserialize` so e1-12 can persist populations as JSON.",
                "Build the baseline candidate from the working tree and pin it at index 0.",
                "Call the mutator role concurrently with `futures::join_all`, one distinct seed per slot, honouring the e1-10 budget check.",
                "Hash each diff, and on collision re-invoke at `temperature + 0.1`, counting attempts and erroring at 3.",
                "Assert population length equals `N` as the final line of `seed_generation_zero`.",
            ],
            "notes": "`MemorySeeder` as a trait is the dependency-inversion seam mandated by architecture-rules section 3: Epic 2 supplies the graph-backed impl without touching this loop.",
        },
    },
    {
        "key": "e1-4-mutation-with-error-feedback",
        "title": "Mutation with error feedback",
        "persona": "Developer",
        "priority": "Must",
        "sizing": "M",
        "tier": "pro",
        "dependencies": ["e1-3-seed-the-first-generation", "e1-6-hard-gates"],
        "jtbd": "When a candidate fails compilation or tests, I want the mutator model provided with the exact compiler errors and failing assertions, so that subsequent mutations converge on passing solutions rather than guessing randomly.",
        "context": "Governed by AD-5. Failed candidates contain high-value signal. The compiler output and first failing test assertion (truncated to 4,000 tokens) are packaged into a structured prompt. Prompt caching prefixes are strictly preserved.",
        "out_of_scope": "Multi-turn conversational debates with the model during a single mutation.",
        "contract_matrix": [
            ("Failed parent `Candidate` + `ExecutionResult`", "Child `Candidate` with `MUTATED_FROM` lineage", "`BudgetExhausted` if the token cap would be crossed"),
            ("Error text larger than the clamp", "Feedback trimmed to 4,000 tokens total", "Truncation marker appended, never a silent cut"),
            ("Two sequential mutation calls", "Byte-identical static prompt prefix", "Cache-miss logged when prefix bytes diverge"),
        ],
        "spec_content": r"""### 1. Feedback Truncation Invariant
- Compiler errors: captured up to first 2,000 tokens.
- Test failures: first failing assertion, test name, and stack trace captured up to 2,000 tokens.
- Total error context hard-clamped to 4,000 tokens.
- Truncation always appends an explicit marker so the model is not misled into believing the dump is complete.
- Token counting uses the same estimator as e1-10 so the clamp and the budget agree.

### 2. Prompt Caching Structure
- **Static Prefix (Cached):** System instructions + repository overview + API contracts + immutable test rules.
- **Dynamic Suffix:** Current parent diff + sandbox execution failure diagnostics.
- The prefix is assembled once per job and reused verbatim; any per-call variation (timestamps, candidate IDs, job IDs) must live in the suffix.

### 3. Lineage
Every child records a `MUTATED_FROM` edge to its parent. Mutation never orphans a candidate.
""",
        "scenarios": [
            ("Given a failed parent candidate", "when its child is drafted", "then the prompt includes the first failing assertion and error text trimmed to 4k tokens"),
            ("Given an error dump far larger than the clamp", "when feedback is assembled", "then the total is hard-clamped to 4,000 tokens and carries a truncation marker"),
            ("Given any child candidate", "when it is stored", "then it is linked to its parent with a `MUTATED_FROM` edge"),
            ("Given repeated mutation calls in one job", "when they are sent", "then repository context sits in a byte-identical static prefix and the cache hit rate is logged"),
        ],
        "tests": [
            ("engine", "test_feedback_prompt_formatting", "Verifies compiler and assertion sections appear in order and the total is clamped to 4k tokens."),
            ("engine", "test_feedback_truncation_marker", "Feeds a 50k-token dump and asserts the result is exactly at the clamp and ends with the truncation marker."),
            ("engine", "test_mutated_from_lineage_edge", "Asserts the `MUTATED_FROM` edge is attached from child to the correct parent id."),
            ("engine", "test_cache_prefix_byte_identity", "Asserts the static prefix bytes are identical across sequential mutations in one job."),
        ],
        "implementation": {
            "crate": "engine",
            "creates": [
                "`crates/evoswarm-engine/src/mutation.rs` — `pub async fn mutate(parent: &Candidate, feedback: &FailureFeedback, deps: &MutationDeps) -> Result<Candidate, MutationError>`.",
                "`crates/evoswarm-engine/src/feedback.rs` — `FailureFeedback`, `pub fn build(result: &ExecutionResult) -> FailureFeedback` and `pub fn render_clamped(&self, budget: usize) -> String`.",
                "`crates/evoswarm-models/src/prompt.rs` — `PromptParts { static_prefix: Vec<u8>, dynamic_suffix: Vec<u8> }` and `pub fn estimate_tokens(bytes: &[u8]) -> usize`.",
                "`crates/evoswarm-models/src/lineage.rs` — `trait LineageSink` with `record_mutation(child, parent)`; graph impl arrives in Epic 2.",
            ],
            "steps": [
                "Implement `estimate_tokens` once here and reuse it in e1-10 so the clamp and budget never disagree.",
                "Build `FailureFeedback` from `ExecutionResult`: split compiler output from the first failing assertion and stack trace.",
                "Apply the 2k/2k/4k clamps in `render_clamped`, appending the truncation marker whenever a section is cut.",
                "Assemble the static prefix once per job and store it; construct each call as `prefix ++ suffix` with no per-call data in the prefix.",
                "Record the `MUTATED_FROM` edge through `LineageSink` immediately after the child is accepted.",
                "Log cache hit rate as `prefix_bytes_identical` plus provider-reported cache reads.",
            ],
            "notes": "Prefix byte-identity is a correctness property, not a cost optimisation: a timestamp in the prefix silently disables caching and inflates every e1-10 projection.",
        },
    },
    {
        "key": "e1-5-crossover-of-two-parents",
        "title": "Crossover of two parents",
        "persona": "Developer",
        "priority": "Should",
        "sizing": "M",
        "tier": "pro",
        "dependencies": ["e1-4-mutation-with-error-feedback"],
        "jtbd": "When two candidates pass different subsets of the test suite, I want a synthesiser model to recombine their complementary strengths, so that partial fixes can be merged into a comprehensive solution.",
        "context": "Governed by AD-6. Crossover pairs are drawn preferentially from candidates that pass disjoint test partitions. Crossover calls use the reasoning synthesiser model and are capped at 25% of total job calls.",
        "out_of_scope": "Synthesising candidates without test provenance data.",
        "contract_matrix": [
            ("Parent A (passes tests 1, 2) + Parent B (passes tests 3, 4)", "Child `Candidate` with synthesiser-reported traits", "Fall back to mutation when no pair has $D > 0$"),
            ("Pass vectors for the whole population", "Pairs ranked by descending Hamming distance", "Empty ranking yields mutation-only generation"),
            ("Crossover dispatch request", "Synthesiser call, two `MERGED_FROM` edges", "`BudgetExhausted` once calls reach 25% of total"),
        ],
        "spec_content": r"""### 1. Parent Selection for Recombination
- Candidate pairs $(P_A, P_B)$ are scored by Hamming distance of test pass vectors:
  $$D(P_A, P_B) = |(Pass_A \setminus Pass_B) \cup (Pass_B \setminus Pass_A)|$$
- Pairs with highest $D > 0$ are scheduled for crossover.
- If every pair has $D = 0$, the generation falls back to mutation only.

### 2. Synthesiser Dispatch & Lineage
- Prompt presents Parent A diff and tests it passes, Parent B diff and tests it passes.
- Synthesiser emits a unified patch and a summary of merged traits.
- Graph edges: `(Child)-[:MERGED_FROM {traits: "..."}]->(Parent_A)` and `(Child)-[:MERGED_FROM {traits: "..."}]->(Parent_B)`.
- Job call budget enforces crossover calls $\le 0.25 \times \text{TotalCalls}$.

### 3. Provenance Requirement
A candidate without recorded test provenance is never eligible as a parent: crossover of
unmeasured code cannot be shown to combine complementary strengths.
""",
        "scenarios": [
            ("Given two parents pass different subsets of tests", "when parents are selected for crossover", "then the pair with the greatest Hamming distance is prioritised"),
            ("Given every candidate passes exactly the same tests", "when a generation is planned", "then no crossover is scheduled and the generation falls back to mutation"),
            ("Given a crossover child is generated", "when it is stored", "then it links to both parents with the traits reported by the synthesiser model"),
            ("Given default budget settings", "when a job runs", "then crossover calls constitute at most 25% of total model calls"),
        ],
        "tests": [
            ("engine", "test_disjoint_test_pair_selection", "Asserts pairs are ranked by descending Hamming distance and the top disjoint pair is chosen."),
            ("engine", "test_no_disjoint_pair_falls_back_to_mutation", "Gives all candidates identical pass vectors and asserts zero crossover calls are scheduled."),
            ("engine", "test_merged_from_dual_lineage", "Verifies the child connects to both parents with merged traits metadata on each edge."),
            ("engine", "test_crossover_call_cap_budget", "Simulates generation dispatch and asserts crossover calls stay at or below 25% of total calls."),
        ],
        "implementation": {
            "crate": "engine",
            "creates": [
                "`crates/evoswarm-engine/src/crossover.rs` — `pub fn rank_pairs(pop: &[Candidate]) -> Vec<(usize, usize, usize)>` returning `(a, b, distance)` sorted descending.",
                "`crates/evoswarm-engine/src/crossover.rs` — `pub async fn crossover(a: &Candidate, b: &Candidate, deps: &CrossoverDeps) -> Result<Candidate, CrossoverError>`.",
                "`crates/evoswarm-core/src/provenance.rs` — `PassVector` bitset keyed by trusted test name.",
                "`crates/evoswarm-engine/src/budget.rs` — extend the e1-10 counter with `crossover_calls` and the 25% cap check.",
            ],
            "steps": [
                "Represent each candidate's trusted-test outcomes as a `PassVector` bitset so distance is a XOR plus `count_ones`.",
                "Implement `rank_pairs`, skipping any candidate lacking provenance and returning early when all distances are 0.",
                "Build the synthesiser prompt from both diffs with their passing test lists, reusing the e1-4 static prefix.",
                "Parse the returned patch and traits summary; on parse failure, discard the child rather than retrying silently.",
                "Record both `MERGED_FROM` edges with the traits payload.",
                "Check the 25% cap before dispatch and degrade to mutation when it would be exceeded.",
            ],
            "notes": "The cap is checked pre-dispatch, not reconciled afterwards: an over-cap call has already spent the tokens it was meant to save.",
        },
    },
    {
        "key": "e1-6-hard-gates",
        "title": "Hard gates",
        "persona": "Reviewer",
        "priority": "Must",
        "sizing": "M",
        "tier": "pro",
        "dependencies": ["e0-7-tamper-proof-tests"],
        "jtbd": "When evaluating any candidate code patch, I want 4 non-negotiable hard gates enforced, so that no candidate that games tests or tampers with harnesses can ever be scored or returned.",
        "context": "Governed by AD-5. Gating is evaluated prior to scoring: (1) Clean build, (2) Visible trusted tests pass, (3) Zero diffs on tests/harnesses, (4) Zero skips/deletions relative to baseline. Any failure sets score $S = 0$.",
        "out_of_scope": "Subjective aesthetic code formatting gating.",
        "contract_matrix": [
            ("`Candidate` patch + `SandboxProfile`", "`GateResult { passed: bool, reason: Option<GateFailure> }`", "First failing gate short-circuits the rest"),
            ("Patch touching `tests/`, `conftest.py`, `Directory.Build.props` or build files", "`GateResult { passed: false, reason: Tamper }`", "Offending paths listed in the report"),
            ("Build or trusted-test failure", "Score forced to `0.0`, candidate excluded from selection", "Failure reason fed back to the mutator via e1-4"),
            ("Fewer tests executed than baseline, or new skips", "`GateResult { passed: false, reason: Skipped }`", "Baseline and candidate counts reported"),
        ],
        "spec_content": r"""### 1. Gate Definitions & Priority Order
Gates run in this order and short-circuit: the first failure is the reported reason.
1. **Gate 1 (Build):** Exit code of the build step $= 0$. Failure reason: `build`.
2. **Gate 2 (Trusted Pass):** Count of passed visible trusted tests $=$ total visible trusted tests. Failure reason: `test_failure`.
3. **Gate 3 (Tamper):** Diff touches only paths in `--paths`. Any touch of `tests/`, `conftest.py`, `Directory.Build.props`, or build files fails immediately with reason: `tamper`.
4. **Gate 4 (No Skips):** Count of executed tests $\ge$ baseline count; zero newly skipped or ignored tests. Failure reason: `skipped`.

Gate 3 is evaluated against the patch file list, not against filesystem state, so a
candidate cannot pass by reverting a tampered file after the run.

### 2. Failure Outcome
- Composite score $S = 0.0$.
- Candidate excluded from winning selection.
- Detailed failure reason logged for mutator feedback (consumed by e1-4).

### 3. Non-Negotiability
No configuration flag, model output or later stage may override a failed gate. Gate
evaluation is pure: identical inputs always yield an identical `GateResult`.
""",
        "scenarios": [
            ("Given a candidate fails to compile", "when evaluated", "then its score is 0 and the reason is `build`"),
            ("Given any visible trusted test fails", "when evaluated", "then its score is 0 and the failing test names are recorded"),
            ("Given the diff touches tests, harness files or build scripts", "when evaluated", "then its score is 0 and the reason is `tamper`"),
            ("Given fewer tests ran than in the baseline or new skips appear", "when evaluated", "then its score is 0 and the reason is `skipped`"),
            ("Given a candidate that fails more than one gate at once", "when evaluated", "then the reported reason is the earliest gate in the defined order"),
        ],
        "tests": [
            ("fitness", "test_build_error_gate", "Simulates a compiler failure and asserts `passed = false`, reason `build`, score 0."),
            ("fitness", "test_assertion_failure_gate", "Simulates a test assertion failure and asserts reason `test_failure` with the failing test names listed."),
            ("fitness", "test_harness_tamper_gate", "Injects a `conftest.py` change into the patch and asserts reason `tamper` naming the path."),
            ("fitness", "test_skipped_test_detection", "Marks one test skipped relative to baseline and asserts reason `skipped` with both counts reported."),
            ("fitness", "test_gate_short_circuit_order", "Fails build and tampers simultaneously and asserts the reported reason is `build`, proving gate order."),
        ],
        "implementation": {
            "crate": "fitness",
            "creates": [
                "`crates/evoswarm-fitness/src/gates.rs` — `pub fn evaluate(candidate: &Candidate, run: &ExecutionResult, baseline: &Baseline) -> GateResult`.",
                "`crates/evoswarm-fitness/src/gates.rs` — `GateResult { passed: bool, reason: Option<GateFailure>, failed_tests: Vec<String> }` and `GateFailure::{Build, TestFailure, Tamper, Skipped}`.",
                "`crates/evoswarm-fitness/src/tamper.rs` — `pub fn detect(patch_paths: &[PathBuf], allowed: &[PathBuf]) -> Vec<PathBuf>` returning offending paths.",
                "`crates/evoswarm-fitness/src/baseline.rs` — `Baseline { test_count, skipped, trusted_names }` captured at job start.",
            ],
            "steps": [
                "Capture `Baseline` once at job start through `SandboxBackend::run` and reuse it for every candidate.",
                "Implement the four gates as separate private functions returning `Option<GateFailure>`, evaluated in order with early return.",
                "Implement tamper detection over the patch's declared path list, matching `tests/`, `conftest.py`, `Directory.Build.props` and build files; compare canonicalised paths, never substrings, so `src/tests_helper.rs` is not a false positive.",
                "Compare executed and skipped counts against `Baseline` for Gate 4.",
                "Force `S = 0.0` and exclusion at the single call site so no later stage can resurrect a gated candidate.",
            ],
            "notes": "Substring matching on `tests/` is the classic false positive here: `mytests/` and `latests.rs` both match. Anchor on path components.",
        },
    },
    {
        "key": "e1-7-weighted-score",
        "title": "Weighted score",
        "persona": "Developer",
        "priority": "Must",
        "sizing": "S",
        "tier": "flash",
        "dependencies": ["e1-6-hard-gates"],
        "jtbd": "When multiple candidates pass all hard gates, I want them ranked by a deterministic, multi-objective score function, so that the highest quality, highest performance, and most concise patch wins.",
        "context": "Governed by AD-5. Calculates $S = w_a A + w_p P + w_s Z$ with default weights $w_a = 0.5, w_p = 0.3, w_s = 0.2$. Weights must sum to 1.0. If adversary tests are absent, $w_a$ is redistributed proportionally.",
        "out_of_scope": "Dynamic weight rebalancing during a running generation.",
        "contract_matrix": [
            ("`A`, `P`, `Z` in $[0.0, 1.0]$ plus `Weights`", "Composite score `S: f64`", "`ScoreError::WeightsNotNormalised` naming the offending sum"),
            (r"Weights summing to $1.0 \pm \epsilon$", "Service starts, scoring proceeds", "Startup abort listing configured weights"),
            ("Zero valid adversary tests", "`S` from redistributed $w'_p$, $w'_s$ only", "`ScoreError::DivisionByZero` if $w_p + w_s = 0$"),
            ("Candidate runtime $= 0$ or diff of 0 lines", "Clamped $P = 1.0$, $Z = 1.0$", "No NaN or infinity may escape"),
        ],
        "spec_content": r"""### 1. Scoring Terms
- $A \in [0.0, 1.0]$: Pass rate on candidate adversary tests: $\frac{\text{Passed Adversary Tests}}{\text{Total Valid Adversary Tests}}$.
- $P \in [0.0, 1.0]$: Relative runtime score: $\text{clamp}\left(\frac{\text{Baseline Runtime}}{\text{Candidate Runtime}}, 0.0, 1.0\right)$.
- $Z \in [0.0, 1.0]$: Size parsimony score: $e^{-\frac{\text{diff\_lines}}{100}}$.

### 2. Composite Score
$$S = w_a A + w_p P + w_s Z$$
with defaults $w_a = 0.5$, $w_p = 0.3$, $w_s = 0.2$. Weights are read from `config.toml`
(e1-2) and must sum to $1.0$ within $\epsilon = 10^{-9}$.

### 3. Redistribution Rule (No Adversary Tests)
When adversary tests are absent, $w_a$ is redistributed proportionally to the remaining terms:
$$w'_p = \frac{w_p}{w_p + w_s} = \frac{0.3}{0.5} = 0.6$$
$$w'_s = \frac{w_s}{w_p + w_s} = \frac{0.2}{0.5} = 0.4$$
$$S = w'_p \cdot P + w'_s \cdot Z$$
If $w_p + w_s = 0$, redistribution is undefined and scoring fails loudly rather than
defaulting silently.

### 4. Numerical Determinism
Identical inputs must produce bitwise identical `f64` output across processes and runs.
Terms are therefore combined in a fixed order with no parallel or reassociated
floating-point reduction, and no input-dependent branching that changes operation order.
""",
        "scenarios": [
            ("Given a candidate passes all gates", "when scored", "then S is computed using adversary pass rate, runtime versus baseline, and diff size"),
            ("Given weights that do not sum to 1.0 in config", "when the service starts", "then it fails with an explicit configuration error showing the weights"),
            ("Given no adversary tests exist", "when scoring", "then the adversary weight is redistributed proportionally to the runtime and diff size terms"),
            ("Given identical inputs", "when scored 1,000 times", "then every returned score is bitwise identical"),
            ("Given a candidate runtime of zero or a zero-line diff", "when scored", "then the result is clamped and contains no NaN or infinity"),
        ],
        "tests": [
            ("fitness", "test_exact_weighted_score_math", "Validates exact arithmetic against a hand-computed vector with default weights."),
            ("fitness", "test_weight_validation_at_startup", "Rejects a config with weights summing to 1.05 and asserts the error prints the configured weights."),
            ("fitness", "test_proportional_redistribution_without_adversary", "Checks $w'_p = 0.6$, $w'_s = 0.4$ when the adversary count is 0."),
            ("fitness", "test_deterministic_scoring", "Scores an identical candidate 1,000 times and asserts bitwise equality via `f64::to_bits`."),
            ("fitness", "test_edge_inputs_are_clamped", "Passes zero runtime and zero diff lines and asserts a finite score in $[0.0, 1.0]$."),
        ],
        "implementation": {
            "crate": "fitness",
            "creates": [
                "`crates/evoswarm-fitness/src/scoring.rs` — `pub struct Weights { pub w_a: f64, pub w_p: f64, pub w_s: f64 }` with `Weights::validate(&self)`.",
                "`crates/evoswarm-fitness/src/scoring.rs` — `pub fn score(terms: &ScoreTerms, weights: &Weights) -> Result<f64, ScoreError>`.",
                "`crates/evoswarm-fitness/src/scoring.rs` — `pub fn adversary_pass_rate(passed: usize, total: usize) -> Option<f64>` returning `None` when `total == 0`.",
                "`crates/evoswarm-fitness/src/scoring.rs` — `pub fn runtime_term(baseline_ms: u64, candidate_ms: u64) -> f64` and `pub fn parsimony_term(diff_lines: usize) -> f64`.",
            ],
            "steps": [
                "Validate weights at config load (e1-2 hook) and again in `score()` so a hand-built `Weights` cannot bypass the sum check.",
                "Return `Option::None` from `adversary_pass_rate` when the denominator is 0; `score()` branches to the redistribution path on `None`.",
                "Guard `candidate_ms == 0` before dividing, returning the clamped maximum rather than infinity.",
                "Compute `w_a * A + w_p * P + w_s * Z` in that fixed source order; do not refactor into a fold over a dynamically ordered slice.",
                "Reject $w_p + w_s = 0$ explicitly in the redistribution path.",
            ],
            "notes": "Bitwise determinism is why the term order is fixed in code. A fold over an unordered map would pass every functional test and still reorder floating-point additions.",
        },
    },
    {
        "key": "e1-8-held-out-tests",
        "title": "Held-out tests",
        "persona": "Reviewer",
        "priority": "Must",
        "sizing": "M",
        "tier": "pro",
        "dependencies": ["e1-6-hard-gates"],
        "jtbd": "When selecting a winning implementation, I want it evaluated against a hidden 20% slice of trusted tests that models never saw, so that overfitting and prompt memorization are prevented.",
        "context": "Governed by AD-5. The user's trusted test suite is partitioned 80/20 at job start using a stable hash seed. Held-out tests are never provided in mutation prompts. The top-scoring candidate must pass 100% of held-out tests.",
        "out_of_scope": "Generating artificial held-out tests with LLMs.",
        "contract_matrix": [
            ("Full trusted test suite, `job_id`", "`(VisibleTests, HeldOutTests)` stable for that job", "Held-out empty when $M < 5$"),
            ("Any outgoing model prompt", "Zero held-out test names or assertions present", "Prompt rejected before dispatch on leak detection"),
            ("Top candidate + held-out tests", "Verified winner, or fallback to next candidate", "Status `no verified winner` when all fail"),
        ],
        "spec_content": r"""### 1. Partitioning Invariant
- Total trusted tests $M$. If $M \ge 5$, held-out count $H = \max(1, \lfloor 0.20 \times M \rfloor)$.
- If $M < 5$, $H = 0$: holding out from a tiny suite would leave too few visible tests to drive search.
- Test names are assigned using SHA-256(JobId + TestName); the $H$ lowest hashes form the held-out set.
- The split is stable for a job: recomputing it always yields the same partition, which is what makes e1-12 resume safe.

### 2. Leakage Barrier
Held-out test names, bodies and assertion text never appear in any prompt payload.
Leakage scanning runs over the assembled prompt before dispatch, not over the raw
template, so interpolated diffs and error dumps are covered too.

### 3. Selection Loop
1. Rank candidates passing hard gates by score $S$ descending.
2. For the top candidate, run held-out tests in the sandbox.
3. If 100% pass $\rightarrow$ candidate is the **Verified Winner**.
4. If any held-out test fails $\rightarrow$ evaluate the next candidate.
5. If all candidates fail $\rightarrow$ emit status `no verified winner` and flag the best-effort patch.
""",
        "scenarios": [
            ("Given a job starts", "when tests are partitioned", "then about 20% are held out using a stable split keyed by job ID"),
            ("Given any model call", "when prompt payloads are scanned", "then no held-out test content appears in any prompt"),
            ("Given the top candidate fails held-out tests", "when selecting the winner", "then the runner falls back to evaluate the next candidate"),
            ("Given no candidate passes held-out tests", "when the job ends", "then it reports `no verified winner` and flags the best-effort patch"),
            ("Given a suite with fewer than 5 trusted tests", "when it is partitioned", "then the held-out set is empty and the reason is recorded"),
        ],
        "tests": [
            ("fitness", "test_stable_partitioning_ratio", "Validates the 80/20 partition and its stability across suites of size 5, 20 and 100."),
            ("fitness", "test_zero_held_out_prompt_leakage", "Scans assembled prompt payloads and asserts zero held-out test names or assertion text."),
            ("fitness", "test_candidate_fallback_ladder", "Forces candidate 1 to fail held-out tests and verifies candidate 2 is selected."),
            ("fitness", "test_no_verified_winner_status", "Fails held-out tests for every candidate and asserts status `no verified winner` with the best-effort patch flagged."),
            ("fitness", "test_small_suite_yields_empty_holdout", "Partitions a 3-test suite and asserts $H = 0$ with the reason recorded."),
        ],
        "implementation": {
            "crate": "fitness",
            "creates": [
                "`crates/evoswarm-fitness/src/holdout.rs` — `pub fn partition(job_id: &str, trusted: &[String]) -> TestSplit` returning `{ visible, held_out }`.",
                "`crates/evoswarm-fitness/src/holdout.rs` — `pub fn assignment_hash(job_id: &str, test_name: &str) -> [u8; 32]`.",
                "`crates/evoswarm-fitness/src/leak_scan.rs` — `pub fn assert_no_leakage(prompt: &[u8], held_out: &[String]) -> Result<(), LeakDetected>`.",
                "`crates/evoswarm-engine/src/selection.rs` — `pub async fn select_verified_winner(ranked: &[Candidate], split: &TestSplit, deps: &SelectionDeps) -> SelectionOutcome`.",
                "`crates/evoswarm-core/src/outcome.rs` — `SelectionOutcome::{Verified(Candidate), NoVerifiedWinner { best_effort: Candidate } }`.",
            ],
            "steps": [
                "Compute `assignment_hash` per trusted test name, sort ascending, take the lowest $H$ as held-out.",
                "Apply the $M < 5$ rule before hashing so a tiny suite never loses a test.",
                "Persist `TestSplit` with the job row so resume (e1-12) reuses the identical partition instead of recomputing from a possibly changed suite.",
                "Call `assert_no_leakage` at the single dispatch chokepoint in `evoswarm-models`, covering every role.",
                "Walk the ranked candidate list, running held-out tests in the sandbox per candidate, stopping at the first 100% pass.",
                "Emit `NoVerifiedWinner` with the highest-scoring gated candidate flagged best-effort when the list is exhausted.",
            ],
            "notes": "Leak scanning belongs at the dispatch chokepoint, not in each prompt builder: one unwired builder is enough to invalidate the entire held-out guarantee.",
        },
    },
    {
        "key": "e1-9-adversary-tests",
        "title": "Adversary tests",
        "persona": "Developer",
        "priority": "Should",
        "sizing": "M",
        "tier": "pro",
        "dependencies": ["e1-6-hard-gates"],
        "jtbd": "When evolving code, I want an adversary model writing edge-case tests against the task specification, so that fragile candidates that pass minimal suites are probed and ranked lower.",
        "context": "Governed by AD-5. Adversary model drafts $K$ candidate tests. Broken tests that do not compile are discarded. Tests failing across baseline and all candidates are tagged `suspect`. Adversary tests never fail gates.",
        "out_of_scope": "Adversary tests failing hard gates or blocking build verification.",
        "contract_matrix": [
            ("Spec + best candidate patch", "Up to $K$ adversary test cases", "Zero valid tests yields `A` absent, triggering e1-7 redistribution"),
            ("Test that does not compile in the sandbox", "Discarded before execution", "Compiler diagnostic logged, not surfaced to scoring"),
            ("Test failing on baseline and all candidates", "Tagged `suspect`, excluded from $A$", "Suspect list included in the e1-11 report"),
            ("Any adversary test outcome", "Never affects a hard gate result", "Gate evaluation receives trusted tests only"),
        ],
        "spec_content": r"""### 1. Generation & Filtering Pipeline
1. Adversary model generates up to $K$ test functions in the target harness.
2. **Compilation Filter:** Test functions are compiled in the sandbox. If compilation fails, discard.
3. **Suspect Filter:** Execute against the baseline. If a test fails on the baseline AND on all Gen 0 candidates, tag it `suspect` and exclude it from scoring term $A$.
4. **Scoring Invariant:** Surviving valid adversary tests contribute to term $A$ in score $S$ (e1-7).

### 2. Gate Isolation Invariant
Adversary tests are structurally excluded from hard-gate evaluation (e1-6). A candidate
can never fail a gate because of an adversary-authored test, and an adversary test can
never be counted in the Gate 2 trusted-test total or the Gate 4 baseline count.

### 3. Provenance Tagging
Every adversary test is tagged with its origin so downstream stages can distinguish it
from user-authored trusted tests. Untagged tests are rejected at ingest.
""",
        "scenarios": [
            ("Given the spec and current best candidate", "when the adversary role runs", "then it generates up to K candidate tests"),
            ("Given an adversary test does not compile", "when collected", "then it is discarded immediately"),
            ("Given an adversary test fails on all candidates and the baseline", "when collected", "then it is flagged `suspect` and excluded from scoring"),
            ("Given any adversary test fails", "when hard gates run on a candidate", "then it never causes that candidate to fail a gate"),
        ],
        "tests": [
            ("engine", "test_adversary_generation_quota", "Asserts the adversary produces at most `K` tests and each carries an origin tag."),
            ("engine", "test_syntax_error_discard", "Feeds an uncompilable test and asserts it is discarded before execution."),
            ("engine", "test_suspect_test_filtering", "Simulates a universally failing test and verifies the `suspect` tag and exclusion from $A$."),
            ("engine", "test_adversary_never_gates_candidate", "Fails an adversary test against a candidate and asserts the e1-6 `GateResult` is unchanged."),
        ],
        "implementation": {
            "crate": "engine",
            "creates": [
                "`crates/evoswarm-engine/src/adversary.rs` — `pub async fn generate(spec: &str, best: &Candidate, k: usize, deps: &AdversaryDeps) -> Vec<AdversaryTest>`.",
                "`crates/evoswarm-core/src/adversary.rs` — `AdversaryTest { name, body, origin: TestOrigin, status: AdversaryStatus }` with `AdversaryStatus::{Valid, Discarded, Suspect}`.",
                "`crates/evoswarm-core/src/provenance.rs` — `TestOrigin::{Trusted, Adversary}` so gates and scoring can filter by origin.",
                "`crates/evoswarm-engine/src/adversary.rs` — `pub fn filter(tests: Vec<AdversaryTest>, baseline: &Baseline) -> Vec<AdversaryTest>` applying compile and suspect filters.",
            ],
            "steps": [
                "Dispatch the adversary role with $K$ and reject any returned test lacking an origin tag.",
                "Compile each test in the sandbox via `SandboxBackend`, discarding failures with the diagnostic logged.",
                "Execute survivors against the baseline and all Gen 0 candidates; tag `suspect` when every run fails.",
                "Pass only `AdversaryStatus::Valid` tests into the e1-7 term $A$ denominator.",
                "Enforce gate isolation in `evoswarm-fitness`: `gates::evaluate` accepts a trusted-only slice, making adversary inclusion a type error rather than a runtime check.",
            ],
            "notes": "Filtering gates by the `TestOrigin` type is deliberate: a boolean flag on a shared list is one forgotten filter away from letting generated tests veto user code.",
        },
    },
    {
        "key": "e1-10-budget-enforcement",
        "title": "Budget enforcement",
        "persona": "Developer",
        "priority": "Must",
        "sizing": "M",
        "tier": "flash",
        "dependencies": ["e1-2-model-roles-in-config"],
        "jtbd": "When running evolutionary jobs, I want strict token, dollar, and generation early-stopping caps enforced before every model dispatch, so that long-running searches never exceed agreed costs.",
        "context": "Governed by AD-6. Checks projected costs against budget caps prior to network dispatch. Early stop terminates if no score improvement across 2 consecutive generations. Emits status `budget_exhausted` and returns the best verified candidate.",
        "out_of_scope": "Dynamic credit card charging or billing API integrations.",
        "contract_matrix": [
            ("Current spend + projected call cost vs dollar cap", "Dispatch proceeds, or loop aborts with `budget_exhausted`", "Best verified candidate returned with the reason"),
            ("Tokens used + requested `max_tokens` vs token cap", "Dispatch proceeds, or loop aborts", "Remaining token headroom reported"),
            ("Best score per generation history", "Continue, or stop with `plateau_early_stop`", "Partial result and generation count reported"),
            ("Cost fields from e1-2 config", "Projected cost per call", "Zero or negative costs rejected at config load"),
        ],
        "spec_content": r"""### 1. Budget Checks
Both checks run **before** dispatch; a call that would cross a cap is never sent.
- Dollar check: $\text{Spent} + \text{ProjectedCallCost} \le \text{DollarCap}$.
- Token check: $\text{TokensUsed} + \text{MaxTokens} \le \text{TokenCap}$.
- $\text{ProjectedCallCost}$ is derived from the role's `cost_per_million_input` / `cost_per_million_output` (e1-2) and the prompt token estimate (e1-4).
- If either check fails: halt generation, mark the job `budget_exhausted`, return the best verified candidate.

### 2. Early-Stop Invariant
- If $\max(S_{G}) \le \max(S_{G-1}) \le \max(S_{G-2})$, terminate the search with reason `plateau_early_stop`.
- Strictly-greater improvement is required to continue; an equal score counts as no improvement.
- Early stop never discards an already-verified winner.

### 3. Accounting Invariant
Spend is accumulated from provider-reported usage, and the projection is reconciled
against it after each call. Accounting is monotonic: recorded spend never decreases.
""",
        "scenarios": [
            ("Given a token and dollar cap", "when a model call is about to be sent", "then the projected cost is checked against the remaining budget before dispatch"),
            ("Given the cap would be exceeded", "when checked", "then the job halts as `budget_exhausted` and returns the best verified candidate so far"),
            ("Given no score improvement over 2 consecutive generations", "when a generation ends", "then the job stops early with reason `plateau_early_stop` and a partial result"),
            ("Given provider-reported usage exceeds the projection", "when reconciled", "then recorded spend rises to the actual value and never decreases"),
        ],
        "tests": [
            ("engine", "test_pre_call_cost_check_halt", "Asserts the loop halts before dispatch when the projected cost would cross the dollar cap."),
            ("engine", "test_token_cap_exhaustion", "Asserts status `budget_exhausted` when the token limit would be reached, with zero further calls dispatched."),
            ("engine", "test_early_stop_score_plateau", "Simulates 3 generations with non-improving best scores and verifies `plateau_early_stop`."),
            ("engine", "test_spend_reconciliation_is_monotonic", "Feeds actual usage above projection and asserts recorded spend rises and never decreases."),
        ],
        "implementation": {
            "crate": "engine",
            "creates": [
                "`crates/evoswarm-engine/src/budget.rs` — `pub struct BudgetGuard { spent_usd, tokens_used, caps, crossover_calls }`.",
                "`crates/evoswarm-engine/src/budget.rs` — `pub fn pre_dispatch(&self, req: &CallRequest) -> Result<(), BudgetExhausted>` and `pub fn record(&mut self, usage: Usage)`.",
                "`crates/evoswarm-engine/src/early_stop.rs` — `pub fn should_stop(history: &[f64]) -> Option<StopReason>` implementing the 3-generation plateau rule.",
                "`crates/evoswarm-core/src/usage.rs` — `Usage { tokens_in, tokens_out, cost_usd }` and `CallRequest { role, max_tokens, prompt_tokens }`.",
            ],
            "steps": [
                "Project cost from the role's per-million rates and the e1-4 token estimator; reject non-positive rates at config load.",
                "Call `pre_dispatch` at the single dispatch chokepoint in `evoswarm-models` so no role can bypass it.",
                "Record actual usage after each response with `record()`, taking `max(projected, actual)` so spend is monotonic.",
                "Implement `should_stop` over the per-generation best-score history, requiring strictly increasing scores to continue.",
                "On halt, transition the job to `BudgetExhausted` and return the best verified candidate from e1-8 selection.",
                "Persist counters to the e1-12 ledger after each call so a crash cannot resurrect spent budget.",
            ],
            "notes": "One chokepoint for `pre_dispatch` mirrors the e1-8 leak-scan decision: per-call-site budget checks are how a new role silently becomes unbudgeted.",
        },
    },
    {
        "key": "e1-11-patch-and-report",
        "title": "Patch and report",
        "persona": "Developer",
        "priority": "Must",
        "sizing": "M",
        "tier": "flash",
        "dependencies": ["e1-6-hard-gates", "e1-7-weighted-score"],
        "jtbd": "When a job finishes, I want the winning patch delivered on a clean git branch alongside a `.patch` file and audit report, so that I can review, inspect lineage, and merge with confidence.",
        "context": "Governed by AD-1 and AD-5. Creates branch `evoswarm/<job-id>` from the base commit, writes `.evoswarm/patches/<job-id>.patch`, and generates a markdown audit report. The base branch remains untouched.",
        "out_of_scope": "Auto-pushing branches to remote git hosts without user confirmation.",
        "contract_matrix": [
            ("Verified winner + base commit", "Branch `evoswarm/<job-id>` with one atomic commit", "Base branch and working tree left untouched"),
            ("Winning candidate diff", "`.evoswarm/patches/<job-id>.patch`", "`ReportError::PatchUnappliable` if it fails on a clean tree"),
            ("Job metrics and lineage", "`.evoswarm/reports/<job-id>.md`", "Missing section fails report generation"),
            ("No verified winner", "Best-effort patch plus report flagging the status", "Report states `no verified winner` explicitly"),
        ],
        "spec_content": r"""### 1. Artifact Outputs
1. **Git Branch:** `evoswarm/<job-id>` branched from the base commit with a single atomic commit.
2. **Patch File:** `.evoswarm/patches/<job-id>.patch`.
3. **Audit Report:** `.evoswarm/reports/<job-id>.md` detailing:
   - Score breakdown ($S$, $A$, $P$, $Z$) and the weights used
   - Visible and held-out test pass lists
   - Model calls, tokens, and dollar cost per role
   - Lineage tree
   - Proposed adversary candidate tests for review

### 2. Isolation Invariant
The user's checked-out branch and working tree are never modified. Branch creation happens
against the object store without switching the working tree, so a dirty checkout cannot
block or corrupt artifact emission.

### 3. Patch Integrity
The emitted `.patch` must apply cleanly to the base commit with `git apply --check`.
Integrity is verified before the job is reported complete; an unappliable patch is a job
failure, not a warning.

### 4. No- Winner Path
When selection returns `no verified winner`, the best-effort patch and report are still
emitted and the report states the outcome explicitly rather than implying success.
""",
        "scenarios": [
            ("Given a verified winner", "when the job completes", "then branch `evoswarm/<job-id>` is created from the base commit and the `.patch` file is emitted"),
            ("Given the emitted patch file", "when applied to a clean checkout of the base commit", "then it applies without conflict"),
            ("Given the markdown report", "when opened", "then it displays score breakdown, held-out passes, model calls, cost, lineage, and proposed tests"),
            ("Given any job", "when it completes", "then the base working tree and active branch are untouched"),
        ],
        "tests": [
            ("cli", "test_git_branch_emission", "Verifies branch creation from the base commit and asserts the active branch is unchanged."),
            ("cli", "test_patch_file_integrity", "Applies the emitted `.patch` to a clean checkout and asserts clean application."),
            ("cli", "test_markdown_report_sections", "Validates the presence and non-emptiness of score, cost, lineage and proposed-test sections."),
            ("cli", "test_working_tree_untouched", "Dirties the working tree before a job and asserts the same uncommitted changes survive completion."),
        ],
        "implementation": {
            "crate": "cli",
            "creates": [
                "`crates/evoswarm-cli/src/artifacts.rs` — `pub fn emit(job: &JobRecord, winner: &SelectionOutcome, repo: &RepoRoot) -> Result<Artifacts, ReportError>`.",
                "`crates/evoswarm-cli/src/git_writer.rs` — branch and patch emission built on `git2`, creating the branch from the base commit without checking it out.",
                "`crates/evoswarm-cli/src/report.rs` — `pub fn render_markdown(job: &JobRecord, winner: &SelectionOutcome) -> String`.",
                "`crates/evoswarm-core/src/report_model.rs` — `ScoreBreakdown`, `RoleUsage`, `LineageNode` view types.",
            ],
            "steps": [
                "Resolve and record the base commit at job start (e1-1) so artifact emission does not depend on HEAD at completion time.",
                "Create the branch from the base commit with `git2`, committing the winning patch as a single atomic commit; never call `checkout` on the user's working tree.",
                "Write the `.patch` and verify it with `git apply --check` against a temporary clean worktree before reporting success.",
                "Render the report from the view types, failing loudly when a section's data is absent rather than omitting the heading.",
                "Handle `NoVerifiedWinner` by emitting the best-effort patch and stating the outcome in the report header.",
                "Ensure no remote push occurs; the artifact set is local-only by design.",
            ],
            "notes": "Creating the branch without a checkout is the whole isolation guarantee: `git switch` would clobber uncommitted user work and is untestable to undo.",
        },
    },
    {
        "key": "e1-12-resume-after-crash",
        "title": "Resume after crash",
        "persona": "Operator",
        "priority": "Must",
        "sizing": "M",
        "tier": "pro",
        "dependencies": ["e1-1-start-a-job-from-the-cli"],
        "jtbd": "When the host box experiences an unexpected reboot or crash mid-search, I want running jobs to resume from their last completed generation without re-spending tokens, so that 24/7 autonomous operations are resilient.",
        "context": "Governed by AD-7 (SQLite Job Ledger). SQLite in WAL mode commits completed generations and model call responses with idempotency hashes. On reboot, incomplete sandbox runs are rescheduled; completed model calls are reused.",
        "out_of_scope": "Live process memory checkpointing via CRIU.",
        "contract_matrix": [
            ("Abrupt SIGKILL during generation $G$", "Daemon resumes at $G + 1$ with completed calls replayed from cache", "Partial generation $G$ state discarded, not merged"),
            ("Prompt hash present in `model_call_cache`", "Stored response returned, 0 API tokens spent", "Cache miss falls through to a real dispatch"),
            ("Sandbox run in flight at crash time", "Run re-executed cleanly from the start", "No partial output treated as a result"),
            ("Ledger interrupted mid-write", "WAL recovery yields the last committed state", "No torn generation row visible to resume"),
        ],
        "spec_content": r"""### 1. SQLite Ledger Schema
```sql
CREATE TABLE IF NOT EXISTS job_generations (
    job_id TEXT NOT NULL,
    generation INTEGER NOT NULL,
    population_json TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY(job_id, generation)
);

CREATE TABLE IF NOT EXISTS model_call_cache (
    idempotency_hash TEXT PRIMARY KEY,
    model_id TEXT NOT NULL,
    response_text TEXT NOT NULL,
    tokens_in INTEGER NOT NULL,
    tokens_out INTEGER NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### 2. Recovery Protocol
- On service restart: scan the `jobs` table for `Running` state.
- Locate the highest completed generation in `job_generations`.
- Resume the evolutionary loop from generation $G + 1$.
- Any model call whose idempotency hash exists in `model_call_cache` is read from disk with 0 API tokens spent.
- Sandbox runs are never cached: an interrupted run is re-executed from the start, because a partial execution result is indistinguishable from a failure.

### 3. Idempotency Key
The hash covers role, model ID, and the exact prompt bytes (static prefix plus dynamic
suffix). A changed prompt is a different key and correctly misses the cache.

### 4. Commit Discipline
A generation row is committed only after every candidate in it has been evaluated and
scored. Partial generations are never written, so resume cannot inherit half-scored
population state.
""",
        "scenarios": [
            ("Given the process is killed mid-generation", "when the service restarts", "then the job resumes from the last completed generation"),
            ("Given a model call completed before the crash", "when resuming", "then it is not repeated, using its stored idempotency key"),
            ("Given sandbox runs were in flight during the crash", "when resuming", "then they are re-run cleanly rather than read from cache"),
            ("Given the prompt bytes change after a crash", "when the call is replayed", "then the cache misses and a fresh dispatch occurs"),
        ],
        "tests": [
            ("engine", "test_resume_from_last_generation", "Simulates a crash during Gen 2 and asserts resume starts at Gen 3."),
            ("engine", "test_model_call_idempotency_cache", "Verifies the cached response is reused on replay with zero network calls."),
            ("engine", "test_interrupted_sandbox_rerun", "Verifies an in-flight sandbox run is re-evaluated on restart rather than cached."),
            ("engine", "test_changed_prompt_invalidates_cache", "Alters one suffix byte and asserts a cache miss triggers a fresh dispatch."),
        ],
        "implementation": {
            "crate": "ledger",
            "creates": [
                "`crates/evoswarm-ledger/src/generations.rs` — `pub fn commit_generation(job_id, generation, population, status)` in a single transaction.",
                "`crates/evoswarm-ledger/src/cache.rs` — `pub fn lookup(hash: &[u8; 32]) -> Option<CachedCall>` and `pub fn store(call: CachedCall)`.",
                "`crates/evoswarm-models/src/idempotency.rs` — `pub fn call_hash(role: Role, model_id: &str, prompt: &[u8]) -> [u8; 32]`.",
                "`crates/evoswarm-engine/src/recovery.rs` — `pub async fn recover(ledger: &JobLedger) -> Vec<ResumableJob>` and the resume entry point in the search loop.",
            ],
            "steps": [
                "Enable WAL and `synchronous=FULL` on the ledger connection; durability here is the whole feature.",
                "Wrap `commit_generation` in one transaction so a crash cannot expose a partial population.",
                "Compute `call_hash` over role, model ID and exact prompt bytes; consult the cache before dispatch and store after.",
                "Never route `SandboxBackend::run` through the cache; keep sandbox execution explicitly uncached in the dispatch path.",
                "On startup, scan for `Running` jobs, read the max committed generation, and re-enter the loop at $G + 1$.",
                "Reconcile e1-10 spend counters from `model_call_cache` so resumed jobs cannot re-spend already-recorded budget.",
            ],
            "notes": "Caching sandbox results would be the tempting optimisation and the wrong one: a killed run's partial output looks exactly like a genuine failure and would poison scoring.",
        },
    },
    {
        "key": "e1-13-benchmark-suite",
        "title": "Benchmark suite",
        "persona": "Team Lead",
        "priority": "Must",
        "sizing": "L",
        "tier": "pro",
        "dependencies": ["e1-11-patch-and-report"],
        "jtbd": "When evaluating whether evolutionary code search pays off, I want a standardized 30-task benchmark comparing EvoSwarm against single-shot generation at equal token budget, so that investment is justified by empirical data.",
        "context": "Mandatory Exit Gate (Gate 1). The benchmark consists of 30 tasks (at least 10 Python, 10 C#) with trusted tests and held-out slices. EvoSwarm must achieve a solve rate $\\ge 15$ points higher than single-shot with test feedback.",
        "out_of_scope": "Continuous benchmarking on every minor commit.",
        "contract_matrix": [
            ("30 benchmark task directories", "Loaded suite with trusted tests and held-out split per task", "Load error naming any task missing tests or a split"),
            ("Equal token cap $T = 500{,}000$ per task", "Both runners terminate at the same budget", "Run invalidated if caps diverge"),
            ("Completed runs for both methods", "Solve rate delta, tokens per solved task, wall time", "Gate fails when $\\Delta < 15.0\\%$"),
            ("Single-shot-with-feedback runner", "1 draft plus sequential retries within budget", "Retry loop must not exceed the cap"),
        ],
        "spec_content": r"""### 1. Benchmark Harness Contract
- 30 tasks with pre-verified failing baselines and human-verified test suites.
- Language mix: at least 10 Python and 10 C# tasks; the remainder may be either.
- Runner executes both methods at equal token caps ($T = 500{,}000$ tokens per task):
  - **Single-shot with feedback:** 1 draft plus sequential retries up to the token budget.
  - **EvoSwarm:** the population evolutionary loop.
- Metric: $\Delta = \text{SolveRate}_{\text{EvoSwarm}} - \text{SolveRate}_{\text{SingleShot}}$.
- Exit gate requires: $\Delta \ge 15.0\%$.

### 2. Fairness Invariants
- Identical token cap, identical model roles and pricing, identical sandbox profile, and
  identical held-out verification for both methods.
- A task counts as solved only when the held-out slice passes 100%, matching e1-8.
- Neither method may see held-out tests; the e1-8 leak scan applies to the single-shot
  runner as well.

### 3. Reporting
The report prints per-task solve status, aggregate solve rate for both methods, the delta,
tokens per solved task, and wall time. Raw per-task results are written to JSON so the
gate decision is auditable after the fact.
""",
        "scenarios": [
            ("Given the benchmark suite", "when loaded", "then it contains 30 tasks, at least 10 Python and 10 C#, each with trusted tests and a held-out split"),
            ("Given the comparison script", "when run", "then EvoSwarm and single-shot-with-feedback execute at an equal token budget"),
            ("Given benchmark execution completes", "when results print", "then they show solve rate, tokens per solved task, and wall time"),
            ("Given a delta below the exit gate", "when results are evaluated", "then the gate is reported as failed with the delta shown"),
        ],
        "tests": [
            ("bench", "test_benchmark_task_integrity", "Validates all 30 tasks have trusted tests, a held-out split and a failing baseline, with at least 10 Python and 10 C#."),
            ("bench", "test_token_budget_equality", "Verifies both runners terminate at identical token budgets and share model and sandbox configuration."),
            ("bench", "test_solve_rate_delta_calculation", "Validates delta calculation from known solve counts and the gate assertion logic at the 15% boundary."),
            ("bench", "test_gate_failure_below_threshold", "Feeds a delta below 15% and asserts the harness reports failure with the delta printed."),
        ],
        "implementation": {
            "crate": "bench",
            "creates": [
                "`bench/tasks/<task-id>/` — 30 task directories, each with `task.md`, `baseline/`, `tests/trusted/` and `tests/held_out/`.",
                "`bench/harness.py` — suite loader validating task integrity and the language mix.",
                "`bench/run_single_shot.py` — single-shot-with-feedback runner honouring the shared token cap.",
                "`bench/run_evoswarm.py` — evolutionary runner invoking the `evoswarm` CLI.",
                "`bench/compare.py` — delta computation, gate assertion, and JSON plus console reporting.",
            ],
            "steps": [
                "Author the loader first and let `test_benchmark_task_integrity` drive the required task directory layout.",
                "Extract the shared budget, model and sandbox configuration into one module both runners import, so equality is structural rather than a runtime assertion.",
                "Implement the single-shot runner as one draft plus sequential retries that stop at the token cap, reusing the e1-8 leak scan.",
                "Implement `compare.py` to count a solve only on a 100% held-out pass, compute $\\Delta$, and assert $\\Delta \\ge 15.0$.",
                "Emit raw per-task JSON before printing the summary so a failed gate remains auditable.",
                "Seed at least 10 Python and 10 C# tasks; task authoring is the bulk of this story's size.",
            ],
            "notes": "This is the Gate 1 decision point for the whole project: if the delta does not reach 15 points, the epic exit gate halts swarm search in favour of a sandboxed test runner.",
        },
    },
]


# Test target names are crate-scoped Cargo integration targets, so
# `cargo test -p <crate> --test <target>` resolves to a real compiled binary.
def test_target(story_key: str) -> str:
    return story_key.replace("-", "_")


def test_rel_path(crate_key: str, story_key: str) -> str:
    return f"crates/{CRATE_OF[crate_key]}/tests/{test_target(story_key)}.rs"


def bench_rel_path(story_key: str) -> str:
    return f"bench/tests/test_{test_target(story_key)}.py"


def gate_command(story: dict) -> str:
    crates = {t[0] for t in story["tests"]}
    if crates == {"bench"}:
        return f"python3 -m pytest {bench_rel_path(story['key'])} -v"
    parts = []
    for crate_key in sorted(crates):
        crate = CRATE_OF[crate_key]
        parts.append(f"cargo test -p {crate} --test {test_target(story['key'])}")
    return " && ".join(parts)


def test_location(story: dict, test: tuple) -> str:
    crate_key, name, _desc = test
    if crate_key == "bench":
        return f"{bench_rel_path(story['key'])}::{name}"
    return f"{test_rel_path(crate_key, story['key'])}::{name}"


def target_files(story: dict) -> list:
    """Crate-scoped test target paths for a story.

    Epic 1 has no shared top-level tests/ directory: each story owns exactly one Cargo
    integration target (or one pytest module for the benchmark harness), which is what
    makes `cargo test -p <crate> --test <target>` resolvable.
    """
    crates = {t[0] for t in story["tests"]}
    if crates == {"bench"}:
        return [bench_rel_path(story["key"])]
    return [test_rel_path(c, story["key"]) for c in sorted(crates)]


def rel_link(from_dir: Path, target: str) -> str:
    return os.path.relpath(REPO_ROOT / target, from_dir).replace(os.sep, "/")


def read_ledger_status(key: str) -> str:
    """Reads a story's status by targeted line lookup.

    lessons-learned section 3 forbids round-tripping sprint-status.yaml through a
    serialiser because that strips its comments and reorders sections.
    """
    ledger = REPO_ROOT / "sprint-status.yaml"
    if not ledger.exists():
        return "unknown"
    for line in ledger.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith(f"{key}:"):
            return stripped.split(":", 1)[1].strip() or "unknown"
    return "unknown"


def render_intent(story: dict, story_dir: Path) -> str:
    epic_link = rel_link(story_dir, "docs/epics/epic-1-evolve-cli-and-fitness.md")
    deps = ", ".join(f"`{d}`" for d in story["dependencies"]) if story["dependencies"] else "None"
    return f"""# Story Intent: {story['title']}

## 1. Metadata
- **Story Key:** `{story['key']}`
- **Epic:** [Epic 1: Evolve CLI and Fitness]({epic_link})
- **Persona:** {story['persona']}
- **Priority:** {story['priority']}
- **Sizing:** {story['sizing']}
- **Execution Tier:** `{story['tier']}`
- **Dependencies:** {deps}
- **Ledger Status:** `{read_ledger_status(story['key'])}` (source of truth: [`sprint-status.yaml`]({rel_link(story_dir, 'sprint-status.yaml')}))

---

## 2. Job-to-be-Done (JTBD)
> {story['jtbd']}

---

## 3. Problem Statement & Architectural Context
{story['context']}

---

## 4. Scope Discipline & Boundary
- **In Scope:** The contracts in [`spec.md`](spec.md) and the work order in [`plan.md`](plan.md); nothing beyond them.
- **Out of Scope:** {story['out_of_scope']}
"""


def render_spec(story: dict) -> str:
    scenarios_text = "\n".join(
        f"- **{given}**, **{when}**, **{then}**." for given, when, then in story["scenarios"]
    )
    matrix_rows = "\n".join(f"| {i} | {o} | {e} |" for i, o, e in story["contract_matrix"])
    return f"""# Story Specification: {story['title']}

## 1. Functional Specification
{story['spec_content']}
---

## 2. Input / Output & Contract Matrix

| Input Contract | Expected Output | Error Handling & Diagnostics |
|---|---|---|
{matrix_rows}

---

## 3. Acceptance Criteria (Gherkin Scenarios)
{scenarios_text}
"""


def render_plan(story: dict, story_dir: Path) -> str:
    arch_link = rel_link(story_dir, ".agents/rules/architecture-rules.md")
    tdd_link = rel_link(story_dir, ".agents/rules/tdd-discipline.md")
    sec_link = rel_link(story_dir, ".agents/rules/security-hygiene.md")
    spine_link = rel_link(story_dir, "docs/architecture/ARCHITECTURE-SPINE.md")
    impl = story["implementation"]
    crates = {t[0] for t in story["tests"]}
    is_bench = crates == {"bench"}
    crate = "bench (Python harness)" if is_bench else CRATE_OF[impl["crate"]]

    red_rows = []
    for (given, when, then), test in zip(story["scenarios"], story["tests"]):
        location = test_location(story, test)
        red_rows.append(f"- `{location}` — {test[2]}")
    red_text = "\n".join(red_rows)

    trace_rows = []
    for idx, ((given, when, then), test) in enumerate(zip(story["scenarios"], story["tests"]), start=1):
        criterion = then[5:] if then.startswith("then ") else then
        trace_rows.append(f"| AC{idx} | {criterion} | `{test[1]}` |")
    trace_text = "\n".join(trace_rows)

    targets_text = ", ".join(f"`{p}`" for p in target_files(story))
    if is_bench:
        targets_note = (
            "All tests for this story live in the single pytest module above. It does not exist "
            "yet: author it in the Red phase before any production code."
        )
        regression_cmd = "python3 -m pytest bench/tests -v"
        refactor_first = "Remove redundant work and any temporary scaffolding introduced to reach green."
        regression_scope = "Re-run the whole benchmark test suite, not just this story's module, to catch regressions."
    else:
        targets_note = (
            "All tests for this story live in the single crate-scoped target above. It does not "
            "exist yet: author it in the Red phase before any production code. Tests must sit at "
            "`crates/<crate>/tests/<target>.rs`, never in a subdirectory: Cargo treats a nested "
            "path as a helper module and never compiles it as a `--test` target."
        )
        regression_cmd = f"cargo test -p {CRATE_OF[impl['crate']]}"
        refactor_first = "Remove redundant allocations and any temporary scaffolding introduced to reach green."
        regression_scope = f"Re-run the full `{CRATE_OF[impl['crate']]}` suite, not just this story's target, to catch regressions."
    creates_text = "\n".join(f"- {c}" for c in impl["creates"])
    steps_text = "\n".join(f"{i}. {s}" for i, s in enumerate(impl["steps"], start=1))
    deps = ", ".join(f"`{d}`" for d in story["dependencies"]) if story["dependencies"] else "None"

    return f"""# Implementation Plan: {story['title']}

**Story:** `{story['key']}` · **Sizing:** {story['sizing']} · **Tier:** `{story['tier']}` · **Target crate:** `{crate}`
**Depends on:** {deps} — confirm each is `done` in [`sprint-status.yaml`]({rel_link(story_dir, 'sprint-status.yaml')}) before starting.

## 1. Pre-Flight Architecture & Rule Check
Complete these before writing code; tick each only when it actually holds.

- [ ] Reviewed [`ARCHITECTURE-SPINE.md`]({spine_link}) and confirmed this story upholds its invariants: {story['context'].split('.')[0]}.
- [ ] Confirmed compliance with [`architecture-rules.md`]({arch_link}), [`tdd-discipline.md`]({tdd_link}) and [`security-hygiene.md`]({sec_link}).
- [ ] Confirmed every dependency listed above is `done`; otherwise stop and report the blocker.
- [ ] Confirmed scope matches the contract matrix and acceptance criteria in [`spec.md`](spec.md) and nothing outside them is being built.
- [ ] Committed to zero AI comments in production code: comments explain **WHY**, never **WHAT**.

---

## 2. Red Phase (Failing Tests to Author First)
Author these tests before any production code, then run the gate command in section 5 and
confirm every test fails for its intended reason. A test that passes before implementation
is a false-positive test and must be rewritten.

{red_text}

**Target file(s):** {targets_text}
{targets_note}

### Acceptance Criteria ↔ Test Traceability
Every acceptance criterion in [`spec.md`](spec.md) maps to exactly one named test. Adding or
changing a criterion requires the matching test to change in the same commit.

| AC | Criterion | Test |
|---|---|---|
{trace_text}

---

## 3. Green Phase (Minimal Production Code)
Create only what the tests above require. Work in this order so each step is independently testable.

### Files to create
{creates_text}

### Work order
{steps_text}

### Implementation note
{impl['notes']}

---

## 4. Refactor Phase
- {refactor_first}
- Verify no production comment describes **WHAT** the code does; keep only **WHY** comments.
- {regression_scope}
- Confirm the diff touches no test assertions (enforced by `--anti-cheat` below).

---

## 5. Verification Gate
```bash
python3 scripts/sprint.py verify --cmd "{gate_command(story)}" --anti-cheat
```
Then advance the ledger:
```bash
python3 scripts/sprint.py update {story['key']} --status done
```
"""


def main() -> int:
    stories_root = REPO_ROOT / "docs" / "stories"
    stories_root.mkdir(parents=True, exist_ok=True)

    print(f"Planning {len(EPIC_1_STORIES)} stories in Epic 1 into {stories_root} ...")
    problems = []
    for s in EPIC_1_STORIES:
        if len(s["scenarios"]) != len(s["tests"]):
            problems.append(
                f"{s['key']}: {len(s['scenarios'])} acceptance criteria but {len(s['tests'])} tests"
            )
        story_dir = stories_root / s["key"]
        story_dir.mkdir(parents=True, exist_ok=True)

        (story_dir / "intent.md").write_text(render_intent(s, story_dir), encoding="utf-8")
        (story_dir / "spec.md").write_text(render_spec(s), encoding="utf-8")
        (story_dir / "plan.md").write_text(render_plan(s, story_dir), encoding="utf-8")

        print(f"  wrote artifact chain {s['key']}/ (intent.md, spec.md, plan.md)")

    if problems:
        print("\nAC-to-test traceability violations:", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1

    print("\nAll Epic 1 stories planned successfully.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
