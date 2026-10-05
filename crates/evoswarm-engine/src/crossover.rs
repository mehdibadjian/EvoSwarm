//! Crossover of two parents (e1-5, AD-5/AD-6).
//!
//! Two candidates that pass *different* subsets of the trusted suite are recombined by the
//! synthesiser role: their complementary strengths can be merged into one child. Parent pairs are
//! ranked by the Hamming distance of their pass vectors, so the most complementary pair is
//! scheduled first. A candidate without recorded provenance is never eligible as a parent, and
//! when no pair is disjoint the generation degrades to mutation-only (spec §1).
//!
//! SEAM: verified against a stub `ModelClient` and an in-memory `LineageSink`. The synthesiser
//! returns a patch plus a traits summary; if the output cannot be parsed the child is discarded
//! rather than silently accepted, and no lineage edge is recorded.

use evoswarm_core::{Candidate, PassVector};
use evoswarm_models::lineage::{Edge, LineageSink};
use evoswarm_models::{call_hash, Role};
use thiserror::Error;

use crate::dispatch::{CompletionRequest, ModelClient};

/// Delimiter separating the synthesiser's patch from its merged-traits summary.
const TRAITS_MARKER: &str = "=== TRAITS ===";
/// Marks the boundary between the cacheable static prefix and the per-call dynamic suffix (e1-4).
const DYNAMIC_MARKER: &str = "\n=== DYNAMIC ===\n";

#[derive(Debug, Error)]
pub enum CrossoverError {
    #[error("synthesiser model failed: {0}")]
    ModelFailure(String),
    #[error("synthesiser output could not be parsed into patch + traits: {0}")]
    ParseFailure(String),
}

/// A candidate paired with the trusted tests it passed. `pass_vector` is `None` when the
/// candidate has no recorded provenance, which disqualifies it as a crossover parent.
#[derive(Debug, Clone, PartialEq)]
pub struct MeasuredCandidate {
    pub candidate: Candidate,
    pub pass_vector: Option<PassVector>,
}

/// The job-invariant context for every crossover call (mirrors the e1-4 mutation context): a
/// static prefix assembled once per job and reused verbatim, with per-call data in the suffix.
#[derive(Debug, Clone)]
pub struct CrossoverContext {
    pub static_prefix: Vec<u8>,
    pub model_id: String,
    pub job_id: String,
}

/// The collaborators crossover needs: the model seam and the lineage sink.
pub struct CrossoverDeps<'a> {
    pub client: &'a dyn ModelClient,
    pub lineage: &'a dyn LineageSink,
}

/// Ranks candidate pairs by descending Hamming distance of their pass vectors, returning
/// `(index_a, index_b, distance)` triples. Candidates without provenance are skipped entirely,
/// and pairs with distance 0 (identical pass sets) are dropped: recombining them would not merge
/// complementary strengths, so the caller degrades to a mutation-only generation.
pub fn rank_pairs(pop: &[MeasuredCandidate]) -> Vec<(usize, usize, usize)> {
    let mut pairs = Vec::new();
    for (i, a) in pop.iter().enumerate() {
        let Some(pa) = &a.pass_vector else { continue };
        for (j, b) in pop.iter().enumerate().skip(i + 1) {
            let Some(pb) = &b.pass_vector else { continue };
            let distance = pa.hamming_distance(pb);
            // Only strictly complementary pairs are worth recombining.
            if distance > 0 {
                pairs.push((i, j, distance));
            }
        }
    }
    // Descending distance; ties broken by index for determinism.
    pairs.sort_by(|x, y| y.2.cmp(&x.2).then(x.0.cmp(&y.0)).then(x.1.cmp(&y.1)));
    pairs
}

/// Recombines two parents via the synthesiser role. On a parseable response, records a
/// `MERGED_FROM` edge from the child to each parent carrying the synthesiser's traits summary,
/// and returns the child at one generation past the older parent.
pub async fn crossover(
    a: &Candidate,
    b: &Candidate,
    ctx: &CrossoverContext,
    deps: &CrossoverDeps<'_>,
) -> Result<Candidate, CrossoverError> {
    // Dynamic suffix: both parent diffs. Static prefix stays byte-identical across calls (e1-4).
    let mut suffix = String::new();
    suffix.push_str(DYNAMIC_MARKER);
    suffix.push_str("## Parent A diff\n");
    suffix.push_str(&String::from_utf8_lossy(&a.patch));
    suffix.push_str("\n\n## Parent B diff\n");
    suffix.push_str(&String::from_utf8_lossy(&b.patch));
    suffix.push_str(&format!("\n\njob={} parents={},{}", ctx.job_id, a.id, b.id));

    let mut prompt = ctx.static_prefix.clone();
    prompt.extend_from_slice(suffix.as_bytes());
    let prompt_hash = call_hash(Role::Synthesiser, &ctx.model_id, &prompt);

    let req = CompletionRequest {
        role: Role::Synthesiser,
        model_id: ctx.model_id.clone(),
        prompt,
    };
    let resp = deps
        .client
        .complete(&req)
        .await
        .map_err(CrossoverError::ModelFailure)?;

    let (patch_text, traits) = parse_response(&resp.text)?;
    let patch = patch_text.into_bytes();
    let child_gen = a.generation.max(b.generation) + 1;
    let child = Candidate {
        id: format!("merge-{}-{}", a.id, b.id),
        diff_hash: crate::dedup::diff_hash(&patch),
        patch,
        generation: child_gen,
        parent_ids: vec![a.id.clone(), b.id.clone()],
        model_id: ctx.model_id.clone(),
        prompt_hash: crate::dedup::hex(&prompt_hash),
    };

    // Both MERGED_FROM edges carry the traits payload; recorded only after the child is accepted.
    for parent in [&a.id, &b.id] {
        deps.lineage
            .record(Edge {
                from: child.id.clone(),
                to: parent.clone(),
                relation: "MERGED_FROM".into(),
                traits: traits.clone(),
            })
            .await;
    }

    Ok(child)
}

/// Splits the synthesiser response into `(patch, traits)` at the traits delimiter. A response
/// without the delimiter is a parse failure — the child is discarded rather than accepted with
/// empty provenance, since traits metadata is what the lineage edges carry.
fn parse_response(text: &str) -> Result<(String, String), CrossoverError> {
    let Some(idx) = text.find(TRAITS_MARKER) else {
        return Err(CrossoverError::ParseFailure(format!(
            "no '{TRAITS_MARKER}' delimiter in synthesiser output"
        )));
    };
    let patch = text[..idx].trim().to_string();
    let traits = text[idx + TRAITS_MARKER.len()..].trim().to_string();
    if traits.is_empty() {
        return Err(CrossoverError::ParseFailure(
            "synthesiser reported empty traits summary".into(),
        ));
    }
    Ok((patch, traits))
}
