pub mod manifest;
pub mod output;
pub mod prompt;
pub mod tree;

use anyhow::Result;
use std::path::Path;

use crate::cli::{CacheCleanArgs, PackArgs, PromptArgs, StatusArgs, TreeArgs};
use crate::config::Config;
use crate::discovery::discover;
use crate::index::{cache::SnapshotCache, compute_hash, state::IndexState};

use output::{pack_auto, pack_full, pack_incremental};

pub fn run_pack(config: &Config, args: &PackArgs) -> Result<()> {
    let profile_name = &args.profile;
    let effective_config;
    let config = if args.stdin {
        effective_config = config_with_stdin_merge(config, profile_name);
        &effective_config
    } else {
        config
    };

    let output = if args.full {
        pack_full(config, profile_name)?
    } else if args.diff {
        pack_incremental(config, profile_name)?
    } else {
        pack_auto(config, profile_name)?
    };

    let profile = config.get_profile(profile_name)?;
    let out_path = args.output.as_deref().unwrap_or_else(|| Path::new(&profile.output.file));

    std::fs::write(out_path, &output.content)?;
    tracing::info!("wrote output to {}", out_path.display());

    if config.global.manifest {
        // Manifest follows the actual output location: with -o it sits next to
        // the override instead of clobbering the profile's default manifest.
        let manifest_path = match &args.output {
            Some(out) => {
                let mut name = out.as_os_str().to_os_string();
                name.push(".manifest");
                std::path::PathBuf::from(name)
            }
            None => std::path::PathBuf::from(&profile.output.manifest),
        };
        manifest::write_manifest(&output, config, profile_name, out_path, &manifest_path)?;
        tracing::info!("wrote manifest to {}", manifest_path.display());
    }

    println!("Packed {} blocks to {}", output.blocks.len(), out_path.display());
    Ok(())
}

fn config_with_stdin_merge(config: &Config, profile_name: &str) -> Config {
    let mut config = config.clone();
    if let Some(profile) = config.profiles.get_mut(profile_name) {
        profile.discovery.stdin_merge = true;
    }
    config
}

pub fn run_status(config: &Config, args: &StatusArgs) -> Result<()> {
    let profile = config.get_profile(&args.profile)?;
    let files = discover(profile)?;

    let index = IndexState::load(Path::new(&config.global.index_file))?;

    let current_map: std::collections::HashMap<String, _> =
        files.iter().map(|f| (f.display_path.clone(), f)).collect();

    let mut new_files = Vec::new();
    let mut deleted_files = Vec::new();
    let mut modified_files = Vec::new();
    let mut unchanged_files = Vec::new();

    for file in &files {
        match index.files.get(&file.display_path) {
            None => new_files.push(file.display_path.clone()),
            Some(entry) => {
                let content = match std::fs::read_to_string(&file.absolute_path) {
                    Ok(c) => c,
                    Err(_) => {
                        new_files.push(file.display_path.clone());
                        continue;
                    }
                };
                let hash = compute_hash(&content);
                if hash == entry.current_hash {
                    unchanged_files.push(file.display_path.clone());
                } else {
                    modified_files.push(file.display_path.clone());
                }
            }
        }
    }

    for (path, entry) in &index.files {
        if !current_map.contains_key(path)
            && entry.status == crate::index::state::FileStatus::Active
        {
            deleted_files.push(path.clone());
        }
    }

    println!("Status for profile '{}':", args.profile);
    println!("  New:      {}", new_files.len());
    println!("  Modified: {}", modified_files.len());
    println!("  Deleted:  {}", deleted_files.len());
    println!("  Unchanged:{}", unchanged_files.len());

    for f in &new_files {
        println!("  + {}", f);
    }
    for f in &modified_files {
        println!("  M {}", f);
    }
    for f in &deleted_files {
        println!("  D {}", f);
    }

    Ok(())
}

