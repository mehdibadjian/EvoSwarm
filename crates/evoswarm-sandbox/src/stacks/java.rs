//! Java stack support (e5-3, AD-1).
//!
//! Prepares a `.m2` repository snapshot keyed by the SHA-256 hash of `pom.xml`, then runs
//! `mvn -o test` inside bwrap with no network and the snapshot mounted read-only as the local
//! Maven repository. Maven Surefire emits JUnit XML, so test-result parsing reuses the shared
//! [`super::python::parse_junit_xml`].
//!
//! ## Containment honesty (roadmap §5, SEAM)
//!
//! `java`/`mvn` are present in this environment, but two halves are **deferred**:
//! - The live `mvn -o test` execution needs a *primed* `~/.m2` (dependency download → network);
//!   building a snapshot is the [`M2Cache::ensure`] cache-miss path, which shells out to
//!   `mvn dependency:go-offline`. It is not exercised in CI (no network for a clean build).
//! - The **2 GB memory cap** (story §2) needs writable cgroup v2 delegation from e0-4, absent
//!   here, so [`java_profile`] records the intended limit but it cannot be enforced yet.
//!
//! What *is* certified here is the pure logic: pom-hash snapshot keying ([`snapshot_key`]),
//! key-scoped cache paths ([`m2_cache_dir`], [`M2Cache::path_for`]), cache-hit reuse without a
//! rebuild ([`M2Cache::ensure`]), and the offline command builder ([`offline_test_command`]).

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Intended hard memory cap for a Java run (story §2: 2 GB). Not enforceable until e0-4
/// provides cgroup v2 delegation; recorded in the profile regardless.
pub const JAVA_MEMORY_LIMIT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// The mount point inside the sandbox where the read-only `.m2` snapshot is bound.
pub const M2_MOUNT: &str = "/deps";

/// SHA-256 hex digest of `pom.xml`, used as the `.m2` snapshot cache key. Two projects with
/// byte-identical poms share a snapshot; any pom change invalidates it.
pub fn snapshot_key(pom: &Path) -> Result<String, std::io::Error> {
    let content = std::fs::read(pom)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// The snapshot directory for `key` under `cache_root`.
pub fn m2_cache_dir(cache_root: &Path, key: &str) -> PathBuf {
    cache_root.join(key)
}

/// The offline `mvn test` invocation used inside the sandbox. `-o` forbids network, `-B` is
/// batch (non-interactive) mode, and `-Dmaven.repo.local` points Maven at the read-only
/// snapshot mounted at `mount` so the build never reaches Maven Central.
pub fn offline_test_command(mount: &str) -> String {
    format!("mvn -B -o -Dmaven.repo.local={mount} test")
}

/// A prepared `.m2` snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct M2Snapshot {
    /// The pom.xml hash key identifying this snapshot.
    pub key: String,
    /// Host path to the snapshot directory (mounted read-only at [`M2_MOUNT`]).
    pub dir: PathBuf,
    /// True when an existing snapshot was reused; false when this call built it.
    pub reused: bool,
}

/// Cache of `.m2` repository snapshots, keyed by pom.xml hash. Snapshots are built *outside* the
/// sandbox (where network is available) and reused across runs, mounted read-only inside so a
/// candidate cannot poison the dependency cache.
pub struct M2Cache {
    root: PathBuf,
}

impl M2Cache {
    /// Creates a cache rooted at `cache_root`.
    pub fn new(cache_root: &Path) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(cache_root)?;
        Ok(Self {
            root: cache_root.to_path_buf(),
        })
    }

    /// The cache root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The snapshot directory for `key`.
    pub fn path_for(&self, key: &str) -> PathBuf {
        m2_cache_dir(&self.root, key)
    }

    /// True when a snapshot for `key` already exists and looks populated.
    pub fn has(&self, key: &str) -> bool {
        self.path_for(key).join("repository").is_dir()
    }

    /// Ensures a snapshot exists for `pom`'s key. On a cache hit the existing directory is
    /// returned without invoking Maven (`reused = true`). On a miss it runs
    /// `mvn dependency:go-offline` to populate the snapshot (`reused = false`) — the live,
    /// network-dependent half that is not exercised in CI.
    pub async fn ensure(&self, pom: &Path) -> Result<M2Snapshot, std::io::Error> {
        let key = snapshot_key(pom)?;
        let dir = self.path_for(&key);

        if self.has(&key) {
            return Ok(M2Snapshot {
                key,
                dir,
                reused: true,
            });
        }

        // Cache miss: build the snapshot outside the sandbox. Requires network (the SEAM half).
        std::fs::create_dir_all(dir.join("repository"))?;
        let project_dir = pom.parent().unwrap_or(Path::new("."));
        let status = tokio::process::Command::new("mvn")
            .arg("-B")
            .arg("dependency:go-offline")
            .arg(format!(
                "-Dmaven.repo.local={}",
                dir.join("repository").display()
            ))
            .current_dir(project_dir)
            .status()
            .await?;
        if !status.success() {
            return Err(std::io::Error::other("failed to build .m2 snapshot"));
        }

        Ok(M2Snapshot {
            key,
            dir,
            reused: false,
        })
    }
}

/// Builds the Java [`evoswarm_core::SandboxProfile`]: the offline test command shape, the
/// read-only snapshot mount, and the intended 2 GB memory cap. The memory cap is recorded but
/// not enforceable until e0-4 supplies cgroup v2 delegation (SEAM).
pub fn java_profile() -> evoswarm_core::SandboxProfile {
    evoswarm_core::SandboxProfile {
        stack: "java".into(),
        wall_timeout_secs: 300,
        memory_limit_bytes: JAVA_MEMORY_LIMIT_BYTES,
        tmpfs_size_bytes: 512 * 1024 * 1024,
        tasks_max: 64,
        read_only_mounts: vec![PathBuf::from("/usr/lib/jvm")],
        dependency_cache_path: PathBuf::from(M2_MOUNT),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_command_is_offline_and_scoped() {
        let cmd = offline_test_command("/deps");
        assert!(cmd.contains("-o"));
        assert!(cmd.contains("-Dmaven.repo.local=/deps"));
        assert!(cmd.ends_with("test"));
    }

    #[test]
    fn m2_cache_dir_joins_root_and_key() {
        assert_eq!(
            m2_cache_dir(Path::new("/root"), "k"),
            PathBuf::from("/root/k")
        );
    }

    #[test]
    fn java_profile_records_intended_cap() {
        let p = java_profile();
        assert_eq!(p.stack, "java");
        assert_eq!(p.memory_limit_bytes, JAVA_MEMORY_LIMIT_BYTES);
        assert_eq!(p.dependency_cache_path, PathBuf::from(M2_MOUNT));
        assert!(p.read_only_mounts.contains(&PathBuf::from("/usr/lib/jvm")));
    }
}
