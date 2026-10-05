use std::path::{Path, PathBuf};

use async_trait::async_trait;
use evoswarm_core::{ExecutionResult, SandboxProfile};
use thiserror::Error;

/// Failures of the isolation layer itself, distinct from a candidate's own non-zero exit
/// (which is reported through `ExecutionResult.status`, not as an error).
#[derive(Debug, Error)]
pub enum SandboxError {
    #[error("failed to prepare ephemeral workdir: {0}")]
    Prepare(String),
    #[error("failed to execute test command: {0}")]
    Execution(String),
    #[error("failed to clean up workdir: {0}")]
    Cleanup(String),
    #[error("sandbox backend unavailable on this host: {0}")]
    Unavailable(String),
}

/// The single lifecycle contract every candidate runs through (AD-1, ARCHITECTURE-SPINE
/// §3.1). The search loop depends only on this trait, so adding a language stack or
/// swapping the isolation mechanism never touches engine, fitness, or CLI code.
///
/// e0-1 supplies the bwrap-backed implementation; this crate defines the boundary.
#[async_trait]
pub trait SandboxBackend: Send + Sync {
    /// Prepares an ephemeral workdir and mount points, applying `candidate_patch`.
    async fn prepare(
        &self,
        profile: &SandboxProfile,
        candidate_patch: &[u8],
    ) -> Result<PathBuf, SandboxError>;

    /// Executes the test command inside the isolation boundary, returning a classified
    /// `ExecutionResult` (success / failed / timeout / oom / tamper).
    async fn run(
        &self,
        workdir: &Path,
        test_command: &str,
    ) -> Result<ExecutionResult, SandboxError>;

    /// Collects logs and removes the ephemeral workdir.
    async fn collect(&self, workdir: PathBuf) -> Result<(), SandboxError>;
}
