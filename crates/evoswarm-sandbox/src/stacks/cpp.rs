//! C/C++ stack support (e5-4, AD-1).
//!
//! Configures CMake in an ephemeral tmpfs build directory against read-only prebuilt test
//! libraries (GoogleTest / Catch2), then builds and runs `ctest` inside bwrap with a 1 GB memory
//! limit and 60s timeout. Compile/link errors are surfaced through the result parsers (which
//! error when no test summary is present) so they reach the model as feedback rather than being
//! counted as "zero tests".
//!
//! ## Containment honesty (roadmap §5, SEAM)
//!
//! gcc/g++/cmake/ctest are present here, but two halves are **deferred**:
//! - GoogleTest/Catch2 headers and *prebuilt* test libraries are not installed, so the live
//!   configure+build+ctest run against them is not exercised in CI.
//! - The **1 GB memory cap** (story §2) needs writable cgroup v2 delegation from e0-4 (absent),
//!   so [`cpp_profile`] records the intended limit but cannot enforce it yet.
//!
//! What *is* certified here is the pure logic: the offline CMake command builder
//! ([`build_in_tmpfs_command`]), the GoogleTest and Catch2 output parsers
//! ([`parse_gtest_output`], [`parse_catch2_output`]), and the profile shape ([`cpp_profile`]).

use std::path::PathBuf;

/// Intended hard memory cap for a C/C++ run (story §2: 1 GB). Not enforceable until e0-4
/// provides cgroup v2 delegation; recorded in the profile regardless.
pub const CPP_MEMORY_LIMIT_BYTES: u64 = 1024 * 1024 * 1024;

/// Intended wall-clock timeout for a C/C++ run (story §2: 60s).
pub const CPP_WALL_TIMEOUT_SECS: u64 = 60;

/// Builds the offline CMake configure + build + ctest pipeline as a single shell command.
///
/// - `-S <source>` is the candidate workdir; `-B <build_dir>` is an ephemeral tmpfs directory so
///   build artifacts never persist or touch the host.
/// - `-DCMAKE_PREFIX_PATH=<libs>` supplies the read-only prebuilt GoogleTest/Catch2 libraries.
/// - `-DFETCHCONTENT_FULLY_DISCONNECTED=ON` forbids CMake from fetching anything over the
///   network (the sandbox is already network-isolated; this fails the configure loudly instead).
/// - `--build` compiles, then `ctest` runs the tests with `--output-on-failure` for diagnostics.
pub fn build_in_tmpfs_command(source: &str, build_dir: &str, libs: &str) -> String {
    format!(
        "cmake -S {source} -B {build_dir} \
         -DCMAKE_PREFIX_PATH={libs} \
         -DFETCHCONTENT_FULLY_DISCONNECTED=ON \
         && cmake --build {build_dir} \
         && ctest --test-dir {build_dir} --output-on-failure"
    )
}

/// Aggregate pass/fail counts parsed from GoogleTest console output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GtestSummary {
    pub passed: u32,
    pub failed: u32,
    pub total: u32,
}

/// Parses GoogleTest console output into pass/fail counts.
///
/// Reads the `[  PASSED  ] N tests.` and `[  FAILED  ] N test` summary lines. Returns `Err` when
/// no `[==========] ... ran.` summary is present — which is what a compile or link failure
/// produces — so the harness reports a build error into feedback instead of a silent zero.
pub fn parse_gtest_output(output: &str) -> Result<GtestSummary, String> {
    if !output.contains("ran.") && !output.contains("[==========]") {
        return Err("no GoogleTest summary found (compile/link failure?)".to_string());
    }

    let mut passed = 0u32;
    let mut failed = 0u32;
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(n) = summary_count(trimmed, "[  PASSED  ]") {
            passed = n;
        } else if let Some(n) = summary_count(trimmed, "[  FAILED  ]") {
            // gtest prints the failed count twice (once inline, once as "listed below:").
            // Take the first occurrence only.
            if failed == 0 {
                failed = n;
            }
        }
    }

    if passed == 0 && failed == 0 {
        return Err("GoogleTest summary had no pass/fail counts".to_string());
    }

    Ok(GtestSummary {
        passed,
        failed,
        total: passed + failed,
    })
}

