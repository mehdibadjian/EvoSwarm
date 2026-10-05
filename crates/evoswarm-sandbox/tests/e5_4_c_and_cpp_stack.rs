//! e5-4 C/C++ stack — acceptance criteria (story §4, AD-1).
//!
//! Red-phase tests authored before the production `stacks::cpp` module exists. The story
//! suggests `tests/stacks/test_cpp_stack.rs`; the verification gate is
//! `cargo test --test e5_4_c_and_cpp_stack`, so this is a flat target in the sandbox crate.
//!
//! SEAM (roadmap §5): gcc/g++/cmake/ctest are present, but GoogleTest/Catch2 headers and
//! prebuilt libraries are NOT installed, and the 1 GB memory cap needs writable cgroup v2
//! delegation from e0-4 (absent). So the *pure* logic is certified here — the offline CMake
//! configure+build+ctest command builder, the tmpfs build-dir derivation, the GoogleTest and
//! Catch2 result parsers, and the profile shape — while the live configure/build/ctest run
//! against prebuilt test libraries and the hard memory cap are the deferred half.

use evoswarm_sandbox::stacks::cpp::{
    build_in_tmpfs_command, cpp_profile, parse_catch2_output, parse_gtest_output, GtestSummary,
};

/// AC1: the candidate is configured and built in an ephemeral tmpfs build dir against read-only
/// prebuilt test libraries, offline (no network), then run via ctest.
#[test]
fn test_cmake_tmpfs_build_command() {
    let cmd = build_in_tmpfs_command("/work", "/dev/shm/build", "/libs");
    // Configure step points CMake at the source and an ephemeral tmpfs build dir.
    assert!(cmd.contains("cmake"), "invokes cmake: {cmd}");
    assert!(
        cmd.contains("-S /work"),
        "source dir is the candidate workdir: {cmd}"
    );
    assert!(
        cmd.contains("-B /dev/shm/build"),
        "build dir is tmpfs: {cmd}"
    );
    // Offline: prebuilt test libs are supplied, no fetching from the network.
    assert!(
        cmd.contains("-DCMAKE_PREFIX_PATH=/libs"),
        "uses prebuilt libs: {cmd}"
    );
    assert!(
        cmd.contains("-DFETCHCONTENT_FULLY_DISCONNECTED=ON"),
        "network fetch disabled: {cmd}"
    );
    // Build + test.
    assert!(
        cmd.contains("--build /dev/shm/build"),
        "builds in tmpfs: {cmd}"
    );
    assert!(cmd.contains("ctest"), "runs ctest: {cmd}");
}

/// AC2: GoogleTest console output is parsed into pass/fail counts.
#[test]
fn test_gtest_parsing() {
    let out = "\
[==========] Running 3 tests from 1 test suite.
[----------] 3 tests from MathTest
[ RUN      ] MathTest.Adds
[       OK ] MathTest.Adds (0 ms)
[ RUN      ] MathTest.Subtracts
[  FAILED  ] MathTest.Subtracts (0 ms)
[ RUN      ] MathTest.Multiplies
[       OK ] MathTest.Multiplies (0 ms)
[==========] 3 tests from 1 test suite ran.
[  PASSED  ] 2 tests.
[  FAILED  ] 1 test, listed below:
";
    let s: GtestSummary = parse_gtest_output(out).expect("parse gtest");
    assert_eq!(s.passed, 2);
    assert_eq!(s.failed, 1);
    assert_eq!(s.total, 3);

    // All-pass run.
    let ok = "[==========] 2 tests ran.\n[  PASSED  ] 2 tests.\n";
    let s2 = parse_gtest_output(ok).expect("parse gtest ok");
    assert_eq!(s2.passed, 2);
    assert_eq!(s2.failed, 0);

    // A compile/link failure leaves no gtest summary; parse reports an error so it is captured
    // in feedback (contract matrix) rather than silently counted as zero tests.
    let compile_err = "error: 'foo' was not declared in this scope\n";
    assert!(parse_gtest_output(compile_err).is_err());
}

/// AC2: Catch2 output is parsed into pass/fail counts.
#[test]
fn test_catch2_parsing() {
    let out = "\
~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
tests is a Catch2 v3.4.0 host application.
-------------------------------------------------------------------------------
adds
-------------------------------------------------------------------------------
tests.cpp:5
...............................................................................

tests.cpp:7: PASSED:

===============================================================================
test cases: 3 | 2 passed | 1 failed
assertions: 5 | 4 passed | 1 failed
";
    let (cases_passed, cases_failed) = parse_catch2_output(out).expect("parse catch2");
    assert_eq!(cases_passed, 2);
    assert_eq!(cases_failed, 1);

    // No summary line (e.g. a link error) -> Err, captured in feedback.
    assert!(parse_catch2_output("undefined reference to `foo'\n").is_err());
}

/// AC3: the profile records the intended 1 GB cap and 60s timeout, and marks the prebuilt test
/// libraries read-only.
#[test]
fn test_cpp_profile_shape() {
    let p = cpp_profile();
    assert_eq!(p.stack, "cpp");
    assert_eq!(p.wall_timeout_secs, 60, "60s timeout (story §2)");
    assert_eq!(
        p.memory_limit_bytes,
        1024 * 1024 * 1024,
        "1 GB cap (story §2)"
    );
    assert!(
        !p.read_only_mounts.is_empty(),
        "prebuilt test libs mounted read-only"
    );
}
