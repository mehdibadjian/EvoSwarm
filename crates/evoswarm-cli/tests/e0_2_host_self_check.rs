//! e0-2 host self-check — acceptance criteria (story §4).
//!
//! Red-phase tests authored before the production `host_check` module exists. The story
//! suggests `tests/test_host_check.rs`; the verification gate is
//! `cargo test --test e0_2_host_self_check`.
//!
//! SEAM: the *decision* logic (all-pass → ready/exit 0; any-fail → degraded/exit 1 with
//! remediation; queued jobs stay queued) is pure and fully certified here against injected
//! [`CheckResult`]s. The *live probes* — `unshare(CLONE_NEWUSER)`, reading
//! `/sys/fs/cgroup` controllers for `memory`/`pids`, and systemd user-linger status — are a
//! seam: unprivileged namespaces work under bwrap here, but cgroup v2 delegation and a systemd
//! user session bus do not (roadmap §6), so the probes are not run for real in CI.

use evoswarm_cli::host_check::{
    check_user_namespaces, decide, CheckId, CheckResult, HostStatus, HOST_EXIT_DEGRADED,
    HOST_EXIT_READY,
};

fn ok(id: CheckId) -> CheckResult {
    CheckResult {
        id,
        passed: true,
        remediation: String::new(),
    }
}

fn fail(id: CheckId, fix: &str) -> CheckResult {
    CheckResult {
        id,
        passed: false,
        remediation: fix.to_string(),
    }
}

/// AC2: when all checks pass, the status is `sandbox: ready` with exit code 0.
#[test]
fn test_all_checks_pass_reports_ready() {
    let results = vec![
        ok(CheckId::UserNamespaces),
        ok(CheckId::CgroupV2),
        ok(CheckId::Linger),
    ];
    let status = decide(&results);

    assert!(matches!(status.verdict, HostStatus::Ready));
    assert_eq!(status.exit_code, HOST_EXIT_READY);
    assert_eq!(status.exit_code, 0);
    assert_eq!(status.label(), "sandbox: ready");
    assert!(status.diagnostics.is_empty(), "no diagnostics when ready");
    // AC3: ready means jobs may run.
    assert!(status.accepts_new_jobs());
}

/// AC1: when a check fails, the status is degraded, exit code is non-zero, the failing check is
/// named with its remediation, and new jobs are refused (queued jobs stay queued).
#[test]
fn test_failing_check_refuses_jobs_with_remediation() {
    let results = vec![
        fail(
            CheckId::UserNamespaces,
            "enable unprivileged user namespaces: sysctl kernel.unprivileged_userns_clone=1",
        ),
        ok(CheckId::CgroupV2),
        ok(CheckId::Linger),
    ];
    let status = decide(&results);

    assert!(matches!(status.verdict, HostStatus::Degraded));
    assert_eq!(status.exit_code, HOST_EXIT_DEGRADED);
    assert_eq!(status.exit_code, 1);
    assert_ne!(status.label(), "sandbox: ready");

    // The failing check's id and remediation are surfaced.
    assert!(status
        .diagnostics
        .iter()
        .any(|d| d.contains("UserNamespaces")));
    assert!(status
        .diagnostics
        .iter()
        .any(|d| d.contains("unprivileged_userns_clone")));

    // AC3: degraded host refuses new jobs rather than failing them — queued work stays queued.
    assert!(!status.accepts_new_jobs());
}

/// Multiple failing checks each contribute a diagnostic; exit code stays 1.
#[test]
fn test_multiple_failures_all_reported() {
    let results = vec![
        fail(CheckId::UserNamespaces, "fix userns"),
        fail(CheckId::CgroupV2, "delegate memory+pids controllers"),
        fail(CheckId::Linger, "loginctl enable-linger <user>"),
    ];
    let status = decide(&results);

    assert!(matches!(status.verdict, HostStatus::Degraded));
    assert_eq!(status.exit_code, 1);
    assert_eq!(
        status.diagnostics.len(),
        3,
        "one diagnostic per failed check"
    );
    assert!(!status.accepts_new_jobs());
}

/// The live user-namespace probe is exposed as a callable check. Under bwrap in this
/// environment `unshare(CLONE_NEWUSER)` succeeds, so the probe reports `passed`; on a host where
/// it is blocked the same probe returns `passed: false` with remediation. We assert only that it
/// produces a well-formed result for the UserNamespaces id (not a specific pass/fail), because
/// the outcome is genuinely host-dependent — that is the SEAM.
#[test]
fn test_check_user_namespaces_produces_result() {
    let result = check_user_namespaces();
    assert_eq!(result.id, CheckId::UserNamespaces);
    // When it fails it must carry a non-empty remediation; when it passes remediation is unused.
    if !result.passed {
        assert!(!result.remediation.is_empty(), "failed probe names its fix");
    }
}
