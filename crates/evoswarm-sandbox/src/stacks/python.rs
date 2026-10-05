//! Python stack support (e0-5).
//!
//! - `lockfile_hash`: SHA-256 of a lockfile for venv cache keying.
//! - `VenvCache`: creates/reuses an offline read-only venv keyed by lockfile hash.
//! - `parse_junit_xml`: parses pytest JUnit XML into pass/fail/skip counts.

use std::path::{Path, PathBuf};

/// Compute SHA-256 hex digest of a lockfile. Used to key the venv cache.
pub fn lockfile_hash(lockfile: &Path) -> Result<String, std::io::Error> {
    use sha2::{Digest, Sha256};
    let content = std::fs::read(lockfile)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Cache for Python virtual environments, keyed by lockfile hash.
///
/// Creates venvs outside the sandbox (with network access) and reuses them
/// across runs. The venv is mounted read-only inside the sandbox to prevent
/// pip install from modifying it.
pub struct VenvCache {
    cache_dir: PathBuf,
}

impl VenvCache {
    /// Create a new venv cache at the specified directory.
    pub fn new(cache_dir: &Path) -> Result<Self, std::io::Error> {
        std::fs::create_dir_all(cache_dir)?;
        Ok(Self {
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    /// Ensure a venv exists for the given lockfile, creating it if necessary.
    ///
    /// Returns the path to the venv directory. If a venv with the same lockfile
    /// hash already exists, it is reused.
    pub async fn ensure_venv(&self, lockfile: &Path) -> Result<PathBuf, std::io::Error> {
        let hash = lockfile_hash(lockfile)?;
        let venv_path = self.cache_dir.join(&hash);

        // If venv already exists, reuse it
        if venv_path.exists() && venv_path.join("bin/python").exists() {
            return Ok(venv_path);
        }

        // Create new venv
        let status = tokio::process::Command::new("python3")
            .arg("-m")
            .arg("venv")
            .arg(&venv_path)
            .status()
            .await?;

        if !status.success() {
            return Err(std::io::Error::other("Failed to create venv"));
        }

        // Install dependencies from lockfile
        let lockfile_content = tokio::fs::read_to_string(lockfile).await?;
        if !lockfile_content.trim().is_empty() {
            let pip_path = venv_path.join("bin/pip");
            let status = tokio::process::Command::new(&pip_path)
                .arg("install")
                .arg("-r")
                .arg(lockfile)
                .status()
                .await?;

            if !status.success() {
                return Err(std::io::Error::other("Failed to install dependencies"));
            }
        }

        Ok(venv_path)
    }
}

/// Per-test outcome parsed from JUnit XML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestOutcome {
    pub classname: String,
    pub name: String,
    pub status: TestStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
}

/// Aggregate counts from a JUnit XML run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JunitSummary {
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub total: u32,
}

/// Parse a pytest JUnit XML string into a summary and per-test outcomes.
///
/// Hand-rolled to avoid pulling in a full XML parser dependency for this
/// simple, well-structured format.
pub fn parse_junit_xml(xml: &str) -> Result<JunitSummary, String> {
    let mut summary = JunitSummary::default();

    // Find all <testcase ...> elements. We handle self-closing and paired tags.
    for tc in extract_testcases(xml) {
        let classname = attr(&tc, "classname").unwrap_or_default();
        let name = attr(&tc, "name").unwrap_or_default();

        let status = if tc.contains("<failure") || tc.contains("<error") {
            TestStatus::Failed
        } else if tc.contains("<skipped") {
            TestStatus::Skipped
        } else {
            TestStatus::Passed
        };

        match status {
            TestStatus::Passed => summary.passed += 1,
            TestStatus::Failed => summary.failed += 1,
            TestStatus::Skipped => summary.skipped += 1,
        }
        summary.total += 1;

        let _ = (classname, name); // consumed for counting; full outcomes available via `parse_junit_outcomes`
    }

    Ok(summary)
}

/// Parse JUnit XML into per-test outcomes.
pub fn parse_junit_outcomes(xml: &str) -> Result<Vec<TestOutcome>, String> {
    let mut outcomes = Vec::new();
    for tc in extract_testcases(xml) {
        let classname = attr(&tc, "classname").unwrap_or_default();
        let name = attr(&tc, "name").unwrap_or_default();
        let status = if tc.contains("<failure") || tc.contains("<error") {
            TestStatus::Failed
        } else if tc.contains("<skipped") {
            TestStatus::Skipped
        } else {
            TestStatus::Passed
        };
        outcomes.push(TestOutcome {
            classname,
            name,
            status,
        });
    }
    Ok(outcomes)
}

fn extract_testcases(xml: &str) -> Vec<String> {
    let mut results = Vec::new();
    let mut search_from = 0;
    while let Some(start) = xml[search_from..].find("<testcase") {
        let abs_start = search_from + start;
        // Find the end: either self-closing "/>" or closing "</testcase>"
        if let Some(self_close) = xml[abs_start..].find("/>") {
            let close_tag = xml[abs_start..].find("</testcase>").map(|p| p + 12);
            let end = match close_tag {
                Some(ct) if ct < self_close => ct,
                _ => self_close + 2,
            };
            results.push(xml[abs_start..abs_start + end].to_string());
            search_from = abs_start + end;
        } else {
            break;
        }
    }
    results
}

fn attr(tag: &str, name: &str) -> Option<String> {
    // Look for the attribute with proper boundary checking to avoid matching
    // substrings (e.g., "name" should not match "classname").
    let pattern = format!(" {name}=\"");
    let start = tag.find(&pattern)? + pattern.len();
    let end = tag[start..].find('"')?;
    Some(tag[start..start + end].to_string())
}
