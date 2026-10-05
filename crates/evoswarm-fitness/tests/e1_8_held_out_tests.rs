//! e1-8: Held-out tests.
//!
//! Verifies the stable ~80/20 partition, the prompt-leakage barrier, and the selection loop's
//! fallback ladder and no-verified-winner outcome. The selection loop lives in the engine, so
//! this fitness test dev-depends on it and drives it through a scripted held-out evaluator.

use std::collections::HashSet;

use async_trait::async_trait;
use evoswarm_core::{Candidate, HoldoutReason, SelectionOutcome, TestSplit};
use evoswarm_engine::{
    select_verified_winner, HeldOutEvaluator, ScoredCandidate, SelectionDeps, SelectionError,
};
use evoswarm_fitness::{assert_no_leakage, assignment_hash, partition};

fn suite(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("test_{i:03}")).collect()
}

/// AC1: about 20% are held out using a stable split keyed by job id. Verified across suite
/// sizes 5, 20 and 100, and the split must be identical on every recomputation.
#[test]
fn test_stable_partitioning_ratio() {
    for (n, expected_held_out) in [(5usize, 1usize), (20, 4), (100, 20)] {
        let s = suite(n);
        let split = partition("job-42", &s);

        // H = max(1, floor(0.20 * M)) = floor(n/5) for these sizes.
        assert_eq!(
            split.held_out_count(),
            expected_held_out,
            "held-out count for M={n}"
        );
        // Roughly 20% held out, 80% visible, and together they cover the suite exactly once.
        assert_eq!(split.visible.len() + split.held_out.len(), n);
        assert_eq!(split.reason, HoldoutReason::SufficientTests);

        // Stability: recomputing for the same job yields the identical partition.
        let again = partition("job-42", &s);
        assert_eq!(split, again, "split must be stable for job-42 at M={n}");

        // Disjointness: no test is both visible and held out.
        for name in &split.held_out {
            assert!(!split.visible.contains(name), "{name} in both sets");
        }
    }

    // A different job id produces a different assignment, proving the split is keyed by job.
    let a = partition("job-A", &suite(20));
    let b = partition("job-B", &suite(20));
    assert_ne!(a.held_out, b.held_out, "split must be keyed by job id");
}

/// AC1 support: the assignment hash itself is stable and job-scoped, which is the mechanism
/// the partition relies on.
#[test]
fn test_assignment_hash_is_stable_and_job_scoped() {
    let h1 = assignment_hash("job-1", "test_a");
    let h2 = assignment_hash("job-1", "test_a");
    assert_eq!(h1, h2, "same job+name must hash identically");
    assert_ne!(
        h1,
        assignment_hash("job-2", "test_a"),
        "hash must be keyed by job id"
    );
    assert_ne!(
        h1,
        assignment_hash("job-1", "test_b"),
        "hash must depend on the test name"
    );
}

/// AC2: no held-out test content appears in any prompt. The scan runs over the assembled
/// payload, so a leaked name or assertion body in an interpolated dump is caught.
#[test]
fn test_zero_held_out_prompt_leakage() {
    let split = partition("job-7", &suite(20));
    assert!(!split.held_out.is_empty(), "need a held-out set to test");

    // Assemble a prompt from ONLY the visible tests: it must pass the leak scan.
    let clean_prompt = format!(
        "Improve performance. Visible tests: {}",
        split.visible.join(", ")
    );
    let held_pairs: Vec<(String, String)> = split
        .held_out
        .iter()
        .map(|n| (n.clone(), format!("assert {n}_result == expected")))
        .collect();
    assert_no_leakage(clean_prompt.as_bytes(), &held_pairs)
        .expect("a prompt built only from visible tests must not leak");

    // A prompt that accidentally interpolates a held-out NAME must be rejected.
    let leaked_name = format!("context: also consider {}", split.held_out[0]);
    let err = assert_no_leakage(leaked_name.as_bytes(), &held_pairs)
        .expect_err("held-out name in prompt must be detected");
    assert!(err.leaked.contains(&split.held_out[0]));

    // A prompt that interpolates held-out ASSERTION text (e.g. an error dump) is also caught.
    let leaked_body = format!("error dump: assert {}_result == expected", split.held_out[0]);
    assert_no_leakage(leaked_body.as_bytes(), &held_pairs)
        .expect_err("held-out assertion text in prompt must be detected");
}

