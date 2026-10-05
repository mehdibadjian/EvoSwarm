use std::path::{Path, PathBuf};

use evoswarm_core::{JobStatus, JobSubmission};
use rusqlite::{params, Connection};
use thiserror::Error;

/// The resolved repository root every submitted `--paths` entry is contained within.
/// Kept as a newtype so containment checks cannot accidentally receive an arbitrary path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRoot(pub PathBuf);

impl RepoRoot {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

#[derive(Debug, Error)]
pub enum LedgerError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("job not found: {0}")]
    NotFound(String),
    #[error("illegal status transition {from:?} -> {to:?}")]
    IllegalTransition { from: JobStatus, to: JobStatus },
}

/// A persisted job row: the submission plus the server-assigned id, live status and the
/// base commit recorded at submission time (so e1-11 can emit a patch without depending
/// on HEAD when the job completes).
#[derive(Debug, Clone, PartialEq)]
pub struct JobRecord {
    pub job_id: String,
    pub status: JobStatus,
    pub submission: JobSubmission,
    pub base_commit: Option<String>,
    /// The held-out split persisted with the job so e1-12 resume reuses the identical
    /// partition instead of recomputing from a possibly-changed suite. Serialised JSON.
    pub split_json: Option<String>,
}

/// Legal job-state transitions. Mirrors the lifecycle in AD-7: a job moves forward
/// through queued → running → a terminal state, and may be cancelled from any
/// non-terminal state. Rejecting illegal jumps is what makes a resumed ledger trustworthy.
const LEGAL: &[JobStatus] = &[
    JobStatus::Queued,
    JobStatus::Running,
    JobStatus::Completed,
    JobStatus::Failed,
    JobStatus::BudgetExhausted,
    JobStatus::Cancelled,
];

fn legal_transition(from: JobStatus, to: JobStatus) -> bool {
    if !LEGAL.contains(&to) {
        return false;
    }
    match from {
        JobStatus::Queued => matches!(
            to,
            JobStatus::Running
                | JobStatus::Completed
                | JobStatus::Failed
                | JobStatus::BudgetExhausted
                | JobStatus::Cancelled
        ),
        JobStatus::Running => matches!(
            to,
            JobStatus::Completed | JobStatus::Failed | JobStatus::BudgetExhausted | JobStatus::Cancelled
        ),
        // A terminal state never moves again; re-running is a new job.
        _ => false,
    }
}

/// The SQLite-backed job ledger. One connection per ledger; WAL + synchronous=FULL make a
/// committed transition durable across a crash, which is the entire point of AD-7.
pub struct JobLedger {
    pub(crate) conn: Connection,
}

