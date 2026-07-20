use jiff::Timestamp;

use super::state::{FileEntry, FileStatus, IndexState};

impl IndexState {
    pub fn allocate_fid(&mut self, display_path: &str, real_path: &str) -> u32 {
        if let Some(entry) = self.files.get_mut(display_path) {
            if entry.status == FileStatus::Inactive {
                entry.status = FileStatus::Active;
                entry.last_updated = Timestamp::now();
            }
            // Keep the mapping current if the root path changed between packs.
            if entry.real_path != real_path {
                entry.real_path = real_path.to_string();
            }
            return entry.fid;
        }

        let fid = self.next_fid;
        self.next_fid += 1;
        let now = Timestamp::now();
        self.files.insert(
            display_path.to_string(),
            FileEntry {
                fid,
                real_path: real_path.to_string(),
                current_hash: String::new(),
                current_gen: 0,
                current_pid: 0,
                status: FileStatus::Active,
                first_seen: now,
                last_updated: now,
            },
        );
        fid
    }

    pub fn deactivate(&mut self, display_path: &str) {
        if let Some(entry) = self.files.get_mut(display_path) {
            entry.status = FileStatus::Inactive;
            entry.last_updated = Timestamp::now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_file_gets_incremented_fid() {
        let mut state = IndexState::default();
        let fid1 = state.allocate_fid("project/src/a.rs", "src/a.rs");
        let fid2 = state.allocate_fid("project/src/b.rs", "src/b.rs");
        assert_eq!(fid1, 1);
        assert_eq!(fid2, 2);
        assert_eq!(state.next_fid, 3);
        assert_eq!(state.files["project/src/a.rs"].real_path, "src/a.rs");
    }

    #[test]
    fn test_existing_file_reuses_fid() {
        let mut state = IndexState::default();
        let fid1 = state.allocate_fid("src/a.rs", "src/a.rs");
        let fid2 = state.allocate_fid("src/a.rs", "src/a.rs");
        assert_eq!(fid1, fid2);
        assert_eq!(state.next_fid, 2);
    }

    #[test]
    fn test_existing_entry_updates_real_path_on_root_move() {
        let mut state = IndexState::default();
        let fid1 = state.allocate_fid("project/src/a.rs", "old-root/src/a.rs");
        let fid2 = state.allocate_fid("project/src/a.rs", "new-root/src/a.rs");

        assert_eq!(fid1, fid2);
        assert_eq!(state.files["project/src/a.rs"].real_path, "new-root/src/a.rs");
    }

    #[test]
    fn test_inactive_file_revived() {
        let mut state = IndexState::default();
        let fid1 = state.allocate_fid("src/a.rs", "src/a.rs");
        state.deactivate("src/a.rs");
        assert_eq!(state.files["src/a.rs"].status, FileStatus::Inactive);

        let fid2 = state.allocate_fid("src/a.rs", "src/a.rs");
        assert_eq!(fid1, fid2);
        assert_eq!(state.files["src/a.rs"].status, FileStatus::Active);
    }

    #[test]
    fn test_deactivate_nonexistent_is_noop() {
        let mut state = IndexState::default();
        state.deactivate("nonexistent");
        assert!(state.files.is_empty());
    }
}
