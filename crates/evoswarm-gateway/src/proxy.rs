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

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use futures::TryStreamExt;
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

/// Shared proxy state: the validated config plus one pooled reqwest client.
pub struct ProxyState {
    pub config: GatewayConfig,
    client: reqwest::Client,
}

impl ProxyState {
    /// Builds the proxy state. The reqwest client is constructed with streaming enabled and no
    /// automatic compression/gzip so the body bytes we forward are exactly what upstream sent.
    pub fn new(config: GatewayConfig) -> Result<Self, ProxyError> {
        // `default-features = false` on reqwest means no automatic gzip/brotli/deflate
        // decoding, so the bytes we forward are exactly what upstream sent. `no_proxy` keeps
        // the gateway from being surprised by ambient HTTP(S)_PROXY env vars. A shared client
        // pools connections so 20 concurrent streams (e4-5) reuse them.
        let client = reqwest::Client::builder()
            .no_proxy()
            .build()
            .map_err(ProxyError::Upstream)?;
        Ok(Self { config, client })
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
    let body = Body::from_stream(byte_stream);
    Ok(builder.body(body).unwrap_or_else(|_| {
        // A response-builder failure is only possible on an invalid header name from upstream;
        // fall back to an empty 502 rather than panicking the gateway.
        (StatusCode::BAD_GATEWAY, "gateway could not build response").into_response()
    }))
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
