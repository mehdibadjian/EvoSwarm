//! Mutation with error feedback (e1-4, AD-5/AD-6).
//!
//! A failed parent is re-drafted by the mutator role with its failure diagnostics appended.
//! The prompt is split into a *static prefix* (assembled once per job, reused verbatim — see
//! `MutationContext`) and a *dynamic suffix* (the parent diff and clamped feedback), so the
//! provider's prompt cache stays hot across every mutation call in the job. On acceptance the
//! child is linked to its parent with a `MUTATED_FROM` edge; mutation never orphans a candidate.
//!
//! SEAM: verified against a stub `ModelClient` and an in-memory `LineageSink`. The model call
//! goes through the same `CompletionRequest` shape the e1-12 idempotency cache keys on.

use evoswarm_core::Candidate;
use evoswarm_models::lineage::{Edge, LineageSink};
use evoswarm_models::{call_hash, Role};
use thiserror::Error;

use crate::dispatch::{CompletionRequest, ModelClient};
use crate::feedback::{render_clamped, FailureFeedback, TOTAL_TOKEN_BUDGET};

/// Marks the boundary between the cacheable static prefix and the per-call dynamic suffix.
/// A visible delimiter keeps the split unambiguous in the prompt bytes the provider caches.
const DYNAMIC_MARKER: &str = "\n=== DYNAMIC ===\n";

#[derive(Debug, Error)]
pub enum MutationError {
    #[error("mutator model failed: {0}")]
    ModelFailure(String),
}

/// The job-invariant context for every mutation call. `static_prefix` is assembled once per job
/// and reused verbatim: it must contain no timestamps, candidate ids or job ids, or the provider
/// cache is silently defeated (spec §2, AC4).
#[derive(Debug, Clone)]
pub struct MutationContext {
    /// Cacheable prefix: system instructions, repository overview, API contracts, test rules.
    pub static_prefix: Vec<u8>,
    pub model_id: String,
    pub job_id: String,
}

/// The collaborators mutation needs: the model seam and the lineage sink.
pub struct MutationDeps<'a> {
    pub client: &'a dyn ModelClient,
    pub lineage: &'a dyn LineageSink,
}

/// Drafts a child from a failed parent by dispatching the mutator role with the parent diff and
/// clamped failure diagnostics in the dynamic suffix. On success records a `MUTATED_FROM` edge
/// from the child to the parent and returns the child candidate at `parent.generation + 1`.
pub async fn mutate(
    parent: &Candidate,
    feedback: &FailureFeedback,
    ctx: &MutationContext,
    deps: &MutationDeps<'_>,
) -> Result<Candidate, MutationError> {
    // Dynamic suffix: the parent diff and the clamped diagnostics. All per-call data lives here,
    // never in the cached static prefix.
    let feedback_text = render_clamped(feedback, TOTAL_TOKEN_BUDGET);
    let mut suffix = String::new();
    suffix.push_str(DYNAMIC_MARKER);
    suffix.push_str("## Parent diff\n");
    suffix.push_str(&String::from_utf8_lossy(&parent.patch));
    suffix.push_str("\n\n");
    suffix.push_str(&feedback_text);
    suffix.push_str(&format!("\n\njob={} parent={}", ctx.job_id, parent.id));

    let mut prompt = ctx.static_prefix.clone();
    prompt.extend_from_slice(suffix.as_bytes());

    let prompt_hash = call_hash(Role::Mutator, &ctx.model_id, &prompt);

    let req = CompletionRequest {
        role: Role::Mutator,
        model_id: ctx.model_id.clone(),
        prompt,
    };
    let resp = deps
        .client
        .complete(&req)
        .await
        .map_err(MutationError::ModelFailure)?;

    let patch = resp.text.into_bytes();
    let child = Candidate {
        id: format!("{}-child-{}", parent.id, parent.generation + 1),
        diff_hash: crate::dedup::diff_hash(&patch),
        patch,
        generation: parent.generation + 1,
        parent_ids: vec![parent.id.clone()],
        model_id: ctx.model_id.clone(),
        prompt_hash: crate::dedup::hex(&prompt_hash),
    };

    // Provenance is recorded only after the child is accepted (spec §3).
    deps.lineage
        .record(Edge {
            from: child.id.clone(),
            to: parent.id.clone(),
            relation: "MUTATED_FROM".into(),
            traits: String::new(),
        })
        .await;

    Ok(child)
}
