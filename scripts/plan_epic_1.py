#!/usr/bin/env python3
"""Plans all 13 stories in Epic 1 (Evolve CLI and Fitness) by creating the three-artifact chain:
intent.md, spec.md, plan.md under docs/stories/<story_key>/
"""

import os
from pathlib import Path

EPIC_1_STORIES = [
    {
        "key": "e1-1-start-a-job-from-the-cli",
        "title": "Start a job from the CLI",
        "persona": "Developer",
        "jtbd": "When I have a task backed by failing or benchmark tests, I want to submit it via `evoswarm run` in a single command, so that EvoSwarm validates the task baseline and searches for a verified passing patch in the background.",
        "context": "Governed by AD-7 (SQLite Job Ledger) and AD-1 (Sandbox Isolation). The CLI must validate inputs, verify baseline reproducibility across 3 runs to prevent flaky tests, check that tests aren't already passing without a perf objective, and register the job in SQLite.",
        "out_of_scope": "Interactive TUI dashboards, distributed cluster orchestration, or multi-repo workspaces.",
        "spec_content": """### 1. Command-Line Interface Contract
```bash
evoswarm run \\
  --task "<task description string>" \\
  --cmd "<test execution command>" \\
  --paths "<relative path 1>,<relative path 2>" \\
  [--budget-tokens <tokens>] \\
  [--budget-dollars <usd>] \\
  [--objective <correctness|perf>] \\
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
4. **Fast Job Ticket:** Outputs `{ "job_id": "<uuid>", "status": "queued" }` within $< 2$ seconds.
""",
        "scenarios": [
            ("Given valid task, tests, paths and budget", "when I run `evoswarm run`", "then a job ID prints within 2 s and the job status is `queued` in SQLite."),
            ("Given the test command cannot execute on the baseline", "when I submit", "then the job is rejected with the command's non-zero output and exit code shown."),
            ("Given baseline results differ across 3 consecutive runs", "when I submit", "then the job is rejected and the flaky tests are listed."),
            ("Given every baseline test already passes and no performance objective is set", "when I submit", "then I am told there is nothing to improve and asked for `--objective perf`.")
        ],
        "test_cases": [
            ("tests/cli/test_run.rs::test_submit_valid_job_fast_return", "Submits valid job and verifies JSON ticket emitted in <2.0s with SQLite job state 'queued'."),
            ("tests/cli/test_run.rs::test_reject_broken_baseline_command", "Supplies invalid binary in test command and verifies immediate rejection with stderr diagnostic."),
            ("tests/cli/test_run.rs::test_reject_flaky_baseline_suite", "Mocks a non-deterministic test runner and asserts rejection with list of flaky tests."),
            ("tests/cli/test_run.rs::test_reject_already_passing_suite_without_perf_flag", "Supplies clean passing suite without --objective perf and verifies guidance error.")
        ]
    },
    {
        "key": "e1-2-model-roles-in-config",
        "title": "Model roles in config",
        "persona": "Operator",
        "jtbd": "When operating EvoSwarm on changing budget or API tier conditions, I want to map distinct model roles (`mutator`, `synthesiser`, `adversary`) in a TOML config file with hot-reload, so that I can optimize token costs without restarting the daemon or recompiling.",
        "context": "Governed by AD-6 (Model Role Segregation). EvoSwarm allocates 75% of calls to fast/cheap mutators and reserves reasoning models for crossover and adversary generation. Dynamic SIGHUP reload prevents interrupting running jobs.",
        "out_of_scope": "Automated bidding on spot LLM auctions, dynamically switching cloud providers mid-generation.",
        "spec_content": """### 1. Configuration Schema (`config.toml`)
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

### 2. Hot-Reload via SIGHUP
- Upon receiving `SIGHUP`, the daemon parses `config.toml`.
- If valid, the runtime atomic reference `ArcSwap<ModelConfig>` is swapped immediately.
- If invalid, the error is logged to stderr, the existing configuration remains active, and the daemon stays healthy.
""",
        "scenarios": [
            ("Given config maps `mutator`, `synthesiser` and `adversary` to model ID, max tokens and temperature", "when a job runs", "then each dispatch uses its configured role settings."),
            ("Given an unknown or malformed model ID", "when the service starts", "then it fails loudly with the offending role named."),
            ("Given I edit config.toml and send SIGHUP", "when the next generation starts", "then the new settings apply seamlessly without daemon restart.")
        ],
        "test_cases": [
            ("tests/config/test_model_config.rs::test_parse_valid_role_configuration", "Verifies all roles deserialized with correct token caps and temperatures."),
            ("tests/config/test_model_config.rs::test_startup_failure_on_invalid_role", "Asserts fatal error when mandatory role is omitted or model ID is empty."),
            ("tests/config/test_model_config.rs::test_sighup_atomic_swap", "Sends SIGHUP with updated temperature and verifies subsequent query returns new value.")
        ]
    },
    {
        "key": "e1-3-seed-the-first-generation",
        "title": "Seed the first generation",
        "persona": "Developer",
        "jtbd": "When starting a search, I want Generation 0 populated with diverse initial attempts alongside the current code baseline, so that the evolutionary loop explores multiple distinct conceptual paths.",
        "context": "Governed by AD-5 and AD-6. Population size N (default 6) is seeded by: (1) baseline code, (2) retrieved memory winners if available (up to 2), and (3) N-1 (or N-3) fresh drafts from the mutator model.",
        "out_of_scope": "Synthesising candidates using third-party web search or unverified external snippets.",
        "spec_content": """### 1. Seeding Algorithm
1. Candidate 0 is initialized from current baseline files.
2. If similar past winners exist in FalkorDB (Epic 2), inject up to 2 past winning patches.
3. Mutator role is invoked concurrently with different random seeds to generate remaining drafts up to $N$.
4. **Deduplication:** Compute SHA-256 of candidate diffs. If two drafts produce identical diffs, drop the duplicate and invoke mutator again with temperature jitter ($+0.1$).
5. Each draft records `model_id`, `prompt_hash`, `generation = 0`, and `parent_ids = []`.
""",
        "scenarios": [
            ("Given population size N", "when generation 0 is built", "then it holds the current code plus N-1 fresh drafts."),
            ("Given each draft", "when it is created", "then it records model ID, prompt hash, and an empty parent list."),
            ("Given two drafts produce identical content hashes", "when generation 0 is finalised", "then the duplicate is dropped and replaced with a unique draft.")
        ],
        "test_cases": [
            ("tests/engine/test_generation_seeding.rs::test_gen0_population_quota", "Verifies exact N candidates in Gen 0."),
            ("tests/engine/test_generation_seeding.rs::test_candidate_metadata_recording", "Verifies generation=0, empty parent IDs, and valid prompt hash."),
            ("tests/engine/test_generation_seeding.rs::test_diff_hash_deduplication", "Injects mock duplicate draft and confirms replacement.")
        ]
    },
    {
        "key": "e1-4-mutation-with-error-feedback",
        "title": "Mutation with error feedback",
        "persona": "Developer",
        "jtbd": "When a candidate fails compilation or tests, I want the mutator model provided with the exact compiler errors and failing assertions, so that subsequent mutations converge on passing solutions rather than guessing randomly.",
        "context": "Governed by AD-5. Failed candidates contain high-value signal. The compiler output and first failing test assertion (truncated to 4,000 tokens) are packaged into a structured prompt. Prompt caching prefixes are strictly preserved.",
        "out_of_scope": "Multi-turn conversational debates with the model during a single mutation.",
        "spec_content": """### 1. Feedback Truncation Invariant
- Compiler errors: captured up to first 2,000 tokens.
- Test failures: first failing assertion, test name, and stack trace captured up to 2,000 tokens.
- Total error context hard-clamped to 4,000 tokens.

### 2. Prompt Caching Structure
- **Static Prefix (Cached):** System instructions + repository overview + API contracts + immutable test rules.
- **Dynamic Suffix:** Current parent diff + sandbox execution failure diagnostics.
""",
        "scenarios": [
            ("Given a failed parent candidate", "when its child is drafted", "then the prompt includes the first failing assertion and error text trimmed to 4k tokens."),
            ("Given any child candidate", "when it is stored", "then it is linked to its parent with `MUTATED_FROM` edge."),
            ("Given repeated mutation calls in one job", "when they are sent", "then repository context sits in a stable prefix and prompt cache hit rate is logged.")
        ],
        "test_cases": [
            ("tests/engine/test_mutation_feedback.rs::test_feedback_prompt_formatting", "Verifies truncation of large error dumps to 4k tokens."),
            ("tests/engine/test_mutation_feedback.rs::test_mutated_from_lineage_edge", "Asserts MUTATED_FROM relationship is correctly attached to child node."),
            ("tests/engine/test_mutation_feedback.rs::test_cache_prefix_byte_identity", "Asserts byte-identical static prompt prefix across sequential mutations.")
        ]
    },
    {
        "key": "e1-5-crossover-of-two-parents",
        "title": "Crossover of two parents",
        "persona": "Developer",
        "jtbd": "When two candidates pass different subsets of the test suite, I want a synthesiser model to recombine their complementary strengths, so that partial fixes can be merged into a comprehensive solution.",
        "context": "Governed by AD-6. Crossover pairs are drawn preferentially from candidates that pass disjoint test partitions. Crossover calls use the reasoning synthesiser model and are capped at 25% of total job calls.",
        "out_of_scope": "Synthesising candidates without test provenance data.",
        "spec_content": """### 1. Parent Selection for Recombination
- Candidate pairs $(P_A, P_B)$ are scored by Hamming distance of test pass vectors:
  $$D(P_A, P_B) = |(Pass_A \setminus Pass_B) \cup (Pass_B \setminus Pass_A)|$$
- Pairs with highest $D > 0$ are scheduled for crossover.

### 2. Synthesiser Dispatch & Lineage
- Prompt presents Parent A diff and tests it passes, Parent B diff and tests it passes.
- Synthesiser emits unified patch and summary of merged traits.
- Graph edge: `(Child)-[:MERGED_FROM {traits: "..."}]->(Parent_A)` and `(Child)-[:MERGED_FROM {traits: "..."}]->(Parent_B)`.
- Job call budget enforces crossover calls $\le 0.25 \times \text{TotalCalls}$.
""",
        "scenarios": [
            ("Given two parents pass different subsets of tests", "when parents are selected for crossover", "then such pairs are prioritized."),
            ("Given a crossover child is generated", "when it is stored", "then it links to both parents with the traits reported by the synthesiser model."),
            ("Given default budget settings", "when a job runs", "then crossover calls constitute at most 25% of total model calls.")
        ],
        "test_cases": [
            ("tests/engine/test_crossover.rs::test_disjoint_test_pair_selection", "Asserts selection logic prefers pairs with complementary test passes."),
            ("tests/engine/test_crossover.rs::test_merged_from_dual_lineage", "Verifies child connects to both parents with merged traits metadata."),
            ("tests/engine/test_crossover.rs::test_crossover_call_cap_budget", "Simulates generation dispatch and asserts crossover <= 25% of calls.")
        ]
    },
    {
        "key": "e1-6-hard-gates",
        "title": "Hard gates",
        "persona": "Reviewer",
        "jtbd": "When evaluating any candidate code patch, I want 4 non-negotiable hard gates enforced, so that no candidate that games tests or tampers with harnesses can ever be scored or returned.",
        "context": "Governed by AD-5. Gating is evaluated prior to scoring: (1) Clean build, (2) Visible trusted tests pass, (3) Zero diffs on tests/harnesses, (4) Zero skips/deletions relative to baseline. Any failure sets score $S = 0$.",
        "out_of_scope": "Subjective aesthetic code formatting gating.",
        "spec_content": """### 1. Gate Definitions & Priority Order
1. **Gate 1 (Build):** Exit code of build step == 0. Failure reason: `build`.
2. **Gate 2 (Trusted Pass):** Count of passed visible trusted tests == Total visible trusted tests. Failure reason: `test_failure`.
3. **Gate 3 (Tamper):** Diff touches only paths in `--paths`. Any touch of `tests/`, `conftest.py`, `Directory.Build.props`, or build files fails immediately with reason: `tamper`.
4. **Gate 4 (No Skips):** Count of executed tests $\ge$ baseline count; zero newly skipped/ignored tests. Failure reason: `skipped`.

### 2. Failure Outcome
- Composite score $S = 0.0$.
- Candidate excluded from winning selection.
- Detailed failure reason logged for mutator feedback.
""",
        "scenarios": [
            ("Given a candidate fails to compile", "when evaluated", "then its score is 0 and status is marked `build`."),
            ("Given any visible trusted test fails", "when evaluated", "then its score is 0 and failing test names are recorded."),
            ("Given the diff touches tests, harness files or build scripts", "when evaluated", "then its score is 0 and status is marked `tamper`."),
            ("Given fewer tests ran than in the baseline or new skips appear", "when evaluated", "then its score is 0 and status is marked `skipped`.")
        ],
        "test_cases": [
            ("tests/fitness/test_hard_gates.rs::test_build_error_gate", "Simulates compiler failure; asserts score 0 and reason 'build'."),
            ("tests/fitness/test_hard_gates.rs::test_assertion_failure_gate", "Simulates test assertion failure; asserts score 0 and list of failed tests."),
            ("tests/fitness/test_hard_gates.rs::test_harness_tamper_gate", "Injects conftest.py in patch; asserts score 0 and reason 'tamper'."),
            ("tests/fitness/test_hard_gates.rs::test_skipped_test_detection", "Skips one test in runner; asserts score 0 and reason 'skipped'.")
        ]
    },
    {
        "key": "e1-7-weighted-score",
        "title": "Weighted score",
        "persona": "Developer",
        "jtbd": "When multiple candidates pass all hard gates, I want them ranked by a deterministic, multi-objective score function, so that the highest quality, highest performance, and most concise patch wins.",
        "context": "Governed by AD-5. Calculates $S = w_a A + w_p P + w_s Z$ with default weights $w_a = 0.5, w_p = 0.3, w_s = 0.2$. Weights must sum to 1.0. If adversary tests are absent, $w_a$ is redistributed proportionally.",
        "out_of_scope": "Dynamic weight rebalancing during a running generation.",
        "spec_content": """### 1. Scoring Terms
- $A \in [0.0, 1.0]$: Pass rate on candidate adversary tests: $\frac{\text{Passed Adversary Tests}}{\text{Total Valid Adversary Tests}}$.
- $P \in [0.0, 1.0]$: Relative runtime score: $\text{clamp}\left(\frac{\text{Baseline Runtime}}{\text{Candidate Runtime}}, 0.0, 1.0\right)$.
- $Z \in [0.0, 1.0]$: Size parsimony score: $e^{-\frac{\text{diff\_lines}}{100}}$.

### 2. Redistribution Rule (No Adversary Tests)
When adversary tests are absent:
$$w'_p = \frac{w_p}{w_p + w_s} = \frac{0.3}{0.5} = 0.6$$
$$w'_s = \frac{w_s}{w_p + w_s} = \frac{0.2}{0.5} = 0.4$$
$$S = w'_p \cdot P + w'_s \cdot Z$$
""",
        "scenarios": [
            ("Given a candidate passes all gates", "when scored", "then S is computed using adversary pass rate, runtime vs baseline, and diff size."),
            ("Given weights that do not sum to 1.0 in config", "when the service starts", "then it fails with an explicit configuration error."),
            ("Given no adversary tests exist", "when scoring", "then adversary weight is redistributed proportionally to runtime and diff size terms."),
            ("Given identical inputs", "when scored twice", "then the returned score is strictly identical.")
        ],
        "test_cases": [
            ("tests/fitness/test_scoring.rs::test_exact_weighted_score_math", "Validates exact floating point computation for known test vector."),
            ("tests/fitness/test_scoring.rs::test_weight_validation_at_startup", "Rejects config with weights summing to 1.05."),
            ("tests/fitness/test_scoring.rs::test_proportional_redistribution_without_adversary", "Checks score calculation when adversary tests = 0."),
            ("tests/fitness/test_scoring.rs::test_deterministic_scoring", "Scores identical candidate 1,000 times and checks for bitwise identical float.")
        ]
    },
    {
        "key": "e1-8-held-out-tests",
        "title": "Held-out tests",
        "persona": "Reviewer",
        "jtbd": "When selecting a winning implementation, I want it evaluated against a hidden 20% slice of trusted tests that models never saw, so that overfitting and prompt memorization are prevented.",
        "context": "Governed by AD-5. The user's trusted test suite is partitioned 80/20 at job start using a stable hash seed. Held-out tests are never provided in mutation prompts. The top-scoring candidate must pass 100% of held-out tests.",
        "out_of_scope": "Generating artificial held-out tests with LLMs.",
        "spec_content": """### 1. Partitioning Invariant
- Total trusted tests $M$. If $M \ge 5$, held-out count $H = \max(1, \lfloor 0.20 \times M \rfloor)$.
- Test indices partitioned using SHA-256(JobId + TestName).
- Visible tests: $M - H$. Held-out tests: $H$.

### 2. Selection Loop
1. Rank candidates passing hard gates by score $S$ descending.
2. For top candidate, run held-out tests in sandbox.
3. If 100% pass $\rightarrow$ candidate is **Verified Winner**.
4. If any held-out test fails $\rightarrow$ evaluate next candidate.
5. If all candidates fail $\rightarrow$ emit status `no verified winner` and flag best-effort patch.
""",
        "scenarios": [
            ("Given a job starts", "when tests are partitioned", "then about 20% are held out using a stable split keyed by job ID."),
            ("Given any model call", "when prompt payloads are scanned", "then no held-out test content appears in any prompt."),
            ("Given the top candidate fails held-out tests", "when selecting the winner", "then the runner falls back to evaluate the next candidate."),
            ("Given no candidate passes held-out tests", "when the job ends", "then it reports `no verified winner` and flags the best-effort patch.")
        ],
        "test_cases": [
            ("tests/fitness/test_held_out.rs::test_stable_partitioning_ratio", "Validates 80/20 partition across test suites of sizes 5, 20, 100."),
            ("tests/fitness/test_held_out.rs::test_zero_held_out_prompt_leakage", "Scans prompt history to verify zero held-out test names or assertions."),
            ("tests/fitness/test_held_out.rs::test_candidate_fallback_ladder", "Forces candidate 1 to fail held-out tests; verifies candidate 2 chosen.")
        ]
    },
    {
        "key": "e1-9-adversary-tests",
        "title": "Adversary tests",
        "persona": "Developer",
        "jtbd": "When evolving code, I want an adversary model writing edge-case tests against the task specification, so that fragile candidates that pass minimal suites are probed and ranked lower.",
        "context": "Governed by AD-5. Adversary model drafts $K$ candidate tests. Broken tests that do not compile are discarded. Tests failing across baseline and all candidates are tagged `suspect`. Adversary tests never fail gates.",
        "out_of_scope": "Adversary tests failing hard gates or blocking build verification.",
        "spec_content": """### 1. Generation & Filtering Pipeline
1. Adversary model generates $K$ test functions in the target harness.
2. **Compilation Filter:** Test functions compiled in sandbox. If compilation fails, discard.
3. **Suspect Filter:** Execute against baseline. If test fails on baseline AND all Gen 0 candidates, tag `suspect` and exclude from scoring $A$.
4. **Scoring Invariant:** Surviving valid adversary tests contribute to term $A$ in score $S$.
""",
        "scenarios": [
            ("Given the spec and current best candidate", "when the adversary role runs", "then it generates up to K candidate tests."),
            ("Given an adversary test does not compile", "when collected", "then it is discarded immediately."),
            ("Given an adversary test fails on all candidates and the baseline", "when collected", "then it is flagged `suspect` and excluded from scoring."),
            ("Given any adversary test", "when hard gates run", "then it never causes a candidate to fail a gate.")
        ],
        "test_cases": [
            ("tests/engine/test_adversary.rs::test_adversary_generation_quota", "Asserts adversary generates up to K tests."),
            ("tests/engine/test_adversary.rs::test_syntax_error_discard", "Feeds uncompilable test and checks discard."),
            ("tests/engine/test_adversary.rs::test_suspect_test_filtering", "Simulates universally failing test and verifies suspect tag.")
        ]
    },
    {
        "key": "e1-10-budget-enforcement",
        "title": "Budget enforcement",
        "persona": "Developer",
        "jtbd": "When running evolutionary jobs, I want strict token, dollar, and generation early-stopping caps enforced before every model dispatch, so that long-running searches never exceed agreed costs.",
        "context": "Governed by AD-6. Checks projected costs against budget caps prior to network dispatch. Early stop terminates if no score improvement across 2 consecutive generations. Emits status `budget_exhausted` and returns best verified candidate.",
        "out_of_scope": "Dynamic credit card charging or billing API integrations.",
        "spec_content": """### 1. Budget Checks
- Before dispatch: $\text{Spent} + \text{ProjectedCallCost} \le \text{DollarCap}$.
- Token check: $\text{TokensUsed} + \text{MaxTokens} \le \text{TokenCap}$.
- If exceeded: halt generation, mark job `budget_exhausted`, return best verified candidate.

### 2. Early-Stop Invariant
- If $\max(S_{G}) \le \max(S_{G-1}) \le \max(S_{G-2})$, terminate search with reason `plateau_early_stop`.
""",
        "scenarios": [
            ("Given a token and dollar cap", "when a model call is about to be sent", "then projected cost is checked against remaining budget."),
            ("Given the cap would be exceeded", "when checked", "then the job halts as `budget_exhausted` and returns the best verified candidate so far."),
            ("Given no score improvement over 2 generations", "when generation ends", "then the job stops early with partial result.")
        ],
        "test_cases": [
            ("tests/engine/test_budget_enforcement.rs::test_pre_call_cost_check_halt", "Asserts loop halts before exceeding dollar cap."),
            ("tests/engine/test_budget_enforcement.rs::test_token_cap_exhaustion", "Asserts status 'budget_exhausted' when token limit reached."),
            ("tests/engine/test_budget_enforcement.rs::test_early_stop_score_plateau", "Simulates 2 generations with identical score and verifies early stop.")
        ]
    },
    {
        "key": "e1-11-patch-and-report",
        "title": "Patch and report",
        "persona": "Developer",
        "jtbd": "When a job finishes, I want the winning patch delivered on a clean git branch alongside a `.patch` file and audit report, so that I can review, inspect lineage, and merge with confidence.",
        "context": "Governed by AD-1 and AD-5. Creates branch `evoswarm/<job-id>` from base commit, writes `.evoswarm/patches/<job-id>.patch`, and generates markdown audit report. Base branch remains untouched.",
        "out_of_scope": "Auto-pushing branches to remote git hosts without user confirmation.",
        "spec_content": """### 1. Artifact Outputs
1. **Git Branch:** `evoswarm/<job-id>` branched from base commit with single atomic commit.
2. **Patch File:** `.evoswarm/patches/<job-id>.patch`.
3. **Audit Report:** `.evoswarm/reports/<job-id>.md` detailing:
   - Score breakdown ($S, A, P, Z$)
   - Visible and held-out test pass lists
   - Model calls, tokens, and dollar cost
   - Lineage tree
   - Proposed adversary candidate tests for review
""",
        "scenarios": [
            ("Given a verified winner", "when the job completes", "then branch `evoswarm/<job-id>` is created from base commit and `.patch` file is emitted."),
            ("Given the markdown report", "when opened", "then it displays score breakdown, held-out passes, model calls, cost, lineage, and proposed tests."),
            ("Given any job", "when it completes", "then the base working tree and active branch are untouched.")
        ],
        "test_cases": [
            ("tests/cli/test_report_output.rs::test_git_branch_emission", "Verifies branch creation and checks base commit equality."),
            ("tests/cli/test_report_output.rs::test_patch_file_integrity", "Applies emitted .patch file to clean repo and asserts clean application."),
            ("tests/cli/test_report_output.rs::test_markdown_report_sections", "Validates presence of score, cost, and lineage sections in report.")
        ]
    },
    {
        "key": "e1-12-resume-after-crash",
        "title": "Resume after crash",
        "persona": "Operator",
        "jtbd": "When the host box experiences an unexpected reboot or crash mid-search, I want running jobs to resume from their last completed generation without re-spending tokens, so that 24/7 autonomous operations are resilient.",
        "context": "Governed by AD-7 (SQLite Job Ledger). SQLite in WAL mode commits completed generations and model call responses with idempotency hashes. On reboot, incomplete sandbox runs are rescheduled; completed model calls are reused.",
        "out_of_scope": "Live process memory checkpointing via CRIU.",
        "spec_content": """### 1. SQLite Ledger Schema
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
- On service restart: scan `jobs` table for `Running` state.
- Locate highest completed generation in `job_generations`.
- Resume evolutionary loop from generation $G + 1$.
- Any model call whose prompt hash exists in `model_call_cache` is read from disk with 0 API tokens spent.
""",
        "scenarios": [
            ("Given process killed mid-generation", "when service restarts", "then job resumes from the last completed generation."),
            ("Given a model call completed before the crash", "when resuming", "then it is not repeated, using its stored idempotency key."),
            ("Given sandbox runs were in flight during crash", "when resuming", "then they are re-run cleanly.")
        ],
        "test_cases": [
            ("tests/engine/test_recovery.rs::test_resume_from_last_generation", "Simulates crash during Gen 2; asserts resume starts at Gen 3."),
            ("tests/engine/test_recovery.rs::test_model_call_idempotency_cache", "Verifies cached response used on replay with 0 network calls."),
            ("tests/engine/test_recovery.rs::test_interrupted_sandbox_rerun", "Verifies in-flight sandbox run re-evaluated on restart.")
        ]
    },
    {
        "key": "e1-13-benchmark-suite",
        "title": "Benchmark suite",
        "persona": "Team Lead",
        "jtbd": "When evaluating whether evolutionary code search pays off, I want a standardized 30-task benchmark comparing EvoSwarm against single-shot generation at equal token budget, so that investment is justified by empirical data.",
        "context": "Mandatory Exit Gate (Gate 1). The benchmark consists of 30 tasks (at least 10 Python, 10 C#) with trusted tests and held-out slices. EvoSwarm must achieve a solve rate $\ge 15$ points higher than single-shot with test feedback.",
        "out_of_scope": "Continuous benchmarking on every minor commit.",
        "spec_content": """### 1. Benchmark Harness Contract
- 30 tasks with pre-verified failing baselines and human-verified test suites.
- Runner executes both methods at equal token caps ($T = 500,000$ tokens per task):
  - **Single-shot with feedback:** 1 draft + sequential retries up to token budget.
  - **EvoSwarm:** Population evolutionary loop.
- Metric: $\Delta = \text{SolveRate}_{\text{EvoSwarm}} - \text{SolveRate}_{\text{SingleShot}}$.
- Exit gate requires: $\Delta \ge 15.0\%$.
""",
        "scenarios": [
            ("Given the benchmark suite", "when loaded", "then it contains 30 tasks, at least 10 Python and 10 C#, each with trusted tests and a held-out split."),
            ("Given the comparison script", "when run", "then EvoSwarm and single-shot-with-feedback execute at equal token budget."),
            ("Given benchmark execution completes", "when results print", "then they show solve rate, tokens per solved task, and wall time.")
        ],
        "test_cases": [
            ("tests/benchmark/test_benchmark_harness.py::test_benchmark_task_integrity", "Validates all 30 benchmark tasks have valid tests and held-out slices."),
            ("tests/benchmark/test_benchmark_harness.py::test_token_budget_equality", "Verifies both runners terminate at identical token budgets."),
            ("tests/benchmark/test_benchmark_harness.py::test_solve_rate_delta_calculation", "Validates delta calculation and gate assertion logic.")
        ]
    }
]


