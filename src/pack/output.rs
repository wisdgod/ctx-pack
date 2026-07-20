use anyhow::Result;

use crate::config::{Config, Profile, SizePolicy};
use crate::detection::load_file_content;
use crate::discovery::discover;
use crate::encoding_layer::build_pipeline;
use crate::extraction::{extract, match_rule};
use crate::index::{cache::SnapshotCache, compute_hash, state::IndexState};

use super::prompt::generate_prompt;
use super::tree::{FileVersion, generate_tree};

#[derive(Debug, Clone, PartialEq)]
pub enum BlockType {
    Prompt,
    Tree,
    File,
    Patch,
    Replace,
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub block_type: BlockType,
    pub fid: Option<u32>,
    pub generation: u32,
    pub pid: Option<u32>,
    pub path: Option<String>,
    pub byte_start: u64,
    pub byte_end: u64,
    pub line_start: u32,
    pub line_end: u32,
    pub content_hash: Option<String>,
}

#[derive(Debug)]
pub struct PackOutput {
    pub content: String,
    pub blocks: Vec<BlockInfo>,
}

struct OutputBuilder {
    content: String,
    blocks: Vec<BlockInfo>,
    current_line: u32,
}

#[derive(Debug, Clone)]
struct BlockMeta {
    block_type: BlockType,
    fid: Option<u32>,
    generation: u32,
    pid: Option<u32>,
    path: Option<String>,
    content_hash: Option<String>,
}

impl BlockMeta {
    fn new(block_type: BlockType) -> Self {
        Self { block_type, fid: None, generation: 0, pid: None, path: None, content_hash: None }
    }

    fn file(fid: u32, generation: u32, path: String, content_hash: String) -> Self {
        Self {
            block_type: BlockType::File,
            fid: Some(fid),
            generation,
            pid: None,
            path: Some(path),
            content_hash: Some(content_hash),
        }
    }

    fn patch(fid: u32, generation: u32, pid: u32, path: String) -> Self {
        Self {
            block_type: BlockType::Patch,
            fid: Some(fid),
            generation,
            pid: Some(pid),
            path: Some(path),
            content_hash: None,
        }
    }

    fn replace(fid: u32, generation: u32, path: String, content_hash: String) -> Self {
        Self {
            block_type: BlockType::Replace,
            fid: Some(fid),
            generation,
            pid: None,
            path: Some(path),
            content_hash: Some(content_hash),
        }
    }
}

struct PreparedBlock {
    meta: BlockMeta,
    content: String,
}

struct ExtractedContent {
    content: String,
    is_partial: bool,
}

impl OutputBuilder {
    fn new() -> Self {
        OutputBuilder { content: String::new(), blocks: Vec::new(), current_line: 1 }
    }

    fn append_block(&mut self, meta: BlockMeta, text: &str) {
        let byte_start = self.content.len() as u64;
        let line_start = self.current_line;
        self.content.push_str(text);
        if !text.ends_with('\n') {
            self.content.push('\n');
        }
        let byte_end = self.content.len() as u64;
        let added_lines = text.lines().count() as u32;
        let line_end = line_start + added_lines.saturating_sub(1);
        self.current_line = line_end + 2; // +1 for blank separator

        self.blocks.push(BlockInfo {
            block_type: meta.block_type,
            fid: meta.fid,
            generation: meta.generation,
            pid: meta.pid,
            path: meta.path,
            byte_start,
            byte_end,
            line_start,
            line_end,
            content_hash: meta.content_hash,
        });
    }

    fn finish(self) -> PackOutput {
        PackOutput { content: self.content, blocks: self.blocks }
    }
}

fn extracted_content(
    content: &str,
    display_path: &str,
    profile: &Profile,
) -> Result<ExtractedContent> {
    let rule =
        match_rule(display_path, &profile.extraction.rules, &profile.extraction.default_mode);
    let extraction = extract(content, rule)?;
    let is_partial = extraction.is_partial;

    if is_partial {
        let ranges: Vec<(u32, u32)> = extraction
            .fragments
            .iter()
            .map(|fragment| (fragment.line_start, fragment.line_end))
            .collect();
        tracing::debug!(
            path = display_path,
            total_lines = extraction.total_lines,
            ?ranges,
            "partial extraction"
        );
    }

    let total_len = extraction.fragments.iter().map(|fragment| fragment.content.len()).sum();
    let mut content = String::with_capacity(total_len);
    for fragment in extraction.fragments {
        content.push_str(&fragment.content);
    }
    Ok(ExtractedContent { content, is_partial })
}

fn file_block_open_tag(
    prefix: &str,
    fid: u32,
    generation: u32,
    path: &str,
    is_partial: bool,
) -> String {
    let extraction_attr = if is_partial { " extraction=\"partial\"" } else { "" };
    format!("<{prefix}:file id=\"{fid}\" gen=\"{generation}\" path=\"{path}\"{extraction_attr}>")
}

