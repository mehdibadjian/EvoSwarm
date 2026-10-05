//! Held-out test split (e1-8, AD-5): the visible tests a candidate may be prompted with and
//! the held-out set used only to verify the winner. The split is persisted with the job so
//! e1-12 resume reuses the identical partition instead of recomputing from a possibly-changed
//! suite. Defined here (not in fitness) so the engine's selection loop can consume it
//! without depending on the fitness crate.

use serde::{Deserialize, Serialize};

/// Why a suite yielded an empty held-out set, recorded for auditability (e1-8 AC5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldoutReason {
    /// The suite had at least 5 trusted tests; ~20% were held out.
    SufficientTests,
    /// Fewer than 5 trusted tests: holding out would leave too few visible tests to drive
    /// search, so the held-out set is empty.
    SuiteTooSmall,
}

/// A stable partition of the trusted suite into visible and held-out test names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestSplit {
    /// Tests the candidate may be prompted with (roughly 80%).
    pub visible: Vec<String>,
    /// Tests withheld and used only to verify the winner (roughly 20%, or empty if the suite
    /// is too small).
    pub held_out: Vec<String>,
    /// Why the held-out set has the size it does.
    pub reason: HoldoutReason,
}

impl TestSplit {
    pub fn held_out_count(&self) -> usize {
        self.held_out.len()
    }

    /// True when nothing was held out, meaning every candidate is trivially "verified"
    /// against an empty held-out set (the caller decides how to treat a tiny suite).
    pub fn is_empty_holdout(&self) -> bool {
        self.held_out.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_serialises_roundtrip() {
        let split = TestSplit {
            visible: vec!["a".into(), "b".into()],
            held_out: vec!["c".into()],
            reason: HoldoutReason::SufficientTests,
        };
        let json = serde_json::to_string(&split).expect("serialise");
        let back: TestSplit = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(back, split);
    }

    #[test]
    fn empty_holdout_reason_roundtrips() {
        let split = TestSplit {
            visible: vec!["a".into()],
            held_out: vec![],
            reason: HoldoutReason::SuiteTooSmall,
        };
        assert!(split.is_empty_holdout());
        let json = serde_json::to_string(&split).expect("serialise");
        assert!(json.contains("suite_too_small"));
    }
}
