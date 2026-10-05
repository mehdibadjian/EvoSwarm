//! Adversary test generation and filtering (e1-9, AD-5).
//!
//! The adversary role generates up to K test functions probing the best candidate for
//! weaknesses. Two filters keep only trustworthy tests: a *compilation filter* discards any test
//! that does not build in the sandbox, and a *suspect filter* excludes a test that fails on the
//! baseline and every Gen 0 candidate (such a test measures nothing about the candidate).
//! Survivors feed the score's adversary term A.
//!
//! Gate isolation is structural, not a runtime check: adversary tests carry
//! `TestOrigin::Adversary` and `fitness::gates::evaluate` only ever compares against the trusted
//! suite, so a generated test can never veto user code.
//!
//! SEAM: verified against a stub `ModelClient` and a scripted `SandboxBackend` — no live provider
//! or real compilation.

use std::path::Path;

use evoswarm_core::{AdversaryStatus, AdversaryTest, Candidate, TestOrigin};
use evoswarm_models::Role;
use evoswarm_sandbox::SandboxBackend;

use crate::dispatch::{CompletionRequest, ModelClient};

/// Delimiter between successive test functions in the adversary response.
const TEST_MARKER: &str = "=== TEST ===";

/// The collaborators adversary generation needs: the model seam and the sandbox it compiles in.
pub struct AdversaryDeps<'a> {
    pub client: &'a dyn ModelClient,
    pub backend: &'a dyn SandboxBackend,
}

/// The baseline evidence the suspect filter runs against: whether the test fails on the baseline
/// and on every Gen 0 candidate. A test failing everywhere measures nothing about any candidate.
#[derive(Debug, Clone)]
pub struct SuspectEvidence {
    pub fails_on_baseline: bool,
    /// True when the test fails on every Gen 0 candidate.
    pub fails_on_all_candidates: bool,
}

/// Generates up to `k` adversary tests for `best` from `spec`. Tests lacking a parseable body are
/// dropped; the count never exceeds `k`. Every returned test carries `TestOrigin::Adversary`.
pub async fn generate(
    spec: &str,
    best: &Candidate,
    k: usize,
    deps: &AdversaryDeps<'_>,
) -> Vec<AdversaryTest> {
    let prompt = format!(
        "adversary-role k={k}\n## Spec\n{spec}\n## Best candidate\n{}",
        String::from_utf8_lossy(&best.patch)
    );

    let req = CompletionRequest {
        role: Role::Adversary,
        model_id: "adversary".into(),
        prompt: prompt.into_bytes(),
    };
    let Ok(resp) = deps.client.complete(&req).await else {
        return Vec::new();
    };

    parse_tests(&resp.text)
        .into_iter()
        .take(k)
        .map(|(name, body)| AdversaryTest {
            name,
            body,
            origin: TestOrigin::Adversary,
            status: AdversaryStatus::Valid,
        })
        .collect()
}

/// Splits the adversary response into `(name, body)` pairs at `TEST_MARKER`. A segment with no
/// name line is discarded rather than accepted as an anonymous test.
fn parse_tests(text: &str) -> Vec<(String, String)> {
    text.split(TEST_MARKER)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|segment| {
            let mut lines = segment.lines();
            let name = lines.next()?.trim().to_string();
            if name.is_empty() {
                return None;
            }
            let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
            Some((name, body))
        })
        .collect()
}

/// Applies the compilation filter: a test whose body fails to compile in the sandbox is marked
/// `Discarded` before any execution. The diagnostic is captured on the returned test rather than
/// surfaced to scoring. Tests that compile keep their current status.
pub async fn compile_filter(
    tests: Vec<AdversaryTest>,
    deps: &AdversaryDeps<'_>,
    workdir: &Path,
) -> Vec<AdversaryTest> {
    let mut out = Vec::with_capacity(tests.len());
    for mut test in tests {
        let compile = deps
            .backend
            .run(workdir, &format!("compile {}", test.name))
            .await;
        let compiled = matches!(&compile, Ok(r) if r.exit_code == 0);
        if !compiled {
            test.status = AdversaryStatus::Discarded;
        }
        out.push(test);
    }
    out
}

/// Applies the suspect filter: a compiled test that fails on the baseline and on every Gen 0
/// candidate is tagged `Suspect` and excluded from term A. Already-discarded tests are untouched.
pub fn suspect_filter(tests: Vec<AdversaryTest>, evidence: &[SuspectEvidence]) -> Vec<AdversaryTest> {
    tests
        .into_iter()
        .zip(evidence.iter())
        .map(|(mut test, ev)| {
            if test.status == AdversaryStatus::Valid
                && ev.fails_on_baseline
                && ev.fails_on_all_candidates
            {
                test.status = AdversaryStatus::Suspect;
            }
            test
        })
        .collect()
}

/// Convenience: the tests eligible to contribute to scoring term A (only `Valid` survivors).
pub fn scoreable(tests: &[AdversaryTest]) -> Vec<&AdversaryTest> {
    tests.iter().filter(|t| t.is_scoreable()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_splits_and_names_tests() {
        let text = "=== TEST ===\ntest_overflow\nassert x > 0\n=== TEST ===\ntest_empty\nassert y";
        let parsed = parse_tests(text);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "test_overflow");
        assert_eq!(parsed[1].0, "test_empty");
    }

    #[test]
    fn parse_discards_empty_segment() {
        // A marker with no following content yields an empty segment, which is dropped rather
        // than accepted as an anonymous test.
        let parsed = parse_tests("=== TEST ===\n\ntest_a\nassert x\n=== TEST ===");
        assert_eq!(parsed.len(), 1, "only the non-empty segment is kept");
        assert_eq!(parsed[0].0, "test_a");
    }

    #[test]
    fn suspect_filter_tags_universal_failure() {
        let tests = vec![AdversaryTest {
            name: "t".into(),
            body: "b".into(),
            origin: TestOrigin::Adversary,
            status: AdversaryStatus::Valid,
        }];
        let ev = vec![SuspectEvidence {
            fails_on_baseline: true,
            fails_on_all_candidates: true,
        }];
        let out = suspect_filter(tests, &ev);
        assert_eq!(out[0].status, AdversaryStatus::Suspect);
        assert!(!out[0].is_scoreable());
    }

    #[test]
    fn suspect_filter_leaves_passing_test_valid() {
        let tests = vec![AdversaryTest {
            name: "t".into(),
            body: "b".into(),
            origin: TestOrigin::Adversary,
            status: AdversaryStatus::Valid,
        }];
        let ev = vec![SuspectEvidence {
            fails_on_baseline: false,
            fails_on_all_candidates: false,
        }];
        let out = suspect_filter(tests, &ev);
        assert_eq!(out[0].status, AdversaryStatus::Valid);
        assert!(out[0].is_scoreable());
    }
}
