//! Local token & cost accounting (e4-2, AD-2).
//!
//! The gateway tees the upstream SSE stream: bytes flow through untouched (e4-1 fidelity),
//! and when the stream ends the recorded bytes are scanned for Anthropic `usage` blocks.
//! Input and cache-read token counts come from the `message_start` event; the final output
//! total comes from `message_delta` (its `usage.output_tokens` is cumulative, and
//! `message_start`'s output figure is only a provisional estimate, so the max is kept).
//! Only counts, session id and timestamp are persisted — never prompt or completion text.

use std::path::Path;
use std::sync::Mutex;

use rusqlite::{params, Connection};
use thiserror::Error;

/// Errors from the usage store.
#[derive(Debug, Error)]
pub enum UsageError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// Token counts extracted from one completed SSE stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
}

/// One persisted usage row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageRecord {
    pub session_id: String,
    pub timestamp_unix: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
}

/// Token pricing in USD per million tokens, per component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsagePricing {
    pub input_per_million: f64,
    pub output_per_million: f64,
    pub cached_per_million: f64,
}

impl Default for UsagePricing {
    /// Sonnet-class list pricing used for local estimates.
    fn default() -> Self {
        Self {
            input_per_million: 3.0,
            output_per_million: 15.0,
            cached_per_million: 0.3,
        }
    }
}

impl UsagePricing {
    /// Estimated cost in USD for a set of token counts.
    pub fn cost(&self, input: u64, output: u64, cached: u64) -> f64 {
        input as f64 * self.input_per_million / 1e6
            + output as f64 * self.output_per_million / 1e6
            + cached as f64 * self.cached_per_million / 1e6
    }
}

/// Per-day aggregate produced by [`UsageStore::daily_summary`] (what
/// `evoswarm usage --since <date>` prints).
#[derive(Debug, Clone, PartialEq)]
pub struct DailyUsage {
    /// UTC day as `YYYY-MM-DD`.
    pub day: String,
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub cost_usd: f64,
}

/// Extracts the merged usage from a completed SSE body, or `None` when the stream
/// carries no usage event (e.g. an error body) — nothing is fabricated.
pub fn parse_sse_usage(body: &[u8]) -> Option<StreamUsage> {
    let text = String::from_utf8_lossy(body);
    let mut usage = None;
    // Each SSE event is a `data:` line carrying one JSON object; usage blocks appear on
    // message_start (input/cached) and message_delta (final output total).
    for line in text.lines() {
        let Some(payload) = line.strip_prefix("data: ") else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(payload) else {
            continue;
        };
        let Some(u) = json.pointer("/message/usage").or_else(|| json.get("usage")) else {
            continue;
        };
        let got = usage.get_or_insert(StreamUsage {
            input_tokens: 0,
            output_tokens: 0,
            cached_tokens: 0,
        });
        if let Some(n) = u.get("input_tokens").and_then(|v| v.as_u64()) {
            got.input_tokens = got.input_tokens.max(n);
        }
        if let Some(n) = u.get("output_tokens").and_then(|v| v.as_u64()) {
            got.output_tokens = got.output_tokens.max(n);
        }
        if let Some(n) = u.get("cache_read_input_tokens").and_then(|v| v.as_u64()) {
            got.cached_tokens = got.cached_tokens.max(n);
        }
    }
    usage
}

/// SQLite-backed usage log. WAL mode so the gateway's worker-task writer never blocks a
/// concurrent `evoswarm usage` reader. One connection, mutex-guarded: rusqlite's
/// `Connection` is `Send` but not `Sync`, and writes here are tiny and infrequent.
pub struct UsageStore {
    conn: Mutex<Connection>,
}

