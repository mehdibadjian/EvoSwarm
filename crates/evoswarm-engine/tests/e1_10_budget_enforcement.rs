//! e1-10: Budget enforcement.
//!
//! Verifies the pre-dispatch caps (dollar and token), the plateau early-stop, and monotonic
//! spend reconciliation. A guarded dispatch helper mirrors the single chokepoint: a call is
//! projected against the caps and only reaches the model client when the projection fits, so
//! "zero further calls dispatched" after exhaustion is directly observable.

use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use evoswarm_core::Usage;
use evoswarm_engine::{
    should_stop, BudgetCaps, BudgetExhausted, BudgetGuard, CallRequest, ExhaustedKind, RoleRates,
    StopReason,
};
use evoswarm_models::Role;

/// A model client that counts real dispatches and reports usage matching the request, so
/// the guard's token accounting stays coherent with what was asked for.
#[async_trait]
trait Model: Send + Sync {
    async fn call(&self, req: &CallRequest) -> Usage;
}

struct FixedClient {
    calls: AtomicUsize,
    cost_per_call: f64,
}

#[async_trait]
impl Model for FixedClient {
    async fn call(&self, req: &CallRequest) -> Usage {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Usage {
            tokens_in: req.prompt_tokens as u64,
            tokens_out: req.max_tokens as u64,
            cost_usd: self.cost_per_call,
        }
    }
}

/// The budget chokepoint: project, and dispatch only if the call fits under both caps.
/// Returns `Err(BudgetExhausted)` without touching the client when a cap would be crossed.
async fn guarded_dispatch(
    guard: &mut BudgetGuard,
    client: &FixedClient,
    req: &CallRequest,
) -> Result<Usage, BudgetExhausted> {
    guard.pre_dispatch(req)?;
    let usage = client.call(req).await;
    guard.record(usage);
    Ok(usage)
}

fn rates(in_rate: f64, out_rate: f64) -> RoleRates {
    RoleRates {
        cost_per_million_input: in_rate,
        cost_per_million_output: out_rate,
    }
}

fn guard(caps: BudgetCaps) -> BudgetGuard {
    // $3/M input, $15/M output for all three roles.
    BudgetGuard::new(caps, rates(3.0, 15.0), rates(3.0, 15.0), rates(3.0, 15.0))
}

fn req(role: Role, max_tokens: u32, prompt_tokens: u32) -> CallRequest {
    CallRequest {
        role,
        max_tokens,
        prompt_tokens,
    }
}

/// AC1: the projected cost is checked against the remaining budget before dispatch — a call
/// that would cross the dollar cap is never sent to the model client.
#[tokio::test]
async fn test_pre_call_cost_check_halt() {
    // Cap $0.10. One 1M-input/1M-output call projects to $18 (input $3 + output $15), which
    // crosses the cap, so the very first dispatch must halt before reaching the client.
    let mut g = guard(BudgetCaps {
        dollar_cap: Some(0.10),
        token_cap: None,
    });
    let client = FixedClient {
        calls: AtomicUsize::new(0),
        cost_per_call: 18.0,
    };

    let err = guarded_dispatch(&mut g, &client, &req(Role::Mutator, 1_000_000, 1_000_000))
        .await
        .expect_err("must halt on dollar cap");
    assert_eq!(err.kind, ExhaustedKind::Dollar);
    assert_eq!(
        client.calls.load(Ordering::SeqCst),
        0,
        "the blocked call must never reach the client"
    );
    assert_eq!(g.spent_usd(), 0.0, "nothing was spent");
}

/// AC2: when the token cap would be reached the job halts as budget_exhausted with zero
/// further calls dispatched.
#[tokio::test]
async fn test_token_cap_exhaustion() {
    // Token cap 100. Each call requests max_tokens 60 + prompt 60 = 120 > 100, so it cannot
    // be dispatched at all: the token cap is exhausted immediately.
    let mut g = guard(BudgetCaps {
        dollar_cap: None,
        token_cap: Some(100),
    });
    let client = FixedClient {
        calls: AtomicUsize::new(0),
        cost_per_call: 0.0,
    };

    let err = guarded_dispatch(&mut g, &client, &req(Role::Mutator, 60, 60))
        .await
        .expect_err("must halt on token cap");
    assert_eq!(err.kind, ExhaustedKind::Token);
    assert_eq!(client.calls.load(Ordering::SeqCst), 0);

    // A smaller call that fits (10 + 10 = 20 <= 100) is dispatched and consumes budget.
    guarded_dispatch(&mut g, &client, &req(Role::Mutator, 10, 10))
        .await
        .expect("small call fits");
    assert_eq!(client.calls.load(Ordering::SeqCst), 1);
    assert_eq!(g.tokens_used(), 20);

    // Now only 80 tokens of headroom remain; a 60+60 call still cannot fit → zero further.
    let err2 = guarded_dispatch(&mut g, &client, &req(Role::Mutator, 60, 60))
        .await
        .expect_err("headroom exhausted");
    assert_eq!(err2.kind, ExhaustedKind::Token);
    assert_eq!(
        client.calls.load(Ordering::SeqCst),
        1,
        "no further call dispatched after token exhaustion"
    );
}

/// AC3: three consecutive generations with non-improving best scores stop the search with
/// reason plateau_early_stop and a partial result.
#[test]
fn test_early_stop_score_plateau() {
    // Generation 0..2 with best scores that never strictly improve.
    let history = vec![0.82, 0.80, 0.80];
    assert_eq!(should_stop(&history), Some(StopReason::PlateauEarlyStop));

    // A plateau reached only at the third generation: the first two cannot stop the run.
    assert_eq!(should_stop(&[0.82]), None);
    assert_eq!(should_stop(&[0.82, 0.80]), None);
    assert_eq!(should_stop(&[0.82, 0.80, 0.79]), Some(StopReason::PlateauEarlyStop));

    // A late improvement keeps the search running (no early stop, no discard of a winner).
    assert_eq!(should_stop(&[0.82, 0.80, 0.90]), None);
}

/// AC4: when provider-reported usage exceeds the projection, recorded spend rises to the
/// actual value and never decreases.
#[tokio::test]
async fn test_spend_reconciliation_is_monotonic() {
    let mut g = guard(BudgetCaps {
        dollar_cap: Some(1_000.0),
        token_cap: Some(1_000_000),
    });
    // Projection for this call: input 1000/1e6*$3 + output 1000/1e6*$15 = $0.018.
    let r = req(Role::Mutator, 1000, 1000);
    assert!(g.pre_dispatch(&r).is_ok());

    // Actual usage came back higher than projected ($0.50 > $0.018). Reconciling must raise
    // recorded spend to the actual value...
    g.record_cost_at_least(0.50, 2000);
    let after_first = g.spent_usd();
    assert!((after_first - 0.50).abs() < 1e-9, "spend rises to actual");

    // ...and a later, *lower* actual must never reduce it (monotonic).
    g.record_cost_at_least(0.20, 1000);
    assert!(
        g.spent_usd() >= after_first,
        "spend must never decrease: {} after lower actual",
        g.spent_usd()
    );
    assert!((g.spent_usd() - 0.50).abs() < 1e-9);
}
