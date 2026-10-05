use evoswarm_core::{Candidate, ExecutionResult, RunStatus};
use evoswarm_fitness::baseline::Baseline;
use evoswarm_fitness::gates::{evaluate, GateFailure};
use std::path::PathBuf;

fn make_candidate() -> Candidate {
    Candidate {
        id: "c1".to_string(),
        patch: vec![],
        diff_hash: [0u8; 32],
        generation: 0,
        parent_ids: vec![],
        model_id: "test".to_string(),
        prompt_hash: "abc".to_string(),
    }
}

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

#[test]
fn test_build_error_gate() {
    let candidate = make_candidate();
    let run = make_run(1, RunStatus::Failed);
    let baseline = make_baseline(5, 0, vec![]);

    let result = evaluate(&candidate, &run, &baseline, &[], &[], 5, 0);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Build));
    assert!(result.failed_tests.is_empty());
}

#[test]
fn test_assertion_failure_gate() {
    let candidate = make_candidate();
    let run = make_run(0, RunStatus::Success);
    let trusted_names = vec![
        "test1".to_string(),
        "test2".to_string(),
        "test3".to_string(),
    ];
    let baseline = make_baseline(3, 0, trusted_names);
    let passed_tests = vec!["test1".to_string(), "test2".to_string()];

    let result = evaluate(&candidate, &run, &baseline, &[], &passed_tests, 3, 0);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::TestFailure));
    assert_eq!(result.failed_tests, vec!["test3".to_string()]);
}

#[test]
fn test_harness_tamper_gate() {
    let candidate = make_candidate();
    let run = make_run(0, RunStatus::Success);
    let baseline = make_baseline(2, 0, vec!["test1".to_string(), "test2".to_string()]);
    let passed_tests = vec!["test1".to_string(), "test2".to_string()];
    let patch_paths = vec![
        PathBuf::from("src/main.rs"),
        PathBuf::from("conftest.py"),
    ];

    let result = evaluate(&candidate, &run, &baseline, &patch_paths, &passed_tests, 2, 0);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Tamper));
}

#[test]
fn test_skipped_test_detection() {
    let candidate = make_candidate();
    let run = make_run(0, RunStatus::Success);
    let baseline = make_baseline(5, 0, vec![]);
    let passed_tests: Vec<String> = vec![];

    let result = evaluate(&candidate, &run, &baseline, &[], &passed_tests, 4, 0);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Skipped));
}

#[test]
fn test_gate_short_circuit_order() {
    let candidate = make_candidate();
    let run = make_run(1, RunStatus::Failed);
    let baseline = make_baseline(5, 0, vec![]);
    let patch_paths = vec![PathBuf::from("conftest.py")];

    let result = evaluate(&candidate, &run, &baseline, &patch_paths, &[], 5, 0);
    assert!(!result.passed);
    assert_eq!(result.reason, Some(GateFailure::Build));
}

#[test]
fn test_tamper_path_component_anchored() {
    let candidate = make_candidate();
    let run = make_run(0, RunStatus::Success);
    let baseline = make_baseline(2, 0, vec!["test1".to_string(), "test2".to_string()]);
    let passed_tests = vec!["test1".to_string(), "test2".to_string()];
    let patch_paths = vec![
        PathBuf::from("src/main.rs"),
        PathBuf::from("mytests/helper.rs"),
    ];

    let result = evaluate(&candidate, &run, &baseline, &patch_paths, &passed_tests, 2, 0);
    assert!(result.passed, "mytests/ should not trigger tamper gate");
}
