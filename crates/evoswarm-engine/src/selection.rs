//! Held-out selection loop (e1-8 §3): rank candidates that passed the hard gates by score,
//! then walk them in order running the held-out tests in the sandbox. The first candidate that
//! passes 100% of the held-out set is the Verified Winner; if the list is exhausted, emit
//! `NoVerifiedWinner` with the highest-scoring gated candidate flagged best-effort.
//!
//! The held-out run is behind the `HeldOutEvaluator` seam so the loop is deterministic in
//! tests and never depends on a live sandbox here; e1-11 wires it to the real backend.

use async_trait::async_trait;
use evoswarm_core::{Candidate, SelectionOutcome, TestSplit};
use thiserror::Error;

/// A candidate paired with the score it earned from the e1-7 weighted scoring. The selection
/// loop consumes these already ranked by score descending; it re-sorts defensively so a caller
/// that passes an unranked list still gets correct behaviour.
#[derive(Debug, Clone, PartialEq)]
pub struct ScoredCandidate {
    pub candidate: Candidate,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Error)]
#[error("held-out evaluation failed: {0}")]
pub struct SelectionError(String);

/// The seam that runs the held-out tests for one candidate. Returns `Ok(true)` when the
/// candidate passes 100% of the held-out set, `Ok(false)` when any held-out test fails.
#[async_trait]
pub trait HeldOutEvaluator: Send + Sync {
    async fn passes_all_held_out(&self, candidate: &Candidate) -> Result<bool, SelectionError>;
}

/// Dependencies for the selection loop. The evaluator is injected; the split tells the loop
/// whether there is anything to hold out at all.
pub struct SelectionDeps<'a, E: HeldOutEvaluator + ?Sized> {
    pub evaluator: &'a E,
}

/// Selects the verified winner from `ranked` (score descending). Walks the list running the
/// held-out tests per candidate and returns the first that passes all of them. When every
/// candidate fails, returns `NoVerifiedWinner` carrying the highest-scoring candidate as the
/// best-effort patch.
///
/// An empty held-out set (a suite too small to withhold from, e1-8 AC5) cannot verify
/// generalisation: the top candidate is returned as best-effort `NoVerifiedWinner` rather than
/// being falsely promoted to `Verified`.
pub async fn select_verified_winner<E: HeldOutEvaluator + ?Sized>(
    ranked: &[ScoredCandidate],
    split: &TestSplit,
    deps: &SelectionDeps<'_, E>,
) -> Result<SelectionOutcome, SelectionError> {
    if ranked.is_empty() {
        return Err(SelectionError("no gated candidates to select from".into()));
    }

    // Defensive re-sort by score descending so the best-effort fallback and the walk order are
    // correct even if the caller passed an unranked list.
    let mut ordered: Vec<&ScoredCandidate> = ranked.iter().collect();
    ordered.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // With nothing held out there is no generalisation signal, so no candidate can be verified.
    if split.is_empty_holdout() {
        return Ok(SelectionOutcome::NoVerifiedWinner {
            best_effort: ordered[0].candidate.clone(),
        });
    }

    for scored in &ordered {
        if deps.evaluator.passes_all_held_out(&scored.candidate).await? {
            return Ok(SelectionOutcome::Verified {
                candidate: scored.candidate.clone(),
            });
        }
    }

    // Exhausted the list: the highest-scoring gated candidate is offered as best-effort.
    Ok(SelectionOutcome::NoVerifiedWinner {
        best_effort: ordered[0].candidate.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use evoswarm_core::HoldoutReason;
    use std::collections::HashSet;

    /// An evaluator that passes exactly the candidates whose id is in `passing`.
    struct ScriptedEvaluator {
        passing: HashSet<String>,
    }

    #[async_trait]
    impl HeldOutEvaluator for ScriptedEvaluator {
        async fn passes_all_held_out(&self, candidate: &Candidate) -> Result<bool, SelectionError> {
            Ok(self.passing.contains(&candidate.id))
        }
    }

    fn cand(id: &str) -> Candidate {
        Candidate {
            id: id.into(),
            patch: Vec::new(),
            diff_hash: [0; 32],
            generation: 0,
            parent_ids: Vec::new(),
            model_id: "m".into(),
            prompt_hash: "h".into(),
        }
    }

    fn scored(id: &str, score: f64) -> ScoredCandidate {
        ScoredCandidate {
            candidate: cand(id),
            score,
        }
    }

    fn split_with_holdout() -> TestSplit {
        TestSplit {
            visible: vec!["a".into()],
            held_out: vec!["secret".into()],
            reason: HoldoutReason::SufficientTests,
        }
    }

    #[tokio::test]
    async fn top_candidate_that_passes_is_verified() {
        let ranked = vec![scored("c1", 0.9), scored("c2", 0.5)];
        let eval = ScriptedEvaluator {
            passing: ["c1".to_string()].into(),
        };
        let deps = SelectionDeps { evaluator: &eval };
        let out = select_verified_winner(&ranked, &split_with_holdout(), &deps)
            .await
            .unwrap();
        assert!(out.is_verified());
        assert_eq!(out.candidate().id, "c1");
    }

    #[tokio::test]
    async fn falls_back_to_next_candidate() {
        // c1 (top score) fails held-out; c2 passes → c2 is the verified winner.
        let ranked = vec![scored("c1", 0.9), scored("c2", 0.5)];
        let eval = ScriptedEvaluator {
            passing: ["c2".to_string()].into(),
        };
        let deps = SelectionDeps { evaluator: &eval };
        let out = select_verified_winner(&ranked, &split_with_holdout(), &deps)
            .await
            .unwrap();
        assert!(out.is_verified());
        assert_eq!(out.candidate().id, "c2");
    }

    #[tokio::test]
    async fn no_winner_when_all_fail() {
        let ranked = vec![scored("c1", 0.9), scored("c2", 0.5)];
        let eval = ScriptedEvaluator {
            passing: HashSet::new(),
        };
        let deps = SelectionDeps { evaluator: &eval };
        let out = select_verified_winner(&ranked, &split_with_holdout(), &deps)
            .await
            .unwrap();
        assert!(!out.is_verified());
        match out {
            SelectionOutcome::NoVerifiedWinner { best_effort } => {
                // The highest-scoring candidate is flagged best-effort.
                assert_eq!(best_effort.id, "c1");
            }
            _ => panic!("expected NoVerifiedWinner"),
        }
    }

    #[tokio::test]
    async fn empty_holdout_yields_best_effort_not_verified() {
        let ranked = vec![scored("c1", 0.9)];
        let split = TestSplit {
            visible: vec!["a".into(), "b".into()],
            held_out: vec![],
            reason: HoldoutReason::SuiteTooSmall,
        };
        let eval = ScriptedEvaluator {
            passing: ["c1".to_string()].into(),
        };
        let deps = SelectionDeps { evaluator: &eval };
        let out = select_verified_winner(&ranked, &split, &deps).await.unwrap();
        // No held-out signal → cannot claim verification, even though the evaluator would pass.
        assert!(!out.is_verified());
        assert_eq!(out.candidate().id, "c1");
    }

    #[tokio::test]
    async fn empty_ranked_list_is_an_error() {
        let eval = ScriptedEvaluator {
            passing: HashSet::new(),
        };
        let deps = SelectionDeps { evaluator: &eval };
        let err = select_verified_winner(&[], &split_with_holdout(), &deps)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no gated candidates"));
    }
}