/// Extracts the leading integer count from a gtest summary line beginning with `marker`, e.g.
/// `[  PASSED  ] 2 tests.` → `Some(2)`.
fn summary_count(line: &str, marker: &str) -> Option<u32> {
    let rest = line.strip_prefix(marker)?;
    let digits: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// Parses Catch2 output into `(test_cases_passed, test_cases_failed)`.
///
/// Reads the `test cases: N | P passed | F failed` summary line. Returns `Err` when that line is
/// absent (e.g. a link error), so a build failure surfaces as feedback rather than a silent zero.
pub fn parse_catch2_output(output: &str) -> Result<(u32, u32), String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("test cases:") {
            let passed = field_count(rest, "passed").unwrap_or(0);
            let failed = field_count(rest, "failed").unwrap_or(0);
            if passed == 0 && failed == 0 {
                return Err("Catch2 summary had no pass/fail counts".to_string());
            }
            return Ok((passed, failed));
        }
    }
    Err("no Catch2 'test cases:' summary found (compile/link failure?)".to_string())
}

/// From a Catch2 summary tail like ` 3 | 2 passed | 1 failed`, extracts the integer that
/// precedes `label`.
fn field_count(summary_tail: &str, label: &str) -> Option<u32> {
    // Split on '|', find the segment containing `label`, take the leading integer.
    for segment in summary_tail.split('|') {
        let seg = segment.trim();
        if seg.ends_with(label) {
            let digits: String = seg.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = digits.parse() {
                return Some(n);
            }
        }
    }
    None
}

/// Builds the C/C++ [`evoswarm_core::SandboxProfile`]: the intended 1 GB memory cap, 60s
/// timeout, and the read-only prebuilt-test-library mounts. The memory cap is recorded but not
/// enforceable until e0-4 supplies cgroup v2 delegation (SEAM).
pub fn cpp_profile() -> evoswarm_core::SandboxProfile {
    evoswarm_core::SandboxProfile {
        stack: "cpp".into(),
        wall_timeout_secs: CPP_WALL_TIMEOUT_SECS,
        memory_limit_bytes: CPP_MEMORY_LIMIT_BYTES,
        tmpfs_size_bytes: 512 * 1024 * 1024,
        tasks_max: 32,
        read_only_mounts: vec![PathBuf::from("/libs"), PathBuf::from("/usr/include")],
        dependency_cache_path: PathBuf::from("/libs"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gtest_parser_handles_zero_failures() {
        let s = parse_gtest_output("[==========] 5 tests ran.\n[  PASSED  ] 5 tests.\n").unwrap();
        assert_eq!(
            s,
            GtestSummary {
                passed: 5,
                failed: 0,
                total: 5
            }
        );
    }

    #[test]
    fn gtest_parser_errors_on_compile_output() {
        assert!(parse_gtest_output("cc1plus: error: unrecognized command line option").is_err());
    }

    #[test]
    fn summary_count_extracts_leading_int() {
        assert_eq!(
            summary_count("[  PASSED  ] 12 tests.", "[  PASSED  ]"),
            Some(12)
        );
        assert_eq!(summary_count("[  PASSED  ]", "[  FAILED  ]"), None);
    }

    #[test]
    fn catch2_field_count_reads_segment() {
        assert_eq!(field_count(" 10 | 7 passed | 3 failed", "passed"), Some(7));
        assert_eq!(field_count(" 10 | 7 passed | 3 failed", "failed"), Some(3));
        assert_eq!(field_count(" 10 | 7 passed", "skipped"), None);
    }

    #[test]
    fn catch2_parser_errors_without_summary() {
        assert!(parse_catch2_output("fatal error: catch2/catch_all.hpp: No such file").is_err());
    }

    #[test]
    fn cpp_profile_records_caps() {
        let p = cpp_profile();
        assert_eq!(p.stack, "cpp");
        assert_eq!(p.wall_timeout_secs, 60);
        assert_eq!(p.memory_limit_bytes, CPP_MEMORY_LIMIT_BYTES);
    }
}
