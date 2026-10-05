use evoswarm_core::{ExecutionResult, RunStatus};
use evoswarm_fitness::baseline::Baseline;
use evoswarm_fitness::gates::{evaluate, GateFailure, GateInput};
use std::path::PathBuf;

fn make_run(exit_code: i32, status: RunStatus) -> ExecutionResult {
    ExecutionResult {
        exit_code,
        stdout: String::new(),
        stderr: String::new(),
        wall_time_ms: 100,
        peak_memory_bytes: 0,
        status,
    }
}

fn make_baseline(test_count: u32, skipped: u32, trusted_names: Vec<String>) -> Baseline {
    Baseline {
        test_count,
        skipped,
        trusted_names,
    }
}

fn allowed() -> Vec<PathBuf> {
    vec![PathBuf::from("src")]
}

#[test]
fn test_build_error_gate() {
    let run = make_run(1, RunStatus::Failed);
    let baseline = make_baseline(5, 0, vec![]);
    let no_paths: Vec<PathBuf> = vec![];
    let no_tests: Vec<String> = vec![];

    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &no_paths,
        allowed_paths: &allowed(),
        passed_tests: &no_tests,
        executed_count: 5,
        skipped_count: 0,
    };
    let result = evaluate(&input);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Build));
    assert!(result.failed_tests.is_empty());
}

#[test]
fn test_assertion_failure_gate() {
    let run = make_run(0, RunStatus::Success);
    let trusted_names = vec!["test1".to_string(), "test2".to_string(), "test3".to_string()];
    let baseline = make_baseline(3, 0, trusted_names);
    let passed_tests = vec!["test1".to_string(), "test2".to_string()];
    let no_paths: Vec<PathBuf> = vec![];

    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &no_paths,
        allowed_paths: &allowed(),
        passed_tests: &passed_tests,
        executed_count: 3,
        skipped_count: 0,
    };
    let result = evaluate(&input);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::TestFailure));
    assert_eq!(result.failed_tests, vec!["test3".to_string()]);
}

#[test]
fn test_harness_tamper_gate() {
    let run = make_run(0, RunStatus::Success);
    let baseline = make_baseline(2, 0, vec!["test1".to_string(), "test2".to_string()]);
    let passed_tests = vec!["test1".to_string(), "test2".to_string()];
    let patch_paths = vec![PathBuf::from("src/main.rs"), PathBuf::from("conftest.py")];

    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &patch_paths,
        allowed_paths: &allowed(),
        passed_tests: &passed_tests,
        executed_count: 2,
        skipped_count: 0,
    };
    let result = evaluate(&input);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Tamper));
}

#[test]
fn test_build_script_tamper_gate() {
    let run = make_run(0, RunStatus::Success);
    let baseline = make_baseline(1, 0, vec!["test1".to_string()]);
    let passed_tests = vec!["test1".to_string()];
    // A Makefile edit is a build-script touch (AC3), and it is outside the src allowlist.
    let patch_paths = vec![PathBuf::from("src/main.rs"), PathBuf::from("Makefile")];

    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &patch_paths,
        allowed_paths: &allowed(),
        passed_tests: &passed_tests,
        executed_count: 1,
        skipped_count: 0,
    };
    let result = evaluate(&input);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Tamper));
}

#[test]
fn test_diff_outside_allowlist_gate() {
    let run = make_run(0, RunStatus::Success);
    let baseline = make_baseline(1, 0, vec!["test1".to_string()]);
    let passed_tests = vec!["test1".to_string()];
    // `docs/notes.md` is neither protected nor within the `src` allowlist: Gate 3 must
    // reject it because the diff is required to touch only paths in --paths.
    let patch_paths = vec![PathBuf::from("src/main.rs"), PathBuf::from("docs/notes.md")];

    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &patch_paths,
        allowed_paths: &allowed(),
        passed_tests: &passed_tests,
        executed_count: 1,
        skipped_count: 0,
    };
    let result = evaluate(&input);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Tamper));
}

#[test]
fn test_skipped_test_detection() {
    let run = make_run(0, RunStatus::Success);
    let baseline = make_baseline(5, 0, vec![]);
    let no_paths: Vec<PathBuf> = vec![];
    let no_tests: Vec<String> = vec![];

    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &no_paths,
        allowed_paths: &allowed(),
        passed_tests: &no_tests,
        executed_count: 4,
        skipped_count: 0,
    };
    let result = evaluate(&input);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Skipped));
}

#[test]
fn test_new_skip_relative_to_baseline() {
    let run = make_run(0, RunStatus::Success);
    let baseline = make_baseline(5, 0, vec![]);
    let no_paths: Vec<PathBuf> = vec![];
    let no_tests: Vec<String> = vec![];

    // Same executed count but a new skip appeared (1 > baseline 0).
    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &no_paths,
        allowed_paths: &allowed(),
        passed_tests: &no_tests,
        executed_count: 5,
        skipped_count: 1,
    };
    let result = evaluate(&input);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Skipped));
}

#[test]
fn test_gate_short_circuit_order() {
    let run = make_run(1, RunStatus::Failed);
    let baseline = make_baseline(5, 0, vec![]);
    // Build fails AND tamper present: the earliest gate (build) must be reported.
    let patch_paths = vec![PathBuf::from("conftest.py")];
    let no_tests: Vec<String> = vec![];

    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &patch_paths,
        allowed_paths: &allowed(),
        passed_tests: &no_tests,
        executed_count: 5,
        skipped_count: 0,
    };
    let result = evaluate(&input);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Build));
}

#[test]
fn test_tamper_path_component_anchored() {
    let run = make_run(0, RunStatus::Success);
    let baseline = make_baseline(2, 0, vec!["test1".to_string(), "test2".to_string()]);
    let passed_tests = vec!["test1".to_string(), "test2".to_string()];
    // `mytests/helper.rs` contains "tests" as a substring but is not the tests dir, and
    // `src/tests_helper.rs` lives inside the allowlist — neither may be flagged.
    let patch_paths = vec![
        PathBuf::from("src/tests_helper.rs"),
        PathBuf::from("mytests/helper.rs"),
    ];
    let allowed_paths = vec![PathBuf::from("src"), PathBuf::from("mytests")];

    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &patch_paths,
        allowed_paths: &allowed_paths,
        passed_tests: &passed_tests,
        executed_count: 2,
        skipped_count: 0,
    };
    let result = evaluate(&input);
    assert!(result.passed, "lookalike paths should not trigger tamper gate");
}

#[test]
fn test_clean_candidate_passes_all_gates() {
    let run = make_run(0, RunStatus::Success);
    let baseline = make_baseline(2, 0, vec!["test1".to_string(), "test2".to_string()]);
    let passed_tests = vec!["test1".to_string(), "test2".to_string()];
    let patch_paths = vec![PathBuf::from("src/main.rs")];

    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &patch_paths,
        allowed_paths: &allowed(),
        passed_tests: &passed_tests,
        executed_count: 2,
        skipped_count: 0,
    };
    let result = evaluate(&input);
    assert!(result.passed);
    assert_eq!(result.reason, None);
}
