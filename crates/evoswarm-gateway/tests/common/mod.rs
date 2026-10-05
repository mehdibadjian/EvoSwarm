//! Shared test support for the e4 gateway stories: a local **stub upstream** that stands in
//! for the Anthropic API so the proxy is exercised end-to-end without an LLM (roadmap §5
//! SEAM: "Axum SSE proxy testable against a local stub upstream").
//!
//! This file lives under `tests/common/` so Cargo does NOT treat it as its own `--test`
//! target; each e4 test pulls it in with `mod common;`.

#![allow(dead_code)]

use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::Response;
use axum::routing::any;
use axum::Router;
use futures::stream;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

/// A recorded Claude Code SSE session, split into the exact frames the stub emits. Each frame
/// ends with the SSE blank-line terminator `\n\n`. `recorded_sse()` is the byte-for-byte
/// reference the gateway must reproduce; it includes a `thinking` block (frames 1-3) and a
/// `tool_use` block (frames 4-6) so AC2 has something to assert on.
pub const SSE_FRAMES: &[&str] = &[
    "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_01\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-3-5-sonnet\",\"content\":[],\"usage\":{\"input_tokens\":25,\"cache_read_input_tokens\":10,\"output_tokens\":1}}}\n\n",
    "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n",
    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"Let me read the file first.\"}}\n\n",
    "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
    "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_01\",\"name\":\"read_file\",\"input\":{}}}\n\n",
    "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\":\\\"src/main.rs\\\"}\"}}\n\n",
    "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
    "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":42}}\n\n",
    "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
];

/// The exact bytes of the recorded session (concatenation of [`SSE_FRAMES`]).
pub fn recorded_sse() -> String {
    SSE_FRAMES.concat()
}

/// Body of the upstream 429 error response; must survive the proxy unchanged.
pub const ERROR_BODY: &str =
    r#"{"type":"error","error":{"type":"rate_limit_error","message":"slow down"}}"#;

/// The `retry-after` value the stub sends on its 429; the proxy must forward it verbatim.
pub const RETRY_AFTER: &str = "30";

/// A running stub upstream bound to an ephemeral loopback port.
pub struct Stub {
    /// Base URL of the stub, e.g. `http://127.0.0.1:54321`.
    pub base_url: String,
    /// Set to true when the `/slow` stream is dropped (i.e. the proxy cancelled the upstream
    /// after the client aborted) — the signal for the abort-propagation test.
    pub slow_dropped: Arc<AtomicBool>,
    pub handle: JoinHandle<()>,
}

impl Stub {
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

/// Spawns the stub upstream and returns its bound address plus the cancellation flag.
pub async fn spawn_stub() -> Stub {
    let slow_dropped = Arc::new(AtomicBool::new(false));
    let app = Router::new()
        .route("/{*rest}", any(stub_handler))
        .with_state(slow_dropped.clone());

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind stub");
    let addr: SocketAddr = listener.local_addr().expect("stub addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve stub");
    });

    Stub {
        base_url: format!("http://127.0.0.1:{}", addr.port()),
        slow_dropped,
        handle,
    }
}

async fn stub_handler(
    State(slow_dropped): State<Arc<AtomicBool>>,
    req: axum::extract::Request,
) -> Response {
    let path = req.uri().path().to_string();
    match path.as_str() {
        // The recorded SSE session, streamed as discrete frames.
        "/v1/messages" => sse_response(),
        // An upstream rate-limit error: status, retry-after and body must pass through.
        "/error" => Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header(header::RETRY_AFTER, RETRY_AFTER)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(ERROR_BODY))
            .unwrap(),
        // An endless slow stream; its drop-guard flips `slow_dropped` when cancelled.
        "/slow" => slow_response(slow_dropped),
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("no such route"))
            .unwrap(),
    }
}

fn sse_response() -> Response {
    let frames: Vec<Result<String, Infallible>> =
        SSE_FRAMES.iter().map(|f| Ok(f.to_string())).collect();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from_stream(stream::iter(frames)))
        .unwrap()
}

/// A guard whose `Drop` records that the slow stream was cancelled.
struct DropGuard(Arc<AtomicBool>);
impl Drop for DropGuard {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn slow_response(slow_dropped: Arc<AtomicBool>) -> Response {
    // The guard lives in the unfold state; dropping the stream (on cancellation) drops it.
    let state = (DropGuard(slow_dropped), 0u64);
    let stream = futures::stream::unfold(state, |(guard, i)| async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        Some((
            Ok::<_, Infallible>(format!("data: chunk-{i}\n\n")),
            (guard, i + 1),
        ))
    });
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .body(Body::from_stream(stream))
        .unwrap()
}

/// A running gateway bound to an ephemeral loopback port, forwarding to `upstream_base_url`.
pub struct Gateway {
    pub base_url: String,
    pub handle: JoinHandle<()>,
}

impl Gateway {
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

/// Boots the real gateway (via its public `ProxyState`/`router` API) pointed at the stub.
pub async fn spawn_gateway(upstream_base_url: &str) -> Gateway {
    use evoswarm_gateway::config::GatewayConfig;
    use evoswarm_gateway::proxy::{router, ProxyState};

    // listen_addr port is ignored: we bind an ephemeral port below and serve on it.
    let config = GatewayConfig::new(upstream_base_url, "127.0.0.1:0".parse().unwrap())
        .expect("valid config");
    let state = ProxyState::new(config).expect("proxy state");
    let app = router(Arc::new(state));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind gateway");
    let addr = listener.local_addr().expect("gateway addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve gateway");
    });

    Gateway {
        base_url: format!("http://127.0.0.1:{}", addr.port()),
        handle,
    }
}

/// Boots the gateway with e4-2 usage logging enabled, persisting to `usage_db`.
pub async fn spawn_gateway_with_usage(
    upstream_base_url: &str,
    usage_db: &std::path::Path,
) -> Gateway {
    use evoswarm_gateway::config::GatewayConfig;
    use evoswarm_gateway::proxy::{router, ProxyState};
    use evoswarm_gateway::usage::UsageStore;

    let config = GatewayConfig::new(upstream_base_url, "127.0.0.1:0".parse().unwrap())
        .expect("valid config");
    let store = Arc::new(UsageStore::open(usage_db).expect("usage store"));
    let state = ProxyState::new_with_usage(config, Some(store)).expect("proxy state");
    let app = router(Arc::new(state));

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind gateway");
    let addr = listener.local_addr().expect("gateway addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve gateway");
    });

    Gateway {
        base_url: format!("http://127.0.0.1:{}", addr.port()),
        handle,
    }
}

/// SHA-256 hex digest, used to assert byte-for-byte body fidelity.
pub fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}
