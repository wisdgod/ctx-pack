use anyhow::Result;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Active,
    Inactive,
}

/// On-disk index format version.
pub const INDEX_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub fid: u32,
    /// Filesystem path used to read/write this file, as produced by discovery.
    /// Distinct from the index key, which is the label-prefixed display path.
    pub real_path: String,
    pub current_hash: String,
    pub current_gen: u32,
    pub current_pid: u32,
    pub status: FileStatus,
    pub first_seen: Timestamp,
    pub last_updated: Timestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexState {
    pub version: u32,
    pub files: HashMap<String, FileEntry>,
    pub next_fid: u32,
}

impl Default for IndexState {
    fn default() -> Self {
        IndexState { version: INDEX_VERSION, files: HashMap::new(), next_fid: 1 }
    }
}

impl IndexState {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let state: IndexState = serde_yaml::from_str(&content)
            .map_err(|e| anyhow::anyhow!("failed to parse index: {}", e))?;
        Ok(state)
    }

    /// Atomic save: write to a sibling temp file, then rename over the target,
    /// so a crash mid-write cannot corrupt the index.
    pub fn save(&self, path: &Path) -> Result<()> {
        let yaml = serde_yaml::to_string(self)?;
        let mut tmp_name = path.as_os_str().to_os_string();
        tmp_name.push(".tmp");
        let tmp_path = std::path::PathBuf::from(tmp_name);
        std::fs::write(&tmp_path, yaml)?;
        std::fs::rename(&tmp_path, path)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_round_trip() {
        let mut state = IndexState::default();
        let now = Timestamp::now();
        state.files.insert(
            "src/main.rs".to_string(),
            FileEntry {
                fid: 1,
                real_path: "src/main.rs".to_string(),
                current_hash: "sha256:abc".to_string(),
                current_gen: 0,
                current_pid: 0,
                status: FileStatus::Active,
                first_seen: now,
                last_updated: now,
            },
        );
        state.next_fid = 2;

        let f = NamedTempFile::new().unwrap();
        state.save(f.path()).unwrap();
        let loaded = IndexState::load(f.path()).unwrap();
        assert_eq!(loaded.next_fid, 2);
        assert!(loaded.files.contains_key("src/main.rs"));
    }

    #[test]
    fn test_load_missing_returns_default() {
        let state = IndexState::load(Path::new("/nonexistent/path.yaml")).unwrap();
        assert_eq!(state.next_fid, 1);
        assert!(state.files.is_empty());
    }
}
