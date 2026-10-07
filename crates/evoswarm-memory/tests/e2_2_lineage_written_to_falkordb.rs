//! e2-2 Lineage written to FalkorDB — acceptance criteria (story §4).
//!
//! Gate: `cargo test -p evoswarm-memory --test e2_2_lineage_written_to_falkordb`.
//!
//! Validates:
//! - AC1: When a completed job is recorded, Task, Implementation, Evaluation, and TestCase
//!   nodes and their edges match the AD-3 graph schema.
//! - AC2: Lookups by `spec_hash`, `repo_fingerprint`, and `toolchain_fingerprint` are indexed.
//! - AC3: When FalkorDB is unavailable, the write operation succeeds by queueing the payload
//!   to an on-disk retry spool, which can be replayed when the database recovers.

use evoswarm_memory::falkordb::{
    EdgeRelation, EvaluationRecord, ImplementationRecord, JobRecord, LineageWriter,
    MockFalkorClient, TaskRecord, TestCaseRecord, WriteOutcome,
};
use tempfile::tempdir;

fn sample_job() -> JobRecord {
    let task = TaskRecord {
        id: "job-001".to_string(),
        spec_hash: "sha256_spec_abc".to_string(),
        repo_fingerprint: "sha256_repo_def".to_string(),
        toolchain_fingerprint: "sha256_toolchain_123".to_string(),
    };

    let test_cases = vec![
        TestCaseRecord {
            name: "test_fast_math".to_string(),
            origin: "trusted".to_string(),
            passed: true,
        },
        TestCaseRecord {
            name: "test_edge_case".to_string(),
            origin: "adversary".to_string(),
            passed: true,
        },
    ];

    let evaluations = vec![EvaluationRecord {
        id: "eval-cand-02".to_string(),
        candidate_id: "cand-02".to_string(),
        score: 0.95,
        passed_gates: true,
        wall_time_ms: 120,
        tests: test_cases,
    }];

    let implementations = vec![
        ImplementationRecord {
            id: "cand-01".to_string(),
            generation: 0,
            patch_blob_sha256: "sha256_blob_parent".to_string(),
            model_role: "mutator".to_string(),
            parents: vec![],
        },
        ImplementationRecord {
            id: "cand-02".to_string(),
            generation: 1,
            patch_blob_sha256: "sha256_blob_child".to_string(),
            model_role: "synthesiser".to_string(),
            parents: vec![
                EdgeRelation {
                    parent_id: "cand-01".to_string(),
                    relation: "MUTATED_FROM".to_string(),
                    traits: "".to_string(),
                },
            ],
        },
    ];

    JobRecord {
        task,
        implementations,
        evaluations,
    }
}

#[tokio::test]
async fn test_graph_schema_conformance() {
    let client = MockFalkorClient::new();
    let temp_spool = tempdir().expect("tempdir");
    let writer = LineageWriter::new(client.clone(), temp_spool.path());

    let job = sample_job();
    let outcome = writer.record_job(&job).await.expect("record job");
    assert_eq!(outcome, WriteOutcome::Committed);

    let cypher = client.executed_statements();
    assert!(!cypher.is_empty(), "Cypher statements must be executed");

    // AC1: Task node and properties
    assert!(
        cypher.iter().any(|q| q.contains("Task")
            && q.contains("spec_hash")
            && q.contains("repo_fingerprint")
            && q.contains("toolchain_fingerprint")),
        "Task node must match schema"
    );

    // AC1: Implementation node
    assert!(
        cypher.iter().any(|q| q.contains("Implementation")
            && q.contains("patch_blob_sha256")
            && q.contains("model_role")),
        "Implementation node must match schema"
    );

    // AC1: Evaluation node
    assert!(
        cypher.iter().any(|q| q.contains("Evaluation") && q.contains("score")),
        "Evaluation node must match schema"
    );

    // AC1: TestCase node
    assert!(
        cypher.iter().any(|q| q.contains("TestCase") && q.contains("origin")),
        "TestCase node must match schema"
    );

    // AC1: Edge relationships
    assert!(
        cypher.iter().any(|q| q.contains("HAS_IMPLEMENTATION")),
        "Relationship HAS_IMPLEMENTATION must exist"
    );
    assert!(
        cypher.iter().any(|q| q.contains("EVALUATED_BY")),
        "Relationship EVALUATED_BY must exist"
    );
    assert!(
        cypher.iter().any(|q| q.contains("TESTED_WITH")),
        "Relationship TESTED_WITH must exist"
    );
    assert!(
        cypher.iter().any(|q| q.contains("MUTATED_FROM")),
        "Relationship MUTATED_FROM must exist"
    );
}

#[tokio::test]
async fn test_index_usage() {
    let client = MockFalkorClient::new();
    let temp_spool = tempdir().expect("tempdir");
    let writer = LineageWriter::new(client.clone(), temp_spool.path());

    // Initialize schema indexes
    writer.ensure_indexes().await.expect("ensure indexes");

    let cypher = client.executed_statements();

    // AC2: Mandatory index creations on lookup keys
    assert!(
        cypher.iter().any(|q| q.contains("INDEX") && q.contains("Task") && q.contains("spec_hash")),
        "Index on Task(spec_hash) must be created"
    );
    assert!(
        cypher.iter().any(|q| q.contains("INDEX") && q.contains("Task") && q.contains("repo_fingerprint")),
        "Index on Task(repo_fingerprint) must be created"
    );
    assert!(
        cypher.iter().any(|q| q.contains("INDEX") && q.contains("Task") && q.contains("toolchain_fingerprint")),
        "Index on Task(toolchain_fingerprint) must be created"
    );

    // Lookups construct indexed query
    let query_spec = writer.query_by_spec_hash("sha256_spec_abc");
    assert!(query_spec.contains("MATCH (t:Task {spec_hash: 'sha256_spec_abc'})"));
}

#[tokio::test]
async fn test_db_downtime_retry_queue() {
    let client = MockFalkorClient::new_failing();
    let temp_spool = tempdir().expect("tempdir");
    let writer = LineageWriter::new(client.clone(), temp_spool.path());

    let job = sample_job();

    // AC3: If DB is unavailable, job still succeeds and write is spooled
    let outcome = writer.record_job(&job).await.expect("record job must not return error");
    assert_eq!(outcome, WriteOutcome::Spooled);

    // Spool files must be present on disk
    assert_eq!(writer.spool_count().expect("spool count"), 1);

    // Now restore the DB and replay the spool
    let online_client = MockFalkorClient::new();
    let writer_online = LineageWriter::new(online_client.clone(), temp_spool.path());

    let replayed = writer_online.replay_spool().await.expect("replay spool");
    assert_eq!(replayed, 1);
    assert_eq!(writer_online.spool_count().expect("spool count"), 0);

    let cypher = online_client.executed_statements();
    assert!(
        cypher.iter().any(|q| q.contains("Task") && q.contains("job-001")),
        "Spooled job was replayed to DB"
    );
}
