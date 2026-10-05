//! Idempotency key derivation (e1-12 §3): the cache key covers the role, the model id and
//! the exact prompt bytes (static prefix plus dynamic suffix). Changing any one of them
//! yields a different key, so a replayed call after a prompt edit correctly misses the cache
//! and dispatches fresh — while an identical call hits it and spends zero tokens.

use sha2::{Digest, Sha256};

use crate::config::Role;

/// Computes the 32-byte idempotency hash over `role`, `model_id` and the exact `prompt`
/// bytes. Length-prefixed framing keeps the domains unambiguous so a role/model/prompt
/// triple cannot collide with a different split of the same bytes.
pub fn call_hash(role: Role, model_id: &str, prompt: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(role.to_string().as_bytes());
    hasher.update([0u8]); // domain separator
    hasher.update(model_id.as_bytes());
    hasher.update([0u8]); // domain separator
    hasher.update((prompt.len() as u64).to_be_bytes());
    hasher.update(prompt);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_inputs_yield_identical_hash() {
        let a = call_hash(Role::Mutator, "m1", b"prompt");
        let b = call_hash(Role::Mutator, "m1", b"prompt");
        assert_eq!(a, b);
    }

    #[test]
    fn changed_prompt_changes_hash() {
        let a = call_hash(Role::Mutator, "m1", b"prompt");
        // Altering one suffix byte must produce a different key (cache miss → fresh call).
        let b = call_hash(Role::Mutator, "m1", b"prompu");
        assert_ne!(a, b);
    }

    #[test]
    fn changed_role_changes_hash() {
        let a = call_hash(Role::Mutator, "m1", b"prompt");
        let b = call_hash(Role::Adversary, "m1", b"prompt");
        assert_ne!(a, b);
    }

    #[test]
    fn changed_model_changes_hash() {
        let a = call_hash(Role::Mutator, "m1", b"prompt");
        let b = call_hash(Role::Mutator, "m2", b"prompt");
        assert_ne!(a, b);
    }

    #[test]
    fn framing_prevents_boundary_collision() {
        // ("ab","c") vs ("a","bc") would collide under naive concatenation; the length
        // prefix on the prompt plus separators keep them distinct.
        let a = call_hash(Role::Mutator, "ab", b"c");
        let b = call_hash(Role::Mutator, "a", b"bc");
        assert_ne!(a, b);
    }
}