impl JobLedger {
    /// Opens (creating if absent) the ledger database at `path`, applies WAL + FULL
    /// durability pragmas and ensures the schema exists.
    pub fn open(path: &Path) -> Result<Self, LedgerError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        // Durability first: WAL survives process kill; synchronous=FULL survives power cut.
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        let ledger = Self { conn };
        ledger.migrate()?;
        Ok(ledger)
    }

    /// An in-memory ledger for tests that do not need on-disk durability.
    pub fn open_in_memory() -> Result<Self, LedgerError> {
        let conn = Connection::open_in_memory()?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        let ledger = Self { conn };
        ledger.migrate()?;
        Ok(ledger)
    }

    fn migrate(&self) -> Result<(), LedgerError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS jobs (
                job_id       TEXT PRIMARY KEY,
                status       TEXT NOT NULL,
                submission   TEXT NOT NULL,
                base_commit  TEXT,
                split_json   TEXT,
                created_at   INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );
            CREATE TABLE IF NOT EXISTS job_generations (
                job_id         TEXT NOT NULL,
                generation     INTEGER NOT NULL,
                population_json TEXT NOT NULL,
                status         TEXT NOT NULL,
                created_at     TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY(job_id, generation)
            );
            CREATE TABLE IF NOT EXISTS model_call_cache (
                idempotency_hash TEXT PRIMARY KEY,
                model_id         TEXT NOT NULL,
                response_text    TEXT NOT NULL,
                tokens_in        INTEGER NOT NULL,
                tokens_out       INTEGER NOT NULL,
                created_at       TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        Ok(())
    }

    /// Inserts a new job in `Queued` state atomically. The submission is stored as JSON so
    /// the schema never has to grow a column per submission field.
    pub fn insert_job(
        &self,
        job_id: &str,
        submission: &JobSubmission,
        base_commit: Option<&str>,
    ) -> Result<(), LedgerError> {
        let json = serde_json::to_string(submission).map_err(|e| {
            LedgerError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        })?;
        self.conn.execute(
            "INSERT INTO jobs (job_id, status, submission, base_commit) VALUES (?1, ?2, ?3, ?4);",
            params![
                job_id,
                status_str(JobStatus::Queued),
                json,
                base_commit
            ],
        )?;
        Ok(())
    }

    /// Atomically moves a job to `to`, rejecting illegal transitions before any write.
    pub fn transition(&self, job_id: &str, to: JobStatus) -> Result<(), LedgerError> {
        let from = self.read_status(job_id)?;
        if !legal_transition(from, to) {
            return Err(LedgerError::IllegalTransition { from, to });
        }
        let updated = self.conn.execute(
            "UPDATE jobs SET status = ?1 WHERE job_id = ?2;",
            params![status_str(to), job_id],
        )?;
        if updated == 0 {
            return Err(LedgerError::NotFound(job_id.to_string()));
        }
        Ok(())
    }

    pub fn read_status(&self, job_id: &str) -> Result<JobStatus, LedgerError> {
        let s: String = self
            .conn
            .query_row(
                "SELECT status FROM jobs WHERE job_id = ?1;",
                params![job_id],
                |row| row.get(0),
            )
            .map_err(|_| LedgerError::NotFound(job_id.to_string()))?;
        Ok(status_from_str(&s))
    }

    /// Reads the full job row, reconstructing the submission from its stored JSON.
    pub fn read_job(&self, job_id: &str) -> Result<JobRecord, LedgerError> {
        self.conn
            .query_row(
                "SELECT job_id, status, submission, base_commit, split_json FROM jobs WHERE job_id = ?1;",
                params![job_id],
                |row| {
                    let status: String = row.get(1)?;
                    let submission_json: String = row.get(2)?;
                    let base_commit: Option<String> = row.get(3)?;
                    let split_json: Option<String> = row.get(4)?;
                    Ok((row.get::<_, String>(0)?, status, submission_json, base_commit, split_json))
                },
            )
            .map_err(|_| LedgerError::NotFound(job_id.to_string()))
            .and_then(|(job_id, status, submission_json, base_commit, split_json)| {
                let submission: JobSubmission = serde_json::from_str(&submission_json)
                    .map_err(|e| {
                        LedgerError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
                    })?;
                Ok(JobRecord {
                    job_id,
                    status: status_from_str(&status),
                    submission,
                    base_commit,
                    split_json,
                })
            })
    }

    /// Persists the held-out split for a job so resume reuses the identical partition.
    pub fn store_split(&self, job_id: &str, split_json: &str) -> Result<(), LedgerError> {
        let updated = self.conn.execute(
            "UPDATE jobs SET split_json = ?1 WHERE job_id = ?2;",
            params![split_json, job_id],
        )?;
        if updated == 0 {
            return Err(LedgerError::NotFound(job_id.to_string()));
        }
        Ok(())
    }

    /// Job ids currently in `Running` state, oldest first — the resume scan of e1-12.
    pub fn running_job_ids(&self) -> Result<Vec<String>, LedgerError> {
        let mut stmt = self
            .conn
            .prepare("SELECT job_id FROM jobs WHERE status = ?1 ORDER BY created_at ASC;")?;
        let ids = stmt
            .query_map(params![status_str(JobStatus::Running)], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(ids)
    }

    /// True when a job id already exists — the idempotency guard that stops a resubmission
    /// from double-creating the same job.
    pub fn exists(&self, job_id: &str) -> Result<bool, LedgerError> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(1) FROM jobs WHERE job_id = ?1;",
            params![job_id],
            |row| row.get(0),
        )?;
        Ok(n > 0)
    }
}

