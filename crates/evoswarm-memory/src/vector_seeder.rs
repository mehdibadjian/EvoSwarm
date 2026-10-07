//! FalkorDB vector-based memory seeder (e2-5, AD-5).
//!
//! Uses embeddings of task descriptions to identify similar past winning tasks and
//! inject up to 2 winning patches into the Generation 0 candidate population.

use async_trait::async_trait;
use evoswarm_core::{Candidate, MemorySeeder, SeedConfig};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::blob_store::BlobStore;
use crate::falkordb::JobRecord;

/// A stored task description embedding record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEmbeddingRecord {
    pub task_id: String,
    pub embedding: Vec<f32>,
}

/// Computes the cosine similarity between two non-empty vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

/// A `MemorySeeder` querying past winners by vector similarity against a query embedding.
pub struct FalkorVectorSeeder {
    spool_dir: PathBuf,
    blob_store: Arc<BlobStore>,
    embeddings: Vec<TaskEmbeddingRecord>,
    query_vector: Vec<f32>,
    similarity_threshold: f32,
}

impl FalkorVectorSeeder {
    pub fn new(
        spool_dir: PathBuf,
        blob_store: Arc<BlobStore>,
        embeddings: Vec<TaskEmbeddingRecord>,
        query_vector: Vec<f32>,
        similarity_threshold: f32,
    ) -> Self {
        Self {
            spool_dir,
            blob_store,
            embeddings,
            query_vector,
            similarity_threshold,
        }
    }

    /// Finds candidate winners from tasks whose embeddings exceed the similarity threshold.
    fn find_similar_winning_candidates(&self) -> Vec<Candidate> {
        let mut scored_tasks: Vec<(f32, &str)> = self
            .embeddings
            .iter()
            .map(|e| {
                let sim = cosine_similarity(&self.query_vector, &e.embedding);
                (sim, e.task_id.as_str())
            })
            .filter(|(sim, _)| *sim >= self.similarity_threshold)
            .collect();

        // Sort descending by similarity
        scored_tasks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut seeds = Vec::new();
        for (_sim, task_id) in scored_tasks {
            if let Some(record) = self.load_job_record(task_id) {
                if let Some(winner) = self.extract_winner(&record) {
                    seeds.push(winner);
                    if seeds.len() >= 2 {
                        break;
                    }
                }
            }
        }

        seeds
    }

    fn load_job_record(&self, task_id: &str) -> Option<JobRecord> {
        if !self.spool_dir.exists() {
            return None;
        }

        // Try direct spool filename or scan
        let spool_path = self.spool_dir.join(format!("spool_{task_id}.json"));
        if spool_path.exists() {
            if let Ok(content) = fs::read_to_string(&spool_path) {
                if let Ok(record) = serde_json::from_str::<JobRecord>(&content) {
                    return Some(record);
                }
            }
        }

        let direct_path = self.spool_dir.join(format!("{task_id}.json"));
        if direct_path.exists() {
            if let Ok(content) = fs::read_to_string(&direct_path) {
                if let Ok(record) = serde_json::from_str::<JobRecord>(&content) {
                    return Some(record);
                }
            }
        }

        // Scan spool dir
        if let Ok(entries) = fs::read_dir(&self.spool_dir) {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(record) = serde_json::from_str::<JobRecord>(&content) {
                        if record.task.id == task_id {
                            return Some(record);
                        }
                    }
                }
            }
        }

        None
    }

    fn extract_winner(&self, record: &JobRecord) -> Option<Candidate> {
        let mut best_imp = None;
        let mut highest_score = -1.0;

        for ev in &record.evaluations {
            if ev.passed_gates && ev.score > highest_score {
                if let Some(imp) = record.implementations.iter().find(|i| i.id == ev.candidate_id) {
                    highest_score = ev.score;
                    best_imp = Some(imp);
                }
            }
        }

        let imp = best_imp?;
        let patch_path = self.blob_store.blob_path(&imp.patch_blob_sha256);
        let patch_bytes = fs::read(&patch_path).ok()?;

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&patch_bytes);
        let diff_hash: [u8; 32] = hasher.finalize().into();

        Some(Candidate {
            id: format!("seed-{}", imp.id),
            patch: patch_bytes,
            diff_hash,
            generation: 0,
            parent_ids: vec![imp.id.clone()],
            model_id: "memory-seeder".into(),
            prompt_hash: "similarity-seed".into(),
        })
    }
}

#[async_trait]
impl MemorySeeder for FalkorVectorSeeder {
    async fn seed(&self, _cfg: &SeedConfig) -> Vec<Candidate> {
        self.find_similar_winning_candidates()
    }
}
