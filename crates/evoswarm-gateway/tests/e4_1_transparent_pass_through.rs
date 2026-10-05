//! e4-1 transparent SSE pass-through — acceptance criteria (story §4).
//!
//! Red-phase tests authored before the `evoswarm-gateway` proxy module exists. The story
//! suggests `tests/gateway/test_sse_passthrough.rs`; the verification gate is
//! `cargo test --test e4_1_transparent_pass_through`, so this is a flat target and the shared
//! stub upstream lives in `tests/common/` (roadmap §5 SEAM: proxy tested against a local stub,
//! no LLM).
//!
//! SEAM honesty: the **stub upstream** stands in for the real Anthropic API, so AC1's
//! "byte-for-byte against the *direct* response" is asserted against the stub's recorded
//! frames (the gateway is transparent, so stub bytes in == bytes out). The real-provider
//! replay half needs live LLM keys and is held at `review`.

mod common;

use std::time::Duration;

use common::{recorded_sse, sha256_hex, spawn_gateway, spawn_stub, ERROR_BODY, RETRY_AFTER};
use futures::StreamExt;
use reqwest::Client;

/// AC1: streaming a recorded SSE session through the gateway reproduces the upstream bytes
/// exactly (SHA-256 of the forwarded body == SHA-256 of the recorded session).
#[tokio::test]
async fn test_byte_for_byte_fidelity() {
    let stub = spawn_stub().await;
    let gw = spawn_gateway(&stub.base_url).await;
    let client = Client::new();

    let resp = client
        .post(gw.url("/v1/messages"))
        .header("content-type", "application/json")
        .body(r#"{"model":"claude-3-5-sonnet","messages":[]}"#)
        .send()
        .await
        .expect("POST through gateway");

    assert_eq!(resp.status(), 200, "gateway forwards upstream 200");
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    assert!(
        ct.contains("text/event-stream"),
        "content-type must be the SSE media type, got {ct:?}"
    );

    let bytes = resp.bytes().await.expect("read forwarded body");
    let forwarded = sha256_hex(&bytes);
    let reference = sha256_hex(recorded_sse().as_bytes());
    assert_eq!(
        forwarded, reference,
        "forwarded SSE bytes must match the recorded session byte-for-byte"
    );
    assert_eq!(
        String::from_utf8_lossy(&bytes),
        recorded_sse(),
        "byte-level equality (not just hash)"
    );
}

/// AC2: `thinking` and `tool_use` blocks survive the proxy intact — every frame arrives in
/// order with its payload unmodified.
#[tokio::test]
async fn test_thinking_and_tool_use_blocks_intact() {
    let stub = spawn_stub().await;
    let gw = spawn_gateway(&stub.base_url).await;
    let client = Client::new();

    let resp = client
        .post(gw.url("/v1/messages"))
        .body("{}")
        .send()
        .await
        .expect("POST");
    let body = resp.text().await.expect("read body");

    // The thinking block.
    assert!(
        body.contains("\"content_block\":{\"type\":\"thinking\""),
        "thinking content_block_start arrives intact"
    );
    assert!(
        body.contains("\"thinking_delta\",\"thinking\":\"Let me read the file first.\""),
        "thinking_delta payload intact"
    );
    // The tool_use block, including its incremental input JSON.
    assert!(
        body.contains(
            "\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_01\",\"name\":\"read_file\""
        ),
        "tool_use content_block_start intact"
    );
    assert!(
        body.contains("input_json_delta"),
        "tool_use input_json_delta intact"
    );
    assert!(
        body.contains("\"partial_json\":\"{\\\"path\\\":\\\"src/main.rs\\\"}\""),
        "escaped tool input JSON preserved verbatim"
    );
    // Frame ordering is preserved (message_start before message_stop).
    assert!(
        body.find("message_start") < body.find("message_stop"),
        "SSE frame order preserved"
    );
}

/// AC3: an upstream 429 is forwarded with its status, body and `retry-after` header unchanged.
#[tokio::test]
async fn test_upstream_error_propagation() {
    let stub = spawn_stub().await;
    let gw = spawn_gateway(&stub.base_url).await;
    let client = Client::new();

    let resp = client
        .post(gw.url("/error"))
        .body("{}")
        .send()
        .await
        .expect("POST");

    assert_eq!(
        resp.status(),
        429,
        "upstream 429 status forwarded unchanged"
    );
    let retry = resp
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .expect("retry-after header present");
    assert_eq!(retry, RETRY_AFTER, "retry-after forwarded verbatim");
    let body = resp.text().await.expect("read error body");
    assert_eq!(body, ERROR_BODY, "error body forwarded unchanged");
}

/// AC4: when the client drops the connection mid-stream, the gateway cancels the upstream
/// request promptly (the stub's endless stream is dropped well within 1 s).
#[tokio::test]
async fn test_client_abort_propagates_within_1s() {
    let stub = spawn_stub().await;
    let gw = spawn_gateway(&stub.base_url).await;
    let client = Client::new();

    // Open the endless /slow stream and read only the first frame, then drop the response.
    let resp = client
        .post(gw.url("/slow"))
        .body("{}")
        .send()
        .await
        .expect("POST /slow");
    let mut stream = resp.bytes_stream();
    let _first = stream.next().await.expect("first chunk");
    // Dropping `stream`/response closes the client side; the gateway must then cancel upstream.
    drop(stream);

    // The stub's drop-guard must flip within 1 s of the client disconnect.
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    let mut dropped = false;
    while std::time::Instant::now() < deadline {
        if stub.slow_dropped.load(std::sync::atomic::Ordering::SeqCst) {
            dropped = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        dropped,
        "gateway must cancel the upstream stream within 1s of client abort"
    );
}
