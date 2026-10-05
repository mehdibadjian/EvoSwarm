use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Isolation and resource shape for a single candidate run (ARCHITECTURE-SPINE §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxProfile {
    pub stack: String,
    pub wall_timeout_secs: u64,
    pub memory_limit_bytes: u64,
    pub tmpfs_size_bytes: u64,
    pub tasks_max: u32,
    pub read_only_mounts: Vec<PathBuf>,
    pub dependency_cache_path: PathBuf,
}

/// Terminal classification of a sandbox run. `Timeout` and `Oom` are distinct from
/// `Failed` so fitness gates (e1-6) and the mutator feedback loop (e1-4) can tell a
/// resource kill apart from an ordinary non-zero exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Success,
    Failed,
    Timeout,
    Oom,
    TamperDetected,
}

/// Outcome of one `SandboxBackend::run` call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub wall_time_ms: u64,
    pub peak_memory_bytes: u64,
    pub status: RunStatus,
}

impl ExecutionResult {
    /// True only for a clean, in-limit success. Any resource kill or tamper flag is
    /// excluded so callers cannot mistake a contained failure for a pass.
    pub fn succeeded(&self) -> bool {
        self.status == RunStatus::Success && self.exit_code == 0
    }
}
