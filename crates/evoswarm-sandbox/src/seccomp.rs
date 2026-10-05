//! seccomp-bpf filter for the bwrap sandbox (e2-7, AD-1).
//!
//! Candidate code never needs to create namespaces, trace other processes, mount filesystems,
//! load kernel key material, install BPF programs, or open perf counters. Allowing those
//! syscalls widens the kernel attack surface the sandbox must defend, so this module builds a
//! classic-BPF (cBPF) filter that kills the process on any of them and lets everything else
//! through. The filter is handed to bubblewrap via `--seccomp FD` (see `bwrap.rs`).
//!
//! ## Why a hand-assembled filter
//!
//! The crate deliberately avoids a `libseccomp`/`seccomp-bpf` dependency: the blocklist is a
//! fixed, auditable set of syscall numbers, so the `sock_filter` program is emitted directly.
//! That keeps `evoswarm-sandbox` free of a C-toolchain dependency and makes the exact bytes
//! reviewable in this file and asserted in tests.
//!
//! ## ABI
//!
//! The syscall numbers below are the **x86-64** values, verified against
//! `/usr/include/x86_64-linux-gnu/asm/unistd_64.h`. They differ from the asm-generic table
//! (e.g. x86-64 `unshare` is 272, not 97), so the filter first checks `seccomp_data.arch`
//! and kills anything that is not `AUDIT_ARCH_X86_64` rather than silently matching the wrong
//! number on another ABI.
//!
//! ## Layout
//!
//! ```text
//!   ld  [arch]                       ; M <- seccomp_data.arch
//!   jeq AUDIT_ARCH_X86_64 ? next : kill
//!   ret KILL_PROCESS                 ; wrong arch
//!   ld  [nr]                         ; M <- seccomp_data.nr
//!   for each blocked nr:
//!     jeq nr ? kill : skip           ; (jt=0 -> next instr is kill; jf=1 -> skip the kill)
//!     ret KILL_PROCESS
//!   ret ALLOW                        ; default
//! ```
//!
//! `KILL_PROCESS` (not `KILL_THREAD`) so a blocked syscall in any thread of the sandbox takes
//! the whole run down deterministically.

use std::mem::size_of;

// --- x86-64 syscall numbers (unistd_64.h) ---------------------------------------------
pub const SYS_PTRACE: u32 = 101;
pub const SYS_MOUNT: u32 = 165;
pub const SYS_UMOUNT2: u32 = 166;
pub const SYS_KEYCTL: u32 = 250;
pub const SYS_UNSHARE: u32 = 272;
pub const SYS_PERF_EVENT_OPEN: u32 = 298;
pub const SYS_BPF: u32 = 321;

/// The blocked syscall numbers, in filter order.
pub const BLOCKED_SYSCALLS: [u32; 7] = [
    SYS_PTRACE,
    SYS_MOUNT,
    SYS_UMOUNT2,
    SYS_KEYCTL,
    SYS_BPF,
    SYS_PERF_EVENT_OPEN,
    SYS_UNSHARE,
];

/// Human-readable names for `BLOCKED_SYSCALLS`, index-aligned (used by the red-team probes
/// and diagnostics).
pub const BLOCKED_SYSCALL_NAMES: [&str; 7] = [
    "ptrace",
    "mount",
    "umount2",
    "keyctl",
    "bpf",
    "perf_event_open",
    "unshare",
];

// --- cBPF instruction encoding (see <linux/filter.h>) ---------------------------------
const BPF_LD: u16 = 0x00;
const BPF_W: u16 = 0x00;
const BPF_ABS: u16 = 0x20;
const BPF_JMP: u16 = 0x05;
const BPF_JEQ: u16 = 0x10;
const BPF_K: u16 = 0x00;
const BPF_RET: u16 = 0x06;

// --- seccomp constants (see <linux/seccomp.h>, <linux/audit.h>) -----------------------
/// `AUDIT_ARCH_X86_64`.
const AUDIT_ARCH_X86_64: u32 = 0xC000_003E;
/// `SECCOMP_RET_KILL_PROCESS`: terminate the whole process.
const SECCOMP_RET_KILL_PROCESS: u32 = 0x8000_0000;
/// `SECCOMP_RET_ALLOW`: no restriction on this syscall.
const SECCOMP_RET_ALLOW: u32 = 0x7FFF_0000;
/// `offsetof(struct seccomp_data, nr)` — the syscall number field.
const OFFSET_NR: u32 = 0;
/// `offsetof(struct seccomp_data, arch)` — the audit arch field.
const OFFSET_ARCH: u32 = 4;

