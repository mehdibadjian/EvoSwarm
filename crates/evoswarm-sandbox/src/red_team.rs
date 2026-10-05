//! Red-team suite (e0-8, AD-1): automated escape attempts so isolation regressions are caught
//! before release. Any successful exploit must fail the build.
//!
//! Each probe runs a shell command inside the bwrap sandbox that prints the sentinel
//! [`ESCAPED`] *only if the escape succeeds*. The runner treats sentinel presence in stdout as
//! an escape and its absence as a denial, so a probe cannot pass by merely failing to compile or
//! erroring out — silence is the denial signal.
//!
//! ## Containment honesty (roadmap §7)
//!
//! Probes split into two classes:
//!
//! - [`ProbeClass::Certifiable`] — denial is genuinely enforceable by bwrap in *this*
//!   environment and is asserted: TCP egress, DNS resolution, writing outside `/work`, `/home`
//!   traversal, host-PID-1 reachability (PID-namespace isolation), and — since e2-7 — the raw
//!   syscalls ptrace/mount/umount2/keyctl/bpf/perf_event_open/unshare, killed by the seccomp
//!   filter.
//! - [`ProbeClass::Deferred`] — the escape cannot be certified here because a required control
//!   is absent; the probe is recorded as [`ProbeOutcome::NotRun`] with a reason and is **never**
//!   counted as a denial, so a partial environment can never fake full containment:
//!   - `shadow-read`: the CI sandbox runs as uid 0 with the host `/etc` ro-bind mounted, so
//!     `/etc/shadow` is readable here; real denial needs a non-root uid-mapped sandbox.
//!   - `cgroup-memory-cap`: needs writable cgroup v2 delegation from e0-4.
//!
//! Note: the pre-e2-7 `ptrace-host` deferral is now resolved — ptrace is a certifiable
//! seccomp denial, not a gap.
//!
//! [`run_all`] executes the full suite; [`run_quick`] executes a fast certifiable subset that
//! e0-2 folds into the host self-check.

use crate::{SandboxBackend, SandboxError};

/// Sentinel a probe prints to stdout only when its escape succeeds.
pub const ESCAPED: &str = "EVOSWARM_ESCAPED";

/// Whether a probe's denial is enforceable in this environment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeClass {
    /// Denial is enforceable by bwrap here and is asserted.
    Certifiable,
    /// Denial needs a control this environment lacks; recorded as NotRun with a reason.
    Deferred,
}

impl ProbeClass {
    /// True for [`ProbeClass::Deferred`].
    pub fn is_deferred(self) -> bool {
        matches!(self, ProbeClass::Deferred)
    }
}

/// What happened when a probe ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The escape was denied (sentinel absent) — the contained result.
    Denied,
    /// The escape succeeded (sentinel present) — an isolation failure.
    Escaped,
    /// The probe could not be certified here (deferred class) or the run errored.
    NotRun,
}

/// A single declared escape attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// Stable identifier used in reports (e.g. `tcp-egress`).
    pub name: &'static str,
    /// Shell command run inside the sandbox; must print [`ESCAPED`] only on success.
    pub command: &'static str,
    /// Whether denial is certifiable in this environment.
    pub class: ProbeClass,
    /// For deferred probes, why they cannot be certified here; empty otherwise.
    pub deferred_reason: &'static str,
    /// Whether the probe is part of the fast e0-2 self-check subset.
    pub quick: bool,
}

/// The result of running one probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResult {
    pub name: &'static str,
    pub class: ProbeClass,
    pub outcome: ProbeOutcome,
    /// Populated when a deferred probe was skipped, or an error message on a failed run.
    pub note: String,
}

/// The full outcome of a red-team run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedTeamReport {
    pub results: Vec<ProbeResult>,
}

