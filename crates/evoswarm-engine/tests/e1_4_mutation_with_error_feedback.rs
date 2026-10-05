//! e1-4: Mutation with error feedback.
//!
//! SEAM story: mutation is verified against a stub `ModelClient` and an in-memory `LineageSink`.
//! The provider-independent invariants under test are feedback clamping/truncation, the
//! `MUTATED_FROM` lineage edge, and byte-identical static prompt prefixes across calls.

use std::sync::Mutex;

use async_trait::async_trait;
use evoswarm_core::{Candidate, ExecutionResult, RunStatus};
use evoswarm_engine::dispatch::{CompletionRequest, CompletionResponse, ModelClient};
use evoswarm_engine::feedback::{build, render_clamped, TRUNCATION_MARKER};
use evoswarm_engine::mutation::{mutate, MutationContext, MutationDeps, MutationError};
use evoswarm_models::lineage::{Edge, RecordingLineageSink};
use evoswarm_models::prompt::estimate_tokens;

/// A stub mutator returning a fixed child patch.
struct StubMutator;

#[async_trait]
impl ModelClient for StubMutator {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
        Ok(CompletionResponse {
            text: "child-patch".into(),
            tokens_in: 10,
            tokens_out: 20,
            from_cache: false,
        })
    }
}

fn failing_result(stderr: &str, stdout: &str) -> ExecutionResult {
    ExecutionResult {
        exit_code: 1,
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        wall_time_ms: 5,
        peak_memory_bytes: 0,
        status: RunStatus::Failed,
    }
}

fn parent() -> Candidate {
    Candidate {
        id: "parent-1".into(),
        patch: b"parent-source".to_vec(),
        diff_hash: [1u8; 32],
        generation: 1,
        parent_ids: vec!["gen0-baseline".into()],
        model_id: "mutator-model".into(),
        prompt_hash: "ph".into(),
    }
}

/// AC1: the assembled prompt includes the first failing assertion and the compiler/error text,
/// trimmed to the 4k-token budget.
#[test]
fn test_feedback_prompt_formatting() {
    let result = failing_result(
        "error[E0308]: mismatched types\n  --> src/lib.rs:10:5",
        "test test_hot_path ... FAILED\nAssertionError: expected 4 got 5\n    at src/lib.rs:42",
    );
    let fb = build(&result);
    let rendered = render_clamped(&fb, 4000);

    // Compiler errors and the failing assertion both appear, compiler section first.
    let compiler_at = rendered.find("mismatched types").expect("compiler error present");
    let assertion_at = rendered.find("AssertionError").expect("assertion present");
    assert!(
        compiler_at < assertion_at,
        "compiler section must precede the assertion section"
    );
    assert!(rendered.contains("test_hot_path"), "failing test named");
    // The whole render respects the 4k token budget.
    assert!(
        estimate_tokens(rendered.as_bytes()) <= 4000,
        "feedback must be clamped to 4000 tokens"
    );
}

/// AC2: a dump far larger than the clamp is hard-clamped to 4,000 tokens and carries the
/// truncation marker, never a silent cut.
#[test]
fn test_feedback_truncation_marker() {
    // ~50k tokens of noise in both streams.
    let huge = "x".repeat(50_000 * 4);
    let result = failing_result(&huge, &huge);
    let fb = build(&result);
    let rendered = render_clamped(&fb, 4000);

    let tokens = estimate_tokens(rendered.as_bytes());
    assert!(
        tokens <= 4000,
        "total must be hard-clamped to 4000 tokens, got {tokens}"
    );
    assert!(
        rendered.contains(TRUNCATION_MARKER),
        "a cut section must carry the truncation marker"
    );
    assert!(
        rendered.trim_end().ends_with(TRUNCATION_MARKER),
        "the clamped render must end with the truncation marker"
    );
}