pub fn pack_full(config: &Config, profile_name: &str) -> Result<PackOutput> {
    let profile = config.get_profile(profile_name)?;
    let pipeline = build_pipeline(&config.global);
    let prefix = &config.global.prefix;

    let mut index = IndexState::load(std::path::Path::new(&config.global.index_file))?;
    let cache = SnapshotCache::new(std::path::Path::new(&config.global.cache_dir));

    let files = discover(profile)?;

    let mut builder = OutputBuilder::new();
    let mut file_versions: Vec<FileVersion> = Vec::new();
    let mut file_blocks: Vec<PreparedBlock> = Vec::new();

    let mut total_size: u64 = 0;

    for file in &files {
        let content = match load_file_content(&file.absolute_path, &config.global)? {
            Some(c) => c,
            None => continue,
        };

        let extracted = extracted_content(&content, &file.display_path, profile)?;

        // Check single file size
        let file_size = extracted.content.len() as u64;
        match config.global.size_policy {
            SizePolicy::Warn if file_size > config.global.max_file_size => {
                tracing::warn!(
                    "file {} ({} bytes) exceeds max_file_size ({} bytes)",
                    file.display_path,
                    file_size,
                    config.global.max_file_size
                );
            }
            SizePolicy::Abort if file_size > config.global.max_file_size => {
                anyhow::bail!(
                    "file {} ({} bytes) exceeds max_file_size ({} bytes)",
                    file.display_path,
                    file_size,
                    config.global.max_file_size
                );
            }
            _ => {}
        }

        let encoded = pipeline.encode_all(&extracted.content);
        let hash = compute_hash(&content);

        let fid = index.allocate_fid(&file.display_path, &file.real_path_string());
        // Reset gen to 0 for full pack
        if let Some(entry) = index.files.get_mut(&file.display_path) {
            entry.current_gen = 0;
            entry.current_pid = 0;
            entry.current_hash = hash.clone();
        }

        cache.store_snapshot(fid, 0, &content)?;
        total_size += encoded.len() as u64;

        file_versions.push(FileVersion {
            fid,
            display_path: file.display_path.clone(),
            generation: 0,
            pid: 0,
        });
        let open_tag =
            file_block_open_tag(prefix, fid, 0, &file.display_path, extracted.is_partial);
        let file_block = format!("{open_tag}\n{encoded}</{p}:file>", p = prefix);
        file_blocks.push(PreparedBlock {
            meta: BlockMeta::file(fid, 0, file.display_path.clone(), hash),
            content: file_block,
        });
    }

    // Check total size
    match config.global.size_policy {
        SizePolicy::Warn if total_size > config.global.max_content_size => {
            tracing::warn!(
                "total output size ({} bytes) exceeds max_content_size ({} bytes)",
                total_size,
                config.global.max_content_size
            );
        }
        SizePolicy::Abort if total_size > config.global.max_content_size => {
            anyhow::bail!(
                "total output size ({} bytes) exceeds max_content_size ({} bytes)",
                total_size,
                config.global.max_content_size
            );
        }
        _ => {}
    }

    // Generate prompt
    if config.global.prompt_generation {
        let prompt_text = generate_prompt(&config.global);
        let prompt_block = format!("<{p}:prompt>\n{}\n</{p}:prompt>", prompt_text, p = prefix);
        builder.append_block(BlockMeta::new(BlockType::Prompt), &prompt_block);
    }

    // Generate tree
    let tree_text = generate_tree(&file_versions);
    let tree_block = format!("<{p}:tree>\n{}</{p}:tree>", tree_text, p = prefix);
    builder.append_block(BlockMeta::new(BlockType::Tree), &tree_block);

    // Generate file blocks
    for block in file_blocks {
        builder.append_block(block.meta, &block.content);
    }

    index.save(std::path::Path::new(&config.global.index_file))?;

    Ok(builder.finish())
}

