//! Transparent SSE reverse proxy (e4-1, AD-2).
//!
//! The gateway forwards each client request to the upstream provider and streams the response
//! back **byte-for-byte**, preserving status, headers (`content-type`, `retry-after`, …) and
//! every SSE frame — including `thinking` and `tool_use` blocks. It never buffers the whole
//! body, so time-to-first-token is bounded by the upstream, not the proxy (AD-2).
//!
//! Cancellation is structural: the response body is a lazily-polled stream. When the client
//! drops its side, axum drops that stream, which drops the reqwest response and tears down the
//! upstream connection — so a client abort propagates without any explicit timer.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use futures::future::Either;
use futures::{StreamExt, TryStreamExt};
use thiserror::Error;

use crate::config::GatewayConfig;

/// Errors forwarding a request upstream.
#[derive(Debug, Error)]
pub enum ProxyError {
    #[error("upstream request failed: {0}")]
    Upstream(#[from] reqwest::Error),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> Response {
        // A gateway-to-upstream failure is reported as a 502; the client sees a clean error
        // rather than a half-open stream. (Upstream 4xx/5xx are forwarded verbatim, not here.)
        (
            StatusCode::BAD_GATEWAY,
            format!("gateway upstream error: {self}"),
        )
            .into_response()
    }
}

/// Shared proxy state: the validated config plus one pooled reqwest client, and — when
/// e4-2 usage logging is enabled — the SQLite usage store the stream tee writes to.
pub struct ProxyState {
    pub config: GatewayConfig,
    client: reqwest::Client,
    usage_store: Option<Arc<crate::usage::UsageStore>>,
}

impl ProxyState {
    /// Builds the proxy state. The reqwest client is constructed with streaming enabled and no
    /// automatic compression/gzip so the body bytes we forward are exactly what upstream sent.
    pub fn new(config: GatewayConfig) -> Result<Self, ProxyError> {
        Self::new_with_usage(config, None)
    }

    /// Builds the proxy state with an optional e4-2 usage store. When present, every
    /// forwarded response body is tee'd: bytes pass through untouched, and on clean stream
    /// end the recorded bytes are scanned for a usage block and logged.
    pub fn new_with_usage(
        config: GatewayConfig,
        usage_store: Option<Arc<crate::usage::UsageStore>>,
    ) -> Result<Self, ProxyError> {
        // `default-features = false` on reqwest means no automatic gzip/brotli/deflate
        // decoding, so the bytes we forward are exactly what upstream sent. `no_proxy` keeps
        // the gateway from being surprised by ambient HTTP(S)_PROXY env vars. A shared client
        // pools connections so 20 concurrent streams (e4-5) reuse them.
        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .map_err(ProxyError::Upstream)?;
        Ok(Self {
            config,
            client,
            usage_store,
        })
    }
}

/// The gateway's axum [`Router`]: every method/path is forwarded transparently.
pub fn router(state: Arc<ProxyState>) -> Router {
    Router::new()
        .route("/{*path}", any(forward))
        .with_state(state)
}

/// Forwards one request to the upstream and streams the response back.
async fn forward(
    State(state): State<Arc<ProxyState>>,
    req: Request,
) -> Result<Response, ProxyError> {
    let (parts, body) = req.into_parts();

    // e4-2: the client session, taken from `x-evoswarm-session` when the client sends one,
    // else a generated id so every completed stream still gets an attributable usage row.
    let session_id = parts
        .headers
        .get("x-evoswarm-session")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Build the upstream URL from the configured base + the original path and query.
    let path_and_query = parts
        .uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or(parts.uri.path());
    let url = format!("{}{}", state.config.upstream_base_url, path_and_query);

    // Preserve the request method; reqwest maps a GET/DELETE without a body correctly.
    let method = Method::from_bytes(parts.method.as_str().as_bytes()).unwrap_or(Method::POST);
    let mut upstream = state.client.request(method, &url);

    // Copy request headers verbatim, except hop-by-hop headers that must not be forwarded and
    // `host` (reqwest sets it from the URL).
    upstream = upstream.headers(strip_hop_by_hop(parts.headers.clone()));

    // Forward the request body as a stream (never buffered in full).
    let req_stream = Body::new(body).into_data_stream();
    upstream = upstream.body(reqwest::Body::wrap_stream(req_stream));

    let upstream_resp = upstream.send().await?;

    // Preserve the upstream status exactly (200 SSE, or 4xx/5xx errors).
    let status = StatusCode::from_u16(upstream_resp.status().as_u16()).unwrap_or(StatusCode::OK);

    // Build the client response: status + upstream headers (minus hop-by-hop) + the body
    // streamed straight through. Dropping this stream (client abort) cancels the upstream.
    let mut builder = Response::builder().status(status);
    if let Some(headers) = builder.headers_mut() {
        *headers = strip_hop_by_hop(upstream_resp.headers().clone());
    }
    let byte_stream = upstream_resp.bytes_stream().map_err(std::io::Error::other);

    // e4-2: tee the stream for usage accounting when a store is configured. Bytes pass
    // through untouched; see `tee_usage` for the completion/abort semantics.
    let byte_stream: Either<_, _> = match &state.usage_store {
        Some(store) => Either::Left(tee_usage(byte_stream, store.clone(), session_id)),
        None => Either::Right(byte_stream),
    };

    let body = Body::from_stream(byte_stream);
    Ok(builder.body(body).unwrap_or_else(|_| {
        // A response-builder failure is only possible on an invalid header name from upstream;
        // fall back to an empty 502 rather than panicking the gateway.
        (StatusCode::BAD_GATEWAY, "gateway could not build response").into_response()
    }))
}

