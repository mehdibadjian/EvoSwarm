//! EvoSwarm CLI crate (e1-1): `evoswarm run` submission, baseline validation, exit codes.
//!
//! Exposed as a library so integration tests can drive `submit` with a scripted
//! `SandboxBackend` fake; `main.rs` is the thin clap binary over the same API.

pub mod approve_tests;
pub mod artifacts;
pub mod baseline;
pub mod exit_codes;
pub mod git_writer;
pub mod host_check;
pub mod lineage;
pub mod report;
pub mod run;
pub mod usage;

pub use approve_tests::{
    approve_candidate_tests, reject_candidate_tests, ApproveError, ApprovedTestsRegistry,
    TestMetadata,
};
pub use artifacts::{emit, Artifacts, ReportInputs};
pub use baseline::{validate_baseline, BaselineRejection};
pub use exit_codes::ExitCode;
pub use lineage::{format_ascii_tree, format_json_export, load_job_lineage, LineageError};
pub use run::{submit, SubmitError, SubmitOutcome};
pub use usage::{format_usage_report, parse_since_date, run_usage_report, UsageCliError};


