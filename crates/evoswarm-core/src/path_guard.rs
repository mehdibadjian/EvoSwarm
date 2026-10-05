use std::path::{Component, Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("path escapes repository root: {candidate:?} resolved outside {root:?}")]
pub struct PathEscape {
    pub root: PathBuf,
    pub candidate: PathBuf,
}

/// Returns `candidate` canonicalised and guaranteed to sit strictly within `root`.
///
/// The check is structural, not lexical: a raw `starts_with` on the joined path is
/// defeated by `..%2f`, symlinks, and `foo/../bar` forms. We canonicalise both sides so
/// symlinks and `..` are already resolved, then require `strip_prefix(root)` to succeed.
///
/// `candidate` may be absolute or relative to `root`. A relative candidate is joined to
/// `root` before canonicalisation. Because canonicalisation requires the path to exist,
/// a non-existent target falls back to a lexical containment decision (normalising `..`
/// and `.` against literal components) so e1-1 can still reject `../outside` before the
/// file is ever created.
pub fn require_within_root(root: &Path, candidate: &Path) -> Result<PathBuf, PathEscape> {
    let reject = || PathEscape {
        root: root.to_path_buf(),
        candidate: candidate.to_path_buf(),
    };

    let root_canon = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => return Err(reject()),
    };

    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root_canon.join(candidate)
    };

    match joined.canonicalize() {
        // Existing path: canonical form has symlinks and `..` resolved; containment is
        // a plain prefix test.
        Ok(candidate_canon) => match candidate_canon.strip_prefix(&root_canon) {
            Ok(_) => Ok(candidate_canon),
            Err(_) => Err(reject()),
        },
        // Non-existent target: decide lexically against the normalised path.
        Err(_) => {
            if normalised_starts_with(&joined, &root_canon) {
                Ok(joined)
            } else {
                Err(reject())
            }
        }
    }
}

/// Normalises `path` (collapsing `.` and `..` against literal components) and reports
/// whether the result begins with `root`'s components.
fn normalised_starts_with(path: &Path, root: &Path) -> bool {
    let mut stack: Vec<PathBuf> = Vec::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                stack.pop();
            }
            Component::CurDir => {}
            other => stack.push(other.as_os_str().to_owned().into()),
        }
    }
    let normalised = stack.iter().fold(PathBuf::new(), |mut acc, p| {
        acc.push(p);
        acc
    });
    normalised.starts_with(root)
}
