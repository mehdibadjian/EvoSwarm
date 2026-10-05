//! Limit calibration (e0-9): measure limits on the operator's own box so timeouts fit their
//! hardware instead of guesses.
//!
//! The operator runs `evoswarm calibrate --stack <python|csharp>`, which measures a batch of
//! baseline builds and derives a tailored [`SandboxProfile`]: wall-clock limit = p95 × 2,
//! memory limit = peak × 1.5 (story §2/§4), plus a worker-concurrency recommendation from
//! available RAM.
//!
//! This module holds the **pure** half — the percentile statistics, the safety-factor profile
//! generation, and the worker recommendation — which is fully deterministic and certifiable.
//! The **live** half (actually executing 100 baseline builds per stack on the host) is a SEAM:
//! it feeds [`RunSample`]s in from a measurement harness that runs the real stack under bwrap.
//! Python is available in this environment but csharp/dotnet is not, and CI does not run 100
//! real builds, so callers inject samples rather than this module spawning them.

use evoswarm_core::SandboxProfile;
use thiserror::Error;

/// Safety factor applied to p95 wall time to produce the wall-clock limit.
pub const WALL_SAFETY_FACTOR: f64 = 2.0;

/// Safety factor applied to peak memory to produce the memory limit.
pub const MEMORY_SAFETY_FACTOR: f64 = 1.5;

/// One baseline-run measurement.
#[derive(Debug, Clone, PartialEq)]
pub struct RunSample {
    /// Wall-clock duration of the run, in seconds.
    pub wall_secs: f64,
    /// Peak resident memory observed during the run, in bytes.
    pub peak_memory_bytes: u64,
}

/// Why calibration could not produce statistics.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CalibrationError {
    /// No run samples were provided; percentiles are undefined over an empty set.
    #[error("calibration needs at least one run sample")]
    NoSamples,
}

/// Percentile statistics derived from a batch of baseline runs.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrationStats {
    /// Number of samples the statistics were computed from.
    pub samples: usize,
    /// Median (p50) wall-clock seconds.
    pub p50_wall_secs: f64,
    /// 95th-percentile wall-clock seconds — the basis for the wall limit.
    pub p95_wall_secs: f64,
    /// Median (p50) peak memory bytes.
    pub p50_memory_bytes: u64,
    /// 95th-percentile peak memory bytes.
    pub p95_memory_bytes: u64,
    /// Maximum observed peak memory bytes — the basis for the memory limit.
    pub peak_memory_bytes: u64,
}

impl CalibrationStats {
    /// Builds a [`SandboxProfile`] by cloning `template` and overwriting its wall-clock and
    /// memory limits with the calibrated values (wall = p95 × 2, memory = peak × 1.5, both
    /// rounded up so a limit never under-covers an observed run).
    pub fn profile(&self, template: &SandboxProfile) -> SandboxProfile {
        let mut profile = template.clone();
        profile.wall_timeout_secs = ceil_u64(self.p95_wall_secs * WALL_SAFETY_FACTOR);
        profile.memory_limit_bytes = ceil_u64(self.peak_memory_bytes as f64 * MEMORY_SAFETY_FACTOR);
        profile
    }
}

/// Computes percentile statistics over `samples`. Returns [`CalibrationError::NoSamples`] when
/// the batch is empty.
pub fn calibrate(samples: &[RunSample]) -> Result<CalibrationStats, CalibrationError> {
    if samples.is_empty() {
        return Err(CalibrationError::NoSamples);
    }

    let mut wall: Vec<f64> = samples.iter().map(|s| s.wall_secs).collect();
    let mut mem: Vec<u64> = samples.iter().map(|s| s.peak_memory_bytes).collect();
    wall.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    mem.sort_unstable();

    let peak_memory_bytes = *mem.last().expect("non-empty");

    Ok(CalibrationStats {
        samples: samples.len(),
        p50_wall_secs: value_at_rank(&wall, percentile_rank(&wall, 50.0)),
        p95_wall_secs: value_at_rank(&wall, percentile_rank(&wall, 95.0)),
        p50_memory_bytes: value_at_rank(&mem, percentile_rank(&mem, 50.0)),
        p95_memory_bytes: value_at_rank(&mem, percentile_rank(&mem, 95.0)),
        peak_memory_bytes,
    })
}

