//! `evoswarm` binary (e1-1): parses `run` and dispatches to `submit`, mapping the outcome
//! to a distinct process exit code so callers can branch without parsing stderr.

use std::path::PathBuf;
use std::process::ExitCode as StdExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use evoswarm_core::{JobObjective, JobSubmission};
use evoswarm_ledger::{JobLedger, RepoRoot};
use evoswarm_sandbox::BwrapBackend;

use evoswarm_cli::run::submit;

#[derive(Parser)]
#[command(name = "evoswarm", version, about = "EvoSwarm evolutionary code search")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Submit a job for evolutionary search.
    Run {
        /// Natural-language description of the task.
        #[arg(long)]
        task: String,
        /// Test command executed in the sandbox.
        #[arg(long = "cmd")]
        test_command: String,
        /// Comma-separated relative paths the candidate may modify.
        #[arg(long, value_delimiter = ',')]
        paths: Vec<String>,
        /// Token budget cap.
        #[arg(long)]
        budget_tokens: Option<u64>,
        /// Dollar budget cap.
        #[arg(long)]
        budget_dollars: Option<f64>,
        /// Optimisation objective.
        #[arg(long, value_enum, default_value_t = ObjectiveArg::Correctness)]
        objective: ObjectiveArg,
        /// Per-run wall-clock timeout in seconds.
        #[arg(long, default_value_t = 30)]
        timeout_secs: u64,
        /// Repository root; defaults to the current directory.
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Print local gateway token/cost usage by day (e4-2).
    Usage {
        /// Only include usage on/after this UTC date (YYYY-MM-DD); defaults to 7 days back.
        #[arg(long)]
        since: Option<String>,
        /// Path to the gateway usage database; defaults to `<repo>/.evoswarm/usage.db`.
        #[arg(long)]
        db: Option<PathBuf>,
        /// Repository root used to locate the default usage db; defaults to current dir.
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Print candidate lineage ancestry tree or JSON (e2-8).
    Lineage {
        /// Job ID to inspect.
        job_id: String,
        /// Export as JSON instead of ASCII tree.
        #[arg(long)]
        json: bool,
        /// Path to the lineage retry spool or archive directory; defaults to `<repo>/.evoswarm/lineage_spool`.
        #[arg(long)]
        spool_dir: Option<PathBuf>,
        /// Repository root used to locate the spool directory; defaults to current dir.
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Approve candidate tests from a completed job and commit them to a branch (e2-6).
    ApproveTests {
        /// Job ID containing the tests.
        job_id: String,
        /// Comma-separated test names/IDs to approve.
        #[arg(long, value_delimiter = ',')]
        ids: Vec<String>,
        /// Path to the lineage retry spool or archive directory; defaults to `<repo>/.evoswarm/lineage_spool`.
        #[arg(long)]
        spool_dir: Option<PathBuf>,
        /// Repository root; defaults to current dir.
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Reject candidate tests so they are not proposed again (e2-6).
    RejectTests {
        /// Job ID containing the tests.
        job_id: String,
        /// Comma-separated test names/IDs to reject.
        #[arg(long, value_delimiter = ',')]
        ids: Vec<String>,
        /// Path to the lineage retry spool or archive directory; defaults to `<repo>/.evoswarm/lineage_spool`.
        #[arg(long)]
        spool_dir: Option<PathBuf>,
        /// Repository root; defaults to current dir.
        #[arg(long)]
        repo: Option<PathBuf>,
    },
}



#[derive(Clone, Copy, ValueEnum)]
enum ObjectiveArg {
    Correctness,
    Perf,
}

impl From<ObjectiveArg> for JobObjective {
    fn from(o: ObjectiveArg) -> Self {
        match o {
            ObjectiveArg::Correctness => JobObjective::Correctness,
            ObjectiveArg::Perf => JobObjective::Performance,
        }
    }
}

#[tokio::main]
async fn main() -> StdExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Run {
            task,
            test_command,
            paths,
            budget_tokens,
            budget_dollars,
            objective,
            timeout_secs,
            repo,
        } => {
            let repo_root = repo.unwrap_or_else(|| PathBuf::from("."));
            let repo = RepoRoot::new(repo_root);
            let ledger_path = repo.as_path().join(".evoswarm/jobs.db");
            let submission = JobSubmission {
                task_description: task,
                test_command,
                target_paths: paths.into_iter().map(PathBuf::from).collect(),
                budget_tokens,
                budget_dollars,
                objective: objective.into(),
                timeout_secs,
            };

            let ledger = match JobLedger::open(&ledger_path) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("failed to open ledger at {}: {e}", ledger_path.display());
                    return evoswarm_cli::ExitCode::Validation.to_std();
                }
            };
            let backend = BwrapBackend::new();
            match submit(submission, &repo, &ledger, &backend).await {
                Ok(outcome) => {
                    println!(
                        "{}",
                        serde_json::to_string(&outcome.ticket).expect("ticket serialises")
                    );
                    if let Some(msg) = &outcome.message {
                        eprintln!("{msg}");
                    }
                    outcome.code.to_std()
                }
                Err(e) => {
                    eprintln!("{}", e);
                    e.exit_code().to_std()
                }
            }
        }
        Command::Usage { since, db, repo } => {
            use evoswarm_cli::usage::{default_since_unix, parse_since_date, run_usage_report};
            use evoswarm_gateway::usage::UsagePricing;

            let db_path = db.unwrap_or_else(|| {
                repo.unwrap_or_else(|| PathBuf::from("."))
                    .join(".evoswarm/usage.db")
            });
            let since_unix = match since.as_deref() {
                Some(date) => match parse_since_date(date) {
                    Ok(ts) => ts,
                    Err(e) => {
                        eprintln!("{e}");
                        return evoswarm_cli::ExitCode::Validation.to_std();
                    }
                },
                None => {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    default_since_unix(now)
                }
            };
            match run_usage_report(&db_path, since_unix, &UsagePricing::default()) {
                Ok(report) => {
                    print!("{report}");
                    evoswarm_cli::ExitCode::Ok.to_std()
                }
                Err(e) => {
                    eprintln!("{e}");
                    evoswarm_cli::ExitCode::Validation.to_std()
                }
            }
        }
        Command::Lineage {
            job_id,
            json,
            spool_dir,
            repo,
        } => {
            use evoswarm_cli::lineage::{format_ascii_tree, format_json_export, load_job_lineage};

            let spool_path = spool_dir.unwrap_or_else(|| {
                repo.unwrap_or_else(|| PathBuf::from("."))
                    .join(".evoswarm/lineage_spool")
            });

            match load_job_lineage(&job_id, &spool_path) {
                Ok(record) => {
                    if json {
                        match format_json_export(&record) {
                            Ok(j) => {
                                println!("{j}");
                                evoswarm_cli::ExitCode::Ok.to_std()
                            }
                            Err(e) => {
                                eprintln!("{e}");
                                evoswarm_cli::ExitCode::Validation.to_std()
                            }
                        }
                    } else {
                        print!("{}", format_ascii_tree(&record));
                        evoswarm_cli::ExitCode::Ok.to_std()
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    evoswarm_cli::ExitCode::Validation.to_std()
                }
            }
        }
        Command::ApproveTests {
            job_id,
            ids,
            spool_dir,
            repo,
        } => {
            use evoswarm_cli::approve_tests::approve_candidate_tests;

            let repo_path = repo.unwrap_or_else(|| PathBuf::from("."));
            let spool_path = spool_dir.unwrap_or_else(|| {
                repo_path.join(".evoswarm/lineage_spool")
            });

            match approve_candidate_tests(&job_id, &ids, &repo_path, &spool_path) {
                Ok(branch) => {
                    println!("Approved tests for {job_id} ({ids:?}) on branch {branch}");
                    evoswarm_cli::ExitCode::Ok.to_std()
                }
                Err(e) => {
                    eprintln!("{e}");
                    evoswarm_cli::ExitCode::Validation.to_std()
                }
            }
        }
        Command::RejectTests {
            job_id,
            ids,
            spool_dir,
            repo,
        } => {
            use evoswarm_cli::approve_tests::reject_candidate_tests;

            let repo_path = repo.unwrap_or_else(|| PathBuf::from("."));
            let spool_path = spool_dir.unwrap_or_else(|| {
                repo_path.join(".evoswarm/lineage_spool")
            });

            match reject_candidate_tests(&job_id, &ids, &repo_path, &spool_path) {
                Ok(()) => {
                    println!("Rejected tests for {job_id} ({ids:?})");
                    evoswarm_cli::ExitCode::Ok.to_std()
                }
                Err(e) => {
                    eprintln!("{e}");
                    evoswarm_cli::ExitCode::Validation.to_std()
                }
            }
        }
    }
}
