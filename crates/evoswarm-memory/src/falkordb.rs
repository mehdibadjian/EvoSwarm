//! FalkorDB lineage writer and graph integration (e2-2, AD-3).
//!
//! Stores task lineage, implementation nodes, evaluation outcomes, and test cases in a
//! FalkorDB graph using openCypher statements.
//!
//! ## Invariants (AD-3, AD-4)
//! - Graph nodes store identifiers, scalar scores, and content hashes only; heavy diffs and
//!   source files are stored in the content-addressed blob store (`patch_blob_sha256`).
//! - Lookups by `spec_hash`, `repo_fingerprint`, and `toolchain_fingerprint` use schema indices.
//! - If FalkorDB is unavailable, job completion is not failed: the record is saved to an
//!   on-disk retry spool and flushed when the database recovers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FalkorError {
    #[error("connection error: {0}")]
    ConnectionError(String),
    #[error("query error: {0}")]
    QueryError(String),
    #[error("spool I/O error: {0}")]
    SpoolError(String),
}

/// A parent provenance relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeRelation {
    pub parent_id: String,
    pub relation: String,
    pub traits: String,
}

/// A test case execution record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCaseRecord {
    pub name: String,
    pub origin: String,
    pub passed: bool,
}

/// An evaluation record for a candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationRecord {
    pub id: String,
    pub candidate_id: String,
    pub score: f64,
    pub passed_gates: bool,
    pub wall_time_ms: u64,
    pub tests: Vec<TestCaseRecord>,
}

/// An evolved candidate implementation node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImplementationRecord {
    pub id: String,
    pub generation: u32,
    pub patch_blob_sha256: String,
    pub model_role: String,
    pub parents: Vec<EdgeRelation>,
}

/// Top-level Task record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRecord {
    pub id: String,
    pub spec_hash: String,
    pub repo_fingerprint: String,
    pub toolchain_fingerprint: String,
}

/// Full record of a completed job ready for graph ingestion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JobRecord {
    pub task: TaskRecord,
    pub implementations: Vec<ImplementationRecord>,
    pub evaluations: Vec<EvaluationRecord>,
}

/// The result of a write attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    Committed,
    Spooled,
}

/// Abstract FalkorDB client interface for executing openCypher statements.
#[async_trait]
pub trait FalkorClient: Send + Sync {
    async fn execute_query(&self, cypher: &str) -> Result<(), FalkorError>;
}

/// An in-process mock client recording Cypher statements for testing and offline environments.
#[derive(Clone, Default)]
pub struct MockFalkorClient {
    executed: Arc<Mutex<Vec<String>>>,
    failing: bool,
}

impl MockFalkorClient {
    pub fn new() -> Self {
        Self {
            executed: Arc::new(Mutex::new(Vec::new())),
            failing: false,
        }
    }

    pub fn new_failing() -> Self {
        Self {
            executed: Arc::new(Mutex::new(Vec::new())),
            failing: true,
        }
    }

    pub fn executed_statements(&self) -> Vec<String> {
        self.executed.lock().unwrap().clone()
    }
}

#[async_trait]
impl FalkorClient for MockFalkorClient {
    async fn execute_query(&self, cypher: &str) -> Result<(), FalkorError> {
        if self.failing {
            return Err(FalkorError::ConnectionError("FalkorDB connection refused".into()));
        }
        self.executed.lock().unwrap().push(cypher.to_string());
        Ok(())
    }
}

/// Manages recording lineage graph structures and retry spooling.
pub struct LineageWriter<C: FalkorClient> {
    client: C,
    spool_dir: PathBuf,
}

impl<C: FalkorClient> LineageWriter<C> {
    pub fn new(client: C, spool_dir: &Path) -> Self {
        Self {
            client,
            spool_dir: spool_dir.to_path_buf(),
        }
    }

    /// Creates the required indices in FalkorDB for rapid fingerprinted replay lookups (AC2).
    pub async fn ensure_indexes(&self) -> Result<(), FalkorError> {
        let index_queries = [
            "CREATE INDEX FOR (t:Task) ON (t.spec_hash)",
            "CREATE INDEX FOR (t:Task) ON (t.repo_fingerprint)",
            "CREATE INDEX FOR (t:Task) ON (t.toolchain_fingerprint)",
            "CREATE INDEX FOR (i:Implementation) ON (i.id)",
        ];

        for query in index_queries {
            self.client.execute_query(query).await?;
        }
        Ok(())
    }

    /// Generates Cypher lookup statement for a `spec_hash`.
    pub fn query_by_spec_hash(&self, spec_hash: &str) -> String {
        format!("MATCH (t:Task {{spec_hash: '{spec_hash}'}}) RETURN t")
    }

    /// Generates Cypher lookup statement for the triple fingerprint invalidation check (AD-4).
    pub fn query_by_triple_fingerprints(
        &self,
        spec_hash: &str,
        repo_fingerprint: &str,
        toolchain_fingerprint: &str,
    ) -> String {
        format!(
            "MATCH (t:Task {{spec_hash: '{spec_hash}', repo_fingerprint: '{repo_fingerprint}', toolchain_fingerprint: '{toolchain_fingerprint}'}}) RETURN t"
        )
    }

    /// Records a completed job in FalkorDB. If FalkorDB is unavailable, writes the payload
    /// to the retry spool on disk so job progression is not blocked (AC3).
    pub async fn record_job(&self, job: &JobRecord) -> Result<WriteOutcome, FalkorError> {
        let statements = self.generate_cypher_statements(job);

        // Attempt writing to FalkorDB
        let mut failed = false;
        for stmt in &statements {
            if let Err(_err) = self.client.execute_query(stmt).await {
                failed = true;
                break;
            }
        }

        if failed {
            self.spool_record(job)?;
            Ok(WriteOutcome::Spooled)
        } else {
            Ok(WriteOutcome::Committed)
        }
    }

