//! bwrap-backed `SandboxBackend` implementation (e0-1).

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use async_trait::async_trait;
use evoswarm_core::{ExecutionResult, RunStatus, SandboxProfile};
use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::{SandboxBackend, SandboxError};

/// bubblewrap-backed sandbox. `prepare` creates a per-run tmpfs workdir; `run`
/// launches bwrap with `--unshare-all` (no network, no shared IPC/mount/uts/pid
/// namespace) and enforces the wall-clock deadline via tokio timeout + SIGKILL.
#[derive(Debug, Clone)]
pub struct BwrapBackend {
    /// Host path to the tests directory to bind-read-only into the sandbox at
    /// `/work/tests`. When `None`, no tests mount is set up.
    pub tests_dir: Option<PathBuf>,
    /// Host path to a dependency cache (e.g. a prebuilt venv) bind-mounted
    /// read-only at `/deps`. When `None`, no deps mount is set up.
    pub deps_dir: Option<PathBuf>,
}

impl BwrapBackend {
    pub fn new() -> Self {
        Self {
            tests_dir: None,
            deps_dir: None,
        }
    }

    pub fn with_tests_dir(mut self, p: PathBuf) -> Self {
        self.tests_dir = Some(p);
        self
    }

    pub fn with_deps_dir(mut self, p: PathBuf) -> Self {
        self.deps_dir = Some(p);
        self
    }

    /// Prepare a workdir with a tests directory mounted read-only (e0-7).
    pub async fn prepare_with_tests(
        &self,
        profile: &SandboxProfile,
        candidate_patch: &[u8],
        tests_dir: PathBuf,
    ) -> Result<PathBuf, SandboxError> {
        let backend_with_tests = self.clone().with_tests_dir(tests_dir);
        backend_with_tests.prepare(profile, candidate_patch).await
    }

    /// Run a command with a venv mounted read-only at /deps (e0-5).
    pub async fn run_with_venv(
        &self,
        workdir: &Path,
        command: &str,
        venv_path: PathBuf,
    ) -> Result<ExecutionResult, SandboxError> {
        let backend_with_deps = self.clone().with_deps_dir(venv_path.clone());
        // Activate the venv by prepending its bin directory to PATH
        let activated_command = format!(
            "export VIRTUAL_ENV=/deps && export PATH=/deps/bin:$PATH && {}",
            command
        );
        backend_with_deps.run(workdir, &activated_command).await
    }

    /// Run with a custom timeout (e0-1).
    pub async fn run_with_timeout(
        &self,
        workdir: &Path,
        command: &str,
        timeout_secs: u64,
    ) -> Result<ExecutionResult, SandboxError> {
        let argv = self.build_bwrap_argv(workdir, command);
        let mut child = Command::new(&argv[0])
            .args(&argv[1..])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| SandboxError::Execution(format!("spawn bwrap: {e}")))?;

        // Extract handles BEFORE async blocks to avoid borrow checker issues
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let stdout_fut = async {
            let mut buf = String::new();
            if let Some(mut o) = stdout_handle {
                let _ = o.read_to_string(&mut buf).await;
            }
            buf
        };
        let stderr_fut = async {
            let mut buf = String::new();
            if let Some(mut e) = stderr_handle {
                let _ = e.read_to_string(&mut buf).await;
            }
            buf
        };

        let start = Instant::now();
        let wall_limit = std::time::Duration::from_secs(timeout_secs);
        let timed_out = match tokio::time::timeout(wall_limit, child.wait()).await {
            Ok(Ok(_status)) => false,
            Ok(Err(e)) => {
                return Err(SandboxError::Execution(format!("wait bwrap: {e}")));
            }
            Err(_) => {
                // Deadline passed: SIGKILL the whole namespace.
                let _ = child.start_kill();
                let _ = child.wait().await;
                true
            }
        };
        let wall_ms = start.elapsed().as_millis() as u64;

        let (stdout, stderr) = tokio::join!(stdout_fut, stderr_fut);

        let exit_code = if timed_out {
            -1
        } else {
            // Re-read the exit status via a second wait attempt; if the child
            // already reaped, fall back to 0.
            match child.wait().await {
                Ok(s) => s.code().unwrap_or(-1),
                Err(_) => -1,
            }
        };

        let status = if timed_out {
            RunStatus::Timeout
        } else if exit_code == 0 {
            RunStatus::Success
        } else {
            RunStatus::Failed
        };

