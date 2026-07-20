pub mod executor;
pub mod reject;
pub mod scanner;

use anyhow::Result;
use std::io::Read;
use std::path::Path;

use crate::cli::ApplyArgs;
use crate::config::Config;
use crate::encoding_layer::{build_content_pipeline, build_pipeline};
use crate::index::{cache::SnapshotCache, state::IndexState};

pub fn run_apply(config: &Config, args: &ApplyArgs) -> Result<()> {
    let input = match &args.file {
        Some(path) => std::fs::read_to_string(path)?,
        None => {
            let mut buf = String::new();
            std::io::stdin().read_to_string(&mut buf)?;
            buf
        }
    };

    let prefix = &config.global.prefix;
    let blocks = scanner::scan_blocks(&input, prefix);

    if blocks.is_empty() {
        println!("No patch/replace blocks found in input.");
        return Ok(());
    }

    println!("Found {} block(s) to apply.", blocks.len());

    let index_path = Path::new(&config.global.index_file);
    let mut index = IndexState::load(index_path)?;
    let cache = SnapshotCache::new(Path::new(&config.global.cache_dir));
    let pipeline = build_pipeline(&config.global);
    let content_pipeline = build_content_pipeline(&config.global);

    let result = executor::execute_apply(
        &blocks,
        &mut index,
        executor::ApplyContext {
            cache: &cache,
            full_pipeline: &pipeline,
            content_pipeline: &content_pipeline,
            prefix,
            anchor_interval: config.global.anchor_interval,
            dry_run: args.dry_run,
        },
    )?;

    if args.dry_run {
        println!(
            "[dry-run] Would apply {} block(s), reject {} block(s).",
            result.applied.len(),
            result.rejected.len()
        );
    } else {
        println!("Applied: {}", result.applied.len());
        println!("Rejected: {}", result.rejected.len());

        for a in &result.applied {
            if a.was_dirty {
                println!("  ~ {} (fid={}, dirty/fuzzy)", a.path, a.fid);
            } else {
                println!("  + {} (fid={})", a.path, a.fid);
            }
        }
        for r in &result.rejected {
            match &r.rej_file {
                Some(rej) => {
                    println!("  ! {} (fid={}): {} -> {}", r.path, r.fid, r.reason, rej.display());
                }
                None => println!("  ! {} (fid={}): {}", r.path, r.fid, r.reason),
            }
        }

        index.save(index_path)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::PackArgs;
    use crate::config::RootEntry;
    use tempfile::TempDir;

    /// Config with a labeled root, exercising the display_path != real_path case
    /// that the version-1 index conflated.
    fn labeled_config(dir: &std::path::Path) -> Config {
        let mut config = Config::default();
        config.global.index_file = dir.join(".ctx-index.yaml").to_string_lossy().into_owned();
        config.global.cache_dir = dir.join(".ctx-cache").to_string_lossy().into_owned();
        config.global.manifest = false;

        let profile = config.profiles.get_mut("default").unwrap();
        profile.roots = vec![RootEntry {
            path: dir.join("work").to_string_lossy().into_owned(),
            label: "project".to_string(),
        }];
        profile.discovery.use_gitignore = false;
        profile.output.file = dir.join("context.ctx").to_string_lossy().into_owned();
        profile.output.manifest = dir.join("context.ctx.manifest").to_string_lossy().into_owned();
        config
    }

    fn pack_full_args() -> PackArgs {
        PackArgs {
            profile: "default".to_string(),
            output: None,
            stdin: false,
            full: true,
            diff: false,
            auto: false,
        }
    }

    #[test]
    fn test_e2e_patch_applies_to_real_file_with_label() {
        let dir = TempDir::new().unwrap();
        let src_dir = dir.path().join("work/src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let main_rs = src_dir.join("main.rs");
        std::fs::write(&main_rs, "fn main() {\n    let x = 1;\n}\n").unwrap();

        let config = labeled_config(dir.path());
        crate::pack::run_pack(&config, &pack_full_args()).unwrap();

        let index = IndexState::load(Path::new(&config.global.index_file)).unwrap();
        let fid = index.files["project/src/main.rs"].fid;

        let llm_response = format!(
            "<ctx:patch fid=\"{fid}\" gen=\"0\" pid=\"1\">\n@@ anchor:1 @@\n [0]fn main() {{\n-[4]let x = 1;\n+[4]let x = 2;\n [0]}}\n</ctx:patch>\n"
        );
        let response_path = dir.path().join("llm.txt");
        std::fs::write(&response_path, llm_response).unwrap();

        run_apply(&config, &crate::cli::ApplyArgs { file: Some(response_path), dry_run: false })
            .unwrap();

        assert_eq!(std::fs::read_to_string(&main_rs).unwrap(), "fn main() {\n    let x = 2;\n}\n");
        // The label must never materialize as a directory anywhere.
        assert!(!dir.path().join("project").exists());
        assert!(!Path::new("project").exists());
    }

    #[test]
    fn test_e2e_replace_applies_to_real_file_with_label() {
        let dir = TempDir::new().unwrap();
        let src_dir = dir.path().join("work/src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let lib_rs = src_dir.join("lib.rs");
        std::fs::write(&lib_rs, "pub fn helper() -> i32 {\n    42\n}\n").unwrap();

        let config = labeled_config(dir.path());
        crate::pack::run_pack(&config, &pack_full_args()).unwrap();

        let index = IndexState::load(Path::new(&config.global.index_file)).unwrap();
        let fid = index.files["project/src/lib.rs"].fid;

        let llm_response = format!(
            "<ctx:replace fid=\"{fid}\" gen=\"1\">\n[0]pub fn helper() -> i32 {{\n[4]43\n[0]}}\n</ctx:replace>\n"
        );
        let response_path = dir.path().join("llm.txt");
        std::fs::write(&response_path, llm_response).unwrap();

        run_apply(&config, &crate::cli::ApplyArgs { file: Some(response_path), dry_run: false })
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(&lib_rs).unwrap(),
            "pub fn helper() -> i32 {\n    43\n}\n"
        );
        assert!(!dir.path().join("project").exists());
        assert!(!Path::new("project").exists());
    }
}
