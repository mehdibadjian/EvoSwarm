//! Held-out leakage barrier (e1-8 §2): no held-out test name or assertion text may appear in
//! any prompt payload. The scan runs over the *assembled* prompt (after interpolation) at the
//! single dispatch chokepoint, so interpolated diffs and error dumps are covered too — scanning
//! the raw template would miss anything injected at build time.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("held-out content leaked into prompt: {leaked:?}")]
pub struct LeakDetected {
    /// The held-out fragments found in the prompt.
    pub leaked: Vec<String>,
}

/// Scans `prompt` for any held-out fragment and rejects the dispatch if one is present.
///
/// Each held-out test contributes two probes: its full name, and each whitespace-delimited
/// token of its assertion body. A fragment counts as leaked only when it appears verbatim in
/// the prompt, so an ordinary word that happens to occur in unrelated text is not
/// over-flagged, while a copied test name or assertion is always caught.
///
/// `held_out` is a list of `(name, assertion_body)` pairs. An empty body contributes only the
/// name probe.
pub fn assert_no_leakage(
    prompt: &[u8],
    held_out: &[(String, String)],
) -> Result<(), LeakDetected> {
    let text = String::from_utf8_lossy(prompt);
    let mut leaked: Vec<String> = Vec::new();

    for (name, body) in held_out {
        if !name.is_empty() && text.contains(name.as_str()) {
            leaked.push(name.clone());
        }
        for token in body.split_whitespace() {
            // Only treat substantive tokens as assertion text; a single-character token
            // would false-positive on ordinary punctuation and spacing.
            if token.chars().count() >= 4 && text.contains(token) {
                leaked.push(token.to_string());
            }
        }
    }

    if leaked.is_empty() {
        Ok(())
    } else {
        leaked.sort();
        leaked.dedup();
        Err(LeakDetected { leaked })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held(name: &str, body: &str) -> (String, String) {
        (name.to_string(), body.to_string())
    }

    #[test]
    fn detects_held_out_name_in_prompt() {
        let prompt = b"Please fix test_secret_edge_case so it passes";
        let err = assert_no_leakage(prompt, &[held("test_secret_edge_case", "")]).unwrap_err();
        assert!(err.leaked.contains(&"test_secret_edge_case".to_string()));
    }

    #[test]
    fn detects_held_out_assertion_text_in_prompt() {
        // The name is absent but the assertion body leaked verbatim into an error dump.
        let prompt = b"error: expected result == compute_checksum_final(input)";
        let err = assert_no_leakage(
            prompt,
            &[held("test_x", "assert compute_checksum_final(input) == result")],
        )
        .unwrap_err();
        assert!(err.leaked.contains(&"compute_checksum_final(input)".to_string()));
    }

    #[test]
    fn clean_prompt_passes() {
        let prompt = b"Optimise the parser hot loop; visible tests: test_a, test_b";
        assert!(assert_no_leakage(prompt, &[held("test_secret", "assert hidden_value == 1")]).is_ok());
    }

    #[test]
    fn short_tokens_are_not_over_flagged() {
        // "x" and "==" are too short/common to treat as leaked assertion text.
        let prompt = b"a == b";
        assert!(assert_no_leakage(prompt, &[held("t", "x == y")]).is_ok());
    }
}
