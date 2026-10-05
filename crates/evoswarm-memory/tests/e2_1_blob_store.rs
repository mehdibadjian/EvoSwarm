use evoswarm_memory::blob_store::BlobStore;
use std::collections::HashSet;
use std::time::{Duration, SystemTime};
use tempfile::TempDir;

fn make_store() -> (TempDir, BlobStore) {
    let dir = TempDir::new().unwrap();
    let store = BlobStore::new(dir.path().to_path_buf()).unwrap();
    (dir, store)
}

#[test]
fn test_deduplication() {
    let (_dir, store) = make_store();
    let data = b"hello world, this is a test payload for dedup";
    let hash1 = store.write(data).unwrap();
    let hash2 = store.write(data).unwrap();
    assert_eq!(hash1, hash2);

    // Verify only one file exists on disk (the blob itself)
    let blob_path = store.blob_path(&hash1);
    assert!(blob_path.exists());

    // Count files in the blob dir
    let entries: Vec<_> = std::fs::read_dir(store.root())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| !e.file_name().to_string_lossy().ends_with(".lastref"))
        .collect();
    assert_eq!(entries.len(), 1, "identical content must exist only once on disk");
}

#[test]
fn test_byte_exact_read() {
    let (_dir, store) = make_store();
    let data: Vec<u8> = (0u8..=255).collect();
    let hash = store.write(&data).unwrap();
    let read_back = store.read(&hash).unwrap();
    assert_eq!(data, read_back, "read must return byte-exact original");
}

#[test]
fn test_read_not_found() {
    let (_dir, store) = make_store();
    let result = store.read("0000000000000000000000000000000000000000000000000000000000000000");
    assert!(result.is_err(), "reading a nonexistent hash must error");
}

#[test]
fn test_gc_sweep_deletes_unreferenced_old_blobs() {
    let (_dir, store) = make_store();

    // Write two distinct blobs
    let hash_a = store.write(b"blob A").unwrap();
    let hash_b = store.write(b"blob B").unwrap();

    // Set both blobs' last-referenced to 10 days ago
    let now = SystemTime::now();
    let ten_days_ago = now - Duration::from_secs(10 * 24 * 3600);
    store.set_last_referenced(&hash_a, ten_days_ago).unwrap();
    store.set_last_referenced(&hash_b, ten_days_ago).unwrap();

    // Only hash_a is referenced; hash_b is unreferenced and >7 days old
    let mut referenced = HashSet::new();
    referenced.insert(hash_a.clone());

    let report = store.gc(now, &referenced).unwrap();
    assert_eq!(report.deleted_count, 1);
    assert!(report.freed_bytes > 0);

    // hash_b should be gone
    assert!(!store.blob_path(&hash_b).exists());
    // hash_a should still exist
    assert!(store.blob_path(&hash_a).exists());
}

#[test]
fn test_gc_does_not_delete_recent_unreferenced() {
    let (_dir, store) = make_store();

    let hash = store.write(b"recent blob").unwrap();

    // Set last-referenced to 2 days ago (within 7-day window)
    let now = SystemTime::now();
    let two_days_ago = now - Duration::from_secs(2 * 24 * 3600);
    store.set_last_referenced(&hash, two_days_ago).unwrap();

    let referenced = HashSet::new(); // unreferenced
    let report = store.gc(now, &referenced).unwrap();
    assert_eq!(report.deleted_count, 0);
    assert_eq!(report.freed_bytes, 0);
    assert!(store.blob_path(&hash).exists());
}

#[test]
fn test_gc_does_not_delete_referenced_old_blob() {
    let (_dir, store) = make_store();

    let hash = store.write(b"referenced old blob").unwrap();

    let now = SystemTime::now();
    let ten_days_ago = now - Duration::from_secs(10 * 24 * 3600);
    store.set_last_referenced(&hash, ten_days_ago).unwrap();

    let mut referenced = HashSet::new();
    referenced.insert(hash.clone());

    let report = store.gc(now, &referenced).unwrap();
    assert_eq!(report.deleted_count, 0);
    assert!(store.blob_path(&hash).exists());
}

#[test]
fn test_gc_reports_zero_when_no_blobs() {
    let (_dir, store) = make_store();
    let now = SystemTime::now();
    let referenced = HashSet::new();
    let report = store.gc(now, &referenced).unwrap();
    assert_eq!(report.deleted_count, 0);
    assert_eq!(report.freed_bytes, 0);
}
