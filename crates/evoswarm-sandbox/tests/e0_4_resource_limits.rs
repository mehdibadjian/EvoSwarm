//! e0-4 Resource limits — acceptance criteria (story §4, AD-1).
//!
//! Gate: `python3 scripts/sprint.py verify --cmd "cargo test --test e0_4_resource_limits" --anti-cheat`
//!
//! Validates:
//! - AC1: Given a candidate runs a fork bomb, when TasksMax is hit, run ends as oom or failed.
//! - AC2: Given a candidate allocates memory exceeding cap, cgroup kill ends as oom.
//! - AC3: Given an infinite loop, timeout triggers with wall limit + 5s SIGKILL grace.
//! - Scope argv builder wraps execution with MemoryMax, TasksMax, and CPUQuota.

use std::path::PathBuf;
use std::time::Duration;
use evoswarm_core::{RunStatus, SandboxProfile};
use evoswarm_sandbox::limits::{
    build_systemd_scope_argv, classify_resource_exit, is_grace_exceeded,
    DEFAULT_CPU_QUOTA, SIGKILL_GRACE_SECS,
};

fn test_profile() -> SandboxProfile {
    SandboxProfile {
        stack: "python".to_string(),
        wall_timeout_secs: 10,
        memory_limit_bytes: 2 * 1024 * 1024 * 1024, // 2GB
        tmpfs_size_bytes: 64 * 1024 * 1024,
        tasks_max: 256,
        read_only_mounts: vec![],
        dependency_cache_path: PathBuf::new(),
    }
}

/// AC1: TasksMax enforcement argv shape and fork bomb failure classification.
#[test]
fn test_tasks_max_and_fork_bomb_contained() {
    let profile = test_profile();
    let inner_argv = vec!["bwrap".to_string(), "--unshare-all".to_string()];
    let scope_argv = build_systemd_scope_argv(&profile, &inner_argv);

    assert_eq!(scope_argv[0], "systemd-run");
    assert!(scope_argv.contains(&"--user".to_string()));
    assert!(scope_argv.contains(&"--scope".to_string()));
    assert!(scope_argv.contains(&format!("-pTasksMax={}", profile.tasks_max)));
    assert!(scope_argv.contains(&format!("-pMemoryMax={}", profile.memory_limit_bytes)));
    assert!(scope_argv.contains(&format!("-pCPUQuota={}", DEFAULT_CPU_QUOTA)));

    // When TasksMax is breached, fork calls fail with EAGAIN
    let fork_bomb_stderr = "bash: fork: retry: Resource temporarily unavailable\nbash: fork: Resource temporarily unavailable";
    let status = classify_resource_exit(false, 1, fork_bomb_stderr);
    assert_eq!(status, RunStatus::Failed, "fork bomb hitting TasksMax ends as Failed");
}

/// AC2: Memory cap enforcement ends as RunStatus::Oom.
#[test]
fn test_memory_cap_enforcement_classification() {
    // Exit 137 is SIGKILL triggered by cgroup OOM killer
    let status_sigkill = classify_resource_exit(false, 137, "killed by cgroup OOM");
    assert_eq!(status_sigkill, RunStatus::Oom, "exit 137 is OOM");

    let status_oom_stderr = classify_resource_exit(false, 1, "MemoryError: Out of memory");
    assert_eq!(status_oom_stderr, RunStatus::Oom, "explicit Out of memory error is OOM");
}

/// AC3: Wall time timeout with 5s SIGKILL grace.
#[test]
fn test_wall_time_sigkill_grace() {
    assert_eq!(SIGKILL_GRACE_SECS, 5, "grace period is exactly 5s");

    let wall_limit = 2; // 2s
    let within_limit = Duration::from_secs(2);
    let during_grace = Duration::from_secs(4);
    let after_grace = Duration::from_secs(7);

    assert!(!is_grace_exceeded(within_limit, wall_limit));
    assert!(!is_grace_exceeded(during_grace, wall_limit));
    assert!(is_grace_exceeded(after_grace, wall_limit));

    // When wall limit is timed out, status is Timeout
    let status = classify_resource_exit(true, -1, "");
    assert_eq!(status, RunStatus::Timeout, "timed out execution marked as Timeout");
}
