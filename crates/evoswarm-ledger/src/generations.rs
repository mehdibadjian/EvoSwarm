//! Generation ledger (e1-12, AD-7 §1): the `job_generations` table records each fully
//! evaluated generation so a crash resumes at the last *completed* generation rather than
//! inheriting half-scored population state.
//!
//! Commit discipline is the whole guarantee: a row is written only after every candidate in
//! the generation is evaluated and scored, inside a single transaction, so a torn write is
//! never visible to resume.

use rusqlite::params;

use crate::ledger::{JobLedger, LedgerError};

/// The lifecycle state of a committed generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationStatus {
    /// Every candidate evaluated and scored; safe to resume past.
    Completed,
}

impl GenerationStatus {
    fn as_str(self) -> &'static str {
        match self {
            GenerationStatus::Completed => "completed",
        }
    }
}

impl JobLedger {
    /// Commits one generation atomically. The population is stored as opaque JSON so the
    /// schema never changes with the candidate representation. A crash mid-call leaves no
    /// row, which is exactly what makes resume trustworthy.
    pub fn commit_generation(
        &self,
        job_id: &str,
        generation: i64,
        population_json: &str,
        status: GenerationStatus,
    ) -> Result<(), LedgerError> {
        // A single statement is already atomic in SQLite; the explicit transaction keeps the
        // intent clear and lets future multi-row generation metadata commit together.
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO job_generations (job_id, generation, population_json, status)
             VALUES (?1, ?2, ?3, ?4);",
            params![job_id, generation, population_json, status.as_str()],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// The highest committed generation for a job, or `None` if no generation completed yet.
    /// Recovery resumes at this value + 1.
    pub fn max_generation(&self, job_id: &str) -> Result<Option<i64>, LedgerError> {
        let max: Option<i64> = self.conn.query_row(
            "SELECT MAX(generation) FROM job_generations WHERE job_id = ?1;",
            params![job_id],
            |row| row.get(0),
        )?;
        Ok(max)
    }

    /// Reads back a committed generation's population JSON, if it exists.
    pub fn read_generation(
        &self,
        job_id: &str,
        generation: i64,
    ) -> Result<Option<String>, LedgerError> {
        let mut stmt = self.conn.prepare(
            "SELECT population_json FROM job_generations WHERE job_id = ?1 AND generation = ?2;",
        )?;
        let mut rows = stmt.query(params![job_id, generation])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commit_then_max_generation() {
        let l = JobLedger::open_in_memory().unwrap();
        assert_eq!(l.max_generation("j").unwrap(), None);
        l.commit_generation("j", 0, "[]", GenerationStatus::Completed)
            .unwrap();
        l.commit_generation("j", 1, "[]", GenerationStatus::Completed)
            .unwrap();
        assert_eq!(l.max_generation("j").unwrap(), Some(1));
    }

    #[test]
    fn partial_generation_is_never_written() {
        // Committing gen 0 only (a crash before gen 1 commits) leaves max at 0, so resume
        // starts at 1 — never inheriting an uncommitted generation.
        let l = JobLedger::open_in_memory().unwrap();
        l.commit_generation("j", 0, "[\"a\"]", GenerationStatus::Completed)
            .unwrap();
        assert_eq!(l.max_generation("j").unwrap(), Some(0));
        assert!(l.read_generation("j", 1).unwrap().is_none());
        assert_eq!(l.read_generation("j", 0).unwrap().as_deref(), Some("[\"a\"]"));
    }

    #[test]
    fn max_generation_is_per_job() {
        let l = JobLedger::open_in_memory().unwrap();
        l.commit_generation("a", 5, "[]", GenerationStatus::Completed)
            .unwrap();
        l.commit_generation("b", 2, "[]", GenerationStatus::Completed)
            .unwrap();
        assert_eq!(l.max_generation("a").unwrap(), Some(5));
        assert_eq!(l.max_generation("b").unwrap(), Some(2));
    }
}