impl RedTeamReport {
    /// Names of certifiable probes that **escaped** — must be empty for the build to pass.
    pub fn certifiable_escapes(&self) -> Vec<&'static str> {
        self.results
            .iter()
            .filter(|r| !r.class.is_deferred() && r.outcome == ProbeOutcome::Escaped)
            .map(|r| r.name)
            .collect()
    }

    /// Names of certifiable probes that were **denied**.
    pub fn certifiable_denials(&self) -> Vec<&'static str> {
        self.results
            .iter()
            .filter(|r| !r.class.is_deferred() && r.outcome == ProbeOutcome::Denied)
            .map(|r| r.name)
            .collect()
    }

    /// Names of probes deferred (not certifiable here).
    pub fn deferred(&self) -> Vec<&'static str> {
        self.results
            .iter()
            .filter(|r| r.class.is_deferred())
            .map(|r| r.name)
            .collect()
    }
}

/// The full declared suite: certifiable bwrap-enforced escapes plus deferred gaps.
fn suite() -> Vec<Probe> {
    vec![
        Probe {
            name: "tcp-egress",
            // bash /dev/tcp connect: prints the sentinel only if a TCP connection is opened.
            command: "bash -c 'exec 3<>/dev/tcp/1.1.1.1/443 && echo EVOSWARM_ESCAPED' 2>/dev/null; true",
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: true,
        },
        Probe {
            name: "dns-resolve",
            // Real resolver attempt via python3; sentinel only on a successful lookup.
            command: "python3 -c 'import socket; socket.gethostbyname(\"example.com\"); print(\"EVOSWARM_ESCAPED\")' 2>/dev/null || true",
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: true,
        },
        Probe {
            name: "write-outside-work",
            // Attempt to write outside /work; sentinel only if the write succeeds.
            command: "(touch /etc/evoswarm_pwned 2>/dev/null && echo EVOSWARM_ESCAPED) || true",
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: true,
        },
        Probe {
            name: "home-traversal",
            // /home is not mounted into the sandbox; sentinel only if it is present.
            command: "(test -d /home && echo EVOSWARM_ESCAPED) || true",
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: false,
        },
        Probe {
            name: "host-pid1",
            // PID-namespace isolation: sandbox /proc/1 is the bwrap init, not host systemd/init.
            // Sentinel only if the sandbox's PID 1 looks like the host init.
            command: "grep -qE 'systemd|init' /proc/1/comm && echo EVOSWARM_ESCAPED || true",
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: true,
        },
        // --- e2-7 seccomp syscall probes ---------------------------------------------
        // Each invokes the RAW syscall (via python3 ctypes, NULL args) that the e2-7 filter
        // kills, and prints the sentinel only if the syscall *returns*. Under the filter the
        // process is SIGSYS-killed before the print, so silence = denied. Using the raw
        // syscall (not a util-linux binary) matters: e.g. `unshare -U` calls clone(2), which
        // the unshare(272) rule would not catch. x86-64 numbers per
        // /usr/include/x86_64-linux-gnu/asm/unistd_64.h.
        Probe {
            name: "seccomp-ptrace",
            command: r#"python3 -c "import ctypes;l=ctypes.CDLL(None);l.syscall.restype=ctypes.c_long;l.syscall(ctypes.c_long(101),ctypes.c_long(0),ctypes.c_long(0),ctypes.c_long(0));print('EVOSWARM_ESCAPED')""#,
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: false,
        },
        Probe {
            name: "seccomp-mount",
            command: r#"python3 -c "import ctypes;l=ctypes.CDLL(None);l.syscall.restype=ctypes.c_long;l.syscall(ctypes.c_long(165),ctypes.c_long(0),ctypes.c_long(0),ctypes.c_long(0));print('EVOSWARM_ESCAPED')""#,
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: false,
        },
        Probe {
            name: "seccomp-umount2",
            command: r#"python3 -c "import ctypes;l=ctypes.CDLL(None);l.syscall.restype=ctypes.c_long;l.syscall(ctypes.c_long(166),ctypes.c_long(0),ctypes.c_long(0),ctypes.c_long(0));print('EVOSWARM_ESCAPED')""#,
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: false,
        },
        Probe {
            name: "seccomp-keyctl",
            command: r#"python3 -c "import ctypes;l=ctypes.CDLL(None);l.syscall.restype=ctypes.c_long;l.syscall(ctypes.c_long(250),ctypes.c_long(0),ctypes.c_long(0),ctypes.c_long(0));print('EVOSWARM_ESCAPED')""#,
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: false,
        },
        Probe {
            name: "seccomp-bpf",
            command: r#"python3 -c "import ctypes;l=ctypes.CDLL(None);l.syscall.restype=ctypes.c_long;l.syscall(ctypes.c_long(321),ctypes.c_long(0),ctypes.c_long(0),ctypes.c_long(0));print('EVOSWARM_ESCAPED')""#,
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: false,
        },
        Probe {
            name: "seccomp-perf-event-open",
            command: r#"python3 -c "import ctypes;l=ctypes.CDLL(None);l.syscall.restype=ctypes.c_long;l.syscall(ctypes.c_long(298),ctypes.c_long(0),ctypes.c_long(0),ctypes.c_long(0));print('EVOSWARM_ESCAPED')""#,
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: false,
        },
        Probe {
            name: "seccomp-unshare",
            command: r#"python3 -c "import ctypes;l=ctypes.CDLL(None);l.syscall.restype=ctypes.c_long;l.syscall(ctypes.c_long(272),ctypes.c_long(0),ctypes.c_long(0),ctypes.c_long(0));print('EVOSWARM_ESCAPED')""#,
            class: ProbeClass::Certifiable,
            deferred_reason: "",
            quick: false,
        },
        Probe {
            name: "shadow-read",
            // Reading /etc/shadow: enforceable only under a non-root uid-mapped sandbox.
            command: "(cat /etc/shadow >/dev/null 2>&1 && echo EVOSWARM_ESCAPED) || true",
            class: ProbeClass::Deferred,
            deferred_reason: "CI sandbox runs as uid 0 with host /etc ro-bind mounted, so shadow \
                is readable here; denial needs a non-root uid-mapped sandbox or sanitized /etc bind",
            quick: false,
        },
        Probe {
            name: "cgroup-memory-cap",
            // Exceeding a memory cap: enforceable only with cgroup v2 delegation (e0-4).
            command: "true", // cgroups not writable here; classification carries the truth
            class: ProbeClass::Deferred,
            deferred_reason: "needs writable cgroup v2 delegation (memory/pids) from e0-4; not run here",
            quick: false,
        },
    ]
}