/// A scripted held-out evaluator: passes exactly the candidate ids in `passing`.
struct ScriptedEvaluator {
    passing: HashSet<String>,
}

#[async_trait]
impl HeldOutEvaluator for ScriptedEvaluator {
    async fn passes_all_held_out(&self, candidate: &Candidate) -> Result<bool, SelectionError> {
        Ok(self.passing.contains(&candidate.id))
    }
}

fn scored(id: &str, score: f64) -> ScoredCandidate {
    ScoredCandidate {
        candidate: Candidate {
            id: id.into(),
            patch: format!("patch-{id}").into_bytes(),
            diff_hash: [0; 32],
            generation: 0,
            parent_ids: Vec::new(),
            model_id: "m".into(),
            prompt_hash: "h".into(),
        },
        score,
    }
}

fn holdout_split() -> TestSplit {
    TestSplit {
        visible: vec!["test_a".into()],
        held_out: vec!["test_secret".into()],
        reason: HoldoutReason::SufficientTests,
    }
}

/// AC3: the top-ranked candidate failing held-out causes the runner to fall back and select
/// the next candidate that passes.
#[tokio::test]
async fn test_candidate_fallback_ladder() {
    // c1 has the best score but fails held-out; c2 is next and passes.
    let ranked = vec![
        scored("c1", 0.95),
        scored("c2", 0.80),
        scored("c3", 0.60),
    ];
    let eval = ScriptedEvaluator {
        passing: ["c2".to_string()].into(),
    };
    let deps = SelectionDeps { evaluator: &eval };
    let outcome = select_verified_winner(&ranked, &holdout_split(), &deps)
        .await
        .expect("selection ok");

    assert!(outcome.is_verified(), "c2 must be a verified winner");
    assert_eq!(outcome.candidate().id, "c2", "must fall back past c1 to c2");
}

/// AC4: when every candidate fails held-out, the job reports no verified winner and flags the
/// best-effort patch (the highest-scoring gated candidate).
#[tokio::test]
async fn test_no_verified_winner_status() {
    let ranked = vec![scored("c1", 0.95), scored("c2", 0.80)];
    let eval = ScriptedEvaluator {
        passing: HashSet::new(),
    };
    let deps = SelectionDeps { evaluator: &eval };
    let outcome = select_verified_winner(&ranked, &holdout_split(), &deps)
        .await
        .expect("selection ok");

    assert!(!outcome.is_verified());
    match outcome {
        SelectionOutcome::NoVerifiedWinner { best_effort } => {
            // The highest-scoring candidate is flagged as the best-effort patch.
            assert_eq!(best_effort.id, "c1");
            assert_eq!(best_effort.patch, b"patch-c1");
        }
        other => panic!("expected NoVerifiedWinner, got {other:?}"),
    }
}

/// AC5: a suite with fewer than 5 trusted tests yields an empty held-out set and records the
/// reason. Selection over such a split can never claim a verified winner.
#[tokio::test]
async fn test_small_suite_yields_empty_holdout() {
    let split = partition("job-tiny", &suite(3));
    assert_eq!(split.held_out_count(), 0, "H must be 0 for M < 5");
    assert!(split.is_empty_holdout());
    assert_eq!(split.reason, HoldoutReason::SuiteTooSmall, "reason recorded");
    assert_eq!(split.visible.len(), 3, "all tests stay visible");

    // Even a top-scoring candidate cannot be "verified" without a held-out signal.
    let ranked = vec![scored("c1", 0.99)];
    let eval = ScriptedEvaluator {
        passing: ["c1".to_string()].into(),
    };
    let deps = SelectionDeps { evaluator: &eval };
    let outcome = select_verified_winner(&ranked, &split, &deps)
        .await
        .expect("selection ok");
    assert!(
        !outcome.is_verified(),
        "an empty holdout cannot verify a winner"
    );
    assert_eq!(outcome.candidate().id, "c1");
}