def render_intent(story: dict) -> str:
    return f"""# Story Intent: {story['title']}

## 1. Metadata
- **Story Key:** `{story['key']}`
- **Epic:** [Epic 1: Evolve CLI and Fitness](file:///workspace/calm-faraday/docs/epics/epic-1-evolve-cli-and-fitness.md)
- **Persona:** {story['persona']}
- **Status:** ready-for-dev

---

## 2. Job-to-be-Done (JTBD)
> {story['jtbd']}

---

## 3. Problem Statement & Architectural Context
{story['context']}

---

## 4. Scope Discipline & Boundary
- **In Scope:** Core execution and contracts defined in specification.
- **Out of Scope:** {story['out_of_scope']}
"""


def render_spec(story: dict) -> str:
    scenarios_text = "\n".join([f"- **{given}**, **{when}**, **{then}**." for given, when, then in story['scenarios']])
    return f"""# Story Specification: {story['title']}

## 1. Functional Specification
{story['spec_content']}

---

## 2. Acceptance Criteria (Gherkin Scenarios)
{scenarios_text}
"""


def render_plan(story: dict) -> str:
    test_rows = "\n".join([f"- [`{t[0]}`](file:///workspace/calm-faraday/{t[0].split('::')[0]}): {t[1]}" for t in story['test_cases']])
    return f"""# Implementation Plan: {story['title']}

## 1. Pre-Flight Architecture & Rule Check
- [x] Conforms to [`.agents/rules/architecture-rules.md`](file:///workspace/calm-faraday/.agents/rules/architecture-rules.md) invariants.
- [x] Follows Kent Beck TDD cycle ([`.agents/rules/tdd-discipline.md`](file:///workspace/calm-faraday/.agents/rules/tdd-discipline.md)).
- [x] Zero AI comments in production code.

---

## 2. Red Phase (Failing Tests to Author First)
Before writing any production code, author failing unit and integration tests:
{test_rows}

---

## 3. Green Phase (Minimal Production Code)
Implement the minimal logic in target crates/modules to satisfy tests:
- Define core structs and traits.
- Implement error handling and bounds checking.
- Connect persistence / CLI / sandbox dispatch.

---

## 4. Refactor Phase & Verification Gate
- Remove any redundant allocations or temporary scaffolding.
- Ensure all comments explain **WHY**, not **WHAT**.
- Execute verification gate:
```bash
python3 scripts/sprint.py verify --cmd "cargo test --test {story['key'].replace('-', '_')}" --anti-cheat
```
"""


def main():
    stories_root = Path("/workspace/calm-faraday/docs/stories")
    stories_root.mkdir(parents=True, exist_ok=True)

    print(f"Planning {len(EPIC_1_STORIES)} stories in Epic 1...")
    for s in EPIC_1_STORIES:
        story_dir = stories_root / s["key"]
        story_dir.mkdir(parents=True, exist_ok=True)

        intent_file = story_dir / "intent.md"
        spec_file = story_dir / "spec.md"
        plan_file = story_dir / "plan.md"

        intent_file.write_text(render_intent(s), encoding="utf-8")
        spec_file.write_text(render_spec(s), encoding="utf-8")
        plan_file.write_text(render_plan(s), encoding="utf-8")

        print(f"  Created artifact chain in {story_dir.name}/ (intent.md, spec.md, plan.md)")

    print("All Epic 1 stories planned successfully!")


if __name__ == "__main__":
    main()
