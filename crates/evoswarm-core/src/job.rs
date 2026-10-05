use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A job as submitted on the CLI (`evoswarm run`, e1-1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobSubmission {
    pub task_description: String,
    pub test_command: String,
    pub target_paths: Vec<PathBuf>,
    pub budget_tokens: Option<u64>,
    pub budget_dollars: Option<f64>,
    pub objective: JobObjective,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobObjective {
    Correctness,
    /// Performance objective; required when the baseline already passes every test.
    #[serde(rename = "perf")]
    Performance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    BudgetExhausted,
    Cancelled,
}

/// The fast ticket printed within 2 s of submission (e1-1), before baseline validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobTicket {
    pub job_id: String,
    pub status: JobStatus,
}
