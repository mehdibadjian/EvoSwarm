//! e0-9 limit calibration — acceptance criteria (story §4).
//!
//! Red-phase tests authored before the production `calibration` module exists. The story
//! suggests `tests/test_calibration.py`; the verification gate is
//! `cargo test --test e0_9_limit_calibration`, so the pure statistics + profile-generation
//! logic is implemented in Rust and tested here.
//!
//! SEAM: the *live* half — actually running 100 baseline builds per stack on the host — is
//! verified against injected [`RunSample`]s. Python is present here but csharp/dotnet is not,
//! and 100 real builds are not run in CI, so the measurement harness stays behind the sample
//! seam; the statistics and profile math (all three ACs' computations) are fully certified.

use evoswarm_core::SandboxProfile;
use evoswarm_sandbox::calibration::{
    calibrate, percentile_rank, recommend_workers, CalibrationStats, RunSample,
};

fn profile_template() -> SandboxProfile {
    SandboxProfile {
        stack: "python".into(),
        wall_timeout_secs: 0,
        memory_limit_bytes: 0,
        tmpfs_size_bytes: 0,
        tasks_max: 0,
        read_only_mounts: Vec::new(),
        dependency_cache_path: "/tmp/dep".into(),
    }
}

fn samples(wall: &[f64], mem: &[u64]) -> Vec<RunSample> {
    assert_eq!(wall.len(), mem.len());
    wall.iter()
        .zip(mem.iter())
        .map(|(&w, &m)| RunSample {
            wall_secs: w,
            peak_memory_bytes: m,
        })
        .collect()
}

/// AC1: p50/p95 wall time and peak memory are computed from the run samples.
#[test]
fn test_calibration_statistics() {
    // 100 samples, wall = 1..=100 secs, mem = 1..=100 MB.
    let wall: Vec<f64> = (1..=100).map(|i| i as f64).collect();
    let mem: Vec<u64> = (1..=100).map(|i| i as u64 * 1_000_000).collect();
    let stats = calibrate(&samples(&wall, &mem)).expect("non-empty samples");

    assert_eq!(stats.samples, 100);
    // Nearest-rank percentiles: p50 = 50th value, p95 = 95th value.
    assert!((stats.p50_wall_secs - 50.0).abs() < 1e-9);
    assert!((stats.p95_wall_secs - 95.0).abs() < 1e-9);
    assert_eq!(stats.p95_memory_bytes, 95_000_000);
    // Peak memory is the max observed, not a percentile.
    assert_eq!(stats.peak_memory_bytes, 100_000_000);

    // Unsorted input yields the same percentiles (calibrate sorts internally).
    let mut shuffled = samples(&wall, &mem);
    shuffled.swap(0, 99);
    shuffled.swap(10, 50);
    let s2 = calibrate(&shuffled).expect("non-empty");
    assert!((s2.p95_wall_secs - 95.0).abs() < 1e-9);
    assert_eq!(s2.peak_memory_bytes, 100_000_000);
}

/// percentile_rank is exposed and uses the nearest-rank (ceiling) definition.
#[test]
fn test_percentile_rank_definition() {
    let v = vec![1.0, 2.0, 3.0, 4.0];
    assert_eq!(percentile_rank(&v, 50.0), 2); // ceil(0.5*4)=2
    assert_eq!(percentile_rank(&v, 95.0), 4); // ceil(0.95*4)=4
    assert_eq!(percentile_rank(&v, 0.0), 1); // clamped to first rank
    assert_eq!(percentile_rank(&v, 100.0), 4);
}

/// AC2: the generated profile applies wall = p95 × 2 and memory = peak × 1.5.
#[test]
fn test_profile_generation() {
    let stats = CalibrationStats {
        samples: 100,
        p50_wall_secs: 50.0,
        p95_wall_secs: 95.0,
        p50_memory_bytes: 50_000_000,
        p95_memory_bytes: 95_000_000,
        peak_memory_bytes: 100_000_000,
    };
    let profile = stats.profile(&profile_template());

    assert_eq!(profile.stack, "python", "template stack preserved");
    // wall = ceil(95 * 2) = 190 secs.
    assert_eq!(profile.wall_timeout_secs, 190);
    // mem = ceil(100_000_000 * 1.5) = 150_000_000 bytes.
    assert_eq!(profile.memory_limit_bytes, 150_000_000);
}

/// AC3: the worker recommendation fits within available RAM using the calibrated per-worker
/// memory limit, and is at least 1.
#[test]
fn test_worker_recommendation() {
    // 1 GB available, 150 MB per worker -> floor(1000/150) = 6.
    assert_eq!(recommend_workers(1_000_000_000, 150_000_000), 6);
    // Too little RAM for even one worker still recommends 1 (never 0).
    assert_eq!(recommend_workers(100_000_000, 150_000_000), 1);
    // Zero per-worker memory is degenerate; guard against divide-by-zero.
    assert_eq!(recommend_workers(1_000_000_000, 0), 1);
}
