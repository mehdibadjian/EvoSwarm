//! Context guard (e3-5, story §2/§3): keep secrets out of every prompt.
//!
//! Before raw repository context is assembled into a model prompt it is passed through this
//! guard, which enforces three rules from the story's acceptance criteria:
//!
//! 1. **Path allowlist** — only files explicitly permitted for the job are included; anything
//!    else is dropped and recorded (AC1).
//! 2. **Default secret-file exclusion** — `.env*` and `*.pem` are excluded by default, even if
//!    they appear on the allowlist (AC3), because they exist to hold credentials.
//! 3. **Inline secret redaction** — remaining content is scanned for AWS access-key ids and
//!    OpenAI-style keys; each match is replaced with [`REDACTION_MARKER`] and logged (AC2).
//!
//! A file containing a *critical* credential — a full PEM `PRIVATE KEY` block — is not merely
//! redacted inline (which could leak the remainder of the key): it is rejected outright and
//! recorded, honouring the contract matrix's "Rejection if critical credentials detected".
//!
//! The pattern scanners are hand-written ASCII matchers rather than `regex`: the crate has no
//! regex dependency, the patterns are small and fixed, and pure functions keep the guard
//! deterministic and offline-safe.

use std::collections::BTreeSet;

/// Replacement text substituted for every inline secret match.
pub const REDACTION_MARKER: &str = "[REDACTED_SECRET]";

/// A single raw repository file offered to the guard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextFile {
    /// Repository-relative path used for allowlist and default-exclusion decisions.
    pub path: String,
    /// Raw file contents.
    pub content: String,
}

/// Per-job guard configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextGuardConfig {
    /// Paths permitted for this job. A file is included only if its path is present here.
    pub allowlist: BTreeSet<String>,
}

/// The class of secret a redaction or rejection matched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretKind {
    /// A 20-character AWS access-key id (`AKIA…`, `ASIA…`, etc.).
    AwsAccessKeyId,
    /// An OpenAI-style secret key (`sk-` followed by ≥32 secret characters).
    OpenAiKey,
    /// A PEM private-key block — treated as a critical credential (rejection, not redaction).
    PrivateKey,
}

/// Why a file was kept out of the sanitized payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExcludeReason {
    /// The path was not in the job's allowlist (AC1).
    NotInAllowlist,
    /// The path matched the default secret-file exclusion, `.env*` or `*.pem` (AC3).
    DefaultExcluded,
}

/// A file dropped before content inspection, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExcludedFile {
    pub path: String,
    pub reason: ExcludeReason,
}

/// An inline redaction applied to a kept file's content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactionLog {
    pub path: String,
    pub kind: SecretKind,
    /// Number of matches of this kind redacted in this file.
    pub count: usize,
}

/// A file rejected because it contained a critical credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectedFile {
    pub path: String,
    pub kind: SecretKind,
}

/// The sanitized prompt payload plus an audit trail of everything the guard did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedContext {
    /// Files that survived the guard, with inline secrets already redacted.
    pub files: Vec<ContextFile>,
    /// Files dropped by the allowlist or default exclusion.
    pub excluded: Vec<ExcludedFile>,
    /// Inline redactions applied to kept files.
    pub redactions: Vec<RedactionLog>,
    /// Files rejected for a critical credential.
    pub rejected: Vec<RejectedFile>,
}

