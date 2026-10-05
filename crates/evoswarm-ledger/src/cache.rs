//! Model-call cache (e1-12, AD-7 §2/§3): keyed by the idempotency hash over the role, the
//! model id and the exact prompt bytes. A resumed job replays a completed call from disk
//! with zero API tokens spent, while a changed prompt misses and dispatches fresh.
//!
//! Sandbox runs are *never* routed through this cache: a killed run's partial output is
//! indistinguishable from a genuine failure and would poison scoring.

use rusqlite::params;

use crate::ledger::{JobLedger, LedgerError};

/// A cached model response, keyed by its idempotency hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedCall {
    pub idempotency_hash: [u8; 32],
    pub model_id: String,
    pub response_text: String,
    pub tokens_in: i64,
    pub tokens_out: i64,
}

/// Hex-encodes the 32-byte hash for use as the SQLite TEXT primary key. A fixed-width hex
/// string keeps the key stable and human-inspectable.
fn hex(hash: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(64);
    for b in hash {
        let _ = write!(s, "{b:02x}");
    }
    s
}

fn unhex(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

impl JobLedger {
    /// Looks up a cached response by idempotency hash. `None` is a cache miss, which the
    /// dispatch path treats as "call the model for real".
    pub fn lookup(&self, hash: &[u8; 32]) -> Result<Option<CachedCall>, LedgerError> {
        let key = hex(hash);
        let mut stmt = self.conn.prepare(
            "SELECT idempotency_hash, model_id, response_text, tokens_in, tokens_out
             FROM model_call_cache WHERE idempotency_hash = ?1;",
        )?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            Some(row) => {
                let stored: String = row.get(0)?;
                let model_id: String = row.get(1)?;
                let response_text: String = row.get(2)?;
                let tokens_in: i64 = row.get(3)?;
                let tokens_out: i64 = row.get(4)?;
                let idempotency_hash = unhex(&stored).ok_or_else(|| {
                    LedgerError::Sqlite(rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(std::io::Error::other("bad hex hash")),
                    ))
                })?;
                Ok(Some(CachedCall {
                    idempotency_hash,
                    model_id,
                    response_text,
                    tokens_in,
                    tokens_out,
                }))
            }
            None => Ok(None),
        }
    }

    /// Stores a completed call. Inserting an existing hash is a no-op overwrite; the hash is
    /// content-addressed so the stored response for a given key never meaningfully changes.
    pub fn store(&self, call: &CachedCall) -> Result<(), LedgerError> {
        self.conn.execute(
            "INSERT OR REPLACE INTO model_call_cache
             (idempotency_hash, model_id, response_text, tokens_in, tokens_out)
             VALUES (?1, ?2, ?3, ?4, ?5);",
            params![
                hex(&call.idempotency_hash),
                call.model_id,
                call.response_text,
                call.tokens_in,
                call.tokens_out,
            ],
        )?;
        Ok(())
    }

    /// Total tokens already spent by a job's cached calls — used to reconcile the e1-10
    /// budget so a resumed job cannot re-spend budget it already recorded.
    pub fn cached_token_totals(&self) -> Result<(i64, i64), LedgerError> {
        let (tin, tout): (i64, i64) = self.conn.query_row(
            "SELECT COALESCE(SUM(tokens_in),0), COALESCE(SUM(tokens_out),0) FROM model_call_cache;",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok((tin, tout))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(byte: u8) -> CachedCall {
        CachedCall {
            idempotency_hash: [byte; 32],
            model_id: "m".into(),
            response_text: format!("resp-{byte}"),
            tokens_in: 10,
            tokens_out: 20,
        }
    }

    #[test]
    fn store_then_lookup_roundtrip() {
        let l = JobLedger::open_in_memory().unwrap();
        let c = call(1);
        assert!(l.lookup(&c.idempotency_hash).unwrap().is_none());
        l.store(&c).unwrap();
        let got = l.lookup(&c.idempotency_hash).unwrap().expect("hit");
        assert_eq!(got, c);
    }

    #[test]
    fn different_hash_is_a_miss() {
        let l = JobLedger::open_in_memory().unwrap();
        l.store(&call(1)).unwrap();
        // A one-byte change in the prompt → different hash → miss (fresh dispatch).
        assert!(l.lookup(&[2; 32]).unwrap().is_none());
    }

    #[test]
    fn token_totals_sum() {
        let l = JobLedger::open_in_memory().unwrap();
        l.store(&call(1)).unwrap();
        l.store(&call(2)).unwrap();
        let (tin, tout) = l.cached_token_totals().unwrap();
        assert_eq!(tin, 20);
        assert_eq!(tout, 40);
    }

    #[test]
    fn hex_roundtrip_is_lossless() {
        let h = [0xab; 32];
        assert_eq!(unhex(&hex(&h)), Some(h));
        assert_eq!(unhex(&hex(&[0; 32])), Some([0; 32]));
    }
}
