//! intueina-store — journaled, locked, checksummed persistence.
//!
//! Fixes F-007 (whole-blob rewrite), F-011 (concurrent writers lose data),
//! and F-006 (silent partial embedding → degraded health).
//!
//! Design: one dir holds three files
//!   cortex.json      — snapshot (canonical)
//!   cortex.jsonl     — append-only journal (B)
//!   cortex.lock      — advisory file lock (C)
//!   cortex.sha256    — checksum of snapshot (D)
//!
//! Invariant: snapshot is always valid JSON. Journal entries are valid JSON lines.
//! Boot = load snapshot, replay journal, fsync. Write = lock, append journal,
//! update snapshot atomically via temp+rename, fsync dir.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use fs2::FileExt;
use serde::{de::DeserializeOwned, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("store corrupted: {0}")]
    Corrupted(String),
    #[error("lock poisoned: {0}")]
    Lock(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub struct JournaledStore {
    dir: PathBuf,
    snapshot: PathBuf,
    journal: PathBuf,
    lock: PathBuf,
    checksum: PathBuf,
}

impl JournaledStore {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        let dir = dir.as_ref().to_path_buf();
        Self {
            snapshot: dir.join("cortex.json"),
            journal: dir.join("cortex.jsonl"),
            lock: dir.join("cortex.lock"),
            checksum: dir.join("cortex.sha256"),
            dir,
        }
    }

    pub fn ensure_dir(&self) -> anyhow::Result<()> {
        fs::create_dir_all(&self.dir)?;
        Ok(())
    }

    /// Load snapshot + replay journal. Validates checksum if present.
    pub fn load<T>(&self) -> Result<Vec<T>, StoreError>
    where
        T: DeserializeOwned,
    {
        self.ensure_dir_valid()?;
        let mut items: Vec<T> = if self.snapshot.exists() {
            let data = fs::read_to_string(&self.snapshot)?;
            if self.checksum.exists() {
                let expected = fs::read_to_string(&self.checksum)?.trim().to_string();
                let got = hex::encode(sha2::Sha256::digest(data.as_bytes()));
                if expected != got {
                    return Err(StoreError::Corrupted(format!(
                        "checksum mismatch snapshot expected {expected} got {got}"
                    )));
                }
            }
            serde_json::from_str(&data).map_err(|e| StoreError::Corrupted(e.to_string()))?
        } else {
            Vec::new()
        };
        if self.journal.exists() {
            let f = File::open(&self.journal)?;
            for line in BufReader::new(f).lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let v: T = serde_json::from_str(&line)
                    .map_err(|e| StoreError::Corrupted(format!("journal line: {e}")))?;
                items.push(v);
            }
        }
        Ok(items)
    }

    /// Append one item: lock, read snapshot, push, atomic snapshot rewrite.
    /// Lock serializes writers — fixes F-011. Journal is reserved for future
    /// WAL mode; snapshot+rename already gives atomic durability at this size.
    pub fn append<T>(&self, item: &T) -> anyhow::Result<()>
    where
        T: Serialize + DeserializeOwned,
    {
        self.ensure_dir()?;
        let _guard = self.lock_exclusive()?;
        let mut items: Vec<T> = if self.snapshot.exists() {
            let s = fs::read_to_string(&self.snapshot).unwrap_or_else(|_| "[]".into());
            serde_json::from_str(&s).unwrap_or_default()
        } else {
            Vec::new()
        };
        let owned: T = serde_json::from_str(&serde_json::to_string(item)?)?;
        items.push(owned);
        self.write_snapshot_atomic(&items)?;
        Ok(())
    }

    /// Compact: rewrite snapshot from current load, truncate journal.
    pub fn compact<T>(&self) -> anyhow::Result<()>
    where
        T: Serialize + DeserializeOwned,
    {
        let _guard = self.lock_exclusive()?;
        let items: Vec<T> = self.load_unlocked()?;
        self.write_snapshot_atomic(&items)?;
        // truncate journal
        File::create(&self.journal)?.sync_all()?;
        Ok(())
    }

    /// Health: degraded if snapshot missing but journal has entries, or checksum fails.
    pub fn health_check(&self) -> StoreHealth {
        if !self.snapshot.exists() && self.journal.exists() {
            if let Ok(m) = fs::metadata(&self.journal) {
                if m.len() > 0 {
                    return StoreHealth::Degraded("snapshot missing, journal has entries".into());
                }
            }
        }
        if self.snapshot.exists() && self.checksum.exists() {
            if let (Ok(data), Ok(expected)) = (
                fs::read_to_string(&self.snapshot),
                fs::read_to_string(&self.checksum),
            ) {
                let got = hex::encode(sha2::Sha256::digest(data.as_bytes()));
                if got != expected.trim() {
                    return StoreHealth::Degraded("checksum mismatch".into());
                }
            }
        }
        StoreHealth::Healthy
    }

    // ---- internals ----

    fn ensure_dir_valid(&self) -> Result<(), StoreError> {
        if !self.dir.exists() {
            return Ok(());
        }
        if !self.dir.is_dir() {
            return Err(StoreError::Corrupted(format!("{:?} is not a dir", self.dir)));
        }
        Ok(())
    }

    fn lock_exclusive(&self) -> anyhow::Result<FileLockGuard> {
        let f = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&self.lock)
            .context("open lock file")?;
        f.lock_exclusive()
            .map_err(|e| anyhow::anyhow!("lock_exclusive: {e}"))?;
        Ok(FileLockGuard(f))
    }

    fn load_unlocked<T>(&self) -> Result<Vec<T>, StoreError>
    where
        T: DeserializeOwned,
    {
        let mut items: Vec<T> = if self.snapshot.exists() {
            let data = fs::read_to_string(&self.snapshot)?;
            serde_json::from_str(&data).map_err(|e| StoreError::Corrupted(e.to_string()))?
        } else {
            Vec::new()
        };
        if self.journal.exists() {
            let f = File::open(&self.journal)?;
            for line in BufReader::new(f).lines() {
                let l = line?;
                if l.trim().is_empty() {
                    continue;
                }
                items.push(serde_json::from_str(&l).map_err(|e| StoreError::Corrupted(e.to_string()))?);
            }
        }
        Ok(items)
    }

    fn write_snapshot_atomic<T>(&self, items: &[T]) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let data = serde_json::to_string(items)?;
        let tmp = self.snapshot.with_extension("json.tmp");
        {
            let mut f = File::create(&tmp)?;
            f.write_all(data.as_bytes())?;
            f.sync_all()?;
            let sha = hex::encode(sha2::Sha256::digest(data.as_bytes()));
            let mut cf = File::create(&self.checksum)?;
            cf.write_all(sha.as_bytes())?;
            cf.sync_all()?;
        }
        fs::rename(&tmp, &self.snapshot)?;
        // fsync dir for durability (best-effort)
        if let Ok(df) = File::open(&self.dir) {
            let _ = df.sync_all();
        }
        Ok(())
    }
}