    fn generate_cypher_statements(&self, job: &JobRecord) -> Vec<String> {
        let mut statements = Vec::new();

        // 1. Task node
        statements.push(format!(
            "MERGE (t:Task {{id: '{id}'}}) \
             SET t.spec_hash = '{spec}', \
                 t.repo_fingerprint = '{repo}', \
                 t.toolchain_fingerprint = '{tool}'",
            id = job.task.id,
            spec = job.task.spec_hash,
            repo = job.task.repo_fingerprint,
            tool = job.task.toolchain_fingerprint
        ));

        // 2. Implementation nodes & lineage edges
        for imp in &job.implementations {
            statements.push(format!(
                "MERGE (i:Implementation {{id: '{id}'}}) \
                 SET i.generation = {gen}, \
                     i.patch_blob_sha256 = '{blob}', \
                     i.model_role = '{role}'",
                id = imp.id,
                gen = imp.generation,
                blob = imp.patch_blob_sha256,
                role = imp.model_role
            ));

            statements.push(format!(
                "MATCH (t:Task {{id: '{task_id}'}}), (i:Implementation {{id: '{imp_id}'}}) \
                 MERGE (t)-[:HAS_IMPLEMENTATION]->(i)",
                task_id = job.task.id,
                imp_id = imp.id
            ));

            for parent in &imp.parents {
                if parent.relation == "MERGED_FROM" {
                    statements.push(format!(
                        "MATCH (i:Implementation {{id: '{child}'}}), (p:Implementation {{id: '{parent_id}'}}) \
                         MERGE (i)-[:MERGED_FROM {{traits: '{traits}'}}]->(p)",
                        child = imp.id,
                        parent_id = parent.parent_id,
                        traits = parent.traits
                    ));
                } else {
                    statements.push(format!(
                        "MATCH (i:Implementation {{id: '{child}'}}), (p:Implementation {{id: '{parent_id}'}}) \
                         MERGE (i)-[:MUTATED_FROM]->(p)",
                        child = imp.id,
                        parent_id = parent.parent_id
                    ));
                }
            }
        }

        // 3. Evaluation nodes & TestCase edges
        for eval in &job.evaluations {
            statements.push(format!(
                "MERGE (e:Evaluation {{id: '{id}'}}) \
                 SET e.score = {score}, \
                     e.passed_gates = {passed}, \
                     e.wall_time_ms = {time}",
                id = eval.id,
                score = eval.score,
                passed = eval.passed_gates,
                time = eval.wall_time_ms
            ));

            statements.push(format!(
                "MATCH (i:Implementation {{id: '{imp_id}'}}), (e:Evaluation {{id: '{eval_id}'}}) \
                 MERGE (i)-[:EVALUATED_BY]->(e)",
                imp_id = eval.candidate_id,
                eval_id = eval.id
            ));

            for test in &eval.tests {
                statements.push(format!(
                    "MERGE (tc:TestCase {{name: '{name}', origin: '{origin}'}})",
                    name = test.name,
                    origin = test.origin
                ));

                statements.push(format!(
                    "MATCH (e:Evaluation {{id: '{eval_id}'}}), (tc:TestCase {{name: '{name}', origin: '{origin}'}}) \
                     MERGE (e)-[:TESTED_WITH {{passed: {passed}}}]->(tc)",
                    eval_id = eval.id,
                    name = test.name,
                    origin = test.origin,
                    passed = test.passed
                ));
            }
        }

        statements
    }

    fn spool_record(&self, job: &JobRecord) -> Result<(), FalkorError> {
        fs::create_dir_all(&self.spool_dir)
            .map_err(|e| FalkorError::SpoolError(format!("mkdir spool: {e}")))?;

        let filename = format!("spool_{}.json", job.task.id);
        let path = self.spool_dir.join(filename);
        let serialized = serde_json::to_string(job)
            .map_err(|e| FalkorError::SpoolError(format!("serialize job: {e}")))?;

        fs::write(&path, serialized)
            .map_err(|e| FalkorError::SpoolError(format!("write spool file: {e}")))?;
        Ok(())
    }

    /// Number of spooled jobs waiting for retry.
    pub fn spool_count(&self) -> Result<usize, FalkorError> {
        if !self.spool_dir.exists() {
            return Ok(0);
        }
        let count = fs::read_dir(&self.spool_dir)
            .map_err(|e| FalkorError::SpoolError(format!("read spool dir: {e}")))?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
            .count();
        Ok(count)
    }

    /// Replays all spooled job records against the client, removing successfully committed files.
    pub async fn replay_spool(&self) -> Result<usize, FalkorError> {
        if !self.spool_dir.exists() {
            return Ok(0);
        }

        let mut replayed = 0;
        let entries = fs::read_dir(&self.spool_dir)
            .map_err(|e| FalkorError::SpoolError(format!("read spool dir: {e}")))?;

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let content = fs::read_to_string(&path)
                    .map_err(|e| FalkorError::SpoolError(format!("read spool file: {e}")))?;
                let job: JobRecord = serde_json::from_str(&content)
                    .map_err(|e| FalkorError::SpoolError(format!("deserialize spool: {e}")))?;

                let statements = self.generate_cypher_statements(&job);
                let mut success = true;
                for stmt in &statements {
                    if let Err(_) = self.client.execute_query(stmt).await {
                        success = false;
                        break;
                    }
                }

                if success {
                    let _ = fs::remove_file(&path);
                    replayed += 1;
                }
            }
        }

        Ok(replayed)
    }
}
