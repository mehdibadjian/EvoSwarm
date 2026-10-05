//! Test provenance (e1-5 §1, e1-9 §3).
//!
//! A candidate's *pass vector* records which trusted tests it passed. Crossover ranks parent
//! pairs by the Hamming distance of these vectors so recombination combines complementary
//! strengths; a candidate without recorded provenance is never eligible as a parent, because
//! crossover of unmeasured code cannot be shown to be complementary. `TestOrigin` tags every
//! test as `Trusted` (user-authored, counted by gates) or `Adversary` (model-authored, e1-9),
//! so gates and scoring can filter by origin — an adversary test never affects a hard gate.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// The origin of a test, used to keep adversary-authored tests out of hard gates (e1-9 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestOrigin {
    /// User-authored, part of the trusted suite; counted by e1-6 gates and e1-7 scoring.
    Trusted,
    /// Model-authored adversary test (e1-9); excluded from gates, contributes only to term A.
    Adversary,
}

/// The set of trusted tests a candidate passes. Stored as a sorted set so it is deterministic
/// and JSON-serialisable for the e1-12 resume ledger.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PassVector {
    tests: BTreeSet<String>,
}

impl PassVector {
    /// Builds a pass vector from the trusted test names that passed.
    pub fn new<I, S>(passed: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            tests: passed.into_iter().map(Into::into).collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tests.is_empty()
    }

    pub fn len(&self) -> usize {
        self.tests.len()
    }

    pub fn contains(&self, test: &str) -> bool {
        self.tests.contains(test)
    }

    /// Hamming distance of two pass vectors (spec §1): the size of the symmetric difference —
    /// tests one candidate passes but the other does not, in either direction. A larger distance
    /// means more complementary strengths to recombine.
    pub fn hamming_distance(&self, other: &PassVector) -> usize {
        let a_only = self.tests.difference(&other.tests).count();
        let b_only = other.tests.difference(&self.tests).count();
        a_only + b_only
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pv(names: &[&str]) -> PassVector {
        PassVector::new(names.iter().copied())
    }

    #[test]
    fn distance_is_symmetric_difference_size() {
        let a = pv(&["t1", "t2"]);
        let b = pv(&["t2", "t3"]);
        // a\b = {t1}, b\a = {t3} -> distance 2
        assert_eq!(a.hamming_distance(&b), 2);
        assert_eq!(b.hamming_distance(&a), 2, "distance is symmetric");
    }

    #[test]
    fn identical_vectors_have_zero_distance() {
        let a = pv(&["t1", "t2"]);
        assert_eq!(a.hamming_distance(&a), 0);
    }

    #[test]
    fn disjoint_vectors_distance_is_union_size() {
        let a = pv(&["t1", "t2"]);
        let b = pv(&["t3", "t4"]);
        assert_eq!(a.hamming_distance(&b), 4);
    }

    #[test]
    fn empty_vector_is_empty() {
        assert!(PassVector::default().is_empty());
        assert!(!pv(&["t1"]).is_empty());
    }
}
