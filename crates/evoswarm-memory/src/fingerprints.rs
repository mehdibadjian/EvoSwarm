use sha2::{Digest, Sha256};
use std::fs;
use std::io;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FingerprintError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, Clone)]
pub struct ToolchainVersions {
    pub compiler: String,
    pub runtime: String,
    pub harness: String,
}

pub fn repo_fingerprint(touched_paths: &[impl AsRef<std::path::Path>], lockfile: &Path) -> Result<String, FingerprintError> {
    let mut hasher = Sha256::new();
    
    // Hash all touched files in sorted order for determinism
    let mut paths: Vec<_> = touched_paths.iter().collect();
    paths.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
    
    for path in paths {
        let content = fs::read(path.as_ref())?;
        hasher.update(&content);
    }
    
    // Hash lockfile
    if lockfile.exists() {
        let content = fs::read(lockfile)?;
        hasher.update(&content);
    }
    
    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

pub fn toolchain_fingerprint(versions: &ToolchainVersions) -> String {
    let mut hasher = Sha256::new();
    
    // Hash all version strings in a deterministic order
    hasher.update(versions.compiler.as_bytes());
    hasher.update(b"|");
    hasher.update(versions.runtime.as_bytes());
    hasher.update(b"|");
    hasher.update(versions.harness.as_bytes());
    
    let result = hasher.finalize();
    format!("{:x}", result)
}