/// Nearest-rank percentile index (1-based) into a **sorted** slice: `ceil(percent/100 * n)`,
/// clamped to `[1, n]`. This is the standard nearest-rank definition — the smallest rank whose
/// cumulative proportion is `>= percent` — so p50 of `1..=100` is `50` and p95 is `95`.
pub fn percentile_rank<T>(sorted: &[T], percent: f64) -> usize {
    let n = sorted.len();
    if n == 0 {
        return 0;
    }
    let raw = (percent / 100.0 * n as f64).ceil() as usize;
    raw.clamp(1, n)
}

/// The value at a 1-based `rank` in a slice (0-based index `rank - 1`). Panics only if the rank
/// is out of bounds, which `percentile_rank` never returns for a non-empty slice.
fn value_at_rank<T: Copy>(sorted: &[T], rank: usize) -> T {
    sorted[rank - 1]
}

/// Recommends a worker concurrency that fits `available_bytes` at `per_worker_bytes` each,
/// rounded down but never below 1 (a box too small for even one full worker still runs one,
/// relying on the memory cap to prevent runaway). A zero per-worker size is treated as
/// degenerate and yields 1 to avoid a divide-by-zero.
pub fn recommend_workers(available_bytes: u64, per_worker_bytes: u64) -> usize {
    if per_worker_bytes == 0 {
        return 1;
    }
    ((available_bytes / per_worker_bytes) as usize).max(1)
}

/// Rounds `value` up to the nearest `u64`, saturating at `u64::MAX`. Negative inputs clamp to 0.
fn ceil_u64(value: f64) -> u64 {
    if value.is_nan() || value <= 0.0 {
        return 0;
    }
    value.ceil() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_samples_error() {
        assert_eq!(calibrate(&[]), Err(CalibrationError::NoSamples));
    }

    #[test]
    fn single_sample_percentiles_equal_that_sample() {
        let stats = calibrate(&[RunSample {
            wall_secs: 3.5,
            peak_memory_bytes: 42,
        }])
        .expect("one sample");
        assert_eq!(stats.p50_wall_secs, 3.5);
        assert_eq!(stats.p95_wall_secs, 3.5);
        assert_eq!(stats.peak_memory_bytes, 42);
        assert_eq!(stats.p95_memory_bytes, 42);
    }

    #[test]
    fn ceil_u64_rounds_up_and_clamps() {
        assert_eq!(ceil_u64(2.1), 3);
        assert_eq!(ceil_u64(2.0), 2);
        assert_eq!(ceil_u64(-1.0), 0);
        assert_eq!(ceil_u64(f64::NAN), 0);
    }

    #[test]
    fn profile_preserves_other_template_fields() {
        let stats = CalibrationStats {
            samples: 1,
            p50_wall_secs: 1.0,
            p95_wall_secs: 10.0,
            p50_memory_bytes: 1,
            p95_memory_bytes: 1,
            peak_memory_bytes: 1000,
        };
        let template = SandboxProfile {
            stack: "csharp".into(),
            wall_timeout_secs: 0,
            memory_limit_bytes: 0,
            tmpfs_size_bytes: 123,
            tasks_max: 7,
            read_only_mounts: vec!["/ro".into()],
            dependency_cache_path: "/cache".into(),
        };
        let p = stats.profile(&template);
        assert_eq!(p.stack, "csharp");
        assert_eq!(p.tmpfs_size_bytes, 123);
        assert_eq!(p.tasks_max, 7);
        assert_eq!(p.read_only_mounts, vec![std::path::PathBuf::from("/ro")]);
        assert_eq!(p.wall_timeout_secs, 20); // 10 * 2
        assert_eq!(p.memory_limit_bytes, 1500); // 1000 * 1.5
    }
}
