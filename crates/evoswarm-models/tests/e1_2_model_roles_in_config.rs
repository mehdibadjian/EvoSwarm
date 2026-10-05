//! Red-phase tests for e1-2 (Model roles in config).
//!
//! SIGHUP is exercised through an injectable channel rather than a real OS
//! signal: production wires `tokio::signal::unix::hangup` into the same
//! channel, and tests drive it deterministically. Sending real signals and
//! polling for an eventual swap is timing-sensitive and flaky, which
//! .agents/rules/lessons-learned.md forbids.

use std::fs;
use std::path::{Path, PathBuf};

use evoswarm_models::config::{load, Role};
use evoswarm_models::error::ConfigError;
use evoswarm_models::hot_reload::{serve_reload_signals, ConfigHandle};
use tokio::sync::mpsc;

fn write_toml(body: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "e1_2_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("config.toml");
    fs::write(&path, body).unwrap();
    path
}

fn valid_config() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/valid.toml"))
        .unwrap()
}

// AC1: each dispatch uses its configured role settings.
#[test]
fn test_parse_valid_role_configuration() {
    let path = write_toml(&valid_config());
    let cfg = load(&path).expect("valid config must load");

    let mutator = cfg.role(Role::Mutator);
    assert_eq!(mutator.model_id, "claude-3-5-haiku-20241022");
    assert_eq!(mutator.max_tokens, 4096);
    assert!((mutator.temperature - 0.4).abs() < f64::EPSILON);

    let synth = cfg.role(Role::Synthesiser);
    assert_eq!(synth.max_tokens, 8192);
    assert!((synth.temperature - 0.2).abs() < f64::EPSILON);

    let adversary = cfg.role(Role::Adversary);
    assert!((adversary.temperature - 0.7).abs() < f64::EPSILON);

    // Every mandatory role is addressable for dispatch.
    for role in [Role::Mutator, Role::Synthesiser, Role::Adversary] {
        assert!(cfg.role(role).max_tokens > 0, "{role} must be configured");
    }
}

// AC2: a mandatory role omitted, or its model_id empty, fails loudly naming the role.
#[test]
fn test_startup_failure_on_invalid_role() {
    let missing = valid_config().replace(
        r#"[models.roles.adversary]
provider = "anthropic"
model_id = "claude-3-5-sonnet-20241022"
max_tokens = 4096
temperature = 0.7
cost_per_million_input = 3.00
cost_per_million_output = 15.00"#,
        "",
    );
    let path = write_toml(&missing);
    match load(&path) {
        Err(ConfigError::MissingRole { role }) => assert_eq!(role, Role::Adversary),
        other => panic!("expected MissingRole(adversary), got {other:?}"),
    }

    let empty_id = valid_config().replace(
        r#"model_id = "claude-3-5-haiku-20241022""#,
        r#"model_id = """#,
    );
    let path = write_toml(&empty_id);
    match load(&path) {
        Err(ConfigError::InvalidField { role, field, .. }) => {
            assert_eq!(role, Role::Mutator);
            assert_eq!(field, "model_id");
        }
        other => panic!("expected InvalidField(mutator, model_id), got {other:?}"),
    }
}

// AC3: SIGHUP with a valid edit swaps the active config without a restart.
#[tokio::test]
async fn test_sighup_atomic_swap() {
    let path = write_toml(&valid_config());
    let handle = ConfigHandle::load(&path).unwrap();
    let before = handle.current();
    assert!((before.role(Role::Mutator).temperature - 0.4).abs() < f64::EPSILON);

    fs::write(
        &path,
        valid_config().replace("temperature = 0.4", "temperature = 0.9"),
    )
    .unwrap();

    let (tx, rx) = mpsc::channel::<()>(1);
    let server = tokio::spawn(serve_reload_signals(handle.clone(), path.clone(), rx));
    tx.send(()).await.unwrap();
    drop(tx);
    server.await.unwrap();

    let after = handle.current();
    assert!((after.role(Role::Mutator).temperature - 0.9).abs() < f64::EPSILON);
    assert!(
        !handle.same_instance(&before),
        "a successful reload must swap to a new config instance"
    );
}

// AC4: SIGHUP with malformed TOML keeps the previous config and stays healthy.
#[tokio::test]
async fn test_sighup_rejects_invalid_config() {
    let path = write_toml(&valid_config());
    let handle = ConfigHandle::load(&path).unwrap();
    let before = handle.current();

    fs::write(&path, "this is = not = valid toml [[[").unwrap();

    let (tx, rx) = mpsc::channel::<()>(1);
    let server = tokio::spawn(serve_reload_signals(handle.clone(), path.clone(), rx));
    tx.send(()).await.unwrap();
    drop(tx);
    server.await.unwrap();

    let after = handle.current();
    assert_eq!(
        after.role(Role::Mutator).model_id,
        before.role(Role::Mutator).model_id,
        "malformed reload must not change the active config"
    );
    assert!(
        handle.same_instance(&before),
        "a failed reload must leave the same config instance active"
    );
}
