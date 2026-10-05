//! Prompt structure and token estimation (e1-4 §2, AD-6).
//!
//! Prompt caching correctness hinges on splitting the prompt into a *static prefix* (system
//! instructions, repository overview, API contracts — assembled once per job and reused
//! verbatim) and a *dynamic suffix* (the current parent diff and sandbox failure diagnostics).
//! Any per-call data (timestamps, candidate ids, job ids) must live in the suffix: a byte that
//! changes in the prefix silently defeats the provider cache and inflates every e1-10 budget
//! projection. `estimate_tokens` is the single token estimator shared with the e1-4 feedback
//! clamp and the e1-10 budget so the clamp and the caps never disagree.

/// A prompt split into its cacheable static prefix and its per-call dynamic suffix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptParts {
    /// Assembled once per job and reused verbatim across every call in that job.
    pub static_prefix: Vec<u8>,
    /// Per-call data: parent diff and failure diagnostics. Never placed in the prefix.
    pub dynamic_suffix: Vec<u8>,
}

impl PromptParts {
    /// Concatenates prefix and suffix into the full prompt bytes sent to the model.
    pub fn assembled(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.static_prefix.len() + self.dynamic_suffix.len());
        out.extend_from_slice(&self.static_prefix);
        out.extend_from_slice(&self.dynamic_suffix);
        out
    }
}

/// Estimates the token count of a byte slice using the project-wide ~4 bytes/token heuristic.
/// A conservative ceiling (round up) so a clamp at `budget` tokens can never under-count and
/// let an oversized prompt through. Deterministic, so a replayed call estimates identically.
pub fn estimate_tokens(bytes: &[u8]) -> usize {
    bytes.len().div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_rounds_up() {
        assert_eq!(estimate_tokens(b""), 0);
        assert_eq!(estimate_tokens(b"abc"), 1); // 3 bytes -> ceil(3/4) = 1
        assert_eq!(estimate_tokens(&[0u8; 8]), 2);
        assert_eq!(estimate_tokens(&[0u8; 9]), 3);
    }

    #[test]
    fn assembled_is_prefix_then_suffix() {
        let p = PromptParts {
            static_prefix: b"PREFIX".to_vec(),
            dynamic_suffix: b"SUFFIX".to_vec(),
        };
        assert_eq!(p.assembled(), b"PREFIXSUFFIX".to_vec());
    }
}
