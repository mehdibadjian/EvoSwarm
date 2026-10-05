//! Adversary test model (e1-9 §1/§3).
//!
//! An adversary test is authored by the adversary role to probe a candidate for weaknesses. It
//! is *never* a trusted test: it cannot affect a hard gate (e1-6) and contributes only to the
//! score's adversary term A (e1-7). The `origin` tag is what lets gates and scoring filter by
//! provenance — an untagged test is rejected at ingest, because a boolean flag on a shared list
//! is one forgotten filter away from letting generated tests veto user code.

use serde::{Deserialize, Serialize};

use crate::provenance::TestOrigin;

/// The lifecycle status of an adversary test through the e1-9 filter pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdversaryStatus {
    /// Survived compilation and is not suspect; contributes to scoring term A.
    Valid,
    /// Failed to compile in the sandbox; discarded before execution.
    Discarded,
    /// Failed on the baseline and every Gen 0 candidate; excluded from A as untrustworthy.
    Suspect,
}

/// One adversary-authored test function, tagged with its origin and filter status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdversaryTest {
    pub name: String,
    pub body: String,
    /// Always `TestOrigin::Adversary` for a test produced by the adversary role; the tag is what
    /// keeps it out of gates. Ingest rejects a test whose origin is not set.
    pub origin: TestOrigin,
    pub status: AdversaryStatus,
}

impl AdversaryTest {
    /// True when this test is eligible to contribute to scoring term A.
    pub fn is_scoreable(&self) -> bool {
        self.status == AdversaryStatus::Valid
    }

    /// True for a test that must be excluded from every trusted-test count and gate.
    pub fn is_adversary(&self) -> bool {
        self.origin == TestOrigin::Adversary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test(status: AdversaryStatus) -> AdversaryTest {
        AdversaryTest {
            name: "test_adv".into(),
            body: "assert x".into(),
            origin: TestOrigin::Adversary,
            status,
        }
    }

    #[test]
    fn only_valid_tests_are_scoreable() {
        assert!(test(AdversaryStatus::Valid).is_scoreable());
        assert!(!test(AdversaryStatus::Discarded).is_scoreable());
        assert!(!test(AdversaryStatus::Suspect).is_scoreable());
    }

    #[test]
    fn adversary_origin_is_flagged() {
        assert!(test(AdversaryStatus::Valid).is_adversary());
    }
}
