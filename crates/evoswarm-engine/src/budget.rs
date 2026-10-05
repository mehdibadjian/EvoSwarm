//! Budget enforcement (e1-10, AD-6): every model call is projected against the token and
//! dollar caps *before* dispatch, so a call that would cross a cap is never sent. Spend is
//! accumulated from provider-reported usage and is monotonic — recorded spend never
//! decreases, which is what lets a resumed job reconcile without resurrecting spent budget.
//!
//! `CallRequest` (the projected spend of a call, tagged with its `Role`) lives here rather
//! than in `evoswarm-core`: `Role` is owned by `evoswarm-models`, and core must stay the
//! dependency leaf. The plan's optional `crossover_calls` counter is intentionally omitted —
//! no e1-10 acceptance criterion depends on it, and e1-5 owns the crossover cap.

use evoswarm_core::Usage;
use evoswarm_models::Role;
use thiserror::Error;

/// A projected model call: what it *would* spend, checked before dispatch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CallRequest {
    pub role: Role,
    /// The role's configured max output tokens (ceiling on generated tokens).
    pub max_tokens: u32,
    /// Estimated prompt tokens for this call.
    pub prompt_tokens: u32,
}

/// Per-role pricing used to project a call's cost, mirroring the e1-2 `RoleConfig` rates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoleRates {
    pub cost_per_million_input: f64,
    pub cost_per_million_output: f64,
}

impl RoleRates {
    /// Projects the dollar cost of a request: input tokens at the input rate plus the
    /// output ceiling at the output rate, both per million.
    pub fn project_cost(&self, req: &CallRequest) -> f64 {
        let input = (req.prompt_tokens as f64 / 1_000_000.0) * self.cost_per_million_input;
        let output = (req.max_tokens as f64 / 1_000_000.0) * self.cost_per_million_output;
        input + output
    }
}

/// The caps a job runs under. `None` means "uncapped" for that dimension.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BudgetCaps {
    pub dollar_cap: Option<f64>,
    pub token_cap: Option<u64>,
}

/// Which cap a pre-dispatch check tripped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExhaustedKind {
    Dollar,
    Token,
}

#[derive(Debug, Clone, PartialEq, Error)]
#[error("budget exhausted ({kind:?}): {message}")]
pub struct BudgetExhausted {
    pub kind: ExhaustedKind,
    pub message: String,
}

/// The single dispatch chokepoint for budget: `pre_dispatch` is called before every model
/// call so no role can bypass the caps. Counters are accumulated via `record`.
#[derive(Debug, Clone)]
pub struct BudgetGuard {
    spent_usd: f64,
    tokens_used: u64,
    caps: BudgetCaps,
    mutator: RoleRates,
    synthesiser: RoleRates,
    adversary: RoleRates,
}

impl BudgetGuard {
    pub fn new(
        caps: BudgetCaps,
        mutator: RoleRates,
        synthesiser: RoleRates,
        adversary: RoleRates,
    ) -> Self {
        Self {
            spent_usd: 0.0,
            tokens_used: 0,
            caps,
            mutator,
            synthesiser,
            adversary,
        }
    }

    fn rates(&self, role: Role) -> RoleRates {
        match role {
            Role::Mutator => self.mutator,
            Role::Synthesiser => self.synthesiser,
            Role::Adversary => self.adversary,
        }
    }

    /// Checks both caps against the projected call before dispatch. Returns `Err` naming the
    /// tripped cap (and the remaining headroom) when the call would cross it, so the loop can
    /// halt as `budget_exhausted` and return the best verified candidate.
    pub fn pre_dispatch(&self, req: &CallRequest) -> Result<(), BudgetExhausted> {
        if let Some(token_cap) = self.caps.token_cap {
            let projected_tokens = self.tokens_used + req.prompt_tokens as u64 + req.max_tokens as u64;
            if projected_tokens > token_cap {
                let headroom = token_cap.saturating_sub(self.tokens_used);
                return Err(BudgetExhausted {
                    kind: ExhaustedKind::Token,
                    message: format!(
                        "projected {} tokens exceed cap {token_cap}; remaining headroom {headroom}",
                        projected_tokens
                    ),
                });
            }
        }
        if let Some(dollar_cap) = self.caps.dollar_cap {
            let projected_cost = self.rates(req.role).project_cost(req);
            let projected_spend = self.spent_usd + projected_cost;
            if projected_spend > dollar_cap {
                let remaining = (dollar_cap - self.spent_usd).max(0.0);
                return Err(BudgetExhausted {
                    kind: ExhaustedKind::Dollar,
                    message: format!(
                        "projected cost ${projected_cost:.6} would exceed cap ${dollar_cap:.6}; \
                         ${remaining:.6} remaining"
                    ),
                });
            }
        }
        Ok(())
    }

    /// Accumulates provider-reported usage after a response. Monotonic by construction:
    /// usage is non-negative, so recorded spend and tokens only ever rise. When a caller
    /// wants the conservative "max(projected, actual)" reconciliation, it passes the larger
    /// actual here — spend still never decreases.
    pub fn record(&mut self, usage: Usage) {
        self.spent_usd += usage.cost_usd;
        self.tokens_used += usage.total_tokens();
    }

    /// Records spend, taking the larger of the already-recorded value and `actual_usd` so a
    /// reconciliation against an under-projection raises spend but never lowers it.
    pub fn record_cost_at_least(&mut self, actual_usd: f64, tokens: u64) {
        self.spent_usd = self.spent_usd.max(actual_usd);
        self.tokens_used = self.tokens_used.max(tokens);
    }

    pub fn spent_usd(&self) -> f64 {
        self.spent_usd
    }

    pub fn tokens_used(&self) -> u64 {
        self.tokens_used
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rates(in_rate: f64, out_rate: f64) -> RoleRates {
        RoleRates {
            cost_per_million_input: in_rate,
            cost_per_million_output: out_rate,
        }
    }

    fn guard(caps: BudgetCaps) -> BudgetGuard {
        BudgetGuard::new(caps, rates(3.0, 15.0), rates(3.0, 15.0), rates(3.0, 15.0))
    }

    fn req(max_tokens: u32, prompt_tokens: u32) -> CallRequest {
        CallRequest {
            role: Role::Mutator,
            max_tokens,
            prompt_tokens,
        }
    }

    #[test]
    fn projects_cost_from_rates() {
        let r = rates(3.0, 15.0);
        // 1M input @ $3 + 1M output @ $15 = $18
        let cost = r.project_cost(&req(1_000_000, 1_000_000));
        assert!((cost - 18.0).abs() < 1e-9);
    }

    #[test]
    fn uncapped_guard_never_blocks() {
        let g = guard(BudgetCaps::default());
        assert!(g.pre_dispatch(&req(1_000_000, 1_000_000)).is_ok());
    }

    #[test]
    fn record_is_monotonic() {
        let mut g = guard(BudgetCaps::default());
        g.record(Usage { tokens_in: 10, tokens_out: 5, cost_usd: 0.10 });
        let s1 = g.spent_usd();
        g.record(Usage { tokens_in: 10, tokens_out: 5, cost_usd: 0.05 });
        assert!(g.spent_usd() > s1, "spend must rise");
    }
}
