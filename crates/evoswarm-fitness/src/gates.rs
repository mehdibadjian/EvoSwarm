use evoswarm_core::{ExecutionResult, RunStatus};
use std::path::PathBuf;

use crate::baseline::Baseline;
use crate::tamper;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateFailure {
    Build,
    TestFailure,
    Tamper,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateResult {
    pub passed: bool,
    pub reason: Option<GateFailure>,
    pub failed_tests: Vec<String>,
}

/// The per-candidate evidence the gate pipeline judges. Bundled into a struct rather than
/// a long positional argument list: several fields share types (`&[String]`, `usize`), so
/// positional args transpose silently. The caller (engine) supplies already-parsed test
/// outcomes so fitness never depends on the sandbox to re-derive them.
#[derive(Debug, Clone)]
pub struct GateInput<'a> {
    /// Sandbox run outcome; its exit code and status drive the build gate.
    pub run: &'a ExecutionResult,
    /// Baseline snapshot captured once at job start.
    pub baseline: &'a Baseline,
    /// Files touched by the candidate patch (from the diff's path list, not the filesystem).
    pub patch_paths: &'a [PathBuf],
    /// Declared `--paths` allowlist Gate 3 enforces the diff against.
    pub allowed_paths: &'a [PathBuf],
    /// Visible trusted tests that passed.
    pub passed_tests: &'a [String],
    /// Number of trusted tests executed on this candidate.
    pub executed_count: usize,
    /// Number of tests skipped/ignored on this candidate.
    pub skipped_count: usize,
}

/// Evaluates all four hard gates in order: build → test → tamper → skipped.
/// Short-circuits on first failure, so the reported reason is always the earliest gate.
/// Pure: identical inputs yield an identical `GateResult`; no config can override a fail.
pub fn evaluate(input: &GateInput<'_>) -> GateResult {
    let baseline = input.baseline;

    // Gate 1: Build must succeed (exit 0 and not a resource kill / tamper flag).
    if input.run.exit_code != 0 || input.run.status != RunStatus::Success {
        return reject(GateFailure::Build, vec![]);
    }

    // Gate 2: All trusted tests must pass.
    let failed: Vec<String> = baseline
        .trusted_names
        .iter()
        .filter(|name| !input.passed_tests.contains(name))
        .cloned()
        .collect();
    if !failed.is_empty() {
        return reject(GateFailure::TestFailure, failed);
    }

    // Gate 3: Diff touches no tests/harness/build files and stays within --paths.
    let offending = tamper::detect(input.patch_paths, input.allowed_paths);
    if !offending.is_empty() {
        return reject(GateFailure::Tamper, vec![]);
    }

    // Gate 4: No skipped tests (executed >= baseline count, no new skips).
    if input.executed_count < baseline.test_count as usize
        || input.skipped_count > baseline.skipped as usize
    {
        return reject(GateFailure::Skipped, vec![]);
    }

    GateResult {
        passed: true,
        reason: None,
        failed_tests: vec![],
    }
}

fn reject(reason: GateFailure, failed_tests: Vec<String>) -> GateResult {
    GateResult {
        passed: false,
        reason: Some(reason),
        failed_tests,
    }
}