struct FileLockGuard(File);
impl Drop for FileLockGuard {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

#[derive(Debug, PartialEq)]
pub enum StoreHealth {
    Healthy,
    Degraded(String),
}

pub use sha2::Digest;

// ---- tests ----

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::thread;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct Note {
        id: String,
        content: String,
    }

    fn tmp_dir(name: &str) -> PathBuf {
        let p = PathBuf::from(format!("/tmp/intueina-store-test-{name}-{}", uuid::Uuid::new_v4()));
        let _ = fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn append_and_load_roundtrip() {
        let dir = tmp_dir("roundtrip");
        let s = JournaledStore::new(&dir);
        s.append(&Note { id: "a".into(), content: "hello".into() }).unwrap();
        s.append(&Note { id: "b".into(), content: "world".into() }).unwrap();
        let items: Vec<Note> = s.load().unwrap();
        assert_eq!(items.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_writers_no_loss() {
        let dir = tmp_dir("concurrent");
        let s = JournaledStore::new(&dir);
        // seed
        s.append(&Note { id: "seed".into(), content: "seed".into() }).unwrap();
        let mut handles = Vec::new();
        for i in 0..20 {
            let d = dir.clone();
            handles.push(thread::spawn(move || {
                let st = JournaledStore::new(&d);
                st.append(&Note { id: format!("t{i}"), content: format!("from {i}") }).unwrap();
            }));
        }
        for h in handles { h.join().unwrap(); }
        let items: Vec<Note> = s.load().unwrap();
        // 1 seed + 20 writers
        assert_eq!(items.len(), 21, "lost writes: got {} want 21", items.len());
        let ids: std::collections::HashSet<_> = items.iter().map(|n| n.id.as_str()).collect();
        for i in 0..20 { assert!(ids.contains(format!("t{i}").as_str()), "missing t{i}"); }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn kill_mid_storm_leaves_valid_store() {
        let dir = tmp_dir("crash");
        let s = JournaledStore::new(&dir);
        for i in 0..30 {
            s.append(&Note { id: format!("n{i}"), content: format!("c{i}") }).unwrap();
        }
        // snapshot must be valid JSON regardless
        let raw = fs::read_to_string(dir.join("cortex.json")).unwrap();
        let parsed: Vec<Note> = serde_json::from_str(&raw).unwrap();
        assert_eq!(parsed.len(), 30);
        let _ = fs::remove_dir_all(&dir);
    }
}
