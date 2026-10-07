//! Generation-zero seeding (e1-3, AD-5/AD-6).
//!
//! This is a SEAM story: the population is built against the `ModelClient` and `MemorySeeder`
//! traits, never a live provider or graph store. The invariants that matter are checked here and
//! are provider-independent:
//!
//! - the population is always exactly `N` — a short Gen 0 would bias every later selection
//!   statistic, so an exhausted dedup retry budget is a hard error, not a truncation;
//! - candidate 0 is the baseline (the current code), pinned so the incumbent is always present;
//! - every draft records generation 0, empty parents, a prompt hash and the configured model id;
//! - drafts are deduplicated by patch SHA-256 and replaced at a jittered temperature on collision.
//!
//! Drafts are drawn sequentially with the slot index as the seed, so a replayed job reproduces
//! Gen 0 byte for byte (spec §3, required by e1-12 idempotency).

use std::collections::HashSet;

use evoswarm_core::Candidate;
use evoswarm_models::{call_hash, Role};
use thiserror::Error;

use crate::dedup::{diff_hash, hex};
use crate::dispatch::{CompletionRequest, ModelClient};

/// Replacement attempts allowed for a colliding draft before seeding gives up (spec §1.4).
const MAX_DEDUP_ATTEMPTS: u32 = 3;
/// Temperature jitter added per replacement attempt (spec §1.4).
const TEMPERATURE_JITTER: f64 = 0.1;
/// At most this many past winners are injected from memory (spec §1.2).
const MAX_MEMORY_SEEDS: usize = 2;

#[derive(Debug, Error)]
pub enum SeedingError {
    #[error("dedup retry budget exhausted after {attempts} replacement attempts")]
    DedupExhausted { attempts: u32 },
    #[error("mutator model failed: {0}")]
    ModelFailure(String),
}
pub use evoswarm_core::{EmptyMemorySeeder, MemorySeeder, SeedConfig};

/// The collaborators seeding needs. Borrowed so a caller owns the client, the memory store and
/// the baseline patch, keeping this module free of ownership and lifetime tangles.
pub struct SeedingDeps<'a> {
    pub client: &'a dyn ModelClient,
    pub memory: &'a dyn MemorySeeder,
    /// The current code, captured at job start; pinned as candidate 0.
    pub baseline_patch: Vec<u8>,
}

/// Builds generation 0: the baseline at index 0, up to two memory-injected past winners, then
/// mutator drafts until the population is exactly `N`. Returns `DedupExhausted` rather than a
/// short population when a draft cannot be made unique within the retry budget.
pub async fn seed_generation_zero(
    cfg: &SeedConfig,
    deps: &SeedingDeps<'_>,
) -> Result<Vec<Candidate>, SeedingError> {
    let n = cfg.population_size;
    let mut pop: Vec<Candidate> = Vec::with_capacity(n);
    let mut seen: HashSet<[u8; 32]> = HashSet::new();

    // Candidate 0 is the baseline, always present and never a model draft.
    let baseline_hash = diff_hash(&deps.baseline_patch);
    seen.insert(baseline_hash);
    pop.push(Candidate {
        id: "gen0-baseline".into(),
        patch: deps.baseline_patch.clone(),
        diff_hash: baseline_hash,
        generation: 0,
        parent_ids: Vec::new(),
        model_id: cfg.model_id.clone(),
        prompt_hash: hex(&baseline_hash),
    });

    // Memory-injected past winners (up to MAX_MEMORY_SEEDS). A collision here is skipped, not
    // fatal: memory is an optimisation, not a population requirement.
    for candidate in deps.memory.seed(cfg).await.into_iter().take(MAX_MEMORY_SEEDS) {
        let h = diff_hash(&candidate.patch);
        if seen.insert(h) {
            pop.push(Candidate { diff_hash: h, ..candidate });
        }
        if pop.len() >= n {
            break;
        }
    }

    // Fill the remaining slots with mutator drafts, deduplicating each against the population.
    let mut slot = 0usize;
    while pop.len() < n {
        let candidate = draw_unique_draft(cfg, deps, &mut seen, slot).await?;
        pop.push(candidate);
        slot += 1;
    }

    // The population invariant is the final, unconditional check (spec §2).
    debug_assert_eq!(pop.len(), n);
    Ok(pop)
}

/// Draws one mutator draft for `slot`, retrying at a jittered temperature on a hash collision
/// until the retry budget is exhausted. The slot index seeds the prompt, so ordering is
/// deterministic for a given configuration.
async fn draw_unique_draft(
    cfg: &SeedConfig,
    deps: &SeedingDeps<'_>,
    seen: &mut HashSet<[u8; 32]>,
    slot: usize,
) -> Result<Candidate, SeedingError> {
    // attempt 0 is the first draw; attempts 1..=MAX are the replacement retries.
    for attempt in 0..=MAX_DEDUP_ATTEMPTS {
        let temperature = cfg.base_temperature + (attempt as f64) * TEMPERATURE_JITTER;
        let prompt = format!(
            "seed-generation-zero slot={slot} attempt={attempt} temperature={temperature}"
        );
        let prompt_bytes = prompt.as_bytes();
        let prompt_hash = call_hash(Role::Mutator, &cfg.model_id, prompt_bytes);

        let req = CompletionRequest {
            role: Role::Mutator,
            model_id: cfg.model_id.clone(),
            prompt: prompt_bytes.to_vec(),
        };
        let resp = deps
            .client
            .complete(&req)
            .await
            .map_err(SeedingError::ModelFailure)?;
        let patch = resp.text.into_bytes();
        let h = diff_hash(&patch);

        if seen.insert(h) {
            return Ok(Candidate {
                id: format!("gen0-slot-{slot}"),
                patch,
                diff_hash: h,
                generation: 0,
                parent_ids: Vec::new(),
                model_id: cfg.model_id.clone(),
                prompt_hash: hex(&prompt_hash),
            });
        }
        // Collision: loop to the next attempt (jittered temperature). Fall through to the error
        // below once the replacement budget is spent.
    }

    Err(SeedingError::DedupExhausted {
        attempts: MAX_DEDUP_ATTEMPTS,
    })
}
