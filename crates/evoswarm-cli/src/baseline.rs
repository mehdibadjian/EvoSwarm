//! Baseline validation (e1-1 spec §3.2–3.3): run the submitted test command three times
//! in a throwaway sandbox and reject the job when the suite is untrustworthy or there is
//! nothing to improve.
//!
//! The seam is `SandboxBackend` itself, so an integration test injects a scripted fake and
//! never needs bwrap: the three-run diff and the zero-diff rule are pure logic over the
//! parsed outcomes.

use std::collections::BTreeMap;

use evoswarm_core::{JobObjective, SandboxProfile};
use evoswarm_sandbox::{parse_junit_outcomes, SandboxBackend, TestStatus};

use crate::exit_codes::ExitCode;

/// A rejection of the job during baseline validation, carrying the CLI exit code and a
/// human-readable diagnostic (the resolved path, the child's stderr, or the flaky names).
#[derive(Debug, Clone, PartialEq)]
pub struct BaselineRejection {
    pub code: ExitCode,
    pub message: String,
}

/// The number of consecutive baseline runs whose outcomes must agree. Three is the spec's
/// flakiness detector: a suite that is genuinely deterministic yields identical maps.
const BASELINE_RUNS: usize = 3;

/// Runs the test command `BASELINE_RUNS` times through `backend` and applies the e1-1
/// baseline rules in order:
///
/// 1. A run that cannot execute or produces no parseable test report → `BaselineCommandFailed`.
/// 2. Outcomes that differ across the runs → `FlakyTestDetected` naming the varying tests.
/// 3. Every test already passing under a non-`perf` objective → `NothingToImprove`.
///
/// Otherwise the baseline is trustworthy and improvable, so the job proceeds.
pub async fn validate_baseline<B: SandboxBackend + ?Sized>(
    backend: &B,
    profile: &SandboxProfile,
    test_command: &str,
    objective: JobObjective,
) -> Result<(), BaselineRejection> {
    let workdir = backend
        .prepare(profile, b"")
        .await
        .map_err(|e| BaselineRejection {
            code: ExitCode::BaselineCommandFailed,
            message: format!("sandbox prepare failed: {e}"),
        })?;

    let mut maps: Vec<BTreeMap<String, TestStatus>> = Vec::with_capacity(BASELINE_RUNS);
    let mut result = Ok(());

    for _ in 0..BASELINE_RUNS {
        match backend.run(&workdir, test_command).await {
            Ok(exec) => {
                let outcomes = parse_junit_outcomes(&exec.stdout);
                match outcomes {
                    Ok(parsed) if !parsed.is_empty() => {
                        let map: BTreeMap<String, TestStatus> = parsed
                            .into_iter()
                            .map(|o| (o.name, o.status))
                            .collect();
                        maps.push(map);
                    }
                    // No parseable report: the command could not run the suite. Echo the
                    // child's exit code and stderr so the caller sees the real failure.
                    _ => {
                        result = Err(BaselineRejection {
                            code: ExitCode::BaselineCommandFailed,
                            message: format!(
                                "baseline command produced no test report (exit {}): {}",
                                exec.exit_code,
                                trim(&exec.stderr)
                            ),
                        });
                        break;
                    }
                }
            }
            Err(e) => {
                result = Err(BaselineRejection {
                    code: ExitCode::BaselineCommandFailed,
                    message: format!("baseline command could not execute: {e}"),
                });
                break;
            }
        }
    }

    let _ = backend.collect(workdir).await;
    result?;

    // Flakiness: any test whose status is not identical across all runs is varying.
    let varying = varying_tests(&maps);
    if !varying.is_empty() {
        return Err(BaselineRejection {
            code: ExitCode::FlakyTestDetected,
            message: format!("flaky baseline tests: {}", varying.join(", ")),
        });
    }

    // Zero-diff prevention: a fully-green baseline under a correctness objective has
    // nothing for evolution to improve, so require an explicit performance objective.
    let first = &maps[0];
    let all_pass = first.values().all(|s| *s == TestStatus::Passed);
    if all_pass && objective != JobObjective::Performance {
        return Err(BaselineRejection {
            code: ExitCode::NothingToImprove,
            message: "every baseline test already passes; re-run with --objective perf".to_string(),
        });
    }

    Ok(())
}

/// Returns the sorted names whose status is not constant across every run's map.
fn varying_tests(maps: &[BTreeMap<String, TestStatus>]) -> Vec<String> {
    let Some(first) = maps.first() else {
        return Vec::new();
    };
    let mut varying: Vec<String> = first
        .keys()
        .filter(|name| !maps.iter().all(|m| m.get(*name) == first.get(*name)))
        .cloned()
        .collect();
    // A test present in some runs but absent in others is also varying.
    for m in maps.iter().skip(1) {
        for name in m.keys() {
            if !first.contains_key(name) && !varying.contains(name) {
                varying.push(name.clone());
            }
        }
    }
    varying.sort();
    varying
}

fn trim(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        "<no stderr>".to_string()
    } else {
        t.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, TestStatus)]) -> BTreeMap<String, TestStatus> {
        pairs
            .iter()
            .map(|(n, s)| (n.to_string(), *s))
            .collect()
    }

    #[test]
    fn varying_detects_status_change() {
        let a = map(&[("t1", TestStatus::Passed), ("t2", TestStatus::Failed)]);
        let b = map(&[("t1", TestStatus::Passed), ("t2", TestStatus::Passed)]);
        let v = varying_tests(&[a.clone(), b, a]);
        assert_eq!(v, vec!["t2".to_string()]);
    }

    #[test]
    fn varying_detects_presence_change() {
        let a = map(&[("t1", TestStatus::Passed)]);
        let b = map(&[("t1", TestStatus::Passed), ("t2", TestStatus::Passed)]);
        let v = varying_tests(&[a.clone(), b, a]);
        assert!(v.contains(&"t2".to_string()));
    }

    #[test]
    fn stable_maps_have_no_variation() {
        let a = map(&[("t1", TestStatus::Passed), ("t2", TestStatus::Failed)]);
        assert!(varying_tests(&[a.clone(), a.clone(), a]).is_empty());
    }
}
