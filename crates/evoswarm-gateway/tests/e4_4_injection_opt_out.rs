//! e4-4 injection opt-out — acceptance criteria (story §4).
//!
//! Gate: `cargo test --test e4_4_injection_opt_out`. Runs against the local stub upstream
//! (`tests/common/`), no LLM.
//!
//! Story contract (§2): support the `x-evoswarm-inject: off` header **or** the config flag
//! `gateway.inject = false`. Either one "bypasses memory lookup entirely while retaining
//! token usage logging" — so the injector must be invoked **zero times** (not merely have its
//! result discarded) and the body must still reach the upstream byte-for-byte.
//!
//! SEAM honesty: e4-4 lands *before* its dependency e4-3 (exemplar injection over FalkorDB is
//! BLOCKED on e2-5), so the injector here is a test `MarkerInjector` behind the
//! [`ExemplarInjector`] seam. The opt-out/bypass logic and the 0-call fast path are real; the
//! real exemplar lookup is not.

mod common;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use common::{recorded_sse, sha256_hex, spawn_gateway_with_injector, spawn_stub};
use evoswarm_gateway::usage::UsageStore;
use evoswarm_gateway::{header_opts_out, ExemplarInjector, INJECT_HEADER, INJECT_OFF_VALUE};
use reqwest::Client;

/// The exact request body used throughout. It has a `system` field so the stub injector has
/// something verifiable to modify, and the echo route lets us read back what the upstream
/// actually received.
const REQUEST_BODY: &str = r#"{"model":"claude-3-5-sonnet","system":"base prompt","messages":[]}"#;

/// The marker the stub injector prepends to the `system` field — modelled on e4-3's
/// "prepend exemplars to the system prompt" contract.
const MARKER: &str = "[EXEMPLAR toolu_42] ";

/// Counts `inject` invocations and rewrites the JSON `system` field. Counting is the point:
/// AC1's "bypasses memory lookup entirely" means this counter must stay at 0 when opted out.
#[derive(Default)]
struct MarkerInjector {
    calls: AtomicU64,
}

