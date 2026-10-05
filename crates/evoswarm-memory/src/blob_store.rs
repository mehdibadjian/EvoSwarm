use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlobError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Blob not found: {0}")]
    NotFound(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GcReport {
    pub deleted_count: usize,
    pub freed_bytes: u64,
}

pub struct BlobStore {
    root: PathBuf,
}

impl BlobStore {
    pub fn new(root: PathBuf) -> Result<Self, BlobError> {
        fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn blob_path(&self, hash: &str) -> PathBuf {
        self.root.join(hash)
    }

    fn lastref_path(&self, hash: &str) -> PathBuf {
        self.root.join(format!("{hash}.lastref"))
    }

    pub fn write(&self, data: &[u8]) -> Result<String, BlobError> {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = format!("{:x}", hasher.finalize());

        let path = self.blob_path(&hash);
        if !path.exists() {
            let mut file = fs::File::create(&path)?;
            file.write_all(data)?;
        }

        // Update last-referenced timestamp
        self.set_last_referenced(&hash, SystemTime::now())?;

        Ok(hash)
    }

    pub fn read(&self, hash: &str) -> Result<Vec<u8>, BlobError> {
        let path = self.blob_path(hash);
        if !path.exists() {
            return Err(BlobError::NotFound(hash.to_string()));
        }
        fs::read(&path).map_err(BlobError::Io)
    }

    pub fn set_last_referenced(&self, hash: &str, time: SystemTime) -> Result<(), BlobError> {
        let path = self.lastref_path(hash);
        let duration = time
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(io::Error::other)?;
        let timestamp = duration.as_secs().to_string();
        fs::write(&path, timestamp).map_err(BlobError::Io)
    }

    fn get_last_referenced(&self, hash: &str) -> Result<Option<SystemTime>, BlobError> {
        let path = self.lastref_path(hash);
        if !path.exists() {
            return Ok(None);
        }
        let timestamp_str = fs::read_to_string(&path)?;
        let secs: u64 = timestamp_str.trim().parse().map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("invalid timestamp: {e}"))
        })?;
        let time = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        Ok(Some(time))
    }

    pub fn gc(&self, now: SystemTime, referenced: &HashSet<String>) -> Result<GcReport, BlobError> {
        let retention = Duration::from_secs(7 * 24 * 3600);
        let cutoff = now.checked_sub(retention).ok_or_else(|| {
            io::Error::other("time underflow computing GC cutoff")
        })?;

        let mut deleted_count = 0;
        let mut freed_bytes = 0u64;

        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let name = file_name.to_string_lossy();

            // Skip .lastref metadata files
            if name.ends_with(".lastref") {
                continue;
            }

            let hash = name.to_string();

            // Skip if referenced
            if referenced.contains(&hash) {
                continue;
            }

            // Check last-referenced time
            if let Some(last_ref) = self.get_last_referenced(&hash)? {
                if last_ref > cutoff {
                    // Still within retention window
                    continue;
                }
            }

            // Delete blob
            let blob_path = entry.path();
            let size = fs::metadata(&blob_path)?.len();
            fs::remove_file(&blob_path)?;

            // Delete lastref metadata if present
            let lastref_path = self.lastref_path(&hash);
            if lastref_path.exists() {
                let _ = fs::remove_file(&lastref_path);
            }

            deleted_count += 1;
            freed_bytes += size;
        }

        Ok(GcReport {
            deleted_count,
            freed_bytes,
        })
    }
}
