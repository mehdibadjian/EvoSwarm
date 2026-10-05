//! Content hashing for population deduplication (e1-3 §1.4, AD-5). Two drafts with identical
//! patch bytes must be recognised as duplicates so the population is never padded with copies;
//! the SHA-256 of the patch is the identity used for that comparison.

use sha2::{Digest, Sha256};

/// SHA-256 of a candidate's patch bytes. Used as the dedup identity when seeding a generation:
/// a draft whose hash already appears in the population is a duplicate and must be replaced.
pub fn diff_hash(patch: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(patch);
    hasher.finalize().into()
}

/// Lower-case hex of a 32-byte hash, used for the persisted `prompt_hash` string field.
pub fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_patch_identical_hash() {
        assert_eq!(diff_hash(b"same"), diff_hash(b"same"));
    }

    #[test]
    fn different_patch_different_hash() {
        assert_ne!(diff_hash(b"one"), diff_hash(b"two"));
    }

    #[test]
    fn empty_patch_hashes_stably() {
        assert_eq!(diff_hash(b"").len(), 32);
        assert_eq!(diff_hash(b""), diff_hash(b""));
    }

    #[test]
    fn hex_is_lowercase_and_full_width() {
        let s = hex(&[0u8; 32]);
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
