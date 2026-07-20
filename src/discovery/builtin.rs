use anyhow::Result;
use globset::{Glob, GlobSetBuilder};
use std::path::PathBuf;

use crate::config::Profile;

pub fn discover_builtin(profile: &Profile) -> Result<Vec<(PathBuf, String)>> {
    let mut include_builder = GlobSetBuilder::new();
    let mut exclude_builder = GlobSetBuilder::new();

    let has_includes = !profile.discovery.include.is_empty();
    for pattern in &profile.discovery.include {
        include_builder.add(Glob::new(pattern)?);
    }
    for pattern in &profile.discovery.exclude {
        exclude_builder.add(Glob::new(pattern)?);
    }

    let include_set = include_builder.build()?;
    let exclude_set = exclude_builder.build()?;

    let mut results: Vec<(PathBuf, String)> = Vec::new();

    for root in &profile.roots {
        let root_path = PathBuf::from(&root.path);
        let walker = ignore::WalkBuilder::new(&root_path)
            .git_ignore(profile.discovery.use_gitignore)
            .git_global(profile.discovery.use_gitignore)
            .git_exclude(profile.discovery.use_gitignore)
            .build();

        for entry in walker {
            let entry = entry?;
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                let abs_path = entry.path().to_path_buf();
                let rel = match abs_path.strip_prefix(&root_path) {
                    Ok(r) => r.to_string_lossy().replace('\\', "/"),
                    Err(_) => abs_path.to_string_lossy().replace('\\', "/"),
                };

                if has_includes && !include_set.is_match(&rel) {
                    continue;
                }
                if exclude_set.is_match(&rel) {
                    continue;
                }

                let display_path = if root.label.is_empty() {
                    rel.clone()
                } else {
                    format!("{}/{}", root.label, rel)
                };

                results.push((abs_path, display_path));
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        DiscoveryConfig, ExtractionConfig, OutputConfig, Profile, RootEntry, VersioningConfig,
    };
    use std::fs;
    use tempfile::TempDir;

    fn make_profile(root: &str, include: Vec<String>, exclude: Vec<String>) -> Profile {
        Profile {
            roots: vec![RootEntry { path: root.to_string(), label: "test".to_string() }],
            discovery: DiscoveryConfig {
                use_gitignore: false,
                include,
                exclude,
                stdin_merge: false,
            },
            extraction: ExtractionConfig::default(),
            versioning: VersioningConfig::default(),
            output: OutputConfig::default(),
        }
    }

    #[test]
    fn test_discover_basic() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.rs"), "fn a() {}").unwrap();
        fs::write(dir.path().join("b.txt"), "text").unwrap();

        let profile = make_profile(dir.path().to_str().unwrap(), vec![], vec![]);
        let files = discover_builtin(&profile).unwrap();
        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_discover_include_filter() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.rs"), "fn a() {}").unwrap();
        fs::write(dir.path().join("b.txt"), "text").unwrap();

        let profile = make_profile(dir.path().to_str().unwrap(), vec!["*.rs".to_string()], vec![]);
        let files = discover_builtin(&profile).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].1.ends_with("a.rs"));
    }

    #[test]
    fn test_discover_exclude_filter() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.rs"), "fn a() {}").unwrap();
        fs::write(dir.path().join("b.txt"), "text").unwrap();

        let profile = make_profile(dir.path().to_str().unwrap(), vec![], vec!["*.txt".to_string()]);
        let files = discover_builtin(&profile).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].1.ends_with("a.rs"));
    }
}