/// Encode one `struct sock_filter { __u16 code; __u8 jt; __u8 jf; __u32 k; }` in native byte
/// order and endianness, matching the kernel's expectation for a `--seccomp` program.
fn sock_filter(code: u16, jt: u8, jf: u8, k: u32) -> [u8; 8] {
    // Sanity: the struct must be exactly 8 bytes with no padding surprises.
    debug_assert_eq!(size_of::<u16>() + 2 * size_of::<u8>() + size_of::<u32>(), 8);
    let mut out = [0u8; 8];
    out[0..2].copy_from_slice(&code.to_ne_bytes());
    out[2] = jt;
    out[3] = jf;
    out[4..8].copy_from_slice(&k.to_ne_bytes());
    out
}

/// Assemble the raw cBPF program that kills the process on any [`BLOCKED_SYSCALLS`] entry
/// (and on a non-x86-64 arch) and allows everything else.
///
/// Returns the concatenated `sock_filter` array as bytes, ready to write to the fd passed to
/// `bwrap --seccomp`. Deterministic for a given build (asserted in tests).
pub fn build_filter() -> Vec<u8> {
    let mut prog: Vec<[u8; 8]> = Vec::with_capacity(4 + 2 * BLOCKED_SYSCALLS.len() + 1);

    // 1. Load arch; if it is x86-64 skip the kill (jt=1), else fall through to kill (jf=0).
    prog.push(sock_filter(BPF_LD | BPF_W | BPF_ABS, 0, 0, OFFSET_ARCH));
    prog.push(sock_filter(
        BPF_JMP | BPF_JEQ | BPF_K,
        1,
        0,
        AUDIT_ARCH_X86_64,
    ));
    // 2. Wrong arch -> kill.
    prog.push(sock_filter(BPF_RET | BPF_K, 0, 0, SECCOMP_RET_KILL_PROCESS));
    // 3. Load the syscall number.
    prog.push(sock_filter(BPF_LD | BPF_W | BPF_ABS, 0, 0, OFFSET_NR));

    // 4. One jeq+kill pair per blocked syscall. On a match (jt=0) the next instruction — the
    //    kill — executes; otherwise (jf=1) it is skipped and evaluation continues.
    for nr in BLOCKED_SYSCALLS.iter() {
        prog.push(sock_filter(BPF_JMP | BPF_JEQ | BPF_K, 0, 1, *nr));
        prog.push(sock_filter(BPF_RET | BPF_K, 0, 0, SECCOMP_RET_KILL_PROCESS));
    }

    // 5. Default: allow.
    prog.push(sock_filter(BPF_RET | BPF_K, 0, 0, SECCOMP_RET_ALLOW));

    prog.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_tables_are_aligned() {
        assert_eq!(BLOCKED_SYSCALLS.len(), BLOCKED_SYSCALL_NAMES.len());
        // The named story syscalls are present exactly once each.
        for nr in [
            SYS_PTRACE,
            SYS_MOUNT,
            SYS_UMOUNT2,
            SYS_KEYCTL,
            SYS_BPF,
            SYS_PERF_EVENT_OPEN,
            SYS_UNSHARE,
        ] {
            assert_eq!(
                BLOCKED_SYSCALLS.iter().filter(|&&n| n == nr).count(),
                1,
                "syscall {nr} listed once"
            );
        }
    }

    #[test]
    fn filter_is_well_formed() {
        let f = build_filter();
        assert_eq!(f.len() % 8, 0, "whole sock_filter instructions");
        assert_eq!(f.len() / 8, 4 + 2 * BLOCKED_SYSCALLS.len() + 1);
        // The very first instruction is `ld [arch]` (k = OFFSET_ARCH = 4).
        let k0 = u32::from_ne_bytes([f[4], f[5], f[6], f[7]]);
        assert_eq!(k0, OFFSET_ARCH);
        // The last instruction is a `ret ALLOW`.
        let last = &f[f.len() - 8..];
        let code = u16::from_ne_bytes([last[0], last[1]]);
        let k = u32::from_ne_bytes([last[4], last[5], last[6], last[7]]);
        assert_eq!(code, BPF_RET | BPF_K);
        assert_eq!(k, SECCOMP_RET_ALLOW);
    }

    #[test]
    fn filter_is_deterministic() {
        assert_eq!(build_filter(), build_filter());
    }
}
