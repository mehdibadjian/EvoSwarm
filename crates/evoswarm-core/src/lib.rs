pub mod candidate;
pub mod execution;
pub mod holdout;
pub mod job;
pub mod outcome;
pub mod path_guard;
pub mod provenance;
pub mod report_model;
pub mod usage;

pub use candidate::Candidate;
pub use execution::{ExecutionResult, RunStatus, SandboxProfile};
pub use holdout::{HoldoutReason, TestSplit};
pub use job::{JobObjective, JobStatus, JobSubmission, JobTicket};
pub use outcome::SelectionOutcome;
pub use path_guard::{require_within_root, PathEscape};
pub use provenance::{PassVector, TestOrigin};
pub use report_model::{LineageNode, RoleUsage, ScoreBreakdown, WinnerSummary};
pub use usage::Usage;