impl UsageStore {
    /// Opens (creating if absent) the usage database at `path`.
    pub fn open(path: &Path) -> Result<Self, UsageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                UsageError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
            })?;
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        let store = Self {
            conn: Mutex::new(conn),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), UsageError> {
        // The schema deliberately has no column that could hold prompt text.
        self.conn.lock().expect("usage mutex").execute_batch(
            "CREATE TABLE IF NOT EXISTS usage_log (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id     TEXT NOT NULL,
                timestamp_unix INTEGER NOT NULL,
                input_tokens   INTEGER NOT NULL,
                output_tokens  INTEGER NOT NULL,
                cached_tokens  INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS usage_log_ts ON usage_log(timestamp_unix);",
        )?;
        Ok(())
    }

    /// Logs one record at the current time.
    pub fn log(&self, session_id: &str, usage: StreamUsage) -> Result<(), UsageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.log_with_timestamp(
            session_id,
            now,
            usage.input_tokens,
            usage.output_tokens,
            usage.cached_tokens,
        )
    }

    /// Logs one record with an explicit timestamp (deterministic tests; summary math).
    pub fn log_with_timestamp(
        &self,
        session_id: &str,
        timestamp_unix: u64,
        input_tokens: u64,
        output_tokens: u64,
        cached_tokens: u64,
    ) -> Result<(), UsageError> {
        self.conn.lock().expect("usage mutex").execute(
            "INSERT INTO usage_log
                (session_id, timestamp_unix, input_tokens, output_tokens, cached_tokens)
             VALUES (?1, ?2, ?3, ?4, ?5);",
            params![
                session_id,
                timestamp_unix as i64,
                input_tokens as i64,
                output_tokens as i64,
                cached_tokens as i64
            ],
        )?;
        Ok(())
    }

    /// Every record, oldest first.
    pub fn all_records(&self) -> Result<Vec<UsageRecord>, UsageError> {
        self.query_records(0, None)
    }

    /// Records for one session, oldest first.
    pub fn records_for_session(&self, session_id: &str) -> Result<Vec<UsageRecord>, UsageError> {
        self.query_records(0, Some(session_id))
    }

    fn query_records(
        &self,
        since: u64,
        session_id: Option<&str>,
    ) -> Result<Vec<UsageRecord>, UsageError> {
        let conn = self.conn.lock().expect("usage mutex");
        let (sql, params): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match session_id {
            Some(s) => (
                "SELECT session_id, timestamp_unix, input_tokens, output_tokens, cached_tokens
                 FROM usage_log WHERE session_id = ?1 AND timestamp_unix >= ?2
                 ORDER BY timestamp_unix ASC, id ASC;",
                vec![Box::new(s.to_string()), Box::new(since as i64)],
            ),
            None => (
                "SELECT session_id, timestamp_unix, input_tokens, output_tokens, cached_tokens
                 FROM usage_log WHERE timestamp_unix >= ?1
                 ORDER BY timestamp_unix ASC, id ASC;",
                vec![Box::new(since as i64)],
            ),
        };
        let mut stmt = conn.prepare(sql)?;
        let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(refs.as_slice(), |row| {
            Ok(UsageRecord {
                session_id: row.get(0)?,
                timestamp_unix: row.get::<_, i64>(1)? as u64,
                input_tokens: row.get::<_, i64>(2)? as u64,
                output_tokens: row.get::<_, i64>(3)? as u64,
                cached_tokens: row.get::<_, i64>(4)? as u64,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Aggregates records from `since_unix` onward into per-UTC-day rows with cost
    /// computed from `pricing`. Days with no records are absent (not zero rows).
    pub fn daily_summary(
        &self,
        since_unix: u64,
        pricing: &UsagePricing,
    ) -> Result<Vec<DailyUsage>, UsageError> {
        let conn = self.conn.lock().expect("usage mutex");
        let mut stmt = conn.prepare(
            "SELECT strftime('%Y-%m-%d', timestamp_unix, 'unixepoch') AS day,
                    COUNT(*) AS requests,
                    COALESCE(SUM(input_tokens), 0),
                    COALESCE(SUM(output_tokens), 0),
                    COALESCE(SUM(cached_tokens), 0)
             FROM usage_log
             WHERE timestamp_unix >= ?1
             GROUP BY day
             ORDER BY day ASC;",
        )?;
        let rows = stmt.query_map(params![since_unix as i64], |row| {
            let (day, requests): (String, i64) = (row.get(0)?, row.get(1)?);
            let (i, o, c): (i64, i64, i64) = (row.get(2)?, row.get(3)?, row.get(4)?);
            Ok((day, requests as u64, i as u64, o as u64, c as u64))
        })?;
        let mut out = Vec::new();
        for r in rows {
            let (day, requests, input_tokens, output_tokens, cached_tokens) = r?;
            out.push(DailyUsage {
                cost_usd: pricing.cost(input_tokens, output_tokens, cached_tokens),
                day,
                requests,
                input_tokens,
                output_tokens,
                cached_tokens,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pricing_cost_math() {
        let p = UsagePricing::default();
        // 1M of each: 3 + 15 + 0.3
        assert!((p.cost(1_000_000, 1_000_000, 1_000_000) - 18.3).abs() < 1e-9);
        assert_eq!(p.cost(0, 0, 0), 0.0);
    }

    #[test]
    fn parse_takes_max_output_across_events() {
        let body = concat!(
            "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":10,\"output_tokens\":1}}}\n\n",
            "data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":7}}\n\n",
        );
        let u = parse_sse_usage(body.as_bytes()).unwrap();
        assert_eq!(
            u,
            StreamUsage {
                input_tokens: 10,
                output_tokens: 7,
                cached_tokens: 0
            }
        );
    }

    #[test]
    fn log_and_read_back() {
        let dir = tempfile::tempdir().unwrap();
        let s = UsageStore::open(&dir.path().join("u.db")).unwrap();
        s.log(
            "sess",
            StreamUsage {
                input_tokens: 1,
                output_tokens: 2,
                cached_tokens: 3,
            },
        )
        .unwrap();
        let recs = s.records_for_session("sess").unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].cached_tokens, 3);
        assert_eq!(s.records_for_session("other").unwrap().len(), 0);
    }
}
