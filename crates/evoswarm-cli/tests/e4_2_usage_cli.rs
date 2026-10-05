//! e4-2: `evoswarm usage --since <date>` end-to-end through the real binary.
//!
//! The store/summary math is asserted in the gateway crate (`e4_2_usage_logging`); this test
//! drives the shipped CLI: seed a usage db, run the binary, and check the printed daily
//! summary and exit codes (story §5 `test_evoswarm_usage_cli`).

use std::process::Command;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_evoswarm")
}

#[test]
fn test_evoswarm_usage_cli() {
    use evoswarm_gateway::usage::{UsagePricing, UsageStore};

    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join(".evoswarm/usage.db");
    {
        let store = UsageStore::open(&db).expect("open store");
        // 2025-07-03T00:00:00Z + 3600s = inside that UTC day.
        store
            .log_with_timestamp("sess-1", 1_751_500_800 + 3_600, 1000, 500, 100)
            .expect("log");
        // 2025-07-02 (the day before): excluded by --since 2025-07-03.
        store
            .log_with_timestamp("sess-2", 1_751_500_800 - 86_400, 7, 3, 0)
            .expect("log");
    }

    let out = Command::new(bin())
        .args([
            "usage",
            "--since",
            "2025-07-03",
            "--db",
            db.to_str().expect("db path"),
        ])
        .output()
        .expect("run evoswarm usage");
    assert!(
        out.status.success(),
        "exit 0, stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("2025-07-03"), "day row printed: {stdout}");
    assert!(stdout.contains("1000"), "input tokens printed");
    assert!(stdout.contains("500"), "output tokens printed");
    assert!(stdout.contains("TOTAL"), "totals line printed");
    assert!(
        !stdout.contains("2025-07-02"),
        "--since filters out earlier days: {stdout}"
    );
    // Cost is deterministic from default pricing: 1000*3/1e6 + 500*15/1e6 + 100*0.3/1e6
    let expected = UsagePricing::default().cost(1000, 500, 100);
    assert!(
        stdout.contains(&format!("{expected:.4}")),
        "cost {expected:.4} printed: {stdout}"
    );
}

#[test]
fn test_usage_cli_rejects_bad_since() {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = Command::new(bin())
        .args([
            "usage",
            "--since",
            "not-a-date",
            "--db",
            dir.path().join("usage.db").to_str().expect("db path"),
        ])
        .output()
        .expect("run evoswarm usage");
    assert_eq!(
        out.status.code(),
        Some(2),
        "malformed --since exits with validation code 2"
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("YYYY-MM-DD"),
        "stderr explains the expected format"
    );
}

#[test]
fn test_usage_cli_empty_db_reports_no_usage() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("usage.db"); // created on open by the CLI
    let out = Command::new(bin())
        .args([
            "usage",
            "--since",
            "1970-01-01",
            "--db",
            db.to_str().expect("db path"),
        ])
        .output()
        .expect("run evoswarm usage");
    assert!(out.status.success(), "exit 0 on empty log");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("no usage recorded"),
        "empty report is explicit"
    );
}
