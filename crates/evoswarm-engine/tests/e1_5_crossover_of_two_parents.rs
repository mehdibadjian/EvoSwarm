//! e1-5: Crossover of two parents.
//!
//! SEAM story: parent-pair ranking and the crossover dispatch are verified against a stub
//! `ModelClient`, an in-memory `LineageSink` and the `CrossoverBudget` counter — no live
//! provider. The provider-independent invariants are Hamming-distance ranking, mutation-only
//! fallback when no pair is disjoint, dual MERGED_FROM lineage, and the 25% crossover call cap.

use async_trait::async_trait;
use evoswarm_core::{Candidate, PassVector};
use evoswarm_engine::budget::{CrossoverBudget, CROSSOVER_CAP_RATIO};
use evoswarm_engine::crossover::{
    crossover, rank_pairs, CrossoverContext, CrossoverDeps, CrossoverError, MeasuredCandidate,
};
use evoswarm_engine::dispatch::{CompletionRequest, CompletionResponse, ModelClient};
use evoswarm_models::lineage::{Edge, RecordingLineageSink};
use evoswarm_models::Role;

/// A stub synthesiser returning a fixed patch and traits summary.
struct StubSynthesiser;

#[async_trait]
impl ModelClient for StubSynthesiser {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
        Ok(CompletionResponse {
            text: "merged-patch\n=== TRAITS ===\nkeeps A's parser, B's cache".into(),
            tokens_in: 20,
            tokens_out: 30,
            from_cache: false,
        })
    }
}

fn candidate(id: &str, gen: u32) -> Candidate {
    Candidate {
        id: id.into(),
        patch: format!("{id}-source").into_bytes(),
        diff_hash: [0u8; 32],
        generation: gen,
        parent_ids: Vec::new(),
        model_id: "synth".into(),
        prompt_hash: "ph".into(),
    }
}

fn measured(id: &str, gen: u32, passed: &[&str]) -> MeasuredCandidate {
    MeasuredCandidate {
        candidate: candidate(id, gen),
        pass_vector: Some(PassVector::new(passed.iter().copied())),
    }
}

/// AC1: pairs are ranked by descending Hamming distance and the top disjoint pair is chosen.
#[test]
fn test_disjoint_test_pair_selection() {
    // A passes {t1,t2}, B passes {t3,t4} (distance 4); C passes {t1,t2} (distance 0 from A).
    let pop = vec![
        measured("A", 1, &["t1", "t2"]),
        measured("B", 1, &["t3", "t4"]),
        measured("C", 1, &["t1", "t2"]),
    ];
    let ranked = rank_pairs(&pop);

    // The top pair is (A,B) with distance 4.
    let (a, b, dist) = ranked[0];
    assert_eq!(dist, 4, "top pair must have the greatest Hamming distance");
    assert!(
        (a == 0 && b == 1) || (a == 1 && b == 0),
        "top pair is A and B, got indices ({a},{b})"
    );
    // Ranking is descending by distance.
    for w in ranked.windows(2) {
        assert!(w[0].2 >= w[1].2, "pairs sorted by descending distance");
    }
    // A candidate lacking provenance is never eligible as a parent.
    let mut with_unknown = pop.clone();
    with_unknown[1].pass_vector = None;
    let ranked2 = rank_pairs(&with_unknown);
    assert!(
        ranked2.iter().all(|(a, b, _)| *a != 1 && *b != 1),
        "a candidate without provenance is excluded from every pair"
    );
}

/// AC2: when every candidate passes exactly the same tests, no pair has D > 0 and no crossover is
/// scheduled — the generation falls back to mutation.
#[test]
fn test_no_disjoint_pair_falls_back_to_mutation() {
    let pop = vec![
        measured("A", 1, &["t1", "t2"]),
        measured("B", 1, &["t1", "t2"]),
        measured("C", 1, &["t1", "t2"]),
    ];
    let ranked = rank_pairs(&pop);
    assert!(
        ranked.is_empty(),
        "identical pass vectors yield no crossover pairs (mutation-only generation), got {ranked:?}"
    );
}

