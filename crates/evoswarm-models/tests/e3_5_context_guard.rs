//! e3-5 context guard — acceptance criteria (story §4).
//!
//! Red-phase tests authored before the production `context_guard` module exists. The story
//! suggests `tests/security/test_context_guard.rs`, but Cargo only treats *flat*
//! `tests/<name>.rs` files as `--test` targets (a subdir path is a helper module, never a
//! target), so the target is named `e3_5_context_guard` to match the verification gate
//! `cargo test --test e3_5_context_guard`.

use std::collections::BTreeSet;

use evoswarm_models::context_guard::{
    sanitize, ContextFile, ContextGuardConfig, ExcludeReason, SecretKind, REDACTION_MARKER,
};

fn cfg(paths: &[&str]) -> ContextGuardConfig {
    ContextGuardConfig {
        allowlist: paths.iter().map(|p| p.to_string()).collect::<BTreeSet<String>>(),
    }
}

/// AC1: no file outside the per-job allowlist is included in the sanitized payload.
#[test]
fn test_path_allowlist_enforcement() {
    let raw = vec![
        ContextFile {
            path: "src/lib.rs".to_string(),
            content: "pub fn ok() {}".to_string(),
        },
        ContextFile {
            path: "secrets/internal.rs".to_string(),
            content: "pub fn leaked() {}".to_string(),
        },
    ];
    let out = sanitize(&raw, &cfg(&["src/lib.rs"]));

    let included: Vec<&str> = out.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(included, vec!["src/lib.rs"], "only allowed file survives");

    // The disallowed file is recorded as excluded with the allowlist reason.
    let excluded = out
        .excluded
        .iter()
        .find(|e| e.path == "secrets/internal.rs")
        .expect("disallowed file recorded as excluded");
    assert_eq!(excluded.reason, ExcludeReason::NotInAllowlist);
}

/// AC2: content matching key patterns is redacted and the redaction is logged.
#[test]
fn test_secret_redaction() {
    let aws = "AKIAIOSFODNN7EXAMPLE"; // 20-char AWS access key id
    let openai = format!("sk-{}", "A".repeat(32)); // OpenAI-style secret key
    let body = format!(
        "config aws_key={aws} and token={openai} done",
        aws = aws,
        openai = openai
    );
    let raw = vec![ContextFile {
        path: "src/config.rs".to_string(),
        content: body.clone(),
    }];
    let out = sanitize(&raw, &cfg(&["src/config.rs"]));

    let file = out.files.iter().find(|f| f.path == "src/config.rs").expect("file kept");
    // Raw secrets are gone; markers are in.
    assert!(!file.content.contains(aws), "aws key redacted");
    assert!(!file.content.contains(&openai), "openai key redacted");
    assert!(file.content.contains(REDACTION_MARKER), "marker present");
    // Non-secret text survives verbatim.
    assert!(file.content.contains("config aws_key="), "surrounding text intact");
    assert!(file.content.ends_with(" done"), "trailing text intact");

    // Both redactions are logged with kind and count.
    let aws_log = out
        .redactions
        .iter()
        .find(|r| r.path == "src/config.rs" && r.kind == SecretKind::AwsAccessKeyId)
        .expect("aws redaction logged");
    assert_eq!(aws_log.count, 1);
    let openai_log = out
        .redactions
        .iter()
        .find(|r| r.path == "src/config.rs" && r.kind == SecretKind::OpenAiKey)
        .expect("openai redaction logged");
    assert_eq!(openai_log.count, 1);
}

/// AC3: default settings exclude `.env` and `*.pem` — even when they are on the allowlist.
#[test]
fn test_env_pem_exclusion() {
    let raw = vec![
        ContextFile {
            path: ".env".to_string(),
            content: "DATABASE_URL=postgres://x".to_string(),
        },
        ContextFile {
            path: "certs/cert.pem".to_string(),
            content: "-----BEGIN CERTIFICATE-----\nabc\n-----END CERTIFICATE-----".to_string(),
        },
        ContextFile {
            path: "src/main.rs".to_string(),
            content: "fn main() {}".to_string(),
        },
    ];
    // All three are on the allowlist; exclusion must come from the default secret-file rule.
    let out = sanitize(&raw, &cfg(&[".env", "certs/cert.pem", "src/main.rs"]));

    let included: Vec<&str> = out.files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(included, vec!["src/main.rs"], ".env and *.pem absent from prompt");

    for p in [".env", "certs/cert.pem"] {
        let ex = out
            .excluded
            .iter()
            .find(|e| e.path == p)
            .unwrap_or_else(|| panic!("{p} recorded as excluded"));
        assert_eq!(ex.reason, ExcludeReason::DefaultExcluded, "{p} default-excluded");
    }
}

/// Contract matrix error handling: a critical credential (a full private key block) is not
/// merely redacted inline — the file is rejected and recorded, so a partial match can never
/// leak the remainder of the key.
#[test]
fn test_private_key_rejection() {
    let raw = vec![ContextFile {
        path: "deploy/id_rsa.txt".to_string(),
        content: "-----BEGIN RSA PRIVATE KEY-----\nMIIEv...\n-----END RSA PRIVATE KEY-----"
            .to_string(),
    }];
    let out = sanitize(&raw, &cfg(&["deploy/id_rsa.txt"]));

    assert!(
        out.files.iter().all(|f| f.path != "deploy/id_rsa.txt"),
        "critical-credential file never included"
    );
    let rejected = out
        .rejected
        .iter()
        .find(|r| r.path == "deploy/id_rsa.txt")
        .expect("rejection recorded");
    assert_eq!(rejected.kind, SecretKind::PrivateKey);
}
