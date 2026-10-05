//! TTFT latency budget harness (e4-5, AD-2 / Gate 4).
//!
//! The gateway exit gate is: **p95 added time-to-first-token < 50 ms under 20 concurrent
//! streams**. "Added" is the gateway path minus the direct-to-upstream path, so provider
//! slowness does not fail the gate — only the proxy's own overhead does.
//!
//! This module holds the measurement *math* (percentiles, budget comparison) as pure,
//! exactly-assertable functions; the load-driving harness lives in the integration test
//! (`tests/e4_5_latency_budget_in_ci.rs`), where failing the p95 assertion fails the build —
//! the CI gate the story asks for.

/// The latency budget: maximum p95 added TTFT before CI fails.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatencyBudget {
    pub max_added_p95_ms: f64,
}

impl Default for LatencyBudget {
    /// Gate 4: 50 ms p95 added TTFT.
    fn default() -> Self {
        Self {
            max_added_p95_ms: 50.0,
        }
    }
}

/// One budget evaluation: the reported percentiles for both paths plus the added latency.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Budget {
    pub direct_p50_ms: f64,
    pub direct_p95_ms: f64,
    pub gateway_p50_ms: f64,
    pub gateway_p95_ms: f64,
    pub added_p50_ms: f64,
    pub added_p95_ms: f64,
}

impl Budget {
    /// True when the p95 added TTFT does not exceed the budget ("exceeds 50 ms" fails,
    /// exactly 50 ms passes).
    pub fn within(&self, budget: LatencyBudget) -> bool {
        self.added_p95_ms <= budget.max_added_p95_ms
    }
}

/// Nearest-rank percentile over an already-sorted sample list: the value at
/// `ceil(p/100 * n)` (1-based). Empty input yields 0.0. Consistent with the calibration
/// percentile in `evoswarm-sandbox` (e0-9) so the two harnesses agree on definitions.
pub fn percentile_ms(sorted_samples: &[f64], percent: f64) -> f64 {
    let n = sorted_samples.len();
    if n == 0 {
        return 0.0;
    }
    let rank = ((percent / 100.0) * n as f64).ceil() as usize;
    let idx = rank.clamp(1, n) - 1;
    sorted_samples[idx]
}

/// Evaluates the budget from two already-sorted TTFT sample lists (milliseconds): the
/// direct-to-upstream path and the gateway path. Added latency is the difference of the
/// corresponding percentiles, floored at 0 (a gateway that measured *faster* than direct
/// — noise — reports zero added latency rather than a negative figure).
pub fn ttft_budget_check(sorted_direct_ms: &[f64], sorted_gateway_ms: &[f64]) -> Budget {
    let direct_p50 = percentile_ms(sorted_direct_ms, 50.0);
    let direct_p95 = percentile_ms(sorted_direct_ms, 95.0);
    let gateway_p50 = percentile_ms(sorted_gateway_ms, 50.0);
    let gateway_p95 = percentile_ms(sorted_gateway_ms, 95.0);
    Budget {
        direct_p50_ms: direct_p50,
        direct_p95_ms: direct_p95,
        gateway_p50_ms: gateway_p50,
        gateway_p95_ms: gateway_p95,
        added_p50_ms: (gateway_p50 - direct_p50).max(0.0),
        added_p95_ms: (gateway_p95 - direct_p95).max(0.0),
    }
}

/// One-line human report for CI logs.
pub fn format_budget(b: &Budget, budget: LatencyBudget) -> String {
    format!(
        "TTFT direct p50={:.2}ms p95={:.2}ms | gateway p50={:.2}ms p95={:.2}ms | added p50={:.2}ms p95={:.2}ms | budget p95<={:.0}ms | {}",
        b.direct_p50_ms,
        b.direct_p95_ms,
        b.gateway_p50_ms,
        b.gateway_p95_ms,
        b.added_p50_ms,
        b.added_p95_ms,
        budget.max_added_p95_ms,
        if b.within(budget) { "PASS" } else { "FAIL" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_nearest_rank() {
        let s: Vec<f64> = (1..=20).map(|i| i as f64).collect();
        assert_eq!(percentile_ms(&s, 50.0), 10.0);
        assert_eq!(percentile_ms(&s, 95.0), 19.0);
        assert_eq!(percentile_ms(&[], 95.0), 0.0);
        assert_eq!(percentile_ms(&[7.5], 50.0), 7.5);
    }

    #[test]
    fn added_latency_floors_at_zero() {
        let direct: Vec<f64> = (10..30).map(|i| i as f64).collect();
        let gateway: Vec<f64> = (1..21).map(|i| i as f64).collect(); // faster than direct
        let b = ttft_budget_check(&direct, &gateway);
        assert_eq!(b.added_p50_ms, 0.0);
        assert_eq!(b.added_p95_ms, 0.0);
        assert!(b.within(LatencyBudget::default()));
    }

    #[test]
    fn budget_boundary_is_inclusive() {
        let mut b = Budget {
            direct_p50_ms: 0.0,
            direct_p95_ms: 0.0,
            gateway_p50_ms: 0.0,
            gateway_p95_ms: 50.0,
            added_p50_ms: 0.0,
            added_p95_ms: 50.0,
        };
        assert!(b.within(LatencyBudget::default()));
        b.added_p95_ms = 50.0001;
        assert!(!b.within(LatencyBudget::default()));
    }

    #[test]
    fn report_contains_pass_or_fail() {
        let b = ttft_budget_check(&[1.0, 2.0], &[3.0, 4.0]);
        assert!(format_budget(&b, LatencyBudget::default()).contains("PASS"));
        let over = ttft_budget_check(&[1.0, 2.0], &[80.0, 90.0]);
        assert!(format_budget(&over, LatencyBudget::default()).contains("FAIL"));
    }
}
