use serde::{Deserialize, Serialize};

/// One member of the evolving population (e1-3 onward). Persisted as JSON by the
/// resume-after-crash ledger (e1-12), so every field is `Serialize`/`Deserialize`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    /// Raw patch bytes applied inside the sandbox.
    pub patch: Vec<u8>,
    /// SHA-256 of `patch`, used for generation-0 deduplication (e1-3).
    pub diff_hash: [u8; 32],
    pub generation: u32,
    /// Empty for generation 0; otherwise the parent id(s) this candidate descends from.
    pub parent_ids: Vec<String>,
    pub model_id: String,
    pub prompt_hash: String,
}
