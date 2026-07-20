use anyhow::Result;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::output::{BlockInfo, BlockType, PackOutput};
use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "gen")]
    pub generation: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub byte_start: u64,
    pub byte_end: u64,
    pub line_start: u32,
    pub line_end: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: u32,
    pub prefix: String,
    pub profile: String,
    pub generated_at: String,
    pub output_file: String,
    pub output_size_bytes: u64,
    pub file_count: u32,
    pub blocks: Vec<ManifestBlock>,
    #[serde(default)]
    pub tag_occurrences: HashMap<String, Vec<[u32; 2]>>,
}

impl From<&BlockInfo> for ManifestBlock {
    fn from(b: &BlockInfo) -> Self {
        ManifestBlock {
            block_type: match b.block_type {
                BlockType::Prompt => "prompt".to_string(),
                BlockType::Tree => "tree".to_string(),
                BlockType::File => "file".to_string(),
                BlockType::Patch => "patch".to_string(),
                BlockType::Replace => "replace".to_string(),
            },
            fid: b.fid,
            generation: if b.fid.is_some() { Some(b.generation) } else { None },
            pid: b.pid,
            path: b.path.clone(),
            byte_start: b.byte_start,
            byte_end: b.byte_end,
            line_start: b.line_start,
            line_end: b.line_end,
            content_hash: b.content_hash.clone(),
        }
    }
}

pub fn write_manifest(
    output: &PackOutput,
    config: &Config,
    profile_name: &str,
    output_path: &Path,
    manifest_path: &Path,
) -> Result<()> {
    let file_count =
        output.blocks.iter().filter(|b| b.block_type == BlockType::File).count() as u32;

    let output_size = output.content.len() as u64;

    let blocks: Vec<ManifestBlock> = output.blocks.iter().map(|b| b.into()).collect();

    let mut tag_occurrences: HashMap<String, Vec<[u32; 2]>> = HashMap::new();
    for block in &output.blocks {
        let tag = format!(
            "{}:{}",
            config.global.prefix,
            match block.block_type {
                BlockType::Prompt => "prompt",
                BlockType::Tree => "tree",
                BlockType::File => "file",
                BlockType::Patch => "patch",
                BlockType::Replace => "replace",
            }
        );
        tag_occurrences.entry(tag).or_default().push([block.line_start, block.line_end]);
    }

    let manifest = Manifest {
        version: 1,
        prefix: config.global.prefix.clone(),
        profile: profile_name.to_string(),
        generated_at: Timestamp::now().to_string(),
        output_file: output_path.to_string_lossy().into_owned(),
        output_size_bytes: output_size,
        file_count,
        blocks,
        tag_occurrences,
    };

    let yaml = serde_yaml::to_string(&manifest)?;
    std::fs::write(manifest_path, yaml)?;
    Ok(())
}
