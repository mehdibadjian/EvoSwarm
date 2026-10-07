//! e0-6 C# stack — acceptance criteria (story §4, AD-1).
//!
//! Red-phase tests authored before the production `stacks::csharp` module exists.
//! Gate: `python3 scripts/sprint.py verify --cmd "cargo test --test e0_6_csharp_stack" --anti-cheat`
//!
//! SEAM: `dotnet` SDK is not installed in this environment. The pure logic:
//! - NuGet cache key computation from packages.lock.json or project file
//! - Isolated per-hash NuGet cache directory resolution
//! - Offline `dotnet test --no-restore` command construction
//! - TRX test result parser (xUnit and Reqnroll output)
//! - Stale cache diagnostic detection ("restore cache stale" instead of generic build failure)
//! is certified deterministically here.

use std::path::Path;
use evoswarm_sandbox::stacks::csharp::{
    nuget_cache_dir, offline_test_command, parse_trx_xml, packages_hash,
    NuGetCache, NuGetCacheError,
};

fn write_file(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, body).expect("write file");
    p
}

/// AC1: Given a project's lock or package references, NuGet restore cache is keyed by their hash.
#[test]
fn test_nuget_cache_restore_keying() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let content = r#"{ "version": 1, "dependencies": { "xunit": "2.4.2" } }"#;

    let f1 = write_file(tmp.path(), "packages.lock.json", content);
    let sub = tmp.path().join("sub");
    std::fs::create_dir_all(&sub).expect("mkdir");
    let f2 = write_file(&sub, "packages.lock.json", content);

    let k1 = packages_hash(&f1).expect("hash 1");
    let k2 = packages_hash(&f2).expect("hash 2");
    assert_eq!(k1, k2, "identical content hashes identically");
    assert_eq!(k1.len(), 64, "SHA-256 hex digest length");

    let f3 = write_file(tmp.path(), "other.lock.json", r#"{ "dependencies": {} }"#);
    let k3 = packages_hash(&f3).expect("hash 3");
    assert_ne!(k1, k3, "different content produces different hash");
}

/// AC1: nuget_cache_dir is key-scoped under cache root.
#[test]
fn test_nuget_cache_dir_is_key_scoped() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cache = NuGetCache::new(tmp.path()).expect("cache");
    let d1 = nuget_cache_dir(cache.root(), "abc123hash");
    assert_eq!(d1, tmp.path().join("abc123hash"));
    assert_eq!(cache.path_for("abc123hash"), d1);
}

/// AC1: Cache-hit reuses existing cache directory without invoking dotnet restore.
#[tokio::test]
async fn test_nuget_cache_hit_reuses_without_rebuild() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let cache_root = tmp.path().join("nuget_cache");
    let cache = NuGetCache::new(&cache_root).expect("cache");

    let lockfile = write_file(
        tmp.path(),
        "packages.lock.json",
        r#"{ "version": 1, "dependencies": { "Reqnroll": "2.0.0" } }"#,
    );
    let key = packages_hash(&lockfile).expect("hash");

    let seeded = cache.path_for(&key);
    std::fs::create_dir_all(seeded.join("packages")).expect("seed packages dir");
    assert!(cache.has(&key), "seeded cache is present");

    let snap = cache
        .ensure(&lockfile)
        .await
        .expect("cache hit must not fail");
    assert_eq!(snap.key, key);
    assert_eq!(snap.dir, seeded);
    assert!(snap.reused, "cache was reused, not rebuilt");
}

/// AC2: When cache exists, offline dotnet test runs with --no-restore and packages dir mount.
#[test]
fn test_offline_dotnet_test_command() {
    let cmd = offline_test_command("/deps/packages");
    assert!(cmd.contains("dotnet test"), "runs dotnet test: {cmd}");
    assert!(cmd.contains("--no-restore"), "forbids restore: {cmd}");
    assert!(
        cmd.contains("--packages /deps/packages"),
        "points to read-only packages mount: {cmd}"
    );
    assert!(
        cmd.contains("--logger \"trx\""),
        "logs to TRX logger: {cmd}"
    );
}

/// AC2: TRX XML results from xUnit / Reqnroll parse into test summary.
#[test]
fn test_trx_parsing_summary() {
    let trx_xml = r#"<?xml version="1.0" encoding="utf-8"?>
<TestRun id="1" name="run" xmlns="http://microsoft.com/schemas/VisualStudio/TeamTest/2010">
  <ResultSummary outcome="Failed">
    <Counters total="5" executed="5" passed="3" failed="1" error="0" timeout="0" aborted="0" inconclusive="0" passedButRunAborted="0" notRunnable="0" notExecuted="0" disconnected="0" warning="0" completed="0" inProgress="0" pending="0" />
  </ResultSummary>
</TestRun>"#;

    let summary = parse_trx_xml(trx_xml).expect("parse trx");
    assert_eq!(summary.total, 5);
    assert_eq!(summary.passed, 3);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.skipped, 1); // 5 - executed(5) or total - (passed + failed)
}

/// AC3: Given a candidate references a package missing from cache, detects 'restore cache stale'.
#[test]
fn test_missing_package_stale_cache_error() {
    let stderr_sample = "error NU1101: Unable to find package FooBar. No packages exist with this id in source(s): /deps/packages\nBuild FAILED.";
    let detected = NuGetCacheError::classify_diagnostic(stderr_sample);
    assert!(detected.is_stale_cache(), "must classify NU1101 as stale cache");
    assert_eq!(detected.message(), "restore cache stale");

    let stderr_another = "error NETSDK1004: Assets file project.assets.json not found. Run a NuGet package restore.";
    let detected2 = NuGetCacheError::classify_diagnostic(stderr_another);
    assert!(detected2.is_stale_cache(), "must classify NETSDK1004 as stale cache");

    let normal_compile_error = "error CS0103: The name 'xyz' does not exist in the current context";
    let detected3 = NuGetCacheError::classify_diagnostic(normal_compile_error);
    assert!(!detected3.is_stale_cache(), "CS0103 is a standard compilation error");
}