/// Sanitizes raw repository context against `config`.
///
/// Files are processed in order; for each file the guard applies the allowlist, then the
/// default secret-file exclusion, then a critical-credential check, then inline redaction.
/// Every decision that removes or alters content is recorded in the returned audit trail so a
/// security reviewer can see exactly what was withheld and why.
pub fn sanitize(raw: &[ContextFile], config: &ContextGuardConfig) -> SanitizedContext {
    let mut out = SanitizedContext {
        files: Vec::new(),
        excluded: Vec::new(),
        redactions: Vec::new(),
        rejected: Vec::new(),
    };

    for file in raw {
        // AC1: path allowlist.
        if !config.allowlist.contains(&file.path) {
            out.excluded.push(ExcludedFile {
                path: file.path.clone(),
                reason: ExcludeReason::NotInAllowlist,
            });
            continue;
        }

        // AC3: default secret-file exclusion (.env*, *.pem), even when allowlisted.
        if is_default_excluded(&file.path) {
            out.excluded.push(ExcludedFile {
                path: file.path.clone(),
                reason: ExcludeReason::DefaultExcluded,
            });
            continue;
        }

        // Contract matrix: reject critical credentials outright rather than risk a partial
        // inline redaction leaking the remainder of a key.
        if let Some(kind) = critical_credential(&file.content) {
            out.rejected.push(RejectedFile {
                path: file.path.clone(),
                kind,
            });
            continue;
        }

        // AC2: inline redaction of remaining secrets, with an audit log entry per kind.
        let mut logs: Vec<RedactionLog> = Vec::new();
        let content = redact_inline(&file.content, &mut |kind| {
            if let Some(existing) = logs.iter_mut().find(|l| l.kind == kind) {
                existing.count += 1;
            } else {
                logs.push(RedactionLog {
                    path: file.path.clone(),
                    kind,
                    count: 1,
                });
            }
        });

        out.files.push(ContextFile {
            path: file.path.clone(),
            content,
        });
        out.redactions.extend(logs);
    }

    out
}

/// True if the path matches the default secret-file exclusion: a `.env` file (or `.env.local`
/// style variant) or any `*.pem`.
fn is_default_excluded(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    name == ".env" || name.starts_with(".env.") || path.ends_with(".pem")
}

/// Detects a critical credential: a PEM `PRIVATE KEY` block. Returns the kind so the caller can
/// record a precise rejection. `None` when no private-key block is present.
fn critical_credential(content: &str) -> Option<SecretKind> {
    // A PEM private key is `-----BEGIN <label> PRIVATE KEY-----`. Scanning for the BEGIN
    // delimiter and testing the label avoids matching the literal in prose.
    let bytes = content.as_bytes();
    let mut from = 0usize;
    while let Some(rel) = find_subslice(&bytes[from..], b"-----BEGIN ") {
        let start = from + rel + b"-----BEGIN ".len();
        if let Some(end_off) = find_subslice(&bytes[start..], b"-----") {
            let label = &content[start..start + end_off];
            if label.contains("PRIVATE KEY") {
                return Some(SecretKind::PrivateKey);
            }
        }
        from = start;
    }
    None
}

/// Scans `text` for inline secrets, replacing each match with [`REDACTION_MARKER`] and invoking
/// `on_redaction` once per match with its kind. Returns the redacted text.
fn redact_inline<F>(text: &str, on_redaction: &mut F) -> String
where
    F: FnMut(SecretKind),
{
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if let Some((len, kind)) = match_secret(&bytes[i..]) {
            out.push_str(REDACTION_MARKER);
            on_redaction(kind);
            i += len;
        } else {
            // Copy one full UTF-8 scalar at a time so multi-byte characters survive intact.
            let ch_len = utf8_char_len(bytes[i]);
            out.push_str(&text[i..i + ch_len]);
            i += ch_len;
        }
    }
    out
}

/// Matches a secret at the start of `s`, returning its byte length and kind.
fn match_secret(s: &[u8]) -> Option<(usize, SecretKind)> {
    match_openai(s).or_else(|| match_aws(s))
}

/// OpenAI-style key: `sk-` followed by ≥32 secret characters ([A-Za-z0-9_-]).
fn match_openai(s: &[u8]) -> Option<(usize, SecretKind)> {
    if !s.starts_with(b"sk-") {
        return None;
    }
    let mut n = 0usize;
    while n + 3 < s.len() && is_secret_char(s[n + 3]) {
        n += 1;
    }
    if n >= 32 {
        Some((3 + n, SecretKind::OpenAiKey))
    } else {
        None
    }
}

