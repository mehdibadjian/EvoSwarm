//! Tamper-proof test harness (e0-7).
//!
//! Two layers of defence:
//! 1. The tests directory is bind-mounted read-only (`--ro-bind`) so any
//!    candidate write into `/work/tests` fails with EROFS.
//! 2. Harness override files (`conftest.py`, `Directory.Build.props`,
//!    `pytest.ini`) are rejected by a path-component anchored check so a
//!    candidate cannot weaken the test harness by dropping a file at the
//!    workdir root.

use std::collections::HashSet;
use std::path::Path;

/// Harness override filenames that are prohibited in the candidate workdir
/// root. A candidate that drops one of these is attempting to weaken the
/// test harness (AD-5).
pub const PROHIBITED_HARNESS_FILES: &[&str] = &[
    "conftest.py",
    "Directory.Build.props",
    "pytest.ini",
];

/// Returns `true` if `workdir` contains any prohibited harness override file
/// at its root (path-component anchored, not substring).
pub fn detect_harness_override(workdir: &Path) -> bool {
    let prohibited: HashSet<&str> = PROHIBITED_HARNESS_FILES.iter().copied().collect();
    if let Ok(entries) = std::fs::read_dir(workdir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if prohibited.contains(name) {
                    return true;
                }
            }
        }
    }
    false
}

/// Compute a SHA-256 hash of all files in `tests_dir` (recursively). Used to
/// verify the tests directory is unchanged after a run.
pub fn hash_tests_dir(tests_dir: &Path) -> Result<String, std::io::Error> {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    collect_paths(tests_dir, &mut paths)?;
    paths.sort();

    for p in paths {
        if p.is_file() {
            let rel = p.strip_prefix(tests_dir).unwrap_or(&p);
            hasher.update(rel.to_string_lossy().as_bytes());
            let content = std::fs::read(&p)?;
            hasher.update(&content);
        }
    }

    Ok(format!("{:x}", hasher.finalize()))
}

fn collect_paths(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), std::io::Error> {
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                collect_paths(&path, out)?;
            } else {
                out.push(path);
            }
        }
    }
    Ok(())
}
