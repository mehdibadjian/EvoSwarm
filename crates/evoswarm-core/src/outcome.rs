//! Selection outcome (e1-8, AD-5): the result of evaluating ranked candidates against the
//! held-out test set. A winner is `Verified` only when it passes 100% of the held-out tests;
//! otherwise the best gated candidate is emitted as `NoVerifiedWinner` and flagged
//! best-effort, never silently promoted to verified.

use serde::{Deserialize, Serialize};

use crate::candidate::Candidate;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum SelectionOutcome {
    /// A candidate passed every held-out test.
    Verified { candidate: Candidate },
    /// No candidate passed the held-out set; the highest-scoring gated candidate is offered
    /// as best-effort and must be surfaced as unverified.
    NoVerifiedWinner { best_effort: Candidate },
}

impl SelectionOutcome {
    pub fn is_verified(&self) -> bool {
        matches!(self, SelectionOutcome::Verified { .. })
    }

    /// The candidate carried by either variant (the verified winner, or the best-effort one).
    pub fn candidate(&self) -> &Candidate {
        match self {
            SelectionOutcome::Verified { candidate } => candidate,
            SelectionOutcome::NoVerifiedWinner { best_effort } => best_effort,
        }
    }
}
