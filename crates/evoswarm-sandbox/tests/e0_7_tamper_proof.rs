//! e0-7: Tamper-proof tests.

use evoswarm_core::{SandboxProfile};
use evoswarm_sandbox::{BwrapBackend, SandboxBackend, detect_harness_override, hash_tests_dir};
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
async fn test_readonly_test_mount() {
    // Create a host tests directory with a sample test file.
    let host_tests = tempfile::tempdir().expect("tempdir");
    std::fs::write(host_tests.path().join("test_sample.py"), "def test_pass(): pass")
        .expect("write test");

    let backend = BwrapBackend::new().with_tests_dir(host_tests.path().to_path_buf());
    let profile = test_profile();
    let workdir = backend.prepare(&profile, b"").await.expect("prepare");

    // Attempt to write into /work/tests (which is mounted read-only).
    let result = backend
        .run(&workdir, "echo 'tamper' > /work/tests/test_foo.py 2>&1; echo $?")
        .await
        .expect("run");

    // The write should fail (EROFS). The exit code of the shell is the exit
    // code of the last command (echo $?), which will be 0, but the stderr or
    // stdout will contain the EROFS error.
    assert!(
        result.stderr.contains("Read-only file system")
            || result.stdout.contains("Read-only file system")
            || result.stderr.contains("EROFS")
            || result.stdout.contains("EROFS"),
        "expected EROFS error in stdout or stderr, got stdout={:?} stderr={:?}",
        result.stdout,
        result.stderr
    );

    // Verify the test file was not modified.
    let content = std::fs::read_to_string(host_tests.path().join("test_sample.py")).unwrap();
    assert_eq!(content, "def test_pass(): pass");

    backend.collect(workdir).await.expect("collect");
}

#[tokio::test]
async fn test_harness_override_rejection() {
    let backend = BwrapBackend::new();
    let profile = test_profile();
    let workdir = backend.prepare(&profile, b"").await.expect("prepare");

    // Simulate a candidate dropping a conftest.py in the workdir root.
    std::fs::write(workdir.join("conftest.py"), "# tampered harness").expect("write conftest");

    assert!(detect_harness_override(&workdir));

    // Clean up the override so the test passes the gate check.
    std::fs::remove_file(workdir.join("conftest.py")).expect("rm conftest");
    assert!(!detect_harness_override(&workdir));

    backend.collect(workdir).await.expect("collect");
}

#[tokio::test]
async fn test_tests_dir_hash_unchanged_after_run() {
    let host_tests = tempfile::tempdir().expect("tempdir");
    std::fs::write(host_tests.path().join("test_a.py"), "def test_a(): pass").expect("write a");
    std::fs::write(host_tests.path().join("test_b.py"), "def test_b(): pass").expect("write b");

    let hash_before = hash_tests_dir(host_tests.path()).expect("hash before");

    let backend = BwrapBackend::new().with_tests_dir(host_tests.path().to_path_buf());
    let profile = test_profile();
    let workdir = backend.prepare(&profile, b"").await.expect("prepare");

    let _result = backend.run(&workdir, "echo test").await.expect("run");

    let hash_after = hash_tests_dir(host_tests.path()).expect("hash after");
    assert_eq!(hash_before, hash_after, "tests dir hash changed after run");

    backend.collect(workdir).await.expect("collect");
}
