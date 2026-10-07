//! e2-5: Seeding from similar winners acceptance tests (AD-5, AD-6).
//!
//! Acceptance criteria (story §4):
//! - AC1: Given a past task above the similarity threshold, when generation 0 is built,
//!   then up to 2 of its winners join as seeds.
//! - AC2: Given a seed, when evaluated, then it passes through the same gates as any candidate
//!   and is returned only if it wins.
//! - AC3: Given the benchmark/report, tokens per solved task are reported and trackable with/without seeding.

use evoswarm_engine::seeding::{seed_generation_zero, MemorySeeder, SeedConfig, SeedingDeps};
use evoswarm_memory::blob_store::BlobStore;
use evoswarm_memory::falkordb::{
    EvaluationRecord, ImplementationRecord, JobRecord, LineageWriter, MockFalkorClient, TaskRecord,
};
use evoswarm_memory::vector_seeder::{
    cosine_similarity, FalkorVectorSeeder, TaskEmbeddingRecord,
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tempfile::tempdir;

struct DummyModelClient {
    calls: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl evoswarm_engine::ModelClient for DummyModelClient {
    async fn complete(
        &self,
        _req: &evoswarm_engine::CompletionRequest,
    ) -> Result<evoswarm_engine::CompletionResponse, String> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(evoswarm_engine::CompletionResponse {
            text: format!("patch-draft-{n}"),
            tokens_in: 50,
            tokens_out: 50,
            from_cache: false,
        })
    }
}

#[test]
fn test_cosine_similarity_math() {
    let a = vec![1.0, 0.0, 0.0];
    let b = vec![1.0, 0.0, 0.0];
    assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

    let c = vec![0.0, 1.0, 0.0];
    assert!(cosine_similarity(&a, &c).abs() < 1e-6);

    let d = vec![1.0, 1.0, 0.0];
    let sim = cosine_similarity(&a, &d);
    assert!((sim - (1.0 / 2.0_f32.sqrt())).abs() < 1e-6);
}

#[tokio::test]
async fn test_vector_similarity_lookup() {
    let dir = tempdir().expect("tempdir");
    let spool_dir = dir.path().join("spool");
    let blob_dir = dir.path().join("blobs");
    let blob_store = Arc::new(BlobStore::new(blob_dir).unwrap());

    let patch1 = b"def past_winner_1(): return 42";
    let hash1 = blob_store.write(patch1).unwrap();

    let patch2 = b"def past_winner_2(): return 99";
    let hash2 = blob_store.write(patch2).unwrap();

    let client = MockFalkorClient::new_failing();
    let writer = LineageWriter::new(client, &spool_dir);

    // Job 1 (very similar: [0.95, 0.05])
    let job1 = JobRecord {
        task: TaskRecord {
            id: "task-similar-1".into(),
            spec_hash: "spec_1".into(),
            repo_fingerprint: "repo_1".into(),
            toolchain_fingerprint: "tool_1".into(),
        },
        implementations: vec![ImplementationRecord {
            id: "cand-sim-1".into(),
            generation: 1,
            patch_blob_sha256: hash1,
            model_role: "mutator".into(),
            parents: vec![],
        }],
        evaluations: vec![EvaluationRecord {
            id: "eval-sim-1".into(),
            candidate_id: "cand-sim-1".into(),
            score: 0.95,
            passed_gates: true,
            wall_time_ms: 50,
            tests: vec![],
        }],
    };
    writer.record_job(&job1).await.unwrap();

    // Job 2 (different topic: [0.1, 0.9])
    let job2 = JobRecord {
        task: TaskRecord {
            id: "task-dissimilar".into(),
            spec_hash: "spec_2".into(),
            repo_fingerprint: "repo_2".into(),
            toolchain_fingerprint: "tool_2".into(),
        },
        implementations: vec![ImplementationRecord {
            id: "cand-dissim".into(),
            generation: 1,
            patch_blob_sha256: hash2,
            model_role: "mutator".into(),
            parents: vec![],
        }],
        evaluations: vec![EvaluationRecord {
            id: "eval-dissim".into(),
            candidate_id: "cand-dissim".into(),
            score: 0.9,
            passed_gates: true,
            wall_time_ms: 50,
            tests: vec![],
        }],
    };
    writer.record_job(&job2).await.unwrap();

    let embeddings = vec![
        TaskEmbeddingRecord {
            task_id: "task-similar-1".into(),
            embedding: vec![0.95, 0.05],
        },
        TaskEmbeddingRecord {
            task_id: "task-dissimilar".into(),
            embedding: vec![0.1, 0.9],
        },
    ];

    let query_vector = vec![0.99, 0.01]; // Target task embedding
    let threshold = 0.85;

    let seeder = FalkorVectorSeeder::new(
        spool_dir.clone(),
        blob_store.clone(),
        embeddings,
        query_vector,
        threshold,
    );

    let cfg = SeedConfig {
        population_size: 5,
        model_id: "mutator-pro".into(),
        base_temperature: 0.7,
    };

    let seeds = seeder.seed(&cfg).await;
    assert_eq!(seeds.len(), 1, "only 1 winner above threshold 0.85");
    assert_eq!(seeds[0].patch, patch1, "contains patch of task-similar-1");
}

#[tokio::test]
async fn test_seed_candidate_gating() {
    let dir = tempdir().expect("tempdir");
    let spool_dir = dir.path().join("spool");
    let blob_dir = dir.path().join("blobs");
    let blob_store = Arc::new(BlobStore::new(blob_dir).unwrap());

    let patch = b"def seed_candidate(): pass";
    let hash = blob_store.write(patch).unwrap();

    let client = MockFalkorClient::new_failing();
    let writer = LineageWriter::new(client, &spool_dir);

    let job = JobRecord {
        task: TaskRecord {
            id: "task-prev".into(),
            spec_hash: "spec_p".into(),
            repo_fingerprint: "repo_p".into(),
            toolchain_fingerprint: "tool_p".into(),
        },
        implementations: vec![ImplementationRecord {
            id: "cand-prev".into(),
            generation: 1,
            patch_blob_sha256: hash,
            model_role: "mutator".into(),
            parents: vec![],
        }],
        evaluations: vec![EvaluationRecord {
            id: "eval-prev".into(),
            candidate_id: "cand-prev".into(),
            score: 0.99,
            passed_gates: true,
            wall_time_ms: 30,
            tests: vec![],
        }],
    };
    writer.record_job(&job).await.unwrap();

    let embeddings = vec![TaskEmbeddingRecord {
        task_id: "task-prev".into(),
        embedding: vec![1.0, 0.0],
    }];
    let query_vector = vec![1.0, 0.0];

    let seeder = FalkorVectorSeeder::new(
        spool_dir,
        blob_store,
        embeddings,
        query_vector,
        0.80,
    );

    let calls = Arc::new(AtomicUsize::new(0));
    let model_client = DummyModelClient {
        calls: Arc::clone(&calls),
    };

    let deps = SeedingDeps {
        client: &model_client,
        memory: &seeder,
        baseline_patch: b"baseline patch".to_vec(),
    };

    let cfg = SeedConfig {
        population_size: 4,
        model_id: "mutator-pro".into(),
        base_temperature: 0.7,
    };

    let pop = seed_generation_zero(&cfg, &deps).await.expect("seed gen 0");

    assert_eq!(pop.len(), 4, "exact population size 4");
    assert_eq!(pop[0].patch, b"baseline patch", "index 0 is baseline");
    // Index 1 must be the injected memory winner
    assert_eq!(pop[1].patch, patch, "index 1 is memory seed");
    assert_eq!(pop[1].generation, 0, "seed candidate is set to generation 0");
    // The remaining slots (2, 3) are drawn from mutator
    assert_eq!(calls.load(Ordering::SeqCst), 2, "2 calls to fill remaining slots");
}
