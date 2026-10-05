//! e2-7 seccomp filter — acceptance criteria (story §4).
//!
//! Red-phase tests authored before the production `seccomp` module exists. The story
//! suggests `tests/security/test_seccomp.rs`; the verification gate is
//! `cargo test --test e2_7_seccomp_filter`, so this is a flat target in the sandbox crate.
//!
//! Unlike the roadmap's SEAM assumption, bwrap `--seccomp FD` genuinely enforces on this
//! host (bubblewrap 0.6.1, x86-64), so the *syscall-denial* half of this story is
//! certifiable end-to-end and is asserted here against real bwrap runs — not stubbed:
//!
//! - AC1: with the filter active, ptrace / mount / umount2 / keyctl / bpf /
//!   perf_event_open / unshare are killed (SIGSYS), while ordinary syscalls still run.
//! - AC2: a normal harness (compute + file IO + fork/exec) still passes under the filter.
//! - AC3: the red-team suite, extended with a probe per blocked syscall, denies every one.
//!
//! Anti-cheat: each raw-syscall probe is also run with seccomp **disabled** to prove the
//! probe really invokes the syscall (it returns and prints its sentinel), so a denial under
//! the filter cannot be a vacuous pass from a broken probe.
//!
//! The syscall numbers are the **x86-64 ABI** values (unshare=272, not the asm-generic 97);
//! they are asserted against the module constants, which are checked against
//! `/usr/include/x86_64-linux-gnu/asm/unistd_64.h`.

use evoswarm_core::{RunStatus, SandboxProfile};
use evoswarm_sandbox::red_team;
use evoswarm_sandbox::seccomp;
use evoswarm_sandbox::{BwrapBackend, SandboxBackend};
use std::path::PathBuf;

fn test_profile() -> SandboxProfile {
    SandboxProfile {
        stack: "seccomp".to_string(),
        wall_timeout_secs: 30,
        memory_limit_bytes: 256 * 1024 * 1024,
        tmpfs_size_bytes: 64 * 1024 * 1024,
        tasks_max: 10,
        read_only_mounts: vec![],
        dependency_cache_path: PathBuf::new(),
    }
}

/// A probe that invokes raw syscall `nr` (with NULL args so write-style syscalls fault on
/// EFAULT rather than dereferencing) and prints `SCALL_RETURNED` only if the syscall
/// actually returns. Under the seccomp filter the process is SIGSYS-killed before the print,
/// so the sentinel's presence/absence is the pass/fail signal.
fn raw_syscall_probe(nr: u32) -> String {
    format!(
        "python3 -c \"import ctypes;l=ctypes.CDLL(None);l.syscall.restype=ctypes.c_long;\
l.syscall(ctypes.c_long({nr}),ctypes.c_long(0),ctypes.c_long(0),ctypes.c_long(0));\
print('SCALL_RETURNED')\""
    )
}

/// The 7 blocked syscalls the story names, as (number, name) pairs, cross-checked against
/// the authoritative x86-64 values.
fn blocked_pairs() -> Vec<(u32, &'static str)> {
    // Guard the ABI assumption: these must be the x86-64 numbers, not asm-generic.
    assert_eq!(seccomp::SYS_UNSHARE, 272, "x86-64 unshare is 272");
    assert_eq!(seccomp::SYS_PTRACE, 101, "x86-64 ptrace is 101");
    assert_eq!(seccomp::SYS_MOUNT, 165, "x86-64 mount is 165");
    assert_eq!(seccomp::SYS_UMOUNT2, 166, "x86-64 umount2 is 166");
    assert_eq!(seccomp::SYS_KEYCTL, 250, "x86-64 keyctl is 250");
    assert_eq!(seccomp::SYS_BPF, 321, "x86-64 bpf is 321");
    assert_eq!(
        seccomp::SYS_PERF_EVENT_OPEN,
        298,
        "x86-64 perf_event_open is 298"
    );

    seccomp::BLOCKED_SYSCALLS
        .iter()
        .copied()
        .zip(seccomp::BLOCKED_SYSCALL_NAMES.iter().copied())
        .collect()
}

/// AC1: with the filter active (default), every blocked syscall is killed — the probe never
/// prints its sentinel and the run does not exit 0. An unblocked syscall (getuid) still runs.
#[tokio::test]
async fn test_blocked_syscalls_sigsys() {
    let backend = BwrapBackend::new(); // seccomp is on by default
    let profile = test_profile();

    for (nr, name) in blocked_pairs() {
        let workdir = backend.prepare(&profile, b"").await.expect("prepare");
        let res = backend
            .run(&workdir, &raw_syscall_probe(nr))
            .await
            .expect("run");
        backend.collect(workdir).await.expect("collect");

        assert!(
            !res.stdout.contains("SCALL_RETURNED"),
            "{name}({nr}) must be killed by seccomp, but the syscall returned: stdout={:?} stderr={:?}",
            res.stdout,
            res.stderr
        );
        assert_ne!(
            res.exit_code, 0,
            "{name}({nr}) must not exit 0 under the filter (status={:?})",
            res.status
        );
        assert_eq!(
            res.status,
            RunStatus::Failed,
            "{name}({nr}) killed by SIGSYS classifies as Failed"
        );
    }

    // A syscall outside the blocklist still executes normally under the same filter.
    let workdir = backend.prepare(&profile, b"").await.expect("prepare");
    let res = backend
        .run(&workdir, &raw_syscall_probe(102 /* getuid */))
        .await
        .expect("run");
    backend.collect(workdir).await.expect("collect");
    assert!(
        res.stdout.contains("SCALL_RETURNED"),
        "getuid(102) is not blocked and must still run: stdout={:?} stderr={:?}",
        res.stdout,
        res.stderr
    );
    assert_eq!(res.exit_code, 0, "allowed syscall exits 0");
}