/// AWS access-key id: a 4-char prefix (`AKIA`, `ASIA`, `AIDA`, `AROA`, `AIPA`) followed by 16
/// more uppercase-alphanumeric characters — 20 total.
fn match_aws(s: &[u8]) -> Option<(usize, SecretKind)> {
    const PREFIXES: [&[u8; 4]; 5] = [b"AKIA", b"ASIA", b"AIDA", b"AROA", b"AIPA"];
    if s.len() < 20 || !PREFIXES.iter().any(|p| s.starts_with(&p[..])) {
        return None;
    }
    if s[..20].iter().all(|&b| b.is_ascii_uppercase() || b.is_ascii_digit()) {
        Some((20, SecretKind::AwsAccessKeyId))
    } else {
        None
    }
}

fn is_secret_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// Byte length of the UTF-8 character starting with `first` (always ≥1 so scanning progresses).
fn utf8_char_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF7 => 4,
        _ => 1,
    }
}

/// Index of the first occurrence of `needle` in `haystack`, or `None`.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aws_needs_full_20_chars() {
        // 19 chars: too short.
        assert_eq!(match_aws(b"AKIAIOSFODNN7EXAMPL"), None);
        // 20 chars, valid prefix: matches exactly 20.
        assert_eq!(match_aws(b"AKIAIOSFODNN7EXAMPLE"), Some((20, SecretKind::AwsAccessKeyId)));
        // Lowercase tail is not an AWS id.
        assert_eq!(match_aws(b"AKIAiosfodnn7example"), None);
        // Unknown prefix is not matched.
        assert_eq!(match_aws(b"ZZZZIOSFODNN7EXAMPLE"), None);
    }

    #[test]
    fn openai_needs_32_secret_chars() {
        // Exactly 32: matches.
        let key = format!("sk-{}", "A".repeat(32));
        assert_eq!(match_openai(key.as_bytes()), Some((35, SecretKind::OpenAiKey)));
        // 31: too short.
        let short = format!("sk-{}", "A".repeat(31));
        assert_eq!(match_openai(short.as_bytes()), None);
        // `sk-` alone is not a key.
        assert_eq!(match_openai(b"sk-"), None);
    }

    #[test]
    fn default_exclusion_covers_env_variants_and_pem() {
        assert!(is_default_excluded(".env"));
        assert!(is_default_excluded("config/.env"));
        assert!(is_default_excluded(".env.local"));
        assert!(is_default_excluded("certs/cert.pem"));
        assert!(!is_default_excluded("src/main.rs"));
        assert!(!is_default_excluded(".environment"));
        assert!(!is_default_excluded("pems.txt"));
    }

    #[test]
    fn critical_credential_detects_private_key_block() {
        assert_eq!(
            critical_credential("-----BEGIN RSA PRIVATE KEY-----\nabc\n-----END RSA PRIVATE KEY-----"),
            Some(SecretKind::PrivateKey)
        );
        assert_eq!(
            critical_credential("-----BEGIN OPENSSH PRIVATE KEY-----\nabc"),
            Some(SecretKind::PrivateKey)
        );
        // A certificate block is not a private key.
        assert_eq!(critical_credential("-----BEGIN CERTIFICATE-----\nabc"), None);
        // Prose mentioning the phrase without the PEM delimiter is not a credential.
        assert_eq!(critical_credential("a BEGIN PRIVATE KEY note"), None);
    }

    #[test]
    fn redact_preserves_multibyte_utf8() {
        let aws = "AKIAIOSFODNN7EXAMPLE";
        let text = format!("héllo→{aws}→世界");
        let out = redact_inline(&text, &mut |_| {});
        assert_eq!(out, format!("héllo→{REDACTION_MARKER}→世界"));
    }

    #[test]
    fn redact_counts_multiple_of_same_kind() {
        let aws = "AKIAIOSFODNN7EXAMPLE";
        let text = format!("{aws} and {aws}");
        let mut n = 0usize;
        let out = redact_inline(&text, &mut |_| n += 1);
        assert_eq!(n, 2);
        assert_eq!(out, format!("{REDACTION_MARKER} and {REDACTION_MARKER}"));
    }
}
