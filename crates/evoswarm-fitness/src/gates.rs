use evoswarm_core::{Candidate, ExecutionResult, RunStatus};
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

/// Evaluates all four hard gates in order: build → test → tamper → skipped.
/// Short-circuits on first failure.
pub fn evaluate(
    _candidate: &Candidate,
    run: &ExecutionResult,
    baseline: &Baseline,
    patch_paths: &[PathBuf],
    passed_tests: &[String],
    executed_count: usize,
    skipped_count: usize,
) -> GateResult {
    // Gate 1: Build must succeed
    if run.exit_code != 0 || run.status != RunStatus::Success {
        return GateResult {
            passed: false,
            reason: Some(GateFailure::Build),
            failed_tests: vec![],
        };
    }

    // Gate 2: All trusted tests must pass
    let failed: Vec<String> = baseline
        .trusted_names
        .iter()
        .filter(|name| !passed_tests.contains(name))
        .cloned()
        .collect();
    
    if !failed.is_empty() {
        return GateResult {
            passed: false,
            reason: Some(GateFailure::TestFailure),
            failed_tests: failed,
        };
    }

    // Gate 3: No tampering with test harness
    let offending = tamper::detect(patch_paths);
    if !offending.is_empty() {
        return GateResult {
            passed: false,
            reason: Some(GateFailure::Tamper),
            failed_tests: vec![],
        };
    }

    // Gate 4: No skipped tests (executed >= baseline count, no new skips)
    if executed_count < baseline.test_count as usize || skipped_count > baseline.skipped as usize {
        return GateResult {
            passed: false,
            reason: Some(GateFailure::Skipped),
            failed_tests: vec![],
        };
    }

    GateResult {
        passed: true,
        reason: None,
        failed_tests: vec![],
    }
}
