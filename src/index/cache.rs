use anyhow::Result;
use std::path::{Path, PathBuf};

use super::state::IndexState;

pub struct SnapshotCache {
    pub cache_dir: PathBuf,
}

impl SnapshotCache {
    pub fn new(cache_dir: &Path) -> Self {
        SnapshotCache { cache_dir: cache_dir.to_path_buf() }
    }

    fn snapshot_path(&self, fid: u32, generation: u32) -> PathBuf {
        self.cache_dir
            .join("snapshots")
            .join(fid.to_string())
            .join(format!("gen{}.raw", generation))
    }

    pub fn store_snapshot(&self, fid: u32, generation: u32, raw_content: &str) -> Result<()> {
        let path = self.snapshot_path(fid, generation);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, raw_content)?;
        Ok(())
    }

    pub fn load_snapshot(&self, fid: u32, generation: u32) -> Result<Option<String>> {
        let path = self.snapshot_path(fid, generation);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(Some(content))
    }

    pub fn cleanup(&self, fid: u32, retain_last_n: u32) -> Result<()> {
        let dir = self.cache_dir.join("snapshots").join(fid.to_string());
        if !dir.exists() {
            return Ok(());
        }

        let mut gens: Vec<u32> = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if let Some(rest) = name.strip_prefix("gen")
                && let Some(n_str) = rest.strip_suffix(".raw")
                && let Ok(n) = n_str.parse::<u32>()
            {
                gens.push(n);
            }
        }

        gens.sort();
        let to_delete = if gens.len() > retain_last_n as usize {
            &gens[..gens.len() - retain_last_n as usize]
        } else {
            &[]
        };

        for generation in to_delete {
            let path = self.snapshot_path(fid, *generation);
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
        }

        Ok(())
    }

    pub fn cleanup_all(&self, index: &IndexState, retain_last_n: u32) -> Result<()> {
        for entry in index.files.values() {
            self.cleanup(entry.fid, retain_last_n)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_load_round_trip() {
        let dir = TempDir::new().unwrap();
        let cache = SnapshotCache::new(dir.path());
        cache.store_snapshot(1, 0, "hello world").unwrap();
        let loaded = cache.load_snapshot(1, 0).unwrap();
        assert_eq!(loaded, Some("hello world".to_string()));
    }

    #[test]
    fn test_load_missing_returns_none() {
        let dir = TempDir::new().unwrap();
        let cache = SnapshotCache::new(dir.path());
        let loaded = cache.load_snapshot(99, 0).unwrap();
        assert_eq!(loaded, None);
    }

    #[test]
    fn test_cleanup_retains_last_n() {
        let dir = TempDir::new().unwrap();
        let cache = SnapshotCache::new(dir.path());
        for generation in 0..5 {
            cache.store_snapshot(1, generation, &format!("gen{}", generation)).unwrap();
        }
        cache.cleanup(1, 3).unwrap();
        assert_eq!(cache.load_snapshot(1, 0).unwrap(), None);
        assert_eq!(cache.load_snapshot(1, 1).unwrap(), None);
        assert!(cache.load_snapshot(1, 2).unwrap().is_some());
        assert!(cache.load_snapshot(1, 3).unwrap().is_some());
        assert!(cache.load_snapshot(1, 4).unwrap().is_some());
    }

    #[test]
    fn test_cleanup_all() {
        let dir = TempDir::new().unwrap();
        let cache = SnapshotCache::new(dir.path());
        for generation in 0..3 {
            cache.store_snapshot(1, generation, "content").unwrap();
        }
        let mut index = IndexState::default();
        index.allocate_fid("file.rs", "file.rs");
        cache.cleanup_all(&index, 1).unwrap();
        assert_eq!(cache.load_snapshot(1, 0).unwrap(), None);
        assert_eq!(cache.load_snapshot(1, 1).unwrap(), None);
        assert!(cache.load_snapshot(1, 2).unwrap().is_some());
    }
}
