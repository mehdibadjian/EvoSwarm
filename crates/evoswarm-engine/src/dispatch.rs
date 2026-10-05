//! Model dispatch with idempotent replay (e1-12, AD-7 §2/§3).
//!
//! There are two execution paths here and the distinction is the whole safety story:
//!
//! - **Model calls** go through `dispatch_model`, which consults the ledger cache first. A
//!   call whose idempotency hash is already stored is replayed from disk with zero API
//!   tokens spent; a miss dispatches to the `ModelClient` and stores the result.
//! - **Sandbox runs** go through `sandbox_run`, which *never* consults the cache. A killed
//!   run's partial output is indistinguishable from a genuine failure, so caching it would
//!   poison scoring. An interrupted run is always re-executed from the start.

use async_trait::async_trait;
use evoswarm_core::ExecutionResult;
use evoswarm_ledger::{CachedCall, JobLedger, LedgerError};
use evoswarm_models::{call_hash, Role};
use evoswarm_sandbox::SandboxBackend;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("ledger error: {0}")]
    Ledger(#[from] LedgerError),
    #[error("model client error: {0}")]
    Client(String),
}

/// A single model completion request. The exact `prompt` bytes participate in the
/// idempotency hash, so any suffix edit produces a new key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionRequest {
    pub role: Role,
    pub model_id: String,
    pub prompt: Vec<u8>,
}

/// A model completion response with its token accounting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionResponse {
    pub text: String,
    pub tokens_in: i64,
    pub tokens_out: i64,
    /// True when the response was replayed from the on-disk cache (zero new API spend).
    pub from_cache: bool,
}

/// The seam every role dispatches through. Wave 3 wires a real provider client; tests inject
/// a scripted fake that counts calls so replay-with-zero-calls is directly observable.
#[async_trait]
pub trait ModelClient: Send + Sync {
    async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse, String>;
}

/// Dispatches a model call through the idempotency cache. On a hit, returns the stored
/// response with `from_cache = true` and never touches the client. On a miss, calls the
/// client, stores the result, and returns it with `from_cache = false`.
pub async fn dispatch_model<C: ModelClient + ?Sized>(
    ledger: &JobLedger,
    client: &C,
    req: &CompletionRequest,
) -> Result<CompletionResponse, DispatchError> {
    let hash = call_hash(req.role, &req.model_id, &req.prompt);

    if let Some(cached) = ledger.lookup(&hash)? {
        return Ok(CompletionResponse {
            text: cached.response_text,
            tokens_in: cached.tokens_in,
            tokens_out: cached.tokens_out,
            from_cache: true,
        });
    }

    let resp = client
        .complete(req)
        .await
        .map_err(DispatchError::Client)?;

    ledger.store(&CachedCall {
        idempotency_hash: hash,
        model_id: req.model_id.clone(),
        response_text: resp.text.clone(),
        tokens_in: resp.tokens_in,
        tokens_out: resp.tokens_out,
    })?;

    Ok(CompletionResponse {
        from_cache: false,
        ..resp
    })
}

/// Runs a candidate's test command in the sandbox. Deliberately uncached: the same logical
/// run after a restart is re-executed from scratch, never served from a stored result.
pub async fn sandbox_run<B: SandboxBackend + ?Sized>(
    backend: &B,
    workdir: &Path,
    test_command: &str,
) -> Result<ExecutionResult, DispatchError> {
    backend
        .run(workdir, test_command)
        .await
        .map_err(|e| DispatchError::Client(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingClient {
        calls: AtomicUsize,
    }

    #[async_trait]
    impl ModelClient for CountingClient {
        async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(CompletionResponse {
                text: "response".into(),
                tokens_in: 5,
                tokens_out: 7,
                from_cache: false,
            })
        }
    }

    fn req(prompt: &[u8]) -> CompletionRequest {
        CompletionRequest {
            role: Role::Mutator,
            model_id: "m1".into(),
            prompt: prompt.to_vec(),
        }
    }

    #[tokio::test]
    async fn second_identical_call_is_served_from_cache() {
        let ledger = JobLedger::open_in_memory().unwrap();
        let client = CountingClient {
            calls: AtomicUsize::new(0),
        };
        let r1 = dispatch_model(&ledger, &client, &req(b"prompt")).await.unwrap();
        assert!(!r1.from_cache);
        assert_eq!(client.calls.load(Ordering::SeqCst), 1);

        let r2 = dispatch_model(&ledger, &client, &req(b"prompt")).await.unwrap();
        assert!(r2.from_cache);
        assert_eq!(r2.text, "response");
        // The client was NOT called again: replay spends zero API tokens.
        assert_eq!(client.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn changed_prompt_dispatches_fresh() {
        let ledger = JobLedger::open_in_memory().unwrap();
        let client = CountingClient {
            calls: AtomicUsize::new(0),
        };
        dispatch_model(&ledger, &client, &req(b"prompt")).await.unwrap();
        let r = dispatch_model(&ledger, &client, &req(b"prompu")).await.unwrap();
        assert!(!r.from_cache);
        assert_eq!(client.calls.load(Ordering::SeqCst), 2);
    }
}
