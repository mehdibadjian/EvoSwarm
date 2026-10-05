//! e4-2 usage logging — acceptance criteria (story §4).
//!
//! Red-phase tests authored before the `usage` module exists. Gate:
//! `cargo test --test e4_2_usage_logging`. Runs against the local stub upstream
//! (`tests/common/`), no LLM.
//!
//! Story contract: extract the `usage` block from completion SSE events, record
//! (session_id, timestamp, in_tokens, out_tokens, cached_tokens) to local SQLite,
//! **without storing prompt content**; surface totals and cost by day via
//! `evoswarm usage --since <date>` (the CLI subcommand is wired in evoswarm-cli and
//! exercised by its own crate tests; the store/summary API is asserted here).

mod common;

use std::sync::Arc;

use common::{spawn_gateway_with_usage, spawn_stub, SSE_FRAMES};
use evoswarm_gateway::usage::{parse_sse_usage, DailyUsage, UsagePricing, UsageRecord, UsageStore};
use reqwest::Client;

/// The usage numbers embedded in the recorded stub session (message_start +
/// message_delta frames), used to assert extraction is correct rather than vacuous.
const EXPECTED_IN: u64 = 25;
// message_delta's output_tokens is CUMULATIVE and final; message_start's (1) is a
// provisional estimate, so the merged value is max(1, 42) = 42, not a sum.
const EXPECTED_OUT: u64 = 42;
const EXPECTED_CACHED: u64 = 10;

/// Parsing: the Anthropic usage protocol is split across events — message_start carries
/// input/cached tokens, message_delta carries the final output total. The parser merges
/// them and takes the max output seen (message_start's is a provisional estimate).
#[test]
fn test_parse_usage_from_sse_frames() {
    let usage = parse_sse_usage(SSE_FRAMES.concat().as_bytes()).expect("usage extracted");
    assert_eq!(usage.input_tokens, EXPECTED_IN);
    assert_eq!(usage.output_tokens, EXPECTED_OUT);
    assert_eq!(usage.cached_tokens, EXPECTED_CACHED);
}

/// Parsing honesty: a stream with no usage event yields None (nothing is fabricated).
#[test]
fn test_parse_usage_absent_is_none() {
    assert!(parse_sse_usage(b"data: {\"type\":\"ping\"}\n\n").is_none());
    assert!(parse_sse_usage(b"not even sse").is_none());
}

/// AC1: after a request completes through the gateway, a usage row is persisted with the
/// session id, token counts and timestamp — and the schema stores no prompt content.
#[tokio::test]
async fn test_usage_record_persistence() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("usage.db");
    let stub = spawn_stub().await;
    let gw = spawn_gateway_with_usage(&stub.base_url, &db).await;

    let client = Client::new();
    let resp = client
        .post(gw.url("/v1/messages"))
        .header("x-evoswarm-session", "sess-abc")
        .body(r#"{"model":"claude-3-5-sonnet","messages":[{"role":"user","content":"SECRET PROMPT TEXT"}]}"#)
        .send()
        .await
        .expect("POST");
    let bytes = resp.bytes().await.expect("body");
    assert!(!bytes.is_empty(), "stream was forwarded");

    // The tee logs on stream end; the store sees one record for the session.
    let store = UsageStore::open(&db).expect("open store");
    let records = store.records_for_session("sess-abc").expect("records");
    assert_eq!(records.len(), 1, "exactly one usage record logged");
    let r: &UsageRecord = &records[0];
    assert_eq!(r.input_tokens, EXPECTED_IN);
    assert_eq!(r.output_tokens, EXPECTED_OUT);
    assert_eq!(r.cached_tokens, EXPECTED_CACHED);
    assert!(r.timestamp_unix > 0, "timestamp recorded");

    // AC1 honesty: no prompt content is stored anywhere in the database file.
    let raw = std::fs::read(&db).expect("read db");
    let raw_str = String::from_utf8_lossy(&raw);
    assert!(
        !raw_str.contains("SECRET PROMPT TEXT"),
        "prompt content must never be persisted"
    );
}

