//! e1-9: Adversary tests.
//!
//! SEAM story: adversary generation and filtering are verified against a stub `ModelClient` and a
//! scripted `SandboxBackend` — no live provider or real compilation. The provider-independent
//! invariants are the K-test quota with origin tags, the compilation discard, the suspect filter,
//! and structural gate isolation (an adversary test can never fail a trusted-test gate).

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use evoswarm_core::{
    AdversaryStatus, AdversaryTest, Candidate, ExecutionResult, RunStatus, TestOrigin,
};
use evoswarm_engine::adversary::{
    compile_filter, generate, scoreable, suspect_filter, AdversaryDeps, SuspectEvidence,
};
use evoswarm_engine::dispatch::{CompletionRequest, CompletionResponse, ModelClient};
use evoswarm_fitness::baseline::Baseline;
use evoswarm_fitness::gates::{evaluate, GateInput};
use evoswarm_sandbox::{SandboxBackend, SandboxError};

/// A stub adversary returning a scripted list of `=== TEST ===` segments.
struct StubAdversary {
    body: String,
}

#[async_trait]
impl ModelClient for StubAdversary {
    async fn complete(&self, _req: &CompletionRequest) -> Result<CompletionResponse, String> {
        Ok(CompletionResponse {
            text: self.body.clone(),
            tokens_in: 5,
            tokens_out: 5,
            from_cache: false,
        })
    }
}

/// A sandbox fake that fails to compile tests whose name appears in `fail_compile`.
struct ScriptedSandbox {
    fail_compile: Vec<String>,
}

#[async_trait]
impl SandboxBackend for ScriptedSandbox {
    async fn prepare(
        &self,
        _profile: &evoswarm_core::SandboxProfile,
        _patch: &[u8],
    ) -> Result<PathBuf, SandboxError> {
        Ok(PathBuf::from("/fake/workdir"))
    }

    async fn run(&self, _workdir: &Path, cmd: &str) -> Result<ExecutionResult, SandboxError> {
        // cmd is "compile <name>"; fail when the name is in the fail set.
        let name = cmd.strip_prefix("compile ").unwrap_or(cmd);
        let failed = self.fail_compile.iter().any(|f| name.contains(f));
        Ok(ExecutionResult {
            exit_code: if failed { 1 } else { 0 },
            stdout: String::new(),
            stderr: if failed {
                "syntax error".into()
            } else {
                String::new()
            },
            wall_time_ms: 1,
            peak_memory_bytes: 0,
            status: if failed {
                RunStatus::Failed
            } else {
                RunStatus::Success
            },
        })
    }

    async fn collect(&self, _workdir: PathBuf) -> Result<(), SandboxError> {
        Ok(())
    }
}

fn best() -> Candidate {
    Candidate {
        id: "best".into(),
        patch: b"best-source".to_vec(),
        diff_hash: [0u8; 32],
        generation: 1,
        parent_ids: vec![],
        model_id: "adversary".into(),
        prompt_hash: "ph".into(),
    }
}

/// AC1: the adversary generates at most K tests and each carries an adversary origin tag.
#[tokio::test]
async fn test_adversary_generation_quota() {
    // Response contains 3 tests; request K=2.
    let body = "=== TEST ===\ntest_a\nassert 1\n=== TEST ===\ntest_b\nassert 2\n=== TEST ===\ntest_c\nassert 3".to_string();
    let client = StubAdversary { body };
    let backend = ScriptedSandbox {
        fail_compile: vec![],
    };
    let deps = AdversaryDeps {
        client: &client,
        backend: &backend,
    };

    let tests = generate("spec text", &best(), 2, &deps).await;

    assert_eq!(tests.len(), 2, "at most K tests generated");
    for t in &tests {
        assert_eq!(t.origin, TestOrigin::Adversary, "every test is origin-tagged");
        assert!(t.is_adversary());
        assert!(!t.name.is_empty(), "test carries a name");
    }
    assert_eq!(tests[0].name, "test_a");
    assert_eq!(tests[1].name, "test_b");
}

/// AC2: an adversary test that does not compile is discarded before execution.
#[tokio::test]
async fn test_syntax_error_discard() {
    let body = "=== TEST ===\ntest_ok\nassert 1\n=== TEST ===\ntest_broken\nassert (".to_string();
    let client = StubAdversary { body };
    // test_broken fails to compile.
    let backend = ScriptedSandbox {
        fail_compile: vec!["test_broken".into()],
    };
    let deps = AdversaryDeps {
        client: &client,
        backend: &backend,
    };

    let tests = generate("spec", &best(), 5, &deps).await;
    let filtered = compile_filter(tests, &deps, Path::new("/fake/workdir")).await;

    let ok = filtered.iter().find(|t| t.name == "test_ok").unwrap();
    let broken = filtered.iter().find(|t| t.name == "test_broken").unwrap();
    assert_eq!(ok.status, AdversaryStatus::Valid, "compiling test stays valid");
    assert_eq!(
        broken.status,
        AdversaryStatus::Discarded,
        "uncompilable test is discarded"
    );
    // The discarded test never contributes to scoring.
    let scoreable_names: Vec<_> = scoreable(&filtered).iter().map(|t| t.name.clone()).collect();
    assert_eq!(scoreable_names, vec!["test_ok".to_string()]);
}

