//! Host self-check (e0-2, AD-1): verify host isolation features at start and via
//! `evoswarm status`, so jobs never run on a box where the sandbox silently degrades.
//!
//! Unprivileged namespaces, cgroup v2, and systemd user lingering can each be broken silently
//! by a host kernel update. This module runs deterministic health checks and reduces them to a
//! single verdict:
//!
//! - **Ready** (`sandbox: ready`, exit 0) when every check passes — the service accepts jobs.
//! - **Degraded** (exit 1) when any check fails — each failure is surfaced with a remediation
//!   message and the service **refuses new jobs**. Queued jobs stay queued rather than failing
//!   (AC3): a degraded host stops intake; it does not tear down work already admitted.
//!
//! The [`decide`] reduction is pure and fully unit/integration tested against injected
//! [`CheckResult`]s for every [`CheckId`]. Of the *live* probes, only [`check_user_namespaces`]
//! is implemented here (it spawns `unshare -U`, which genuinely works under bwrap). The cgroup
//! v2 *delegation* and systemd *user-linger* probes are a SEAM: neither a delegated cgroup
//! subtree nor a user session bus exists in this environment (roadmap §6), so those two live
//! probes are **not** wired and their `CheckId` variants are exercised only through injected
//! results. They must be backed by real checks before the degraded path can be certified
//! end-to-end on a production host.

use std::process::Command;

/// Exit code when every host check passes.
pub const HOST_EXIT_READY: i32 = 0;

/// Exit code when any host check fails.
pub const HOST_EXIT_DEGRADED: i32 = 1;

/// Which isolation feature a check probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckId {
    /// Unprivileged user namespaces (`unshare(CLONE_NEWUSER)`).
    UserNamespaces,
    /// cgroup v2 delegation of `memory` and `pids` controllers to the runtime user.
    CgroupV2,
    /// systemd user lingering enabled for the runtime user.
    Linger,
}

impl CheckId {
    /// A stable, human-readable name used in diagnostics.
    pub fn name(self) -> &'static str {
        match self {
            CheckId::UserNamespaces => "UserNamespaces",
            CheckId::CgroupV2 => "CgroupV2",
            CheckId::Linger => "Linger",
        }
    }
}

/// The outcome of one host check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    /// Which feature was probed.
    pub id: CheckId,
    /// Whether the feature is usable.
    pub passed: bool,
    /// Fix instructions, populated only when `passed` is false.
    pub remediation: String,
}

/// The overall verdict of the self-check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostStatus {
    /// Every check passed; the sandbox is fully capable.
    Ready,
    /// At least one check failed; the sandbox would degrade, so intake is refused.
    Degraded,
}

/// The reduced result of running the host self-check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusReport {
    /// Ready vs Degraded.
    pub verdict: HostStatus,
    /// Process exit code: [`HOST_EXIT_READY`] or [`HOST_EXIT_DEGRADED`].
    pub exit_code: i32,
    /// One remediation line per failed check, empty when Ready.
    pub diagnostics: Vec<String>,
}

impl StatusReport {
    /// The single-line status label printed by `evoswarm status`.
    pub fn label(&self) -> &'static str {
        match self.verdict {
            HostStatus::Ready => "sandbox: ready",
            HostStatus::Degraded => "sandbox: degraded",
        }
    }

    /// Whether the service should accept new jobs. A degraded host stops intake so queued jobs
    /// stay queued (AC3) instead of being admitted into a broken sandbox.
    pub fn accepts_new_jobs(&self) -> bool {
        matches!(self.verdict, HostStatus::Ready)
    }
}

/// Reduces a set of check results into a single verdict. Pure: given the same results it always
/// yields the same report, so it is fully testable without touching the host.
pub fn decide(results: &[CheckResult]) -> StatusReport {
    let diagnostics: Vec<String> = results
        .iter()
        .filter(|r| !r.passed)
        .map(|r| format!("{}: {}", r.id.name(), r.remediation))
        .collect();

    if diagnostics.is_empty() {
        StatusReport {
            verdict: HostStatus::Ready,
            exit_code: HOST_EXIT_READY,
            diagnostics,
        }
    } else {
        StatusReport {
            verdict: HostStatus::Degraded,
            exit_code: HOST_EXIT_DEGRADED,
            diagnostics,
        }
    }
}

/// Live probe: can this host create an unprivileged user namespace? Spawns `unshare -U true` and
/// treats a zero exit as success. Returns remediation naming the common sysctl/`kernel.apparmor`
/// causes when it fails or when the `unshare` utility is unavailable.
pub fn check_user_namespaces() -> CheckResult {
    let remediation = "enable unprivileged user namespaces \
        (sysctl kernel.unprivileged_userns_clone=1) and ensure the sandbox tool is not blocked by \
        AppArmor; install util-linux if `unshare` is missing";

    match Command::new("unshare").arg("-U").arg("true").status() {
        Ok(status) if status.success() => CheckResult {
            id: CheckId::UserNamespaces,
            passed: true,
            remediation: String::new(),
        },
        _ => CheckResult {
            id: CheckId::UserNamespaces,
            passed: false,
            remediation: remediation.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_results_are_ready() {
        let status = decide(&[]);
        assert!(matches!(status.verdict, HostStatus::Ready));
        assert_eq!(status.exit_code, 0);
        assert!(status.accepts_new_jobs());
    }

    #[test]
    fn diagnostic_names_check_and_fix() {
        let status = decide(&[CheckResult {
            id: CheckId::CgroupV2,
            passed: false,
            remediation: "delegate memory+pids".into(),
        }]);
        assert_eq!(
            status.diagnostics,
            vec!["CgroupV2: delegate memory+pids".to_string()]
        );
        assert_eq!(status.label(), "sandbox: degraded");
        assert!(!status.accepts_new_jobs());
    }

    #[test]
    fn check_ids_have_distinct_names() {
        let names = [
            CheckId::UserNamespaces.name(),
            CheckId::CgroupV2.name(),
            CheckId::Linger.name(),
        ];
        let mut set = std::collections::HashSet::new();
        for n in names {
            assert!(set.insert(n), "duplicate check name {n}");
        }
    }

    #[test]
    fn userns_probe_is_well_formed() {
        let r = check_user_namespaces();
        assert_eq!(r.id, CheckId::UserNamespaces);
        if !r.passed {
            assert!(!r.remediation.is_empty());
        }
    }
}
