//! Usage accounting (e1-10, AD-6): the shared vocabulary for budget projection and
//! enforcement. `Usage` is the actual spend reported after a model response; the engine's
//! budget guard reconciles it against the caps monotonically.
//!
//! `CallRequest` (the *projected* spend of a call, tagged with its `Role`) lives in the
//! engine's budget module rather than here: `Role` is owned by `evoswarm-models`, and core
//! must stay the dependency leaf so `models` can later depend on core without a cycle.

use serde::{Deserialize, Serialize};

/// Actual token and cost usage reported after a model response.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
}

impl Usage {
    pub fn zero() -> Self {
        Self {
            tokens_in: 0,
            tokens_out: 0,
            cost_usd: 0.0,
        }
    }

    pub fn total_tokens(&self) -> u64 {
        self.tokens_in + self.tokens_out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_total_sums_in_and_out() {
        let u = Usage {
            tokens_in: 100,
            tokens_out: 40,
            cost_usd: 0.5,
        };
        assert_eq!(u.total_tokens(), 140);
    }

    #[test]
    fn zero_usage_is_zero() {
        assert_eq!(Usage::zero().total_tokens(), 0);
        assert_eq!(Usage::zero().cost_usd, 0.0);
    }
}