impl MarkerInjector {
    fn calls(&self) -> u64 {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ExemplarInjector for MarkerInjector {
    async fn inject(&self, body: &[u8]) -> Option<Vec<u8>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut value: serde_json::Value = serde_json::from_slice(body).ok()?;
        let system = value.get("system")?.as_str()?;
        value["system"] = serde_json::Value::String(format!("{MARKER}{system}"));
        Some(serde_json::to_vec(&value).ok()?)
    }
}

/// POSTs [`REQUEST_BODY`] to the gateway's `/echo` route and returns the body the upstream
/// received (the stub echoes it back verbatim).
async fn echo_through(gw_base: &str, client: &Client, inject_header: Option<&str>) -> String {
    let mut req = client
        .post(format!("{gw_base}/echo"))
        .header("content-type", "application/json")
        .body(REQUEST_BODY);
    if let Some(v) = inject_header {
        req = req.header(INJECT_HEADER, v);
    }
    let resp = req.send().await.expect("POST through gateway");
    assert_eq!(resp.status(), 200, "gateway forwards the echo 200");
    let bytes = resp.bytes().await.expect("read echoed body");
    String::from_utf8(bytes.to_vec()).expect("echoed body is UTF-8")
}

/// The injected form of [`REQUEST_BODY`], i.e. what the upstream sees when injection fires.
/// Built from the original JSON so the test asserts the exact transform, not a hand-typed copy.
fn injected_body() -> String {
    let mut value: serde_json::Value = serde_json::from_str(REQUEST_BODY).unwrap();
    let system = value["system"].as_str().unwrap().to_string();
    value["system"] = serde_json::Value::String(format!("{MARKER}{system}"));
    serde_json::to_string(&value).unwrap()
}

/// AC1 (header): `x-evoswarm-inject: off` ⇒ the injector is never called and the body is
/// forwarded unmodified.
#[tokio::test]
async fn test_header_opt_out() {
    let inj = Arc::new(MarkerInjector::default());
    let stub = spawn_stub().await;
    let gw = spawn_gateway_with_injector(&stub.base_url, None, inj.clone(), true).await;
    let client = Client::new();

    let echoed = echo_through(&gw.base_url, &client, Some(INJECT_OFF_VALUE)).await;

    assert_eq!(
        inj.calls(),
        0,
        "opt-out must bypass the injector entirely (0 memory lookups)"
    );
    assert_eq!(
        echoed, REQUEST_BODY,
        "body must reach upstream byte-for-byte unmodified"
    );
    assert!(
        !echoed.contains(MARKER),
        "nothing may be injected when opted out"
    );
}

/// AC1 (config): `gateway.inject = false` achieves the identical result, with no header.
#[tokio::test]
async fn test_config_flag_opt_out() {
    let inj = Arc::new(MarkerInjector::default());
    let stub = spawn_stub().await;
    let gw = spawn_gateway_with_injector(&stub.base_url, None, inj.clone(), false).await;
    let client = Client::new();

    let echoed = echo_through(&gw.base_url, &client, None).await;

    assert_eq!(
        inj.calls(),
        0,
        "config flag must bypass the injector entirely"
    );
    assert_eq!(
        echoed, REQUEST_BODY,
        "config flag result is identical to the header result"
    );
}

/// Anti-cheat: with no opt-out the injector really does fire and the modified body is what
/// reaches upstream — otherwise the tests above would pass on a never-injecting stub.
#[tokio::test]
async fn test_injection_active_without_opt_out() {
    let inj = Arc::new(MarkerInjector::default());
    let stub = spawn_stub().await;
    let gw = spawn_gateway_with_injector(&stub.base_url, None, inj.clone(), true).await;
    let client = Client::new();

    let echoed = echo_through(&gw.base_url, &client, None).await;

    assert_eq!(
        inj.calls(),
        1,
        "injector invoked exactly once per injected request"
    );
    assert!(
        echoed.contains(MARKER),
        "injection must actually modify the body"
    );
    assert_eq!(
        echoed,
        injected_body(),
        "the injected JSON is what upstream receives"
    );
    assert!(
        echoed.contains("base prompt"),
        "injection prepends; it does not drop the original system prompt"
    );
}

/// AC1 precision: only the exact `off` value (case-insensitive) opts out. Anything else —
/// including near-misses and an empty value — still injects.
#[tokio::test]
async fn test_other_header_values_do_not_opt_out() {
    let inj = Arc::new(MarkerInjector::default());
    let stub = spawn_stub().await;
    let gw = spawn_gateway_with_injector(&stub.base_url, None, inj.clone(), true).await;
    let client = Client::new();

    let mut expected_calls = 0u64;
    for value in ["on", "OFF-but-not-really", ""] {
        expected_calls += 1;
        let echoed = echo_through(&gw.base_url, &client, Some(value)).await;
        assert_eq!(
            inj.calls(),
            expected_calls,
            "{value:?} must NOT opt out of injection"
        );
        assert!(
            echoed.contains(MARKER),
            "header value {value:?} is not `off`, so injection still applies"
        );
    }

    // `OFF` (uppercase) does opt out: matching is case-insensitive.
    let echoed = echo_through(&gw.base_url, &client, Some("OFF")).await;
    assert_eq!(
        inj.calls(),
        expected_calls,
        "uppercase OFF opts out without an extra call"
    );
    assert_eq!(
        echoed, REQUEST_BODY,
        "uppercase OPT-OUT forwards the original body"
    );

    // Unit-level: the same predicate the proxy uses, on a bare HeaderMap.
    use axum::http::HeaderMap;
    let mut h = HeaderMap::new();
    h.insert(INJECT_HEADER, "off".parse().unwrap());
    assert!(header_opts_out(&h));
    let mut h = HeaderMap::new();
    h.insert(INJECT_HEADER, "OFF".parse().unwrap());
    assert!(header_opts_out(&h));
    let mut h = HeaderMap::new();
    h.insert(INJECT_HEADER, "OFF-but-not-really".parse().unwrap());
    assert!(!header_opts_out(&h));
}

/// An injector that declines every request (e4-3's shape for "no relevant exemplars" or a
/// lookup over the 20 ms budget). Declining must leave the request exactly as it arrived —
/// `None` means "forward unmodified", not "send nothing".
#[derive(Default)]
struct DecliningInjector {
    calls: AtomicU64,
}

impl DecliningInjector {
    fn calls(&self) -> u64 {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ExemplarInjector for DecliningInjector {
    async fn inject(&self, _body: &[u8]) -> Option<Vec<u8>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        None
    }
}

/// A declining injector (returns `None`) forwards the original body untouched — the gateway
/// keeps the caller's bytes rather than substituting an empty or fabricated body.
#[tokio::test]
async fn test_injector_decline_forwards_original_body() {
    let inj = Arc::new(DecliningInjector::default());
    let stub = spawn_stub().await;
    let gw = spawn_gateway_with_injector(&stub.base_url, None, inj.clone(), true).await;
    let client = Client::new();

    let echoed = echo_through(&gw.base_url, &client, None).await;

    assert_eq!(
        inj.calls(),
        1,
        "the injector was consulted, and it declined"
    );
    assert_eq!(
        echoed, REQUEST_BODY,
        "a declined injection must forward the caller's original bytes"
    );
}

/// AC2: injection off still logs usage, and the streamed SSE response is untouched.
#[tokio::test]
async fn test_usage_still_logged_when_opted_out() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("usage.db");
    let inj = Arc::new(MarkerInjector::default());
    let stub = spawn_stub().await;
    let gw = spawn_gateway_with_injector(&stub.base_url, Some(&db), inj.clone(), true).await;
    let client = Client::new();

    let resp = client
        .post(gw.url("/v1/messages"))
        .header("content-type", "application/json")
        .header(INJECT_HEADER, INJECT_OFF_VALUE)
        .header("x-evoswarm-session", "sess-optout")
        .body(REQUEST_BODY)
        .send()
        .await
        .expect("POST");
    assert_eq!(resp.status(), 200);
    let bytes = resp.bytes().await.expect("read forwarded body");

    // The opt-out must not disturb e4-1 pass-through fidelity.
    assert_eq!(
        sha256_hex(&bytes),
        sha256_hex(recorded_sse().as_bytes()),
        "SSE bytes are forwarded byte-for-byte when opted out"
    );
    assert_eq!(
        inj.calls(),
        0,
        "usage logging must not drag injection back in"
    );

    // ...and e4-2 accounting still runs: exactly one row for the session.
    let store = UsageStore::open(&db).expect("open store");
    let records = store.records_for_session("sess-optout").expect("records");
    assert_eq!(
        records.len(),
        1,
        "usage is still logged while injection is off"
    );
    assert_eq!(records[0].input_tokens, 25);
    assert_eq!(records[0].output_tokens, 42);
    assert_eq!(records[0].cached_tokens, 10);
}
