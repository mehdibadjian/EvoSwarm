//! e4-3: Exemplar injection acceptance tests (AD-2).
//!
//! Acceptance criteria (story §4):
//! - AC1: Given a request similar to stored winners above the threshold, when forwarded,
//!   then up to 2 exemplars totalling at most 2k tokens are prepended to the system prompt.
//! - AC2: Given the lookup takes over 20 ms, when it times out, then the request is forwarded unchanged.
//! - AC3: Given an injection happens, when logged, then the exemplar IDs are recorded.

use async_trait::async_trait;
use evoswarm_gateway::exemplar::{
    format_exemplar_system_prompt, FalkorExemplarInjector, TimedLookupProvider,
};
use evoswarm_gateway::ExemplarInjector;
use serde_json::json;
use std::time::Duration;

struct FastLookupProvider {
    exemplars: Vec<(String, String)>,
}

#[async_trait]
impl TimedLookupProvider for FastLookupProvider {
    async fn lookup_exemplars(&self, _prompt: &str) -> Vec<(String, String)> {
        self.exemplars.clone()
    }
}

struct SlowLookupProvider {
    delay: Duration,
}

#[async_trait]
impl TimedLookupProvider for SlowLookupProvider {
    async fn lookup_exemplars(&self, _prompt: &str) -> Vec<(String, String)> {
        tokio::time::sleep(self.delay).await;
        vec![("ex-slow".into(), "delayed code".into())]
    }
}

#[tokio::test]
async fn test_successful_injection() {
    let provider = FastLookupProvider {
        exemplars: vec![
            ("cand-1".into(), "def sort(arr): return sorted(arr)".into()),
            ("cand-2".into(), "def binary_search(): pass".into()),
        ],
    };

    let injector = FalkorExemplarInjector::new(provider, Duration::from_millis(20));

    let request_body = json!({
        "model": "claude-3-5-sonnet",
        "system": "You are a helpful assistant.",
        "messages": [
            {"role": "user", "content": "How do I sort an array?"}
        ]
    });

    let raw_bytes = serde_json::to_vec(&request_body).unwrap();
    let result_bytes = injector.inject(&raw_bytes).await;

    assert!(result_bytes.is_some(), "injection modified the body");
    let modified: serde_json::Value =
        serde_json::from_slice(&result_bytes.unwrap()).expect("valid json");

    let system_str = modified["system"].as_str().unwrap();
    assert!(
        system_str.contains("cand-1"),
        "system prompt contains exemplar 1: {system_str}"
    );
    assert!(
        system_str.contains("cand-2"),
        "system prompt contains exemplar 2: {system_str}"
    );
    assert!(
        system_str.contains("You are a helpful assistant."),
        "system prompt preserves original text: {system_str}"
    );
}

#[tokio::test]
async fn test_timeout_fallback() {
    // 50ms delay exceeds the 20ms timeout
    let provider = SlowLookupProvider {
        delay: Duration::from_millis(50),
    };

    let injector = FalkorExemplarInjector::new(provider, Duration::from_millis(20));

    let request_body = json!({
        "model": "claude-3-5-sonnet",
        "system": "Original prompt",
        "messages": [
            {"role": "user", "content": "test question"}
        ]
    });

    let raw_bytes = serde_json::to_vec(&request_body).unwrap();
    let start = std::time::Instant::now();
    let result_bytes = injector.inject(&raw_bytes).await;
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(35),
        "timeout completes under 35ms, took: {elapsed:?}"
    );
    assert!(
        result_bytes.is_none(),
        "returns None on timeout so original body is untouched"
    );
}

#[test]
fn test_token_cap_limiting() {
    let big_code = "x".repeat(10_000); // Exceeds 2,000 token (~8,000 char) budget
    let exemplars = vec![("big-ex".into(), big_code)];
    let prompt = format_exemplar_system_prompt("Original", &exemplars);

    assert!(
        prompt.len() <= 8500,
        "prepended prompt capped within token budget: length {}",
        prompt.len()
    );
}