pub fn run_tree(config: &Config, args: &TreeArgs) -> Result<()> {
    let profile = config.get_profile(&args.profile)?;
    let files = discover(profile)?;
    let index = IndexState::load(Path::new(&config.global.index_file))?;

    let file_versions: Vec<tree::FileVersion> = files
        .iter()
        .filter_map(|f| {
            let fid = index.files.get(&f.display_path).map(|e| e.fid)?;
            let entry = index.files.get(&f.display_path)?;
            Some(tree::FileVersion {
                fid,
                display_path: f.display_path.clone(),
                generation: entry.current_gen,
                pid: entry.current_pid,
            })
        })
        .collect();

    if file_versions.is_empty() {
        // No index yet, just list with fid=0
        for (i, f) in files.iter().enumerate() {
            println!("[{}] {} (gen0)", i + 1, f.display_path);
        }
    } else {
        print!("{}", tree::generate_tree(&file_versions));
    }

    Ok(())
}

pub fn run_prompt(config: &Config, args: &PromptArgs) -> Result<()> {
    let _profile = config.get_profile(&args.profile)?;
    let prompt = prompt::generate_prompt(&config.global);
    println!("{}", prompt);
    Ok(())
}

pub fn run_cache_clean(config: &Config, args: &CacheCleanArgs) -> Result<()> {
    let _profile = config.get_profile(&args.profile)?;
    let index = IndexState::load(Path::new(&config.global.index_file))?;
    let cache = SnapshotCache::new(Path::new(&config.global.cache_dir));
    cache.cleanup_all(&index, config.global.cache_retention)?;
    println!("Cache cleaned (profile: {})", args.profile);
    Ok(())
}

pub fn run_cache_info(config: &Config) -> Result<()> {
    let cache_dir = Path::new(&config.global.cache_dir);
    if !cache_dir.exists() {
        println!("Cache directory does not exist: {}", cache_dir.display());
        return Ok(());
    }

    let mut total_size: u64 = 0;
    let mut snapshot_count: u64 = 0;

    let snapshots_dir = cache_dir.join("snapshots");
    if snapshots_dir.exists() {
        for entry in std::fs::read_dir(&snapshots_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                for snap in std::fs::read_dir(entry.path())? {
                    let snap = snap?;
                    total_size += snap.metadata()?.len();
                    snapshot_count += 1;
                }
            }
        }
    }

    println!("Cache directory: {}", cache_dir.display());
    println!("Snapshots: {}", snapshot_count);
    println!("Total size: {} bytes", total_size);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::PackArgs;
    use crate::config::RootEntry;
    use tempfile::TempDir;

    #[test]
    fn test_manifest_follows_output_override() {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("work")).unwrap();
        std::fs::write(dir.path().join("work/a.rs"), "fn a() {}\n").unwrap();

        let mut config = Config::default();
        config.global.index_file =
            dir.path().join(".ctx-index.yaml").to_string_lossy().into_owned();
        config.global.cache_dir = dir.path().join(".ctx-cache").to_string_lossy().into_owned();
        let profile = config.profiles.get_mut("default").unwrap();
        profile.roots = vec![RootEntry {
            path: dir.path().join("work").to_string_lossy().into_owned(),
            label: "project".to_string(),
        }];
        profile.discovery.use_gitignore = false;
        profile.output.file = dir.path().join("context.ctx").to_string_lossy().into_owned();
        profile.output.manifest =
            dir.path().join("default.manifest").to_string_lossy().into_owned();

        let override_out = dir.path().join("update.ctx");
        let args = PackArgs {
            profile: "default".to_string(),
            output: Some(override_out.clone()),
            stdin: false,
            full: true,
            diff: false,
            auto: false,
        };
        run_pack(&config, &args).unwrap();

        assert!(override_out.exists());
        assert!(dir.path().join("update.ctx.manifest").exists());
        assert!(!dir.path().join("default.manifest").exists());
    }
}
