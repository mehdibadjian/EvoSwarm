//! e1-3: Seed the first generation.
//!
//! SEAM story: generation-zero seeding is verified against a stub `ModelClient` (never a live
//! provider). The population invariant, per-draft metadata, diff-hash deduplication and the
//! exhausted-retry hard error are all deterministic against the scripted fake.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use evoswarm_core::Candidate;
use evoswarm_engine::dispatch::{CompletionRequest, CompletionResponse, ModelClient};
use evoswarm_engine::seeding::{
    seed_generation_zero, EmptyMemorySeeder, SeedConfig, SeedingDeps, SeedingError,
};

/// A stub mutator returning a distinct patch per call (`draft-0`, `draft-1`, ...), so a normal
/// seeding run yields an all-unique population without any collision.
struct CountingMutator {
    counter: AtomicUsize,
}

#[async_trait]
impl ModelClient for CountingMutator {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(CompletionResponse {
            text: format!("draft-{n}"),
            tokens_in: 4,
            tokens_out: 4,
            from_cache: false,
        })
    }
}

/// A stub mutator returning a scripted sequence of patches, repeating the last entry once the
/// script is exhausted. Used to force collisions on demand.
struct ScriptedMutator {
    script: Vec<String>,
    idx: Mutex<usize>,
}

impl ScriptedMutator {
    fn new(script: Vec<String>) -> Self {
        Self {
            script,
            idx: Mutex::new(0),
        }
    }
}

#[async_trait]
impl ModelClient for ScriptedMutator {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
        let mut i = self.idx.lock().unwrap();
        let at = (*i).min(self.script.len() - 1);
        *i += 1;
        Ok(CompletionResponse {
            text: self.script[at].clone(),
            tokens_in: 4,
            tokens_out: 4,
            from_cache: false,
        })
    }
}

fn cfg(n: usize) -> SeedConfig {
    SeedConfig {
        population_size: n,
        model_id: "mutator-model".into(),
        base_temperature: 0.7,
    }
}

const BASELINE: &[u8] = b"BASELINE_SOURCE";

/// AC1: generation 0 holds the current code (pinned at index 0) plus N-1 fresh drafts, for a
/// population of exactly N.
#[tokio::test]
async fn test_gen0_population_quota() {
    let n = 4;
    let client = CountingMutator {
        counter: AtomicUsize::new(0),
    };
    let memory = EmptyMemorySeeder;
    let deps = SeedingDeps {
        client: &client,
        memory: &memory,
        baseline_patch: BASELINE.to_vec(),
    };

    let pop = seed_generation_zero(&cfg(n), &deps).await.expect("seed");

    assert_eq!(pop.len(), n, "population must be exactly N");
    // Index 0 is the baseline, not a model draft.
    assert_eq!(pop[0].patch, BASELINE.to_vec(), "candidate 0 is the baseline");
    // The remaining N-1 are distinct model drafts.
    let drafts: Vec<&Candidate> = pop.iter().skip(1).collect();
    assert_eq!(drafts.len(), n - 1);
    for (i, d) in drafts.iter().enumerate() {
        assert_eq!(d.patch, format!("draft-{i}").as_bytes(), "draft {i} content");
    }
}

/// AC2: every candidate records generation 0, an empty parent list, a non-empty prompt hash and
/// the configured model id.
#[tokio::test]
async fn test_candidate_metadata_recording() {
    let n = 3;
    let client = CountingMutator {
        counter: AtomicUsize::new(0),
    };
    let memory = EmptyMemorySeeder;
    let deps = SeedingDeps {
        client: &client,
        memory: &memory,
        baseline_patch: BASELINE.to_vec(),
    };

    let pop = seed_generation_zero(&cfg(n), &deps).await.expect("seed");

    for c in &pop {
        assert_eq!(c.generation, 0, "gen 0 for every candidate");
        assert!(c.parent_ids.is_empty(), "no parents in gen 0");
        assert!(!c.prompt_hash.is_empty(), "prompt hash must be recorded");
        assert_eq!(c.model_id, "mutator-model", "configured model id");
    }
    // The baseline's diff hash is the SHA-256 of its patch, distinct across candidates.
    let mut hashes: Vec<_> = pop.iter().map(|c| c.diff_hash).collect();
    hashes.sort();
    hashes.dedup();
    assert_eq!(hashes.len(), pop.len(), "diff hashes must be unique");
}

/// AC3: when two drafts produce identical content hashes, the duplicate is dropped and replaced
/// with a unique draft (re-invoked at jittered temperature).
#[tokio::test]
async fn test_diff_hash_deduplication() {
    // slot0 -> "A" (unique), slot1 -> "A" (collides with slot0), retry slot1 -> "B" (unique).
    let client = ScriptedMutator::new(vec!["A".into(), "A".into(), "B".into()]);
    let memory = EmptyMemorySeeder;
    let deps = SeedingDeps {
        client: &client,
        memory: &memory,
        baseline_patch: BASELINE.to_vec(),
    };

    let pop = seed_generation_zero(&cfg(3), &deps).await.expect("seed");

    assert_eq!(pop.len(), 3, "population stays at N after replacement");
    let patches: Vec<String> = pop
        .iter()
        .map(|c| String::from_utf8_lossy(&c.patch).to_string())
        .collect();
    // The colliding "A" from slot1 was replaced by "B"; slot0 keeps its "A".
    assert!(patches.contains(&"A".to_string()), "slot0 draft A retained");
    assert!(patches.contains(&"B".to_string()), "replacement draft B accepted");
    // All diff hashes unique — no duplicate survived.
    let mut hashes: Vec<_> = pop.iter().map(|c| c.diff_hash).collect();
    let before = hashes.len();
    hashes.sort();
    hashes.dedup();
    assert_eq!(hashes.len(), before, "no duplicate diff hashes remain");
}

/// AC4: when every retry also collides, seeding fails loudly with DedupExhausted rather than
/// returning a short population.
#[tokio::test]
async fn test_dedup_retry_budget_exhaustion() {
    // The mutator always returns the baseline content, so slot 0's draft collides with the
    // pinned baseline hash on every attempt and exhausts the replacement budget.
    let client = ScriptedMutator::new(vec![String::from_utf8(BASELINE.to_vec()).unwrap()]);
    let memory = EmptyMemorySeeder;
    let deps = SeedingDeps {
        client: &client,
        memory: &memory,
        baseline_patch: BASELINE.to_vec(),
    };

    let err = seed_generation_zero(&cfg(2), &deps)
        .await
        .expect_err("must fail loudly on exhausted dedup budget");
    assert!(
        matches!(err, SeedingError::DedupExhausted { .. }),
        "expected DedupExhausted, got {err:?}"
    );
}
