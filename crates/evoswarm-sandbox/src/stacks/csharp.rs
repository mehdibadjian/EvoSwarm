//! C# stack support (e0-6, AD-1).
//!
//! Prepares a NuGet package cache snapshot keyed by SHA-256 hash of `packages.lock.json`
//! (or `.csproj`), then runs `dotnet test --no-restore` inside bwrap with no network
//! and the snapshot mounted read-only at `/deps/packages`.
//! Parses TRX test results from xUnit / Reqnroll and detects stale cache diagnostics.
//!
//! ## Containment honesty (roadmap §6, SEAM)
//!
//! `dotnet` SDK is absent in this environment. Live execution is deferred.
//! Pure logic:
//! - Hash-based package cache keying ([`packages_hash`])
//! - Key-scoped cache paths ([`nuget_cache_dir`], [`NuGetCache::path_for`])
//! - Cache-hit reuse without restore invocation ([`NuGetCache::ensure`])
//! - Offline command builder ([`offline_test_command`])
//! - TRX XML parsing ([`parse_trx_xml`])
//! - Stale cache diagnostic detection ([`NuGetCacheError::classify_diagnostic`])

use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Intended hard memory cap for a C# run (story §2: 2 GB).
pub const CSHARP_MEMORY_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// The mount point inside the sandbox where the read-only NuGet cache is bound.
pub const NUGET_MOUNT: &str = "/deps/packages";

/// Computes SHA-256 hex digest of lockfile or project file content for cache keying.
pub fn packages_hash(path: &Path) -> Result<String, std::io::Error> {
    let content = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// The snapshot directory for `key` under `cache_root`.
pub fn nuget_cache_dir(cache_root: &Path, key: &str) -> PathBuf {
    cache_root.join(key)
}

/// The offline `dotnet test` invocation used inside the sandbox.
pub fn offline_test_command(packages_dir: &str) -> String {
    format!("dotnet test --no-restore --packages {packages_dir} --logger \"trx\"")
}

/// A prepared NuGet cache snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NuGetSnapshot {
    pub key: String,
    pub dir: PathBuf,
    pub reused: bool,
}

/// Cache of NuGet packages keyed by package reference hash.
pub struct NuGetCache {
    root: PathBuf,
}

impl NuGetCache {
    pub fn new(cache_root: &Path) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(cache_root)?;
        Ok(Self {
            root: cache_root.to_path_buf(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path_for(&self, key: &str) -> PathBuf {
        nuget_cache_dir(&self.root, key)
    }

    pub fn has(&self, key: &str) -> bool {
        self.path_for(key).join("packages").is_dir()
    }

    pub async fn ensure(&self, lockfile: &Path) -> Result<NuGetSnapshot, std::io::Error> {
        let key = packages_hash(lockfile)?;
        let dir = self.path_for(&key);

        if self.has(&key) {
            return Ok(NuGetSnapshot {
                key,
                dir,
                reused: true,
            });
        }

        // Live restore would run `dotnet restore --packages <dir>/packages` outside sandbox.
        // For offline / test stub:
        std::fs::create_dir_all(dir.join("packages"))?;
        Ok(NuGetSnapshot {
            key,
            dir,
            reused: false,
        })
    }
}

/// Summary counts from a TRX test execution report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrxSummary {
    pub total: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
}

/// Parses TRX XML string into TrxSummary.
pub fn parse_trx_xml(xml: &str) -> Result<TrxSummary, String> {
    // Look for <Counters total="X" passed="Y" failed="Z" ... />
    let total_re = regex_find_attr(xml, "total").unwrap_or(0);
    let passed_re = regex_find_attr(xml, "passed").unwrap_or(0);
    let failed_re = regex_find_attr(xml, "failed").unwrap_or(0);
    let _executed_re = regex_find_attr(xml, "executed").unwrap_or(total_re);

    let skipped = total_re.saturating_sub(passed_re + failed_re);

    Ok(TrxSummary {
        total: total_re,
        passed: passed_re,
        failed: failed_re,
        skipped,
    })
}

fn regex_find_attr(xml: &str, attr: &str) -> Option<u32> {
    let needle = format!("{attr}=\"");
    if let Some(pos) = xml.find(&needle) {
        let rest = &xml[pos + needle.len()..];
        if let Some(end) = rest.find('"') {
            return rest[..end].parse().ok();
        }
    }
    None
}

/// Classification of NuGet cache errors.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum NuGetCacheError {
    #[error("restore cache stale")]
    StaleCache,
    #[error("build error: {0}")]
    Other(String),
}

impl NuGetCacheError {
    pub fn is_stale_cache(&self) -> bool {
        matches!(self, NuGetCacheError::StaleCache)
    }

    pub fn message(&self) -> &'static str {
        match self {
            NuGetCacheError::StaleCache => "restore cache stale",
            NuGetCacheError::Other(_) => "build error",
        }
    }

    /// Inspects stderr for missing package / assets errors indicating stale cache.
    pub fn classify_diagnostic(stderr: &str) -> Self {
        if stderr.contains("NU1101")
            || stderr.contains("NU1102")
            || stderr.contains("NETSDK1004")
            || stderr.contains("Unable to find package")
            || stderr.contains("Run a NuGet package restore")
        {
            NuGetCacheError::StaleCache
        } else {
            NuGetCacheError::Other(stderr.to_string())
        }
    }
}
