//! Held-out partitioning (e1-8 §1): a stable ~80/20 split of the trusted suite keyed by the
//! job id. Assignment uses SHA-256(job_id + test_name) and holds out the `H` lowest hashes,
//! so the same job always recomputes the same partition — the property that makes e1-12
//! resume safe. The `M < 5` rule is applied before hashing so a tiny suite never loses a test.

use evoswarm_core::{HoldoutReason, TestSplit};
use sha2::{Digest, Sha256};

/// Below this trusted-suite size nothing is held out: withholding tests from a tiny suite
/// would leave too few visible tests to drive the search.
const MIN_SUITE_FOR_HOLDOUT: usize = 5;

/// The held-out fraction (spec: H = max(1, floor(0.20 * M))).
const HOLDOUT_RATIO_NUM: usize = 1;
const HOLDOUT_RATIO_DEN: usize = 5;

/// Computes the 32-byte assignment hash for a test name under a job. Concatenating the job id
/// and test name with a separator keeps assignments job-scoped and collision-resistant.
pub fn assignment_hash(job_id: &str, test_name: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(job_id.as_bytes());
    hasher.update([0u8]);
    hasher.update(test_name.as_bytes());
    hasher.finalize().into()
}

/// Partitions `trusted` into visible and held-out sets for `job_id`, stably.
///
/// - If `M < 5`, every test is visible and the reason is `SuiteTooSmall` (AC5).
/// - Otherwise `H = max(1, floor(M/5))` tests with the lowest assignment hashes are held out
///   and the rest are visible, with the reason `SufficientTests`.
///
/// Both vectors are returned sorted by name so the split is deterministic regardless of the
/// input order.
pub fn partition(job_id: &str, trusted: &[String]) -> TestSplit {
    if trusted.len() < MIN_SUITE_FOR_HOLDOUT {
        let mut visible = trusted.to_vec();
        visible.sort();
        return TestSplit {
            visible,
            held_out: Vec::new(),
            reason: HoldoutReason::SuiteTooSmall,
        };
    }

    let h = (trusted.len() * HOLDOUT_RATIO_NUM / HOLDOUT_RATIO_DEN).max(1);

    // Rank tests by their assignment hash (ascending); the lowest `h` are held out.
    let mut ranked: Vec<(String, [u8; 32])> = trusted
        .iter()
        .map(|name| (name.clone(), assignment_hash(job_id, name)))
        .collect();
    ranked.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let mut held_out: Vec<String> = ranked.iter().take(h).map(|(n, _)| n.clone()).collect();
    let mut visible: Vec<String> = ranked.iter().skip(h).map(|(n, _)| n.clone()).collect();
    held_out.sort();
    visible.sort();

    TestSplit {
        visible,
        held_out,
        reason: HoldoutReason::SufficientTests,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suite(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("test_{i:03}")).collect()
    }

    #[test]
    fn partition_is_stable_across_calls() {
        let s = suite(20);
        let a = partition("job-1", &s);
        let b = partition("job-1", &s);
        assert_eq!(a, b, "same job + suite must yield the identical split");
    }

    #[test]
    fn partition_is_order_independent() {
        let s = suite(20);
        let mut shuffled = s.clone();
        shuffled.reverse();
        assert_eq!(
            partition("job-1", &s),
            partition("job-1", &shuffled),
            "input order must not change the split"
        );
    }

    #[test]
    fn held_out_is_disjoint_and_complete() {
        let s = suite(20);
        let split = partition("job-1", &s);
        assert_eq!(split.visible.len() + split.held_out.len(), s.len());
        for name in &split.held_out {
            assert!(!split.visible.contains(name), "{name} in both sets");
        }
    }

    #[test]
    fn different_jobs_yield_different_splits() {
        let s = suite(20);
        let a = partition("job-1", &s);
        let b = partition("job-2", &s);
        // The held-out membership differs because the hash is keyed by job id.
        assert_ne!(a.held_out, b.held_out);
    }
}
