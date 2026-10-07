//! e0-3 AppArmor profile for bwrap — acceptance criteria (story §4).
//!
//! Gate: `cargo test -p evoswarm-sandbox --test e0_3_apparmor_profile_for_bwrap`.
//!
//! Validates:
//! - AC1: A ready-made AppArmor profile for `bwrap` is shipped, granting `userns` permissions
//!   so unprivileged user namespaces work on modern Ubuntu Server (23.10+, 24.04+).
//! - AC2: `scripts/install_apparmor.sh` installs the profile and supports a custom destination
//!   directory (`--dest-dir <DIR>`).
//! - AC3: The installer is idempotent: running it twice results in an identical file and zero exit code.
//! - AC4: When run with an unwritable destination (or without required privileges), it fails cleanly
//!   with a non-zero exit code.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("parent of crates")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

#[test]
fn test_installer_script_exists_and_executable() {
    let root = repo_root();
    let script_path = root.join("scripts/install_apparmor.sh");

    assert!(
        script_path.is_file(),
        "scripts/install_apparmor.sh must exist: {:?}",
        script_path
    );

    let perms = fs::metadata(&script_path)
        .expect("metadata")
        .permissions();
    assert_ne!(
        perms.mode() & 0o111,
        0,
        "scripts/install_apparmor.sh must be executable"
    );
}

#[test]
fn test_apparmor_profile_syntax_and_userns_rule() {
    let root = repo_root();
    // Profile can be shipped in packaging/apparmor/bwrap or alongside the script
    let profile_candidates = [
        root.join("packaging/apparmor/bwrap"),
        root.join("packaging/apparmor.d/bwrap"),
        root.join("etc/apparmor.d/bwrap"),
    ];

    let profile_path = profile_candidates
        .iter()
        .find(|p| p.is_file())
        .expect("AppArmor bwrap profile file must be shipped in repository");

    let content = fs::read_to_string(profile_path).expect("read profile");

    assert!(
        content.contains("profile bwrap") || content.contains("profile /"),
        "Profile must declare bwrap profile"
    );
    assert!(
        content.contains("userns"),
        "Profile must explicitly permit unprivileged user namespaces ('userns')"
    );
    assert!(
        content.contains("unconfined"),
        "Profile must include unconfined flags for bwrap payload isolation"
    );
}

#[test]
fn test_installer_idempotency_and_custom_dest() {
    let root = repo_root();
    let script_path = root.join("scripts/install_apparmor.sh");
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let dest = temp_dir.path().to_path_buf();

    // First install run
    let output1 = Command::new(&script_path)
        .arg("--dest-dir")
        .arg(&dest)
        .output()
        .expect("execute install script run 1");

    assert!(
        output1.status.success(),
        "install script run 1 failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output1.stdout),
        String::from_utf8_lossy(&output1.stderr)
    );

    let installed_file = dest.join("bwrap");
    assert!(
        installed_file.is_file(),
        "installed profile must exist at {:?}",
        installed_file
    );

    let content1 = fs::read(&installed_file).expect("read installed profile 1");

    // Second install run (assert idempotency)
    let output2 = Command::new(&script_path)
        .arg("--dest-dir")
        .arg(&dest)
        .output()
        .expect("execute install script run 2");

    assert!(
        output2.status.success(),
        "install script run 2 failed: stdout={:?} stderr={:?}",
        String::from_utf8_lossy(&output2.stdout),
        String::from_utf8_lossy(&output2.stderr)
    );

    let content2 = fs::read(&installed_file).expect("read installed profile 2");
    assert_eq!(
        content1, content2,
        "Idempotency failure: installed profile differs between runs"
    );
}

#[test]
fn test_installer_handles_unwritable_dest() {
    let root = repo_root();
    let script_path = root.join("scripts/install_apparmor.sh");
    let non_existent_protected = PathBuf::from("/proc/sys/fs/protected_apparmor_test_invalid_path");

    let output = Command::new(&script_path)
        .arg("--dest-dir")
        .arg(&non_existent_protected)
        .output()
        .expect("execute install script with unwritable dest");

    assert!(
        !output.status.success(),
        "Installer must return non-zero exit code when destination is not writable"
    );
}
