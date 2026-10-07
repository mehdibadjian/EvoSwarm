use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Candidate;

/// Seeding configuration: how big the population is and which model draws the drafts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeedConfig {
    pub population_size: usize,
    pub model_id: String,
    pub base_temperature: f64,
}

/// The graph-backed memory seam. Injects past winning patches into Gen 0.
#[async_trait]
pub trait MemorySeeder: Send + Sync {
    async fn seed(&self, cfg: &SeedConfig) -> Vec<Candidate>;
}

/// The no-op memory seeder: injects nothing, so Gen 0 is the baseline plus mutator drafts.
pub struct EmptyMemorySeeder;

#[async_trait]
impl MemorySeeder for EmptyMemorySeeder {
    async fn seed(&self, _cfg: &SeedConfig) -> Vec<Candidate> {
        Vec::new()
    }
}
