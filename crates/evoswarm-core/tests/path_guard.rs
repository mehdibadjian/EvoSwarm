use std::fs;
use std::path::Path;

use evoswarm_core::path_guard::require_within_root;

fn temp_root() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "// code\n").unwrap();
    dir
}

#[test]
fn accepts_existing_path_inside_root() {
    let root = temp_root();
    let got = require_within_root(root.path(), Path::new("src/lib.rs")).unwrap();
    assert!(got.starts_with(root.path().canonicalize().unwrap()));
    assert!(got.ends_with("src/lib.rs"));
}

#[test]
fn accepts_nonexistent_path_inside_root() {
    let root = temp_root();
    let got = require_within_root(root.path(), Path::new("src/new_module.rs")).unwrap();
    assert!(got.ends_with("src/new_module.rs"));
}

#[test]
fn rejects_dotdot_escape() {
    let root = temp_root();
    let err = require_within_root(root.path(), Path::new("../outside")).unwrap_err();
    assert_eq!(err.candidate, Path::new("../outside"));
}

#[test]
fn rejects_absolute_path_outside_root() {
    let root = temp_root();
    let err = require_within_root(root.path(), Path::new("/etc/passwd")).unwrap_err();
    assert_eq!(err.candidate, Path::new("/etc/passwd"));
}

#[test]
fn rejects_symlink_escape() {
    let root = temp_root();
    // A symlink inside the repo that points at /etc must be rejected once canonicalised,
    // proving the guard resolves symlinks rather than trusting the lexical prefix.
    std::os::unix::fs::symlink("/etc", root.path().join("sneaky")).unwrap();
    let err = require_within_root(root.path(), Path::new("sneaky/passwd")).unwrap_err();
    assert_eq!(err.candidate, Path::new("sneaky/passwd"));
}