/// Safety cap on the per-stream recorded copy (16 MiB — far above any usage-bearing SSE
/// body, small enough that 20 concurrent streams cannot exhaust memory). Past the cap the
/// stream stops being recorded and nothing is logged for it.
const RECORDER_CAP_BYTES: usize = 16 * 1024 * 1024;

/// Wraps a forwarded byte stream so its bytes are also accumulated (up to
/// [`RECORDER_CAP_BYTES`]) for e4-2 usage extraction. Logging happens exactly once, on
/// clean end-of-stream: a sentinel item appended after the upstream's last byte triggers
/// parse + persist. A client abort drops the stream before the sentinel, and an upstream
/// error suppresses it, so neither logs a partial body. The forwarded bytes are never
/// modified — the recorder only observes them.
fn tee_usage<S>(
    stream: S,
    store: Arc<crate::usage::UsageStore>,
    session_id: String,
) -> impl futures::Stream<Item = Result<bytes::Bytes, std::io::Error>>
where
    S: futures::Stream<Item = Result<bytes::Bytes, std::io::Error>>,
{
    let recorder = Arc::new(UsageRecorder::new(store, session_id));
    stream
        .inspect({
            let recorder = recorder.clone();
            move |item| match item {
                Ok(bytes) => recorder.observe(bytes),
                // Stream error: mark the recorder so the trailing sentinel does not log a
                // partial body. The error still passes through to the client untouched.
                Err(_) => recorder.suppress(),
            }
        })
        // The sentinel: emitted only when the upstream stream completed cleanly. It
        // finalizes (parse + persist), then yields an empty chunk — zero bytes on the wire.
        .chain(futures::stream::once(async move {
            recorder.finalize();
            Ok(bytes::Bytes::new())
        }))
}

/// Accumulates a copy of a forwarded SSE body and logs its usage exactly once on clean
/// completion (e4-2). Only token counts leave the recorder; the recorded bytes are dropped
/// after parsing and are never persisted.
struct UsageRecorder {
    store: Arc<crate::usage::UsageStore>,
    session_id: String,
    buf: Mutex<Vec<u8>>,
    /// True once the sentinel finalized (or suppressed) this recorder.
    finalized: AtomicBool,
    /// True when recording was abandoned (cap exceeded): suppresses logging.
    suppressed: AtomicBool,
}

impl UsageRecorder {
    fn new(store: Arc<crate::usage::UsageStore>, session_id: String) -> Self {
        Self {
            store,
            session_id,
            buf: Mutex::new(Vec::new()),
            finalized: AtomicBool::new(false),
            suppressed: AtomicBool::new(false),
        }
    }

    fn observe(&self, bytes: &[u8]) {
        let mut buf = self.buf.lock().expect("recorder mutex");
        if buf.len() + bytes.len() > RECORDER_CAP_BYTES {
            self.suppressed.store(true, Ordering::SeqCst);
            buf.clear();
            return;
        }
        buf.extend_from_slice(bytes);
    }

    /// Marks the recorder so `finalize` becomes a no-op (upstream stream error).
    fn suppress(&self) {
        self.suppressed.store(true, Ordering::SeqCst);
    }

    fn finalize(&self) {
        if self.finalized.swap(true, Ordering::SeqCst) || self.suppressed.load(Ordering::SeqCst) {
            return;
        }
        let buf = self.buf.lock().expect("recorder mutex");
        // No usage block (error bodies, non-SSE responses, capped streams): log nothing.
        let Some(usage) = crate::usage::parse_sse_usage(&buf) else {
            return;
        };
        // A persistence failure must never break the (already delivered) forwarded stream.
        let _ = self.store.log(&self.session_id, usage);
    }
}

/// Hop-by-hop headers (RFC 7230 §6.1) that a proxy must not forward, plus `host` (reqwest
/// derives it from the URL) and `content-length` (the streamed body's length is not known
/// ahead and axum frames it; forwarding a stale length would corrupt the transfer).
fn strip_hop_by_hop(mut headers: HeaderMap) -> HeaderMap {
    use axum::http::HeaderName;
    // Built at runtime: `HeaderName` has interior mutability, so it cannot live in a `const`.
    let hop_by_hop = [
        header::CONNECTION,
        header::TRANSFER_ENCODING,
        header::UPGRADE,
        header::HOST,
        header::CONTENT_LENGTH,
        HeaderName::from_static("keep-alive"),
        HeaderName::from_static("proxy-authenticate"),
        HeaderName::from_static("proxy-authorization"),
        HeaderName::from_static("te"),
        HeaderName::from_static("trailer"),
    ];
    for name in hop_by_hop {
        headers.remove(name);
    }
    headers
}