impl From<std::io::Error> for LedgerError {
    fn from(e: std::io::Error) -> Self {
        LedgerError::Sqlite(rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }
}

fn status_str(s: JobStatus) -> &'static str {
    match s {
        JobStatus::Queued => "queued",
        JobStatus::Running => "running",
        JobStatus::Completed => "completed",
        JobStatus::Failed => "failed",
        JobStatus::BudgetExhausted => "budget_exhausted",
        JobStatus::Cancelled => "cancelled",
    }
}

fn status_from_str(s: &str) -> JobStatus {
    match s {
        "running" => JobStatus::Running,
        "completed" => JobStatus::Completed,
        "failed" => JobStatus::Failed,
        "budget_exhausted" => JobStatus::BudgetExhausted,
        "cancelled" => JobStatus::Cancelled,
        _ => JobStatus::Queued,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use evoswarm_core::JobObjective;

    fn sub() -> JobSubmission {
        JobSubmission {
            task_description: "t".into(),
            test_command: "pytest".into(),
            target_paths: vec![PathBuf::from("src")],
            budget_tokens: Some(1000),
            budget_dollars: Some(1.0),
            objective: JobObjective::Correctness,
            timeout_secs: 30,
        }
    }

    #[test]
    fn insert_and_read_roundtrip() {
        let l = JobLedger::open_in_memory().unwrap();
        l.insert_job("j1", &sub(), Some("abc123")).unwrap();
        assert_eq!(l.read_status("j1").unwrap(), JobStatus::Queued);
        let rec = l.read_job("j1").unwrap();
        assert_eq!(rec.submission, sub());
        assert_eq!(rec.base_commit.as_deref(), Some("abc123"));
    }

    #[test]
    fn legal_transition_queued_to_running_to_completed() {
        let l = JobLedger::open_in_memory().unwrap();
        l.insert_job("j1", &sub(), None).unwrap();
        l.transition("j1", JobStatus::Running).unwrap();
        assert_eq!(l.read_status("j1").unwrap(), JobStatus::Running);
        l.transition("j1", JobStatus::Completed).unwrap();
        assert_eq!(l.read_status("j1").unwrap(), JobStatus::Completed);
    }

    #[test]
    fn terminal_state_cannot_transition() {
        let l = JobLedger::open_in_memory().unwrap();
        l.insert_job("j1", &sub(), None).unwrap();
        l.transition("j1", JobStatus::Completed).unwrap();
        let err = l.transition("j1", JobStatus::Running).unwrap_err();
        assert!(matches!(err, LedgerError::IllegalTransition { .. }));
    }

    #[test]
    fn read_missing_job_is_not_found() {
        let l = JobLedger::open_in_memory().unwrap();
        assert!(matches!(
            l.read_status("nope").unwrap_err(),
            LedgerError::NotFound(_)
        ));
    }

    #[test]
    fn exists_reflects_insert() {
        let l = JobLedger::open_in_memory().unwrap();
        assert!(!l.exists("j1").unwrap());
        l.insert_job("j1", &sub(), None).unwrap();
        assert!(l.exists("j1").unwrap());
    }

    #[test]
    fn on_disk_ledger_uses_wal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".evoswarm/jobs.db");
        let l = JobLedger::open(&path).unwrap();
        let mode: String = l
            .conn
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
        l.insert_job("j1", &sub(), None).unwrap();
        // Reopen from disk: durability check.
        drop(l);
        let l2 = JobLedger::open(&path).unwrap();
        assert_eq!(l2.read_status("j1").unwrap(), JobStatus::Queued);
    }

    #[test]
    fn running_job_ids_scan() {
        let l = JobLedger::open_in_memory().unwrap();
        l.insert_job("a", &sub(), None).unwrap();
        l.insert_job("b", &sub(), None).unwrap();
        l.transition("a", JobStatus::Running).unwrap();
        let running = l.running_job_ids().unwrap();
        assert_eq!(running, vec!["a".to_string()]);
    }

    #[test]
    fn store_and_read_split() {
        let l = JobLedger::open_in_memory().unwrap();
        l.insert_job("j1", &sub(), None).unwrap();
        l.store_split("j1", r#"{"visible":[],"held_out":[]}"#).unwrap();
        assert_eq!(
            l.read_job("j1").unwrap().split_json.as_deref(),
            Some(r#"{"visible":[],"held_out":[]}"#)
        );
    }
}
