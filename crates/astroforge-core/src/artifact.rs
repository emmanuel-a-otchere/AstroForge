use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactId {
    pub session_id: String,
    pub stage_id: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRecord {
    pub id: ArtifactId,
    pub path: PathBuf,
    pub format: ArtifactFormat,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactFormat {
    Fits,
    Tiff,
    Png,
    Jpeg,
    Xisf,
    Json,
}

pub struct ArtifactStore {
    base_dir: PathBuf,
}

impl ArtifactStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn path_for(&self, id: &ArtifactId) -> PathBuf {
        self.base_dir
            .join(&id.session_id)
            .join(&id.stage_id)
            .join(&id.kind)
    }

    pub fn save(
        &self,
        id: &ArtifactId,
        data: &[u8],
        _format: ArtifactFormat,
    ) -> Result<PathBuf, std::io::Error> {
        let path = self.path_for(id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, data)?;
        Ok(path)
    }

    pub fn load(&self, id: &ArtifactId) -> Result<Vec<u8>, std::io::Error> {
        let path = self.path_for(id);
        std::fs::read(&path)
    }

    pub fn exists(&self, id: &ArtifactId) -> bool {
        self.path_for(id).exists()
    }
}

// ─── CR-02.3 — content-addressed artifact store ─────────────────────────────

use sha2::{Digest, Sha256};

/// Outcome of a `ContentStore::put`.
#[derive(Debug, Clone)]
pub struct PutOutcome {
    /// SHA-256 hex digest of the stored bytes (CR-02 §7).
    pub hash: String,
    /// Final content-addressed path: `<dir>/<hash[..2]>/<hash>.<ext>`.
    pub path: PathBuf,
    pub size: u64,
    /// True when identical content was already stored (dedup by hash).
    pub already_existed: bool,
}

/// CR-02 §7/§27/§29 — content-addressed artifact storage.
///
/// Files are addressed by SHA-256 of their contents, which gives duplicate
/// detection, safe caching, and integrity verification for free. Writes are
/// atomic per CR-02 §29: temp file in the same directory → sync → rename.
/// A crash mid-write can only leave an orphaned `.tmp-*` file, never a
/// partial artifact at the content address.
///
/// The SQLite half (artifact rows with lineage + producer linkage) lives in
/// `domain_store.rs`; callers compose `put` → `record_artifact` so the DB
/// record is committed only after the bytes are durable (§29 ordering).
pub struct ContentStore {
    artifacts_dir: PathBuf,
}

impl ContentStore {
    pub fn new(artifacts_dir: PathBuf) -> Self {
        Self { artifacts_dir }
    }

    pub fn sha256_hex(data: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(data);
        format!("{:x}", h.finalize())
    }

    pub fn hash_file(path: &PathBuf) -> std::io::Result<String> {
        let data = std::fs::read(path)?;
        Ok(Self::sha256_hex(&data))
    }

    /// Content address for a hash: two-level fan-out keeps directories small
    /// on large projects (CR-02 §27 lifecycle at scale).
    pub fn path_for_hash(&self, hash: &str, ext: &str) -> PathBuf {
        let (shard, _) = hash.split_at(2.min(hash.len()));
        self.artifacts_dir.join(shard).join(format!("{hash}.{ext}"))
    }

    /// Store `data` atomically. Idempotent: identical content returns the
    /// existing address with `already_existed = true`.
    pub fn put(&self, data: &[u8], ext: &str) -> std::io::Result<PutOutcome> {
        use std::io::Write;
        let hash = Self::sha256_hex(data);
        let target = self.path_for_hash(&hash, ext);
        if target.exists() {
            return Ok(PutOutcome {
                hash,
                path: target,
                size: data.len() as u64,
                already_existed: true,
            });
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // §29: write temp in the SAME directory (same filesystem, so the
        // rename below is atomic), sync, then rename into place.
        let tmp = target.with_extension(format!("tmp-{}", std::process::id()));
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(data)?;
            f.sync_all()?;
        }
        // Re-validate the temp copy before it becomes the canonical bytes.
        if Self::hash_file(&tmp)? != hash {
            let _ = std::fs::remove_file(&tmp);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "artifact hash mismatch after write",
            ));
        }
        std::fs::rename(&tmp, &target)?;
        Ok(PutOutcome {
            hash,
            path: target,
            size: data.len() as u64,
            already_existed: false,
        })
    }

    /// Integrity check (CR-02 §7): re-hash the stored bytes and compare
    /// against the expected content address. False = corruption/missing.
    pub fn validate(&self, hash: &str, ext: &str) -> bool {
        let path = self.path_for_hash(hash, ext);
        match Self::hash_file(&path) {
            Ok(actual) => actual == hash,
            Err(_) => false,
        }
    }

    pub fn load(&self, hash: &str, ext: &str) -> std::io::Result<Vec<u8>> {
        std::fs::read(self.path_for_hash(hash, ext))
    }
}

#[cfg(test)]
mod content_store_tests {
    use super::*;

    fn temp_store(tag: &str) -> (PathBuf, ContentStore) {
        let dir = std::env::temp_dir().join(format!(
            "astroforge-content-store-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        (dir.clone(), ContentStore::new(dir))
    }

    #[test]
    fn sha256_hex_matches_known_vector() {
        // NIST test vector: SHA-256("abc").
        assert_eq!(
            ContentStore::sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn put_load_validate_round_trip() {
        let (dir, store) = temp_store("roundtrip");
        let data = b"fake-fits-bytes".as_slice();
        let out = store.put(data, "fits").unwrap();
        assert!(!out.already_existed);
        assert_eq!(out.size, data.len() as u64);
        assert!(out.path.exists());
        assert_eq!(store.load(&out.hash, "fits").unwrap(), data);
        assert!(store.validate(&out.hash, "fits"));
        // Content address layout: <dir>/<2-char shard>/<hash>.<ext>.
        assert_eq!(
            out.path,
            dir.join(&out.hash[..2]).join(format!("{}.fits", out.hash))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn put_deduplicates_identical_content() {
        let (dir, store) = temp_store("dedup");
        let first = store.put(b"same-bytes", "tif").unwrap();
        let second = store.put(b"same-bytes", "tif").unwrap();
        assert!(!first.already_existed);
        assert!(second.already_existed);
        assert_eq!(first.hash, second.hash);
        assert_eq!(first.path, second.path);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn put_leaves_no_temp_files() {
        let (dir, store) = temp_store("notmp");
        store.put(b"payload", "bin").unwrap();
        let mut leftover_tmps = 0;
        for shard in std::fs::read_dir(&dir).unwrap() {
            for entry in std::fs::read_dir(shard.unwrap().path()).unwrap() {
                let name = entry.unwrap().file_name().into_string().unwrap();
                if name.contains(".tmp-") {
                    leftover_tmps += 1;
                }
            }
        }
        assert_eq!(leftover_tmps, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validate_detects_corruption_and_missing() {
        let (dir, store) = temp_store("corrupt");
        let out = store.put(b"integrity", "fits").unwrap();
        // Flip a byte in the stored artifact.
        let mut bytes = std::fs::read(&out.path).unwrap();
        bytes[0] ^= 0xFF;
        std::fs::write(&out.path, bytes).unwrap();
        assert!(!store.validate(&out.hash, "fits"));
        // Missing file also fails validation (never silently ok).
        assert!(!store.validate(&out.hash, "tif"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
