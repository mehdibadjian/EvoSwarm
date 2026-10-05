//! e0-8 red-team suite — acceptance criteria (story §4).
//!
//! Red-phase tests authored before the production `red_team` module exists. The story
//! suggests `tests/red_team/test_*.rs`; the verification gate is
//! `cargo test --test e0_8_red_team_suite`, so this is a flat target in the sandbox crate.
//!
//! SEAM (partial containment, roadmap §5/§7): the **escape-denial subset that bwrap alone
//! provides** is genuinely certifiable here and is asserted — TCP egress, DNS resolution,
//! writing outside `/work`, `/home` traversal, and host-PID-1 reachability (PID-namespace
//! isolation). Probes that need capabilities this environment lacks are classified `Deferred`
//! and reported with an explicit reason rather than asserted:
//!   - reading `/etc/shadow`: the CI sandbox runs as **uid 0** with the host `/etc` ro-bind
//!     mounted, so shadow is *readable* here — denial needs a non-root uid-mapped sandbox or a
//!     sanitized `/etc` bind;
//!   - `ptrace` on host PIDs: needs the seccomp filter from e2-7 (and no ptrace tooling here);
//!   - cgroup memory/pids containment: needs writable cgroup v2 delegation from e0-4.
//!
//! The suite fails the build if any *certifiable* probe escapes (AC2); deferred gaps are made
//! visible, never silently passed.

use evoswarm_sandbox::red_team::{
    certifiable_probe_names, deferred_probes, run_all, run_quick, ProbeOutcome, RedTeamReport,
};
use evoswarm_sandbox::BwrapBackend;

/// AC1/AC2: every certifiable escape attempt is denied; none escapes. If bwrap ever stops
/// containing, `certifiable_escapes` becomes non-empty and this test fails the build.
#[tokio::test]
async fn test_certifiable_escapes_all_denied() {
    let backend = BwrapBackend::new();
    let report: RedTeamReport = run_all(&backend).await.expect("red-team run");

    let escapes = report.certifiable_escapes();
    assert!(
        escapes.is_empty(),
        "certifiable escape attempts must all be denied, but escaped: {escapes:?}"
    );

    // Every certifiable probe actually ran and was denied (not skipped).
    let denials = report.certifiable_denials();
    assert_eq!(
        denials.len(),
        certifiable_probe_names().len(),
        "all certifiable probes ran and were denied: {denials:?}"
    );
    assert!(denials.contains(&"tcp-egress"), "TCP egress denied");
    assert!(denials.contains(&"dns-resolve"), "DNS resolution denied");
    assert!(
        denials.contains(&"write-outside-work"),
        "write outside /work denied"
    );
    assert!(
        denials.contains(&"host-pid1"),
        "host PID 1 unreachable (PID-ns isolation)"
    );
    // e2-7: the raw-syscall probes are now certifiable seccomp denials (previously the
    // `ptrace-host` gap was deferred). They run in the same suite, so assert a couple here.
    assert!(
        denials.contains(&"seccomp-ptrace"),
        "ptrace(101) denied by seccomp filter"
    );
    assert!(
        denials.contains(&"seccomp-unshare"),
        "unshare(272) denied by seccomp filter"
    );
}

/// AC1 (honesty half): probes this environment cannot certify are Deferred with a non-empty
/// reason, and are NOT counted as denials. This prevents a false "all contained" claim.
#[tokio::test]
async fn test_deferred_probes_flagged_with_reasons() {
    let backend = BwrapBackend::new();
    let report = run_all(&backend).await.expect("red-team run");

    let deferred = report.deferred();
    assert!(
        deferred.contains(&"shadow-read"),
        "shadow-read is deferred, not falsely denied"
    );
    assert!(
        deferred.contains(&"cgroup-memory-cap"),
        "cgroup cap deferred (needs e0-4)"
    );
    // e2-7 resolved the former `ptrace-host` deferral: ptrace is now a certifiable
    // seccomp denial, so it must NOT appear among the deferred gaps anymore.
    assert!(
        !deferred.contains(&"ptrace-host"),
        "ptrace-host deferral is resolved by e2-7 seccomp; it is now certifiable"
    );

    // Each deferred probe carries an explanatory reason.
    for name in ["shadow-read", "cgroup-memory-cap"] {
        let reason = deferred_probes()
            .iter()
            .find(|p| p.name == name)
            .unwrap_or_else(|| panic!("{name} is a declared probe"))
            .deferred_reason;
        assert!(
            !reason.is_empty(),
            "{name} deferral names its unblock condition"
        );
    }

    // Deferred probes are recorded as NotRun, never as a denial.
    for r in &report.results {
        if r.class.is_deferred() {
            assert_eq!(
                r.outcome,
                ProbeOutcome::NotRun,
                "{} must not be asserted",
                r.name
            );
        }
    }
}

/// AC3: a quick subset of the certifiable suite runs fast and denies — this is the subset e0-2
/// folds into the host self-check. It must be a strict, non-empty subset of the certifiable
/// probes and all-denied.
#[tokio::test]
async fn test_quick_subset_is_certifiable_and_denied() {
    let backend = BwrapBackend::new();
    let quick = run_quick(&backend).await.expect("quick red-team run");

    assert!(!quick.results.is_empty(), "quick subset is non-empty");
    assert!(
        quick.certifiable_escapes().is_empty(),
        "quick subset denies every escape: {:?}",
        quick.certifiable_escapes()
    );
    // Every quick probe is also a certifiable (non-deferred) probe.
    let certifiable = certifiable_probe_names();
    for r in &quick.results {
        assert!(
            certifiable.contains(&r.name),
            "quick probe {} must be certifiable",
            r.name
        );
    }
}