/// AC1 with no session header: the gateway still logs usage under a generated session id.
#[tokio::test]
async fn test_usage_logged_without_session_header() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("usage.db");
    let stub = spawn_stub().await;
    let gw = spawn_gateway_with_usage(&stub.base_url, &db).await;

    let client = Client::new();
    client
        .post(gw.url("/v1/messages"))
        .body("{}")
        .send()
        .await
        .expect("POST")
        .bytes()
        .await
        .expect("body");

    let store = UsageStore::open(&db).expect("open store");
    let all = store.all_records().expect("all records");
    assert_eq!(all.len(), 1, "logged under a generated session id");
    assert!(!all[0].session_id.is_empty());
}

/// AC1 edge: a non-SSE error response logs nothing (no usage block to extract).
#[tokio::test]
async fn test_error_response_logs_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("usage.db");
    let stub = spawn_stub().await;
    let gw = spawn_gateway_with_usage(&stub.base_url, &db).await;

    let client = Client::new();
    let resp = client
        .post(gw.url("/error"))
        .body("{}")
        .send()
        .await
        .expect("POST");
    assert_eq!(resp.status(), 429);
    resp.bytes().await.expect("body");

    let store = UsageStore::open(&db).expect("open store");
    assert_eq!(
        store.all_records().expect("records").len(),
        0,
        "a 429 error body carries no usage event and must not create a record"
    );
}

/// AC2: the daily summary (what `evoswarm usage --since <date>` prints) aggregates tokens
/// and computes cost from pricing — the math is asserted exactly.
#[test]
fn test_daily_summary_totals_and_cost() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = UsageStore::open(&dir.path().join("usage.db")).expect("open store");

    // Two records on the same UTC day, one on an earlier day.
    let day1 = 1_751_500_000u64; // 2025-07-03 (UTC)
    let day0 = day1 - 86_400; // 2025-07-02
    store
        .log_with_timestamp("s1", day1, 100, 50, 10)
        .expect("log r1");
    store
        .log_with_timestamp("s2", day1, 200, 60, 0)
        .expect("log r2");
    store
        .log_with_timestamp("s3", day0, 7, 3, 0)
        .expect("log r3");

    let pricing = UsagePricing::default(); // $3 in / $15 out / $0.30 cached per 1M tokens
    let since_day0 = day0;
    let days: Vec<DailyUsage> = store.daily_summary(since_day0, &pricing).expect("summary");
    assert_eq!(
        days.len(),
        2,
        "both days included since --since covers them"
    );

    let d1 = days
        .iter()
        .find(|d| d.input_tokens == 300)
        .expect("day1 row");
    assert_eq!(d1.output_tokens, 110);
    assert_eq!(d1.cached_tokens, 10);
    assert_eq!(d1.requests, 2);
    // cost = 300*3/1e6 + 110*15/1e6 + 10*0.3/1e6 = 0.0009 + 0.00165 + 0.000003
    let expected = 300.0 * 3.0 / 1e6 + 110.0 * 15.0 / 1e6 + 10.0 * 0.3 / 1e6;
    assert!(
        (d1.cost_usd - expected).abs() < 1e-12,
        "cost math exact: got {} want {}",
        d1.cost_usd,
        expected
    );

    // --since filters: only day1 when since is inside day0's next day boundary.
    let filtered = store.daily_summary(day1, &pricing).expect("filtered");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].requests, 2);
}

/// The store survives reopen (on-disk durability) and concurrent gateway writes are safe
/// (WAL mode) — the gateway runs in a worker task while tests read.
#[tokio::test]
async fn test_store_reopen_and_shared_handle() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("usage.db");
    {
        let store = UsageStore::open(&db).expect("open");
        store
            .log_with_timestamp("s1", 1_751_500_000, 1, 2, 0)
            .expect("log");
    }
    let store = UsageStore::open(&db).expect("reopen");
    assert_eq!(store.all_records().expect("records").len(), 1);

    // Shared handle across tasks (Arc) — the gateway's tee writes from a spawned task.
    let shared: Arc<UsageStore> = Arc::new(store);
    let c = shared.clone();
    tokio::spawn(async move {
        c.log_with_timestamp("s2", 1_751_500_001, 3, 4, 0)
            .expect("log2");
    })
    .await
    .expect("join");
    assert_eq!(shared.all_records().expect("records").len(), 2);
}