/// Anti-cheat half of AC1: with seccomp DISABLED, the same probes really do invoke the
/// syscall and return (printing the sentinel). This proves the denials above come from the
/// filter and not from a probe that never reached the kernel.
#[tokio::test]
async fn test_probe_would_escape_without_seccomp() {
    let backend = BwrapBackend::new().with_seccomp(false);
    let profile = test_profile();

    for (nr, name) in blocked_pairs() {
        let workdir = backend.prepare(&profile, b"").await.expect("prepare");
        let res = backend
            .run(&workdir, &raw_syscall_probe(nr))
            .await
            .expect("run");
        backend.collect(workdir).await.expect("collect");

        assert!(
            res.stdout.contains("SCALL_RETURNED"),
            "{name}({nr}) must RETURN when seccomp is off (probe is real): status={:?} stdout={:?} stderr={:?}",
            res.status,
            res.stdout,
            res.stderr
        );
    }
}

/// AC2: a normal harness — compute, file IO, fork/exec — still passes under the filter, so
/// blocking these 7 syscalls does not break ordinary candidate test suites.
#[tokio::test]
async fn test_harness_compatibility() {
    let backend = BwrapBackend::new(); // seccomp on
    let profile = test_profile();
    let workdir = backend.prepare(&profile, b"").await.expect("prepare");

    // Exercises open/read/write, mkdtemp, and subprocess fork/exec/wait — all allowed.
    let harness = "python3 -c \"import subprocess,tempfile;\
d=tempfile.mkdtemp();f=open(d+'/x','w');f.write('hi');f.close();\
assert open(d+'/x').read()=='hi';\
assert subprocess.run(['echo','ok']).returncode==0;print('HARNESS_OK')\"";

    let res = backend.run(&workdir, harness).await.expect("run");
    backend.collect(workdir).await.expect("collect");

    assert_eq!(
        res.status,
        RunStatus::Success,
        "harness must succeed under seccomp: stdout={:?} stderr={:?}",
        res.stdout,
        res.stderr
    );
    assert_eq!(res.exit_code, 0);
    assert!(res.stdout.contains("HARNESS_OK"));
}

/// AC3: the red-team suite, extended with a certifiable probe per blocked syscall, denies
/// every one and reports no escapes.
#[tokio::test]
async fn test_red_team_syscall_probes_denied() {
    let backend = BwrapBackend::new(); // seccomp on
    let report = red_team::run_all(&backend).await.expect("red-team run");

    let denials = report.certifiable_denials();
    for name in [
        "seccomp-ptrace",
        "seccomp-mount",
        "seccomp-umount2",
        "seccomp-keyctl",
        "seccomp-bpf",
        "seccomp-perf-event-open",
        "seccomp-unshare",
    ] {
        assert!(
            denials.contains(&name),
            "{name} probe must be denied by the filter; denials={denials:?}"
        );
    }
    assert!(
        report.certifiable_escapes().is_empty(),
        "no certifiable probe may escape: {:?}",
        report.certifiable_escapes()
    );
}

/// The assembler is deterministic and produces a well-formed cBPF program: a whole number of
/// 8-byte `sock_filter` instructions, with the expected count (4 header + 2×7 blocked + 1
/// allow = 19) and the arch-check as its first load.
#[test]
fn test_filter_is_deterministic_and_well_formed() {
    let a = seccomp::build_filter();
    let b = seccomp::build_filter();
    assert_eq!(a, b, "build_filter must be deterministic");
    assert_eq!(a.len() % 8, 0, "filter is whole sock_filter instructions");

    let instrs = a.len() / 8;
    let expected = 4 + 2 * seccomp::BLOCKED_SYSCALLS.len() + 1;
    assert_eq!(instrs, expected, "instruction count matches the layout");

    // First instruction loads seccomp_data.arch (offset 4): the k field (bytes 4..8) is 4.
    let k0 = u32::from_ne_bytes([a[4], a[5], a[6], a[7]]);
    assert_eq!(k0, 4, "first instruction loads the arch field at offset 4");
}

/// Sanity: the workdir path is still threaded through when seccomp argv is inserted, and the
/// seccomp flag toggles the `--seccomp 0` argv pair.
#[test]
fn test_argv_includes_seccomp_flag_when_enabled() {
    let wd = PathBuf::from("/tmp/wd");
    let on = BwrapBackend::new().build_bwrap_argv(&wd, "echo hi");
    assert!(
        on.windows(2).any(|w| w[0] == "--seccomp" && w[1] == "0"),
        "seccomp-on argv must contain `--seccomp 0`: {on:?}"
    );

    let off = BwrapBackend::new()
        .with_seccomp(false)
        .build_bwrap_argv(&wd, "echo hi");
    assert!(
        !off.iter().any(|a| a == "--seccomp"),
        "seccomp-off argv must not contain `--seccomp`: {off:?}"
    );
}
