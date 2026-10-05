//! e0-1: Sandbox runner interface tests.

use evoswarm_core::{RunStatus, SandboxProfile};
use evoswarm_sandbox::{BwrapBackend, SandboxBackend};
use std::path::PathBuf;

fn test_profile() -> SandboxProfile {
    SandboxProfile {
        stack: "test".to_string(),
        wall_timeout_secs: 30,
        memory_limit_bytes: 256 * 1024 * 1024,
        tmpfs_size_bytes: 64 * 1024 * 1024,
        tasks_max: 10,
        read_only_mounts: vec![],
        dependency_cache_path: PathBuf::new(),
    }
}

#[tokio::test]
async fn test_sandbox_runner_lifecycle_success() {
    let backend = BwrapBackend::new();
    let profile = test_profile();
    let workdir = backend.prepare(&profile, b"").await.expect("prepare");

    let result = backend.run(&workdir, "echo hello").await.expect("run");

    assert_eq!(result.exit_code, 0);
    assert_eq!(result.status, RunStatus::Success);
    assert!(result.stdout.contains("hello"));
    assert!(result.wall_time_ms > 0);
    assert!(result.succeeded());

    backend.collect(workdir).await.expect("collect");
}

#[tokio::test]
async fn test_sandbox_runner_timeout_marking() {
    let backend = BwrapBackend::new();
    let profile = test_profile();
    let workdir = backend.prepare(&profile, b"").await.expect("prepare");

    // Use run_with_timeout to enforce a 1-second deadline on a 60-second sleep
    let result = backend
        .run_with_timeout(&workdir, "sleep 60", 1)
        .await
        .expect("run");

    assert_eq!(result.status, RunStatus::Timeout);
    assert_eq!(result.exit_code, -1);
    assert!(!result.succeeded());

    backend.collect(workdir).await.expect("collect");
}

#[tokio::test]
async fn test_sandbox_runner_concurrent_isolation() {
    let backend1 = BwrapBackend::new();
    let backend2 = BwrapBackend::new();
    let profile = test_profile();

    let workdir1 = backend1.prepare(&profile, b"").await.expect("prepare1");
    let workdir2 = backend2.prepare(&profile, b"").await.expect("prepare2");

    // Each run writes a unique file to /work. Because each run gets its own
    // tmpfs workdir and --unshare-all isolates the mount namespace, neither
    // can see the other's files.
    let (r1, r2) = tokio::join!(
        backend1.run(&workdir1, "echo run1 > /work/test.txt && cat /work/test.txt"),
        backend2.run(&workdir2, "echo run2 > /work/test.txt && cat /work/test.txt"),
    );

    let r1 = r1.expect("run1");
    let r2 = r2.expect("run2");

    assert_eq!(r1.status, RunStatus::Success);
    assert_eq!(r2.status, RunStatus::Success);
    assert!(r1.stdout.contains("run1"));
    assert!(r2.stdout.contains("run2"));

    // Verify the host workdirs are distinct and contain their own files.
    let content1 = std::fs::read_to_string(workdir1.join("test.txt")).unwrap_or_default();
    let content2 = std::fs::read_to_string(workdir2.join("test.txt")).unwrap_or_default();
    assert!(content1.contains("run1"));
    assert!(content2.contains("run2"));
    assert!(!content1.contains("run2"));
    assert!(!content2.contains("run1"));

    backend1.collect(workdir1).await.expect("collect1");
    backend2.collect(workdir2).await.expect("collect2");
}
