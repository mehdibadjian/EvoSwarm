//! Resource limits (e0-4, AD-1): memory, process count, and wall-clock time caps.
//!
//! Under AD-1, runaway candidate code is constrained by:
//! 1. `systemd-run --user --scope` invocation configuring:
//!    - `MemoryMax=<bytes>` (cgroup memory ceiling, terminating run as `RunStatus::Oom` on breach)
//!    - `TasksMax=<count>` (process cap to contain fork bombs, terminating run as `RunStatus::Failed` or `RunStatus::Oom`)
//!    - `CPUQuota=100%` (CPU bandwidth cap)
//! 2. Outer process timeout with 5s SIGKILL grace terminating run as `RunStatus::Timeout`.
//!
//! ## Containment honesty (roadmap §5, SEAM)
//!
//! Live `systemd-run --user --scope` invocation requires cgroup v2 delegation and a systemd
//! session bus, which are absent in container/PRoot environments.
//! The pure logic is certifiable and deterministic:
//! - Construction of `systemd-run` command lines with memory, tasks, and CPU caps
//! - Classification of exit signals and statuses (`Oom`, `Timeout`, `Failed`, `Success`)
//! - Wall-clock timeout enforcement with 5s SIGKILL grace calculations
//! - Diagnostic detection for OOM killer and task limit breaches.

use std::time::Duration;
use evoswarm_core::{RunStatus, SandboxProfile};

/// Grace period beyond wall-clock limit before issuing SIGKILL.
pub const SIGKILL_GRACE_SECS: u64 = 5;

/// Default CPU quota configuration for candidate scopes.
pub const DEFAULT_CPU_QUOTA: &str = "100%";

/// Builds the `systemd-run --user --scope` command prefix wrapping the inner sandbox command.
pub fn build_systemd_scope_argv(profile: &SandboxProfile, inner_argv: &[String]) -> Vec<String> {
    let mut argv = vec![
        "systemd-run".to_string(),
        "--user".to_string(),
        "--scope".to_string(),
        "-q".to_string(),
        format!("-pMemoryMax={}", profile.memory_limit_bytes),
        format!("-pTasksMax={}", profile.tasks_max),
        format!("-pCPUQuota={}", DEFAULT_CPU_QUOTA),
        "--".to_string(),
    ];
    argv.extend_from_slice(inner_argv);
    argv
}

/// Classifies a process execution into [`RunStatus`] based on timeout, exit code, and stderr/cgroup hints.
pub fn classify_resource_exit(
    timed_out: bool,
    exit_code: i32,
    stderr: &str,
) -> RunStatus {
    if timed_out {
        return RunStatus::Timeout;
    }

    // Exit code 137 (SIGKILL) or OOM markers indicate cgroup memory kills
    if exit_code == 137 || stderr.contains("Out of memory") || stderr.contains("killed by cgroup OOM") {
        return RunStatus::Oom;
    }

    // Fork bomb hitting TasksMax typically results in Resource temporarily unavailable (EAGAIN) or exit code > 0
    if exit_code != 0 {
        if stderr.contains("fork: retry: Resource temporarily unavailable")
            || stderr.contains("fork: Resource temporarily unavailable")
            || stderr.contains("cannot fork")
        {
            return RunStatus::Failed;
        }
        return RunStatus::Failed;
    }

    RunStatus::Success
}

/// Helper that checks whether the total elapsed time exceeded wall timeout plus grace.
pub fn is_grace_exceeded(elapsed: Duration, wall_limit_secs: u64) -> bool {
    elapsed >= Duration::from_secs(wall_limit_secs + SIGKILL_GRACE_SECS)
}
