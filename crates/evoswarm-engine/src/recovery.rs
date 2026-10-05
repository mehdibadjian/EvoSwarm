//! Crash recovery (e1-12, AD-7 §2): on service restart, scan for jobs left `Running`,
//! read the highest committed generation for each, and re-enter the loop at G + 1. A
//! generation is only ever resumed *past* if it was fully committed, so a crash mid-
//! generation discards the partial population rather than merging it.

use evoswarm_ledger::{JobLedger, LedgerError};
use serde::{Deserialize, Serialize};

/// A job eligible to resume after a crash, with the generation the loop should re-enter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumableJob {
    pub job_id: String,
    /// The generation to resume at: the highest committed generation plus one. When no
    /// generation ever completed, this is `0` (restart from the beginning).
    pub resume_at_generation: i64,
}

/// Scans the ledger for jobs interrupted in the `Running` state and computes, for each, the
/// generation the evolutionary loop should resume at. Completed/failed/cancelled jobs are
/// not resumable; a job with no committed generation resumes at 0.
pub fn recover(ledger: &JobLedger) -> Result<Vec<ResumableJob>, LedgerError> {
    let running = ledger.running_job_ids()?;
    let mut resumable = Vec::with_capacity(running.len());
    for job_id in running {
        let resume_at = match ledger.max_generation(&job_id)? {
            Some(g) => g + 1,
            None => 0,
        };
        resumable.push(ResumableJob {
            job_id,
            resume_at_generation: resume_at,
        });
    }
    Ok(resumable)
}

#[cfg(test)]
mod tests {
    use super::*;
    use evoswarm_core::{JobObjective, JobStatus, JobSubmission};
    use evoswarm_ledger::GenerationStatus;
    use std::path::PathBuf;

    fn sub() -> JobSubmission {
        JobSubmission {
            task_description: "t".into(),
            test_command: "pytest".into(),
            target_paths: vec![PathBuf::from("src")],
            budget_tokens: None,
            budget_dollars: None,
            objective: JobObjective::Correctness,
            timeout_secs: 30,
        }
    }

    #[test]
    fn recover_resumes_after_last_committed_generation() {
        let l = JobLedger::open_in_memory().unwrap();
        l.insert_job("j", &sub(), None).unwrap();
        l.transition("j", JobStatus::Running).unwrap();
        // Crash during Gen 2 means Gens 0 and 1 committed, Gen 2 never did.
        l.commit_generation("j", 0, "[]", GenerationStatus::Completed)
            .unwrap();
        l.commit_generation("j", 1, "[]", GenerationStatus::Completed)
            .unwrap();

        let r = recover(&l).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].job_id, "j");
        // Resume at last committed (1) + 1 = 2: the interrupted generation is re-run.
        assert_eq!(r[0].resume_at_generation, 2);
    }

    #[test]
    fn recover_job_with_no_generation_resumes_at_zero() {
        let l = JobLedger::open_in_memory().unwrap();
        l.insert_job("j", &sub(), None).unwrap();
        l.transition("j", JobStatus::Running).unwrap();
        let r = recover(&l).unwrap();
        assert_eq!(r[0].resume_at_generation, 0);
    }

    #[test]
    fn recover_ignores_non_running_jobs() {
        let l = JobLedger::open_in_memory().unwrap();
        l.insert_job("done", &sub(), None).unwrap();
        l.transition("done", JobStatus::Completed).unwrap();
        l.insert_job("queued", &sub(), None).unwrap();
        // Neither is Running, so neither is resumable.
        assert!(recover(&l).unwrap().is_empty());
    }
}