/// AC3: the accepted child is linked to its parent with a MUTATED_FROM edge.
#[tokio::test]
async fn test_mutated_from_lineage_edge() {
    let client = StubMutator;
    let lineage = RecordingLineageSink::default();
    let deps = MutationDeps {
        client: &client,
        lineage: &lineage,
    };
    let ctx = MutationContext {
        static_prefix: b"SYSTEM: improve the code\nREPO: evoswarm\n".to_vec(),
        model_id: "mutator-model".into(),
        job_id: "job-1".into(),
    };
    let feedback = build(&failing_result("boom", "test_x FAILED"));

    let child = mutate(&parent(), &feedback, &ctx, &deps)
        .await
        .expect("mutate ok");

    // The child records its parent and the lineage sink captured a MUTATED_FROM edge.
    assert_eq!(child.parent_ids, vec!["parent-1".to_string()]);
    assert_eq!(child.generation, 2, "child is one generation past the parent");
    let edges = lineage.edges();
    let mutated: Vec<&Edge> = edges
        .iter()
        .filter(|e| e.relation == "MUTATED_FROM")
        .collect();
    assert_eq!(mutated.len(), 1, "exactly one MUTATED_FROM edge");
    assert_eq!(mutated[0].from, child.id);
    assert_eq!(mutated[0].to, "parent-1");
}

/// AC4: the static prompt prefix is byte-identical across sequential mutations in one job — no
/// per-call data (timestamps, candidate ids) leaks into the cached prefix.
#[tokio::test]
async fn test_cache_prefix_byte_identity() {
    // A client that records the static prefix it sees on each call, derived by stripping the
    // dynamic suffix. We assert the prefix bytes repeat verbatim across calls.
    #[derive(Default)]
    struct PrefixRecorder {
        prefixes: Mutex<Vec<Vec<u8>>>,
    }
    #[async_trait]
    impl ModelClient for PrefixRecorder {
        async fn complete(&self, req: &CompletionRequest) -> Result<CompletionResponse, String> {
            // The prefix is everything up to the dynamic marker the mutation assembler inserts.
            let marker = b"\n=== DYNAMIC ===\n";
            let prefix = match find_subslice(&req.prompt, marker) {
                Some(i) => req.prompt[..i].to_vec(),
                None => req.prompt.clone(),
            };
            self.prefixes.lock().unwrap().push(prefix);
            Ok(CompletionResponse {
                text: "child".into(),
                tokens_in: 1,
                tokens_out: 1,
                from_cache: false,
            })
        }
    }

    fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
        hay.windows(needle.len()).position(|w| w == needle)
    }

    let client = PrefixRecorder::default();
    let lineage = RecordingLineageSink::default();
    let deps = MutationDeps {
        client: &client,
        lineage: &lineage,
    };
    let ctx = MutationContext {
        static_prefix: b"SYSTEM: stable instructions\nREPO: overview\nAPI: contracts\n".to_vec(),
        model_id: "mutator-model".into(),
        job_id: "job-1".into(),
    };

    // Two sequential mutations of DIFFERENT parents with DIFFERENT feedback.
    let mut p1 = parent();
    p1.id = "parent-A".into();
    let mut p2 = parent();
    p2.id = "parent-B".into();
    let f1 = build(&failing_result("err one", "test_a FAILED"));
    let f2 = build(&failing_result("err two", "test_b FAILED"));

    mutate(&p1, &f1, &ctx, &deps).await.expect("mutate 1");
    mutate(&p2, &f2, &ctx, &deps).await.expect("mutate 2");

    let prefixes = client.prefixes.lock().unwrap();
    assert_eq!(prefixes.len(), 2, "two mutation calls dispatched");
    assert_eq!(
        prefixes[0], prefixes[1],
        "static prefix must be byte-identical across calls in one job"
    );
    assert_eq!(prefixes[0], ctx.static_prefix, "prefix is the assembled static prefix");
}

/// A mutation whose model call fails surfaces MutationError::ModelFailure rather than a child.
#[tokio::test]
async fn test_model_failure_surfaces_error() {
    struct FailingClient;
    #[async_trait]
    impl ModelClient for FailingClient {
        async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
            Err("provider 503".into())
        }
    }
    let client = FailingClient;
    let lineage = RecordingLineageSink::default();
    let deps = MutationDeps {
        client: &client,
        lineage: &lineage,
    };
    let ctx = MutationContext {
        static_prefix: b"prefix".to_vec(),
        model_id: "m".into(),
        job_id: "job-1".into(),
    };
    let feedback = build(&failing_result("boom", "test_x FAILED"));

    let err = mutate(&parent(), &feedback, &ctx, &deps)
        .await
        .expect_err("must fail");
    assert!(
        matches!(err, MutationError::ModelFailure(_)),
        "expected ModelFailure, got {err:?}"
    );
    // No lineage edge is recorded for a failed mutation.
    assert!(lineage.edges().is_empty(), "failed mutation records no edge");
}
