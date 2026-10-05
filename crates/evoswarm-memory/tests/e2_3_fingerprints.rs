use evoswarm_memory::fingerprints::{repo_fingerprint, toolchain_fingerprint, ToolchainVersions};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_touched_file_invalidation() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    // Create touched file and lockfile
    let touched = root.join("main.rs");
    let lockfile = root.join("Cargo.lock");
    fs::write(&touched, b"fn main() {}").unwrap();
    fs::write(&lockfile, b"lockfile v1").unwrap();

    let touched_paths = vec![touched.clone()];
    let fp1 = repo_fingerprint(&touched_paths, &lockfile).unwrap();

    // Modify touched file
    fs::write(&touched, b"fn main() { println!(\"changed\"); }").unwrap();
    let fp2 = repo_fingerprint(&touched_paths, &lockfile).unwrap();

    assert_ne!(fp1, fp2, "changing a touched file must change repo fingerprint");
}

#[test]
fn test_lockfile_invalidation() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    let touched = root.join("lib.rs");
    let lockfile = root.join("package-lock.json");
    fs::write(&touched, b"pub fn foo() {}").unwrap();
    fs::write(&lockfile, b"lockfile v1").unwrap();

    let touched_paths = vec![touched.clone()];
    let fp1 = repo_fingerprint(&touched_paths, &lockfile).unwrap();

    // Modify lockfile
    fs::write(&lockfile, b"lockfile v2").unwrap();
    let fp2 = repo_fingerprint(&touched_paths, &lockfile).unwrap();

    assert_ne!(fp1, fp2, "changing the lockfile must change repo fingerprint");
}

#[test]
fn test_untouched_file_insensitivity() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();

    let touched = root.join("app.py");
    let untouched = root.join("README.md");
    let lockfile = root.join("requirements.lock");
    fs::write(&touched, b"def main(): pass").unwrap();
    fs::write(&untouched, b"# README").unwrap();
    fs::write(&lockfile, b"lock v1").unwrap();

    let touched_paths = vec![touched.clone()];
    let fp1 = repo_fingerprint(&touched_paths, &lockfile).unwrap();

    // Modify untouched sibling
    fs::write(&untouched, b"# README updated").unwrap();
    let fp2 = repo_fingerprint(&touched_paths, &lockfile).unwrap();

    assert_eq!(fp1, fp2, "changing an untouched file must NOT change repo fingerprint");
}

#[test]
fn test_toolchain_version_hash() {
    let versions1 = ToolchainVersions {
        compiler: "rustc 1.70.0".to_string(),
        runtime: "python 3.11.0".to_string(),
        harness: "pytest 7.4.0".to_string(),
    };
    let fp1 = toolchain_fingerprint(&versions1);

    // Change compiler version
    let versions2 = ToolchainVersions {
        compiler: "rustc 1.71.0".to_string(),
        runtime: "python 3.11.0".to_string(),
        harness: "pytest 7.4.0".to_string(),
    };
    let fp2 = toolchain_fingerprint(&versions2);
    assert_ne!(fp1, fp2, "changing compiler version must change toolchain fingerprint");

    // Change runtime version
    let versions3 = ToolchainVersions {
        compiler: "rustc 1.70.0".to_string(),
        runtime: "python 3.12.0".to_string(),
        harness: "pytest 7.4.0".to_string(),
    };
    let fp3 = toolchain_fingerprint(&versions3);
    assert_ne!(fp1, fp3, "changing runtime version must change toolchain fingerprint");

    // Change harness version
    let versions4 = ToolchainVersions {
        compiler: "rustc 1.70.0".to_string(),
        runtime: "python 3.11.0".to_string(),
        harness: "pytest 8.0.0".to_string(),
    };
    let fp4 = toolchain_fingerprint(&versions4);
    assert_ne!(fp1, fp4, "changing harness version must change toolchain fingerprint");
}

#[test]
fn test_toolchain_fingerprint_deterministic() {
    let versions = ToolchainVersions {
        compiler: "rustc 1.70.0".to_string(),
        runtime: "python 3.11.0".to_string(),
        harness: "pytest 7.4.0".to_string(),
    };
    let fp1 = toolchain_fingerprint(&versions);
    let fp2 = toolchain_fingerprint(&versions);
    assert_eq!(fp1, fp2, "same versions must produce same fingerprint");
}