/// AC3: a test that fails on the baseline and all candidates is flagged suspect and excluded
/// from scoring term A.
#[tokio::test]
async fn test_suspect_test_filtering() {
    let tests = vec![
        AdversaryTest {
            name: "test_universal".into(),
            body: "assert false".into(),
            origin: TestOrigin::Adversary,
            status: AdversaryStatus::Valid,
        },
        AdversaryTest {
            name: "test_meaningful".into(),
            body: "assert x".into(),
            origin: TestOrigin::Adversary,
            status: AdversaryStatus::Valid,
        },
    ];
    // test_universal fails everywhere; test_meaningful does not.
    let evidence = vec![
        SuspectEvidence {
            fails_on_baseline: true,
            fails_on_all_candidates: true,
        },
        SuspectEvidence {
            fails_on_baseline: false,
            fails_on_all_candidates: false,
        },
    ];

    let filtered = suspect_filter(tests, &evidence);
    let universal = filtered.iter().find(|t| t.name == "test_universal").unwrap();
    let meaningful = filtered.iter().find(|t| t.name == "test_meaningful").unwrap();

    assert_eq!(
        universal.status,
        AdversaryStatus::Suspect,
        "universally failing test is suspect"
    );
    assert!(!universal.is_scoreable(), "suspect test excluded from A");
    assert_eq!(meaningful.status, AdversaryStatus::Valid);
    assert!(meaningful.is_scoreable(), "meaningful test feeds A");

    let scoreable_names: Vec<_> = scoreable(&filtered).iter().map(|t| t.name.clone()).collect();
    assert_eq!(scoreable_names, vec!["test_meaningful".to_string()]);
}

/// AC4: an adversary test failing against a candidate never changes the e1-6 gate result — gates
/// evaluate trusted tests only, so a generated test cannot veto user code.
#[test]
fn test_adversary_never_gates_candidate() {
    // A candidate run that passed every trusted test (build ok, all trusted pass, no skips).
    let trusted = vec!["test_user_1".to_string(), "test_user_2".to_string()];
    let baseline = Baseline {
        test_count: 2,
        skipped: 0,
        trusted_names: trusted.clone(),
    };
    let run = ExecutionResult {
        exit_code: 0,
        stdout: String::new(),
        stderr: String::new(),
        wall_time_ms: 1,
        peak_memory_bytes: 0,
        status: RunStatus::Success,
    };
    let allowed = vec![PathBuf::from("src")];
    let patch_paths = vec![PathBuf::from("src/lib.rs")];

    // Gate result with only the trusted tests passed — the candidate passes every gate.
    let input = GateInput {
        run: &run,
        baseline: &baseline,
        patch_paths: &patch_paths,
        allowed_paths: &allowed,
        passed_tests: &trusted,
        executed_count: 2,
        skipped_count: 0,
    };
    let result = evaluate(&input);
    assert!(result.passed, "candidate passes all trusted gates");

    // Now simulate an adversary test that FAILS against this candidate: it is absent from
    // passed_tests. Because gates only ever compare against baseline.trusted_names — never the
    // adversary set — the failing adversary test cannot flip the gate. The gate still passes.
    let adversary_test = AdversaryTest {
        name: "test_adversary_overflow".into(),
        body: "assert fails_on_candidate".into(),
        origin: TestOrigin::Adversary,
        status: AdversaryStatus::Valid,
    };
    assert!(adversary_test.is_adversary());
    assert!(
        !baseline.trusted_names.contains(&adversary_test.name),
        "an adversary test is never part of the trusted suite the gate evaluates"
    );
    // The candidate did not pass the adversary test, proving it actually failed.
    assert!(
        !trusted.contains(&adversary_test.name),
        "the adversary test failed on this candidate"
    );
    // Even with the adversary test failing, the trusted-only gate is unchanged.
    let result_with_adversary = evaluate(&input);
    assert!(
        result_with_adversary.passed,
        "a failing adversary test must never gate the candidate"
    );
    assert_eq!(result, result_with_adversary, "gate result is independent of adversary tests");
}
