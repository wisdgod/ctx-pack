pub mod builtin;
pub mod stdin;

use anyhow::Result;
use std::collections::HashSet;
use std::path::PathBuf;

use crate::config::Profile;

#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    pub absolute_path: PathBuf,
    pub display_path: String,
}

impl DiscoveredFile {
    /// The path recorded in the index for apply to read/write. Must be exactly
    /// the path pack reads from, so apply writes where pack read.
    pub fn real_path_string(&self) -> String {
        self.absolute_path.to_string_lossy().into_owned()
    }
}

pub fn discover(profile: &Profile) -> Result<Vec<DiscoveredFile>> {
    let mut seen = HashSet::new();
    let mut files = Vec::new();

    let builtin_files = builtin::discover_builtin(profile)?;
    for (abs_path, display_path) in builtin_files {
        let canonical = abs_path.canonicalize().unwrap_or(abs_path.clone());
        if seen.insert(canonical) {
            files.push(DiscoveredFile { absolute_path: abs_path, display_path });
        }
    }

    if profile.discovery.stdin_merge {
        let stdin_files = stdin::discover_stdin()?;
        for path in stdin_files {
            let canonical = path.canonicalize().unwrap_or(path.clone());
            if seen.insert(canonical) {
                let display = path.to_string_lossy().replace('\\', "/");
                files.push(DiscoveredFile { absolute_path: path, display_path: display });
            }
        }
    }

    files.sort_by(|a, b| a.display_path.cmp(&b.display_path));
    Ok(files)
}
