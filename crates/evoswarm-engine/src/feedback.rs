//! Failure feedback assembly and clamping (e1-4 §1, AD-6).
//!
//! The mutator is shown *why* the parent failed: compiler output and the first failing assertion
//! with its stack trace. Both are captured but hard-clamped (compiler ≤ 2k tokens, test ≤ 2k
//! tokens, total ≤ 4k tokens) so a runaway dump cannot blow the e1-10 budget. Every cut appends
//! an explicit truncation marker — a silent cut would mislead the model into believing it has the
//! complete diagnostic. Token counting uses the same `estimate_tokens` as the budget projection
//! so the clamp and the caps never disagree.

use evoswarm_core::ExecutionResult;
use evoswarm_models::prompt::estimate_tokens;

/// Appended whenever a section is cut, so the model knows the dump is incomplete.
pub const TRUNCATION_MARKER: &str = "\n[... truncated ...]";

/// Per-section token clamp (spec §1): compiler errors and test failures each cap at 2,000.
const SECTION_TOKEN_BUDGET: usize = 2_000;
/// Total error-context hard clamp (spec §1).
pub const TOTAL_TOKEN_BUDGET: usize = 4_000;

/// The failure diagnostics extracted from a sandbox run, split so the compiler section can be
/// rendered before the assertion section (spec §1 ordering).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailureFeedback {
    /// Compiler / stderr output.
    pub compiler: String,
    /// Test-failure output: first failing assertion, test name and stack trace (stdout).
    pub test_failure: String,
}

/// Builds feedback from a run: stderr is the compiler section, stdout carries the failing test,
/// assertion and stack trace.
pub fn build(result: &ExecutionResult) -> FailureFeedback {
    FailureFeedback {
        compiler: result.stderr.clone(),
        test_failure: result.stdout.clone(),
    }
}

/// Renders the feedback clamped to `budget` tokens total. Each section is first clamped to
/// `SECTION_TOKEN_BUDGET`, then the concatenation is hard-clamped to `budget`. A truncation
/// marker is appended at every cut, including the final one, so the render always ends with the
/// marker when anything was dropped.
pub fn render_clamped(fb: &FailureFeedback, budget: usize) -> String {
    let (compiler, _) = clamp_to_tokens(&fb.compiler, SECTION_TOKEN_BUDGET);
    let (test, _) = clamp_to_tokens(&fb.test_failure, SECTION_TOKEN_BUDGET);

    let mut out = String::new();
    out.push_str("## Compiler errors\n");
    out.push_str(&compiler);
    out.push_str("\n\n## Failing test\n");
    out.push_str(&test);

    // Final hard clamp on the whole context (spec §1: total ≤ budget).
    let (clamped, _) = clamp_to_tokens(&out, budget);
    clamped
}

/// Clamps `text` to at most `budget` tokens (~4 bytes/token). Returns the text unchanged when it
/// already fits; otherwise truncates at a char boundary, reserving room for the marker, and
/// appends `TRUNCATION_MARKER`. The second tuple element reports whether a cut happened.
fn clamp_to_tokens(text: &str, budget: usize) -> (String, bool) {
    if estimate_tokens(text.as_bytes()) <= budget {
        return (text.to_string(), false);
    }
    let max_bytes = budget.saturating_mul(4);
    let marker = TRUNCATION_MARKER;
    // Reserve the marker's own bytes so prefix + marker stays within max_bytes (== budget tokens).
    let keep = max_bytes.saturating_sub(marker.len());
    let mut cut = keep.min(text.len());
    // Back off to a char boundary so we never split a UTF-8 sequence.
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    (format!("{}{}", &text[..cut], marker), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result(stderr: &str, stdout: &str) -> ExecutionResult {
        ExecutionResult {
            exit_code: 1,
            stdout: stdout.into(),
            stderr: stderr.into(),
            wall_time_ms: 1,
            peak_memory_bytes: 0,
            status: evoswarm_core::RunStatus::Failed,
        }
    }

    #[test]
    fn short_feedback_is_not_truncated() {
        let fb = build(&result("err", "test_x FAILED"));
        let r = render_clamped(&fb, TOTAL_TOKEN_BUDGET);
        assert!(!r.contains(TRUNCATION_MARKER));
        assert!(r.contains("err"));
        assert!(r.contains("test_x FAILED"));
    }

    #[test]
    fn clamp_respects_token_budget() {
        let text = "y".repeat(10_000);
        let (clamped, truncated) = clamp_to_tokens(&text, 100);
        assert!(truncated);
        assert!(estimate_tokens(clamped.as_bytes()) <= 100);
        assert!(clamped.ends_with(TRUNCATION_MARKER));
    }
}