/// AC3: the crossover child links to both parents with the traits reported by the synthesiser.
#[tokio::test]
async fn test_merged_from_dual_lineage() {
    let client = StubSynthesiser;
    let lineage = RecordingLineageSink::default();
    let deps = CrossoverDeps {
        client: &client,
        lineage: &lineage,
    };
    let ctx = CrossoverContext {
        static_prefix: b"SYSTEM: merge two candidates\n".to_vec(),
        model_id: "synth".into(),
        job_id: "job-1".into(),
    };
    let a = candidate("A", 2);
    let b = candidate("B", 2);

    let child = crossover(&a, &b, &ctx, &deps).await.expect("crossover ok");

    // The child records both parents and is one generation past them.
    let mut parents = child.parent_ids.clone();
    parents.sort();
    assert_eq!(parents, vec!["A".to_string(), "B".to_string()]);
    assert_eq!(child.generation, 3, "child is one generation past the parents");

    // Two MERGED_FROM edges, one per parent, each carrying the synthesiser's traits.
    let edges = lineage.edges();
    let merged: Vec<&Edge> = edges.iter().filter(|e| e.relation == "MERGED_FROM").collect();
    assert_eq!(merged.len(), 2, "one MERGED_FROM edge per parent");
    for e in &merged {
        assert_eq!(e.from, child.id);
        assert!(
            e.traits.contains("parser") && e.traits.contains("cache"),
            "each edge carries the synthesiser traits, got {:?}",
            e.traits
        );
    }
    let targets: Vec<&str> = merged.iter().map(|e| e.to.as_str()).collect();
    assert!(targets.contains(&"A") && targets.contains(&"B"), "edges point at both parents");
}

/// AC4: crossover calls constitute at most 25% of total model calls, enforced pre-dispatch.
#[tokio::test]
async fn test_crossover_call_cap_budget() {
    let mut budget = CrossoverBudget::new();
    // Simulate a generation: attempt a crossover before every dispatch, recording whatever the
    // cap permits and falling back to a mutation call when it does not.
    let mut attempted_crossovers = 0usize;
    for _ in 0..40 {
        attempted_crossovers += 1;
        if budget.pre_dispatch_crossover().is_ok() {
            budget.record_call(Role::Synthesiser); // a permitted crossover
        } else {
            budget.record_call(Role::Mutator); // degrade to mutation
        }
        // Each round also issues one mutation regardless, mirroring a real generation.
        budget.record_call(Role::Mutator);
    }

    let total = budget.total_calls();
    let cross = budget.crossover_calls();
    assert!(total > 0);
    let ratio = cross as f64 / total as f64;
    assert!(
        ratio <= CROSSOVER_CAP_RATIO + 1e-9,
        "crossover calls must stay at or below 25% of total, got {cross}/{total} = {ratio}"
    );
    assert!(
        cross > 0,
        "the cap must still permit some crossovers, not starve them entirely"
    );
    assert_eq!(attempted_crossovers, 40);
}

/// A crossover whose synthesiser response cannot be parsed (no traits delimiter) is discarded
/// rather than silently accepted, and no lineage edge is recorded.
#[tokio::test]
async fn test_unparseable_synthesiser_output_discarded() {
    struct NoTraits;
    #[async_trait]
    impl ModelClient for NoTraits {
        async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
            Ok(CompletionResponse {
                text: "just a patch with no traits delimiter".into(),
                tokens_in: 1,
                tokens_out: 1,
                from_cache: false,
            })
        }
    }
    let client = NoTraits;
    let lineage = RecordingLineageSink::default();
    let deps = CrossoverDeps {
        client: &client,
        lineage: &lineage,
    };
    let ctx = CrossoverContext {
        static_prefix: b"prefix".to_vec(),
        model_id: "synth".into(),
        job_id: "job-1".into(),
    };
    let err = crossover(&candidate("A", 1), &candidate("B", 1), &ctx, &deps)
        .await
        .expect_err("must discard unparseable output");
    assert!(
        matches!(err, CrossoverError::ParseFailure(_)),
        "expected ParseFailure, got {err:?}"
    );
    assert!(
        lineage.edges().is_empty(),
        "a discarded child records no lineage edge"
    );
}
