//! EvoSwarm CLI crate (e1-1): `evoswarm run` submission, baseline validation, exit codes.
//!
//! Exposed as a library so integration tests can drive `submit` with a scripted
//! `SandboxBackend` fake; `main.rs` is the thin clap binary over the same API.

pub mod artifacts;
pub mod baseline;
pub mod exit_codes;
pub mod git_writer;
pub mod report;
pub mod run;

pub use artifacts::{emit, Artifacts, ReportInputs};
pub use baseline::{validate_baseline, BaselineRejection};
pub use exit_codes::ExitCode;
pub use run::{submit, SubmitError, SubmitOutcome};