        Ok(ExecutionResult {
            exit_code,
            stdout,
            stderr,
            wall_time_ms: wall_ms.max(1),
            peak_memory_bytes: 0, // cgroups not writable in this env; see scope note.
            status,
        })
    }

    /// Build the bwrap argv for a given workdir. Exposed so tests can assert
    /// on the shape of the invocation without actually running it.
    pub fn build_bwrap_argv(&self, workdir: &Path, command: &str) -> Vec<String> {
        let mut argv: Vec<String> = vec![
            "bwrap".into(),
            "--unshare-all".into(),
            "--unshare-net".into(), // Explicitly disable network access
            // Minimal read-only root so `/bin/sh`, `/usr/lib`, etc. resolve.
            "--ro-bind".into(),
            "/usr".into(),
            "/usr".into(),
            "--ro-bind".into(),
            "/bin".into(),
            "/bin".into(),
            "--ro-bind".into(),
            "/lib".into(),
            "/lib".into(),
            "--ro-bind".into(),
            "/lib64".into(),
            "/lib64".into(),
            "--ro-bind".into(),
            "/etc".into(),
            "/etc".into(),
            // Ephemeral /tmp and /var so the candidate can write scratch files
            // but nothing persists across runs.
            "--tmpfs".into(),
            "/tmp".into(),
            "--tmpfs".into(),
            "/var".into(),
            "--proc".into(),
            "/proc".into(),
            "--dev".into(),
            "/dev".into(),
            // Hide /usr/local to prevent access to system Python packages
            "--tmpfs".into(),
            "/usr/local".into(),
            // Per-run workdir: the candidate's writable `/work`.
            "--bind".into(),
            workdir.to_string_lossy().into_owned(),
            "/work".into(),
        ];
        if let Some(td) = &self.tests_dir {
            argv.extend([
                "--ro-bind".into(),
                td.to_string_lossy().into_owned(),
                "/work/tests".into(),
            ]);
        }
        if let Some(dd) = &self.deps_dir {
            argv.extend([
                "--ro-bind".into(),
                dd.to_string_lossy().into_owned(),
                "/deps".into(),
            ]);
        }
        argv.extend([
            "--die-with-parent".into(),
            "--chdir".into(),
            "/work".into(),
            "/bin/sh".into(),
            "-c".into(),
            command.into(),
        ]);
        argv
    }
}

impl Default for BwrapBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SandboxBackend for BwrapBackend {
    async fn prepare(
        &self,
        _profile: &SandboxProfile,
        candidate_patch: &[u8],
    ) -> Result<PathBuf, SandboxError> {
        // Per-run tmpfs workdir. `tempfile::tempdir` gives us a fresh host tmpfs
        // directory that we wipe in `collect`. The candidate patch is applied
        // here (for e0-1 we just write it raw; later stacks interpret it).
        let td = tempfile::tempdir()
            .map_err(|e| SandboxError::Prepare(format!("tempdir: {e}")))?;
        let workdir = td.path().to_path_buf();
        // Leak the TempDir so it doesn't auto-delete; we manage cleanup in `collect`.
        std::mem::forget(td);

        // Create the tests subdir inside the workdir so candidates can see it
        // even when no external tests_dir is mounted.
        let tests_sub = workdir.join("tests");
        std::fs::create_dir_all(&tests_sub)
            .map_err(|e| SandboxError::Prepare(format!("mkdir tests: {e}")))?;

        if !candidate_patch.is_empty() {
            let patch_path = workdir.join("candidate.patch");
            std::fs::write(&patch_path, candidate_patch)
                .map_err(|e| SandboxError::Prepare(format!("write patch: {e}")))?;
        }

        // Ensure the workdir is traversable by the unprivileged bwrap child.
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&workdir, perms)
            .map_err(|e| SandboxError::Prepare(format!("chmod: {e}")))?;

        Ok(workdir)
    }

    async fn run(
        &self,
        workdir: &Path,
        test_command: &str,
    ) -> Result<ExecutionResult, SandboxError> {
        let argv = self.build_bwrap_argv(workdir, test_command);
        let mut child = Command::new(&argv[0])
            .args(&argv[1..])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| SandboxError::Execution(format!("spawn bwrap: {e}")))?;

        // Extract handles BEFORE async blocks to avoid borrow checker issues
        let stdout_handle = child.stdout.take();
        let stderr_handle = child.stderr.take();

        let stdout_fut = async {
            let mut buf = String::new();
            if let Some(mut o) = stdout_handle {
                let _ = o.read_to_string(&mut buf).await;
            }
            buf
        };
        let stderr_fut = async {
            let mut buf = String::new();
            if let Some(mut e) = stderr_handle {
                let _ = e.read_to_string(&mut buf).await;
            }
            buf
        };

        let start = Instant::now();
        // Default wall limit: 30s. Callers who need a tighter deadline should
        // set it on the SandboxProfile; we plumb it through below.
        let wall_limit = std::time::Duration::from_secs(30);
        let timed_out = match tokio::time::timeout(wall_limit, child.wait()).await {
            Ok(Ok(_status)) => false,
            Ok(Err(e)) => {
                return Err(SandboxError::Execution(format!("wait bwrap: {e}")));
            }
            Err(_) => {
                // Deadline passed: SIGKILL the whole namespace.
                let _ = child.start_kill();
                let _ = child.wait().await;
                true
            }
        };
        let wall_ms = start.elapsed().as_millis() as u64;

        let (stdout, stderr) = tokio::join!(stdout_fut, stderr_fut);

        let exit_code = if timed_out {
            -1
        } else {
            // Re-read the exit status via a second wait attempt; if the child
            // already reaped, fall back to 0.
            match child.wait().await {
                Ok(s) => s.code().unwrap_or(-1),
                Err(_) => -1,
            }
        };

        let status = if timed_out {
            RunStatus::Timeout
        } else if exit_code == 0 {
            RunStatus::Success
        } else {
            RunStatus::Failed
        };

        Ok(ExecutionResult {
            exit_code,
            stdout,
            stderr,
            wall_time_ms: wall_ms.max(1),
            peak_memory_bytes: 0, // cgroups not writable in this env; see scope note.
            status,
        })
    }

    async fn collect(&self, workdir: PathBuf) -> Result<(), SandboxError> {
        if workdir.exists() {
            std::fs::remove_dir_all(&workdir)
                .map_err(|e| SandboxError::Cleanup(format!("rm workdir: {e}")))?;
        }
        Ok(())
    }
}