pub fn pack_incremental(config: &Config, profile_name: &str) -> Result<PackOutput> {
    let profile = config.get_profile(profile_name)?;
    let pipeline = build_pipeline(&config.global);
    let prefix = &config.global.prefix;

    let index_path = std::path::Path::new(&config.global.index_file);
    let mut index = IndexState::load(index_path)?;
    let cache = SnapshotCache::new(std::path::Path::new(&config.global.cache_dir));

    let files = discover(profile)?;

    let mut builder = OutputBuilder::new();
    let mut file_versions: Vec<FileVersion> = Vec::new();
    let mut file_blocks: Vec<PreparedBlock> = Vec::new();

    let current_display_paths: std::collections::HashSet<String> =
        files.iter().map(|f| f.display_path.clone()).collect();

    // Mark files in index that are no longer discovered as inactive
    let inactive_paths: Vec<String> =
        index.files.keys().filter(|p| !current_display_paths.contains(*p)).cloned().collect();
    for path in inactive_paths {
        index.deactivate(&path);
    }

    for file in &files {
        let content = match load_file_content(&file.absolute_path, &config.global)? {
            Some(c) => c,
            None => continue,
        };

        let extracted = extracted_content(&content, &file.display_path, profile)?;

        let new_hash = compute_hash(&content);
        let new_encoded = pipeline.encode_all(&extracted.content);

        let fid = index.allocate_fid(&file.display_path, &file.real_path_string());
        let entry = index.files.get(&file.display_path).cloned();

        if let Some(entry) = entry {
            if entry.current_hash == new_hash {
                // Unchanged
                file_versions.push(FileVersion {
                    fid,
                    display_path: file.display_path.clone(),
                    generation: entry.current_gen,
                    pid: entry.current_pid,
                });
                continue;
            }

            // File changed - load old snapshot
            if let Some(old_raw) = cache.load_snapshot(fid, entry.current_gen)? {
                let old_extracted = extracted_content(&old_raw, &file.display_path, profile)?;
                let old_encoded = pipeline.encode_all(&old_extracted.content);
                let action = crate::version::determine_action(
                    fid,
                    &old_encoded,
                    &new_encoded,
                    crate::version::VersionContext {
                        current_gen: entry.current_gen,
                        current_pid: entry.current_pid,
                        config: &profile.versioning,
                        prefix,
                        anchor_interval: config.global.anchor_interval,
                    },
                );

                match action {
                    crate::version::VersionAction::Unchanged => {
                        file_versions.push(FileVersion {
                            fid,
                            display_path: file.display_path.clone(),
                            generation: entry.current_gen,
                            pid: entry.current_pid,
                        });
                        continue;
                    }
                    crate::version::VersionAction::Patch { pid, content: patch_content } => {
                        let new_pid = pid;
                        let new_gen = entry.current_gen;
                        if let Some(e) = index.files.get_mut(&file.display_path) {
                            e.current_hash = new_hash.clone();
                            e.current_pid = new_pid;
                            e.last_updated = jiff::Timestamp::now();
                        }
                        cache.store_snapshot(fid, new_gen, &content)?;
                        file_versions.push(FileVersion {
                            fid,
                            display_path: file.display_path.clone(),
                            generation: new_gen,
                            pid: new_pid,
                        });
                        file_blocks.push(PreparedBlock {
                            meta: BlockMeta::patch(
                                fid,
                                new_gen,
                                new_pid,
                                file.display_path.clone(),
                            ),
                            content: patch_content,
                        });
                        continue;
                    }
                    crate::version::VersionAction::Replace {
                        generation,
                        content: replace_content,
                    } => {
                        let new_gen = generation;
                        if let Some(e) = index.files.get_mut(&file.display_path) {
                            e.current_hash = new_hash.clone();
                            e.current_gen = new_gen;
                            e.current_pid = 0;
                            e.last_updated = jiff::Timestamp::now();
                        }
                        cache.store_snapshot(fid, new_gen, &content)?;
                        file_versions.push(FileVersion {
                            fid,
                            display_path: file.display_path.clone(),
                            generation: new_gen,
                            pid: 0,
                        });
                        file_blocks.push(PreparedBlock {
                            meta: BlockMeta::replace(
                                fid,
                                new_gen,
                                file.display_path.clone(),
                                new_hash.clone(),
                            ),
                            content: replace_content,
                        });
                        continue;
                    }
                }
            }
        }

        // New file - output as file block gen=0
        let hash_for_index = new_hash.clone();
        if let Some(e) = index.files.get_mut(&file.display_path) {
            e.current_gen = 0;
            e.current_pid = 0;
            e.current_hash = hash_for_index;
        }
        cache.store_snapshot(fid, 0, &content)?;
        file_versions.push(FileVersion {
            fid,
            display_path: file.display_path.clone(),
            generation: 0,
            pid: 0,
        });
        let open_tag =
            file_block_open_tag(prefix, fid, 0, &file.display_path, extracted.is_partial);
        let file_block_content =
            format!("{open_tag}\n{encoded}</{p}:file>", p = prefix, encoded = new_encoded);
        file_blocks.push(PreparedBlock {
            meta: BlockMeta::file(fid, 0, file.display_path.clone(), new_hash),
            content: file_block_content,
        });
    }

    if config.global.prompt_generation {
        let prompt_text = generate_prompt(&config.global);
        let prompt_block = format!("<{p}:prompt>\n{}\n</{p}:prompt>", prompt_text, p = prefix);
        builder.append_block(BlockMeta::new(BlockType::Prompt), &prompt_block);
    }

    let tree_text = generate_tree(&file_versions);
    let tree_block = format!("<{p}:tree>\n{}</{p}:tree>", tree_text, p = prefix);
    builder.append_block(BlockMeta::new(BlockType::Tree), &tree_block);

    for block in file_blocks {
        builder.append_block(block.meta, &block.content);
    }

    index.save(index_path)?;
    Ok(builder.finish())
}

pub fn pack_auto(config: &Config, profile_name: &str) -> Result<PackOutput> {
    let index_path = std::path::Path::new(&config.global.index_file);
    if index_path.exists() {
        pack_incremental(config, profile_name)
    } else {
        pack_full(config, profile_name)
    }
}
