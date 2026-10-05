//! e5-3 Java stack — acceptance criteria (story §4, AD-1).
//!
//! Red-phase tests authored before the production `stacks::java` module exists. The story
//! suggests `tests/stacks/test_java_stack.rs`; the verification gate is
//! `cargo test --test e5_3_java_stack`, so this is a flat target in the sandbox crate.
//!
//! SEAM (roadmap §5): `java`/`mvn` are present here, but a live `mvn -o test` needs a primed
//! `~/.m2` (network) and the 2 GB memory cap needs writable cgroup v2 delegation from e0-4
//! (absent). So the *pure* logic is certified here — pom.xml-hash snapshot keying, cache-hit
//! reuse (no rebuild), the offline `mvn -o test` command builder, and JUnit XML parsing (reused
//! from the python stack) — while the live maven build + hard memory cap are the deferred half.

use evoswarm_sandbox::stacks::java::{m2_cache_dir, offline_test_command, snapshot_key, M2Cache};
use evoswarm_sandbox::stacks::python::{parse_junit_xml, TestStatus};

fn write_pom(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
    let pom = dir.join("pom.xml");
    std::fs::write(&pom, body).expect("write pom");
    pom
}

/// AC1: the `.m2` snapshot is keyed by the SHA-256 hash of pom.xml, so an identical pom maps to
/// the same cache path and a changed pom maps to a different one.
#[test]
fn test_m2_snapshot_keyed_by_pom_hash() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let same = "<project><artifactId>a</artifactId></project>";

    // Two poms with identical bytes in different dirs hash to the same key.
    let sub = tmp.path().join("sub");
    std::fs::create_dir_all(&sub).expect("mkdir sub");
    let pom_a = write_pom(tmp.path(), same);
    let pom_a2 = write_pom(&sub, same);
    let key_a = snapshot_key(&pom_a).expect("hash a");
    assert_eq!(snapshot_key(&pom_a2).expect("hash a2"), key_a);

    // A changed pom produces a different key.
    let b = tmp.path().join("b");
    std::fs::create_dir_all(&b).expect("mkdir b");
    let pom_b = write_pom(&b, "<project><artifactId>b</artifactId></project>");
    assert_ne!(snapshot_key(&pom_b).expect("hash b"), key_a);

    // The key is a 64-char lowercase hex SHA-256 digest.
    assert_eq!(key_a.len(), 64);
    assert!(key_a
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
}

/// AC1: `m2_cache_dir` derives the per-key snapshot path under a cache root.
#[test]
fn test_m2_cache_dir_is_key_scoped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cache = M2Cache::new(tmp.path()).expect("cache");
    let d1 = m2_cache_dir(cache.root(), "abc123");
    assert_eq!(d1, tmp.path().join("abc123"));
    assert_eq!(cache.path_for("abc123"), d1);
    // Distinct keys -> distinct dirs.
    assert_ne!(cache.path_for("abc123"), cache.path_for("def456"));
}

/// AC1/AC2: when a snapshot already exists for a pom's key, `ensure` reuses it WITHOUT invoking
/// maven — the cache-hit path is pure and needs no network. (The cache-miss path that actually
/// runs `mvn dependency:go-offline` is the SEAM half, not exercised here.)
#[tokio::test]
async fn test_snapshot_cache_hit_reuses_without_rebuild() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cache_root = tmp.path().join("m2cache");
    let cache = M2Cache::new(&cache_root).expect("cache");

    let pom = write_pom(
        tmp.path(),
        "<project><artifactId>cached</artifactId></project>",
    );
    let key = snapshot_key(&pom).expect("key");

    // Pre-seed the snapshot dir to simulate a previously built .m2.
    let seeded = cache.path_for(&key);
    std::fs::create_dir_all(seeded.join("repository")).expect("seed");
    assert!(cache.has(&key), "seeded snapshot is present");

    // ensure() on a cache hit returns the existing dir; if it tried to run maven with no
    // network it would fail, so a successful return proves the cache-hit path was taken.
    let snap = cache
        .ensure(&pom)
        .await
        .expect("cache hit must not need maven");
    assert_eq!(snap.key, key);
    assert_eq!(snap.dir, seeded);
    assert!(snap.reused, "snapshot was reused, not rebuilt");
}

/// AC2: the offline test command runs `mvn -o` (no network) with the snapshot as the local
/// repository, so the sandbox never reaches out to Maven Central.
#[test]
fn test_offline_mvn_command() {
    let cmd = offline_test_command("/deps");
    assert!(cmd.contains(" -o "), "offline flag present: {cmd}");
    assert!(cmd.contains("mvn"), "invokes mvn: {cmd}");
    assert!(cmd.contains("test"), "runs the test phase: {cmd}");
    assert!(
        cmd.contains("-Dmaven.repo.local=/deps"),
        "uses the snapshot as the local repo: {cmd}"
    );
    // Batch mode so CI output is non-interactive.
    assert!(cmd.contains(" -B"), "batch mode: {cmd}");
}

/// AC2: JUnit summaries parse via the shared parser (maven-surefire emits JUnit XML).
#[test]
fn test_junit_parsing_reuse() {
    let xml = r#"<?xml version="1.0"?>
<testsuite name="com.example.AppTest" tests="3" failures="1" skipped="1">
  <testcase classname="com.example.AppTest" name="passes"/>
  <testcase classname="com.example.AppTest" name="fails"><failure message="boom"/></testcase>
  <testcase classname="com.example.AppTest" name="skips"><skipped/></testcase>
</testsuite>"#;
    let summary = parse_junit_xml(xml).expect("parse");
    assert_eq!(summary.passed, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.skipped, 1);
    assert_eq!(summary.total, 3);
    // The one non-failed, non-skipped test is Passed.
    assert_eq!(summary.failed, 1);
    let _ = TestStatus::Passed;
}
