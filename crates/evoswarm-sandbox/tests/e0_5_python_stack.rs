//! e0-5: Python stack tests.

use evoswarm_core::SandboxProfile;
use evoswarm_sandbox::stacks::python::VenvCache;
use evoswarm_sandbox::{parse_junit_outcomes, parse_junit_xml, BwrapBackend, SandboxBackend, TestStatus};
use std::path::PathBuf;

#[test]
fn test_junit_xml_parsing() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<testsuites>
  <testsuite name="pytest" errors="0" failures="1" skipped="1" tests="5" time="0.123">
    <testcase classname="tests.test_example" name="test_pass_1" time="0.001"/>
    <testcase classname="tests.test_example" name="test_pass_2" time="0.002"/>
    <testcase classname="tests.test_example" name="test_fail" time="0.003">
      <failure message="assert False">AssertionError</failure>
    </testcase>
    <testcase classname="tests.test_example" name="test_skip" time="0.000">
      <skipped type="pytest.skip" message="unconditional skip"/>
    </testcase>
    <testcase classname="tests.test_example" name="test_pass_3" time="0.001"/>
  </testsuite>
</testsuites>"#;

    let summary = parse_junit_xml(xml).expect("parse");
    assert_eq!(summary.total, 5);
    assert_eq!(summary.passed, 3);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.skipped, 1);

    let outcomes = parse_junit_outcomes(xml).expect("parse outcomes");
    assert_eq!(outcomes.len(), 5);
    assert_eq!(outcomes[0].name, "test_pass_1");
    assert_eq!(outcomes[0].status, TestStatus::Passed);
    assert_eq!(outcomes[2].name, "test_fail");
    assert_eq!(outcomes[2].status, TestStatus::Failed);
    assert_eq!(outcomes[3].name, "test_skip");
    assert_eq!(outcomes[3].status, TestStatus::Skipped);
}

#[test]
fn test_junit_xml_empty() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
<testsuites>
  <testsuite name="pytest" errors="0" failures="0" skipped="0" tests="0" time="0.000"/>
</testsuites>"#;

    let summary = parse_junit_xml(xml).expect("parse");
    assert_eq!(summary.total, 0);
    assert_eq!(summary.passed, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.skipped, 0);
}

#[test]
fn test_lockfile_hash_deterministic() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let lockfile = tempdir.path().join("requirements.txt");
    std::fs::write(&lockfile, "pytest==7.0.0\nrequests==2.28.0\n").expect("write");

    let hash1 = evoswarm_sandbox::stacks::python::lockfile_hash(&lockfile).expect("hash1");
    let hash2 = evoswarm_sandbox::stacks::python::lockfile_hash(&lockfile).expect("hash2");

    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), 64); // SHA-256 hex digest is 64 chars
}

#[test]
fn test_lockfile_hash_changes_with_content() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let lockfile = tempdir.path().join("requirements.txt");

    std::fs::write(&lockfile, "pytest==7.0.0\n").expect("write1");
    let hash1 = evoswarm_sandbox::stacks::python::lockfile_hash(&lockfile).expect("hash1");

    std::fs::write(&lockfile, "pytest==7.1.0\n").expect("write2");
    let hash2 = evoswarm_sandbox::stacks::python::lockfile_hash(&lockfile).expect("hash2");

    assert_ne!(hash1, hash2);
}

#[tokio::test]
async fn test_venv_cache_reuse() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let cache_dir = tempdir.path().join("venv-cache");
    let lockfile = tempdir.path().join("requirements.txt");
    std::fs::write(&lockfile, "pytest==7.0.0\n").expect("write lockfile");

    let cache = VenvCache::new(&cache_dir).expect("create cache");

    // First creation should build the venv
    let venv1 = cache.ensure_venv(&lockfile).await.expect("first ensure");
    assert!(venv1.exists(), "venv should exist after first creation");
    assert!(venv1.join("bin/python").exists(), "venv should have python binary");

    // Second call with same lockfile should reuse the cached venv
    let venv2 = cache.ensure_venv(&lockfile).await.expect("second ensure");
    assert_eq!(venv1, venv2, "same lockfile hash should return same venv path");

    // Verify the venv is actually usable
    let output = std::process::Command::new(venv1.join("bin/python"))
        .arg("--version")
        .output()
        .expect("run python");
    assert!(output.status.success(), "cached venv python should work");
}

#[tokio::test]
async fn test_pip_install_network_isolation() {
    let tempdir = tempfile::tempdir().expect("tempdir");
    let cache_dir = tempdir.path().join("venv-cache");
    let lockfile = tempdir.path().join("requirements.txt");
    std::fs::write(&lockfile, "").expect("write empty lockfile");

    let cache = VenvCache::new(&cache_dir).expect("create cache");
    let venv_path = cache.ensure_venv(&lockfile).await.expect("ensure venv");

    let backend = BwrapBackend::new();
    let profile = SandboxProfile {
        stack: "python".to_string(),
        wall_timeout_secs: 30,
        memory_limit_bytes: 256 * 1024 * 1024,
        tmpfs_size_bytes: 64 * 1024 * 1024,
        tasks_max: 10,
        read_only_mounts: vec![],
        dependency_cache_path: PathBuf::new(),
    };

    let workdir = backend.prepare(&profile, b"").await.expect("prepare");

    // Try to install a package - should fail due to network isolation
    // Use python3 -m pip instead of pip directly to avoid shebang path issues
    let result = backend
        .run_with_venv(&workdir, "python3 -m pip install requests", venv_path.clone())
        .await
        .expect("run");

    // The command should fail (non-zero exit) because network is disabled
    assert_ne!(result.exit_code, 0, "pip install should fail without network");
    assert!(
        result.stderr.contains("Could not find a version") ||
        result.stderr.contains("No matching distribution") ||
        result.stderr.contains("network") ||
        result.stderr.contains("Connection") ||
        result.stderr.contains("resolve"),
        "error should mention network/package resolution failure, got: {}",
        result.stderr
    );

    backend.collect(workdir).await.expect("collect");
}