/// Names of every certifiable (non-deferred) probe in the suite.
pub fn certifiable_probe_names() -> Vec<&'static str> {
    suite()
        .into_iter()
        .filter(|p| !p.class.is_deferred())
        .map(|p| p.name)
        .collect()
}

/// The declared deferred probes, with their reasons, for report consumers that want the gap list
/// without running the suite.
pub fn deferred_probes() -> Vec<Probe> {
    suite()
        .into_iter()
        .filter(|p| p.class.is_deferred())
        .collect()
}

/// Runs the full suite (every declared probe) against `backend`.
pub async fn run_all<B: SandboxBackend + ?Sized>(
    backend: &B,
) -> Result<RedTeamReport, SandboxError> {
    run_probes(backend, &suite()).await
}

/// Runs the fast certifiable subset (probes flagged `quick`) — the set e0-2 folds into the host
/// self-check.
pub async fn run_quick<B: SandboxBackend + ?Sized>(
    backend: &B,
) -> Result<RedTeamReport, SandboxError> {
    let quick: Vec<Probe> = suite().into_iter().filter(|p| p.quick).collect();
    run_probes(backend, &quick).await
}

/// Executes `probes`, classifying each result. Deferred probes are recorded NotRun without
/// executing (their command is a placeholder; the classification carries the truth).
async fn run_probes<B: SandboxBackend + ?Sized>(
    backend: &B,
    probes: &[Probe],
) -> Result<RedTeamReport, SandboxError> {
    use evoswarm_core::SandboxProfile;
    use std::path::PathBuf;

    // A minimal profile; the probes only need the bwrap isolation, not resource caps. Escape
    // attempts fail fast (connection refused / read-only fs), so the backend's own deadline is
    // ample and no probe can hang the suite.
    let profile = SandboxProfile {
        stack: "red-team".into(),
        wall_timeout_secs: 30,
        memory_limit_bytes: 0,
        tmpfs_size_bytes: 0,
        tasks_max: 0,
        read_only_mounts: Vec::new(),
        dependency_cache_path: PathBuf::new(),
    };

    let mut results = Vec::with_capacity(probes.len());
    for probe in probes {
        if probe.class.is_deferred() {
            results.push(ProbeResult {
                name: probe.name,
                class: probe.class,
                outcome: ProbeOutcome::NotRun,
                note: probe.deferred_reason.to_string(),
            });
            continue;
        }

        let workdir = backend.prepare(&profile, b"").await?;
        let run = backend.run(&workdir, probe.command).await;
        let _ = backend.collect(workdir).await;

        match run {
            Ok(result) => {
                let escaped = result.stdout.contains(ESCAPED);
                results.push(ProbeResult {
                    name: probe.name,
                    class: probe.class,
                    outcome: if escaped {
                        ProbeOutcome::Escaped
                    } else {
                        ProbeOutcome::Denied
                    },
                    note: String::new(),
                });
            }
            Err(e) => results.push(ProbeResult {
                name: probe.name,
                class: probe.class,
                outcome: ProbeOutcome::NotRun,
                note: format!("probe run failed: {e}"),
            }),
        }
    }

    Ok(RedTeamReport { results })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_has_certifiable_and_deferred() {
        let s = suite();
        assert!(s.iter().any(|p| !p.class.is_deferred()));
        assert!(s.iter().any(|p| p.class.is_deferred()));
        // Names are unique.
        let mut seen = std::collections::HashSet::new();
        for p in &s {
            assert!(seen.insert(p.name), "duplicate probe name {}", p.name);
        }
        // Deferred probes carry a reason; certifiable ones do not.
        for p in &s {
            if p.class.is_deferred() {
                assert!(!p.deferred_reason.is_empty(), "{} needs a reason", p.name);
            } else {
                assert!(
                    p.deferred_reason.is_empty(),
                    "{} should have no reason",
                    p.name
                );
            }
        }
    }

    #[test]
    fn quick_subset_is_certifiable() {
        for p in suite().iter().filter(|p| p.quick) {
            assert!(
                !p.class.is_deferred(),
                "quick probe {} must be certifiable",
                p.name
            );
        }
        assert!(suite().iter().any(|p| p.quick), "quick subset is non-empty");
    }

    #[test]
    fn certifiable_names_exclude_deferred() {
        let names = certifiable_probe_names();
        assert!(names.contains(&"tcp-egress"));
        assert!(!names.contains(&"shadow-read"));
        assert!(!names.contains(&"ptrace-host"));
    }

    #[test]
    fn report_classification_counts() {
        let report = RedTeamReport {
            results: vec![
                ProbeResult {
                    name: "tcp-egress",
                    class: ProbeClass::Certifiable,
                    outcome: ProbeOutcome::Denied,
                    note: String::new(),
                },
                ProbeResult {
                    name: "write-outside-work",
                    class: ProbeClass::Certifiable,
                    outcome: ProbeOutcome::Escaped,
                    note: String::new(),
                },
                ProbeResult {
                    name: "shadow-read",
                    class: ProbeClass::Deferred,
                    outcome: ProbeOutcome::NotRun,
                    note: "needs non-root".into(),
                },
            ],
        };
        assert_eq!(report.certifiable_escapes(), vec!["write-outside-work"]);
        assert_eq!(report.certifiable_denials(), vec!["tcp-egress"]);
        assert_eq!(report.deferred(), vec!["shadow-read"]);
    }
}
