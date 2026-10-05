use std::path::{Component, Path, PathBuf};

/// File names that change *how* tests run rather than the code under test (AD-5).
const HARNESS_FILES: &[&str] = &[
    "conftest.py",
    "pytest.ini",
    "tox.ini",
    "Directory.Build.props",
    "Directory.Build.targets",
    ".runsettings",
];

/// Build scripts and CI config that must never be touched by a candidate diff.
const BUILD_AND_CI_COMPONENTS: &[&str] = &[
    "Makefile",
    "CMakeLists.txt",
    "build.gradle",
    "build.sbt",
    ".github",
    ".gitlab-ci.yml",
    ".circleci",
];

/// Returns the subset of `patch_paths` that violate e1-6 Gate 3: any path that is
/// protected (tests dir, harness file, build script, CI config) OR that falls outside
/// every prefix in `allowed` (the declared `--paths`).
///
/// Matching is anchored on **path components**, never substrings, so `mytests/helper.rs`
/// and `src/tests_helper.rs` are correctly *not* flagged while `tests/foo.py` and
/// `a/conftest.py` are.
pub fn detect(patch_paths: &[PathBuf], allowed: &[PathBuf]) -> Vec<PathBuf> {
    patch_paths
        .iter()
        .filter(|p| is_protected(p) || !is_within_allowed(p, allowed))
        .cloned()
        .collect()
}

fn is_protected(path: &Path) -> bool {
    let components: Vec<_> = path.components().collect();

    for (idx, component) in components.iter().enumerate() {
        if let Component::Normal(name) = component {
            let name_str = name.to_string_lossy();
            // A `tests`/`test` directory component (not the final component) marks the
            // protected test tree. Anchored on the component, so `mytests/` is safe.
            if (name_str == "tests" || name_str == "test") && idx < components.len() - 1 {
                return true;
            }
            if HARNESS_FILES.contains(&name_str.as_ref())
                || BUILD_AND_CI_COMPONENTS.contains(&name_str.as_ref())
            {
                return true;
            }
        }
    }

    false
}

/// True when `path` is within (or equal to) at least one allowed prefix, compared
/// component-wise via `starts_with` so `/work/src_extra` is not treated as inside `/work/src`.
fn is_within_allowed(path: &Path, allowed: &[PathBuf]) -> bool {
    allowed.iter().any(|prefix| path.starts_with(prefix))
}
