# The Crucible (Sandbox)

"The Crucible" is the isolation boundary in `evoswarm-sandbox` where untrusted candidate code is built and tested. It exists so a candidate can never reach the network, the host filesystem outside its workdir, or the test harness that judges it.

## The contract: `SandboxBackend`

Defined in `lib.rs`, every candidate run follows one lifecycle trait. The engine depends only on this trait, so adding a language stack or swapping the isolation mechanism never touches engine, fitness, or CLI code.

```rust
#[async_trait]
pub trait SandboxBackend: Send + Sync {
    async fn prepare(&self, profile: &SandboxProfile, candidate_patch: &[u8]) -> Result<PathBuf, SandboxError>;
    async fn run(&self, workdir: &Path, test_command: &str) -> Result<ExecutionResult, SandboxError>;
    async fn collect(&self, workdir: PathBuf) -> Result<(), SandboxError>;
}
```

- **`prepare`** — create an ephemeral workdir (tmpfs) and apply the candidate patch.
- **`run`** — execute the test command inside the boundary and return a classified `ExecutionResult`.
- **`collect`** — gather logs and tear the workdir down.

A key design rule: `SandboxError` describes a failure **of the isolation layer itself** (`Prepare`, `Execution`, `Cleanup`, `Unavailable`). A candidate that builds but fails its tests is *not* a `SandboxError` — it comes back as data in `ExecutionResult.status` (`success` / `failed` / `timeout` / `oom` / `tamper`). This keeps "the sandbox broke" distinct from "the patch is wrong."

## `BwrapBackend` — bubblewrap implementation

`bwrap.rs` enforces the isolation invariants:

- **No network** — `--unshare-all` tears down all namespaces including the network.
- **Read-only mounts** — toolchains and, critically, test directories are mounted `--ro-bind` so a candidate cannot rewrite the tests judging it.
- **Ephemeral workdirs** — each run gets a fresh tmpfs directory, removed in `collect`.
- **Wall-clock timeouts** — enforced via Tokio + SIGKILL.
- **cgroups v2** — resource limits (memory/PID/CPU) are applied via host cgroups (`systemd-run --user`).

## seccomp filter

`seccomp.rs` (story `e2-7`, **done**) adds a syscall filter layer on top of the namespace isolation, blocking syscalls a candidate should never need. Red-team tests assert the denied syscalls.

## Tamper detection

`tamper.rs` guards the harness:

- `detect_harness_override` — flags attempts to override conftest / props files.
- `hash_tests_dir` — a fingerprint of the test tree, so silent test edits are detectable.
- `PROHIBITED_HARNESS_FILES` — the denylist (`conftest.py`, `.props`, build scripts).

Feeds the Gate-3 tamper check in fitness (see [Fitness & Scoring](Fitness-and-Scoring.md)).

## Language stacks (`stacks/`)

| Stack | Module | Status |
|---|---|---|
| Python | `python.rs` (venv cache, JUnit XML parsing: `parse_junit_xml`, `parse_junit_outcomes`, `JunitSummary`, `lockfile_hash`) | `e0-5` **done** |
| Java | `java.rs` (pom-hash keying, cache reuse, offline `mvn -o` command) | `e5-3` review (SEAM) — live `mvn -o test` needs a primed `~/.m2` |
| C / C++ | `cpp.rs` (cmake command, gtest/catch2 result parsers) | `e5-4` review (SEAM) — live build needs gtest/catch2 libs |
| C# | (planned `e0-6`) | backlog — needs the `dotnet` SDK |

## Calibration & red-team

- `calibration.rs` — pure stats/profile math for sizing resource limits. `e0-9` is review (SEAM): the math is verified, but the live 100-run measurement needs the C#/dotnet toolchain that isn't present.
- `red_team.rs` (~417 lines) — an escape/containment suite. `e0-8` is review (SEAM): the bwrap escape-denial subset (egress, DNS, writes, pid-1, and since `e2-7` the seccomp syscalls) is asserted; shadow-read and cgroup-escape cases remain deferred because they need cgroup delegation the sandbox host doesn't expose.

## Verification status

Implemented and green: runner interface (`e0-1`), Python stack (`e0-5`), tamper-proof tests (`e0-7`), seccomp (`e2-7`). Verified-against-seams (SEAM, at `review`): red-team subset (`e0-8`), calibration math (`e0-9`), Java (`e5-3`), C/C++ (`e5-4`). See [Roadmap & Status](Roadmap-and-Status.md) for why each sits at `review` rather than `done`.
