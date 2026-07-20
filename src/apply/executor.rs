use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::encoding_layer::Pipeline;
use crate::index::{
    cache::SnapshotCache,
    compute_hash,
    state::{FileEntry, IndexState},
};

use super::reject::{reject_path, write_reject};
use super::scanner::ScannedBlock;

#[derive(Debug)]
pub struct ApplyResult {
    pub applied: Vec<AppliedFile>,
    pub rejected: Vec<RejectedFile>,
}

#[derive(Debug)]
pub struct AppliedFile {
    pub fid: u32,
    pub path: String,
    pub was_dirty: bool,
}

#[derive(Debug)]
pub struct RejectedFile {
    pub fid: u32,
    pub path: String,
    pub reason: String,
    /// Where the rejected block was saved. `None` when the reject file could
    /// not be written (the rejection itself is still reported).
    pub rej_file: Option<PathBuf>,
}

pub struct ApplyContext<'a> {
    pub cache: &'a SnapshotCache,
    pub full_pipeline: &'a Pipeline,
    pub content_pipeline: &'a Pipeline,
    pub prefix: &'a str,
    pub anchor_interval: u32,
    pub dry_run: bool,
}

pub fn execute_apply(
    blocks: &[ScannedBlock],
    index: &mut IndexState,
    context: ApplyContext<'_>,
) -> Result<ApplyResult> {
    let mut applied = Vec::new();
    let mut rejected = Vec::new();

    for block in blocks {
        match block {
            ScannedBlock::Replace { fid, generation, body } => {
                let (path_str, real_str) = match resolve_by_fid(index, *fid) {
                    Some(r) => r,
                    None => {
                        tracing::warn!("fid {} not found in index, skipping replace", fid);
                        continue;
                    }
                };

                let raw_content = context.full_pipeline.decode_all(body);
                let file_path = Path::new(&real_str);

                let current_entry = index.files.get(&path_str).cloned();
                if let Some(entry) = &current_entry
                    && let Err(reason) = validate_replace_version(entry, *generation)
                {
                    reject_block(
                        &mut rejected,
                        RejectContext {
                            file_path,
                            block,
                            reason: &reason,
                            prefix: context.prefix,
                            dry_run: context.dry_run,
                            fid: *fid,
                            path: &path_str,
                        },
                    );
                    continue;
                }

                let was_dirty = if file_path.exists() {
                    match std::fs::read_to_string(file_path) {
                        Ok(disk_content) => {
                            let disk_hash = compute_hash(&disk_content);
                            let index_hash = index
                                .files
                                .get(&path_str)
                                .map(|e| e.current_hash.as_str())
                                .unwrap_or("");
                            if disk_hash != index_hash {
                                tracing::warn!(
                                    "file {} was modified externally (dirty), rejecting replace",
                                    path_str
                                );
                                true
                            } else {
                                false
                            }
                        }
                        Err(_) => false,
                    }
                } else {
                    false
                };

                if was_dirty {
                    reject_block(
                        &mut rejected,
                        RejectContext {
                            file_path,
                            block,
                            reason: "file modified externally; refusing full replace",
                            prefix: context.prefix,
                            dry_run: context.dry_run,
                            fid: *fid,
                            path: &path_str,
                        },
                    );
                    continue;
                }

                if !context.dry_run {
                    if let Some(parent) = file_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    std::fs::write(file_path, &raw_content)?;

                    let new_hash = compute_hash(&raw_content);
                    if let Some(entry) = index.files.get_mut(&path_str) {
                        entry.current_hash = new_hash.clone();
                        entry.current_gen = *generation;
                        entry.current_pid = 0;
                        entry.last_updated = jiff::Timestamp::now();
                    }
                    context.cache.store_snapshot(*fid, *generation, &raw_content)?;
                }

                applied.push(AppliedFile { fid: *fid, path: path_str, was_dirty });
            }

            ScannedBlock::Patch { fid, generation, pid, body } => {
                let (path_str, real_str) = match resolve_by_fid(index, *fid) {
                    Some(r) => r,
                    None => {
                        tracing::warn!("fid {} not found in index, skipping patch", fid);
                        continue;
                    }
                };

                let file_path = Path::new(&real_str);
                let current_entry = index.files.get(&path_str).cloned();
                if let Some(entry) = &current_entry
                    && let Err(reason) = validate_patch_version(entry, *generation, *pid)
                {
                    reject_block(
                        &mut rejected,
                        RejectContext {
                            file_path,
                            block,
                            reason: &reason,
                            prefix: context.prefix,
                            dry_run: context.dry_run,
                            fid: *fid,
                            path: &path_str,
                        },
                    );
                    continue;
                }

                let disk_content = match std::fs::read_to_string(file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::warn!("failed to read {} ({}): {}", path_str, real_str, e);
                        reject_block(
                            &mut rejected,
                            RejectContext {
                                file_path,
                                block,
                                reason: &format!("cannot read file: {}", e),
                                prefix: context.prefix,
                                dry_run: context.dry_run,
                                fid: *fid,
                                path: &path_str,
                            },
                        );
                        continue;
                    }
                };

                let disk_hash = compute_hash(&disk_content);
                let index_hash =
                    index.files.get(&path_str).map(|e| e.current_hash.as_str()).unwrap_or("");
                let was_dirty = disk_hash != index_hash;
                if was_dirty {
                    tracing::warn!(
                        "file {} was modified externally (dirty), attempting patch anyway",
                        path_str
                    );
                }

                let encoded = context.content_pipeline.encode_all(&disk_content);
                let encoded_lines: Vec<String> = encoded.lines().map(|l| l.to_string()).collect();

                let hunks = match parse_patch_hunks(body) {
                    Ok(hunks) => hunks,
                    Err(e) => {
                        reject_block(
                            &mut rejected,
                            RejectContext {
                                file_path,
                                block,
                                reason: &e,
                                prefix: context.prefix,
                                dry_run: context.dry_run,
                                fid: *fid,
                                path: &path_str,
                            },
                        );
                        continue;
                    }
                };

                match apply_hunks(&encoded_lines, &hunks, context.anchor_interval) {
                    Ok(new_encoded_lines) => {
                        let new_encoded = new_encoded_lines.join("\n") + "\n";
                        let new_raw = context.content_pipeline.decode_all(&new_encoded);

                        if !context.dry_run {
                            std::fs::write(file_path, &new_raw)?;
                            let new_hash = compute_hash(&new_raw);
                            if let Some(entry) = index.files.get_mut(&path_str) {
                                entry.current_hash = new_hash.clone();
                                entry.current_pid = *pid;
                                entry.last_updated = jiff::Timestamp::now();
                            }
                            context.cache.store_snapshot(*fid, *generation, &new_raw)?;
                        }

                        applied.push(AppliedFile { fid: *fid, path: path_str, was_dirty });
                    }
                    Err(e) => {
                        reject_block(
                            &mut rejected,
                            RejectContext {
                                file_path,
                                block,
                                reason: &e,
                                prefix: context.prefix,
                                dry_run: context.dry_run,
                                fid: *fid,
                                path: &path_str,
                            },
                        );
                    }
                }
            }
        }
    }

    Ok(ApplyResult { applied, rejected })
}

struct RejectContext<'a> {
    /// Real filesystem path the reject file is written next to.
    file_path: &'a Path,
    block: &'a ScannedBlock,
    reason: &'a str,
    prefix: &'a str,
    dry_run: bool,
    fid: u32,
    path: &'a str,
}

/// A failure to persist the reject file must not abort the whole apply run:
/// the rejection is still recorded, only without a .rej file on disk.
fn reject_block(rejected: &mut Vec<RejectedFile>, context: RejectContext<'_>) {
    let rej_file = if context.dry_run {
        Some(reject_path(context.file_path))
    } else {
        match write_reject(context.file_path, context.block, context.reason, context.prefix) {
            Ok(path) => Some(path),
            Err(e) => {
                tracing::warn!(
                    "failed to write reject file for {}: {} (rejection still recorded)",
                    context.path,
                    e
                );
                None
            }
        }
    };

    rejected.push(RejectedFile {
        fid: context.fid,
        path: context.path.to_string(),
        reason: context.reason.to_string(),
        rej_file,
    });
}

/// Resolve a fid to its (display_path, real_path) pair: the display path names
/// the file in reports, the real path is where reads/writes happen.
fn resolve_by_fid(index: &IndexState, fid: u32) -> Option<(String, String)> {
    index
        .files
        .iter()
        .find(|(_, e)| e.fid == fid)
        .map(|(path, entry)| (path.clone(), entry.real_path.clone()))
}

fn validate_replace_version(entry: &FileEntry, generation: u32) -> Result<(), String> {
    let expected = entry.current_gen + 1;
    if generation != expected {
        return Err(format!(
            "stale replace version: expected gen {}, got gen {}",
            expected, generation
        ));
    }
    Ok(())
}

fn validate_patch_version(entry: &FileEntry, generation: u32, pid: u32) -> Result<(), String> {
    if generation != entry.current_gen {
        return Err(format!(
            "stale patch generation: expected gen {}, got gen {}",
            entry.current_gen, generation
        ));
    }

    let expected_pid = entry.current_pid + 1;
    if pid != expected_pid {
        return Err(format!("stale patch pid: expected pid {}, got pid {}", expected_pid, pid));
    }

    Ok(())
}

#[derive(Debug)]
struct PatchHunk {
    anchor_line: u32,
    lines: Vec<PatchLine>,
}

#[derive(Debug)]
enum PatchLine {
    Context(String),
    Remove(String),
    Add(String),
}

fn parse_patch_hunks(body: &str) -> Result<Vec<PatchHunk>, String> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<PatchHunk> = None;

    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("@@ anchor:") {
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }
            let anchor_str = rest.trim_end_matches(" @@").trim();
            if let Ok(anchor) = anchor_str.parse::<u32>() {
                current_hunk = Some(PatchHunk { anchor_line: anchor, lines: Vec::new() });
            } else {
                return Err(format!("invalid patch anchor: {}", anchor_str));
            }
        } else if let Some(hunk) = current_hunk.as_mut() {
            if let Some(rest) = line.strip_prefix('-') {
                hunk.lines.push(PatchLine::Remove(rest.to_string()));
            } else if let Some(rest) = line.strip_prefix('+') {
                hunk.lines.push(PatchLine::Add(rest.to_string()));
            } else if let Some(rest) = line.strip_prefix(' ') {
                hunk.lines.push(PatchLine::Context(rest.to_string()));
            } else if !line.is_empty() {
                // Treat as context (no prefix = context per spec)
                hunk.lines.push(PatchLine::Context(line.to_string()));
            }
        }
    }
    if let Some(hunk) = current_hunk {
        hunks.push(hunk);
    }

    if hunks.is_empty() {
        return Err("patch contains no hunks".to_string());
    }

    if hunks.iter().any(|hunk| {
        !hunk.lines.iter().any(|line| matches!(line, PatchLine::Remove(_) | PatchLine::Add(_)))
    }) {
        return Err("patch hunk contains no add/remove lines".to_string());
    }

    Ok(hunks)
}

fn apply_hunks(
    encoded_lines: &[String],
    hunks: &[PatchHunk],
    anchor_interval: u32,
) -> Result<Vec<String>, String> {
    let mut result = encoded_lines.to_vec();
    let mut offset: i64 = 0;
    let fuzzy_window = anchor_interval.max(20) as usize;

    for hunk in hunks {
        let parts = split_hunk(hunk);

        // Find anchor position using fuzzy match
        let target = (hunk.anchor_line as i64 - 1 + offset).max(0) as usize;
        let insert_pos = fuzzy_locate(
            &result,
            target,
            &parts.context_before,
            &parts.removes,
            &parts.context_after,
            fuzzy_window,
        )?;

        // Remove old lines and insert new lines
        let skip_ctx = parts.context_before.len();
        let start = insert_pos + skip_ctx;
        let remove_count = parts.removes.len();

        if start + remove_count > result.len() {
            return Err(format!(
                "patch out of bounds at anchor:{} (start={}, remove={}, len={})",
                hunk.anchor_line,
                start,
                remove_count,
                result.len()
            ));
        }

        // Verify removes match
        for (i, expected) in parts.removes.iter().enumerate() {
            if result[start + i] != *expected {
                return Err(format!(
                    "patch remove mismatch at line {}: expected '{}', got '{}'",
                    start + i + 1,
                    expected,
                    result[start + i]
                ));
            }
        }

        for (i, expected) in parts.context_after.iter().enumerate() {
            let idx = start + remove_count + i;
            if idx >= result.len() || result[idx] != *expected {
                let got = result.get(idx).map(String::as_str).unwrap_or("<eof>");
                return Err(format!(
                    "patch context mismatch after edit at line {}: expected '{}', got '{}'",
                    idx + 1,
                    expected,
                    got
                ));
            }
        }

        let mut new_result = Vec::with_capacity(result.len() - remove_count + parts.adds.len());
        new_result.extend_from_slice(&result[..start]);
        new_result.extend(parts.adds.iter().map(|s| s.to_string()));
        new_result.extend_from_slice(&result[start + remove_count..]);

        let size_delta = parts.adds.len() as i64 - parts.removes.len() as i64;
        offset += size_delta;
        result = new_result;
    }

    Ok(result)
}

struct HunkParts<'a> {
    context_before: Vec<&'a str>,
    removes: Vec<&'a str>,
    adds: Vec<&'a str>,
    context_after: Vec<&'a str>,
}

fn split_hunk(hunk: &PatchHunk) -> HunkParts<'_> {
    let mut context_before = Vec::new();
    let mut removes = Vec::new();
    let mut adds = Vec::new();
    let mut context_after = Vec::new();
    let mut seen_change = false;

    for line in &hunk.lines {
        match line {
            PatchLine::Context(s) if seen_change => context_after.push(s.as_str()),
            PatchLine::Context(s) => context_before.push(s.as_str()),
            PatchLine::Remove(s) => {
                seen_change = true;
                removes.push(s.as_str());
            }
            PatchLine::Add(s) => {
                seen_change = true;
                adds.push(s.as_str());
            }
        }
    }

    HunkParts { context_before, removes, adds, context_after }
}

fn fuzzy_locate(
    lines: &[String],
    target: usize,
    context_before: &[&str],
    removes: &[&str],
    context_after: &[&str],
    window: usize,
) -> Result<usize, String> {
    // Try exact anchor position first
    if let Some(pos) = try_locate_at(lines, target, context_before, removes, context_after) {
        return Ok(pos);
    }

    // Fuzzy: try anchor ± window.
    for delta in 1..=window {
        let candidates = [target.saturating_sub(delta), target + delta];
        for &candidate in &candidates {
            if candidate < lines.len()
                && let Some(pos) =
                    try_locate_at(lines, candidate, context_before, removes, context_after)
            {
                tracing::warn!("fuzzy match: anchor offset by {} lines", delta);
                return Ok(pos);
            }
        }
    }

    Err(format!("could not locate hunk near line {} (context: {:?})", target + 1, context_before))
}

fn try_locate_at(
    lines: &[String],
    anchor: usize,
    context_before: &[&str],
    removes: &[&str],
    context_after: &[&str],
) -> Option<usize> {
    // The context_before should precede the removes at 'anchor'
    let ctx_len = context_before.len();
    let start = if anchor >= ctx_len {
        anchor - ctx_len
    } else {
        return None;
    };

    // Verify context
    for (i, ctx) in context_before.iter().enumerate() {
        if start + i >= lines.len() || lines[start + i] != *ctx {
            return None;
        }
    }

    // Verify first remove matches if any
    if !removes.is_empty() {
        let remove_start = start + ctx_len;
        if remove_start >= lines.len() || lines[remove_start] != removes[0] {
            return None;
        }
    } else if !context_after.is_empty() {
        let after_start = start + ctx_len;
        if after_start >= lines.len() || lines[after_start] != context_after[0] {
            return None;
        }
    }

    Some(start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding_layer::{AnchorEncoder, IndentEncoder, Pipeline};
    use crate::index::cache::SnapshotCache;
    use crate::index::state::IndexState;
    use tempfile::TempDir;

    fn setup() -> (TempDir, IndexState, SnapshotCache, Pipeline) {
        let dir = TempDir::new().unwrap();
        let index = IndexState::default();
        let cache = SnapshotCache::new(&dir.path().join("cache"));
        let pipeline = Pipeline::new();
        (dir, index, cache, pipeline)
    }

    fn apply_context<'a>(
        cache: &'a SnapshotCache,
        full_pipeline: &'a Pipeline,
        content_pipeline: &'a Pipeline,
        dry_run: bool,
    ) -> ApplyContext<'a> {
        ApplyContext {
            cache,
            full_pipeline,
            content_pipeline,
            prefix: "ctx",
            anchor_interval: 10,
            dry_run,
        }
    }

    #[test]
    fn test_replace_apply() {
        let (dir, mut index, cache, pipeline) = setup();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, "old content\n").unwrap();

        let abs_str = file_path.to_string_lossy().to_string();
        let fid = index.allocate_fid(&abs_str, &abs_str);
        if let Some(e) = index.files.get_mut(&abs_str) {
            e.current_hash = crate::index::compute_hash("old content\n");
        }

        let blocks =
            vec![ScannedBlock::Replace { fid, generation: 1, body: "new content\n".to_string() }];

        let result =
            execute_apply(&blocks, &mut index, apply_context(&cache, &pipeline, &pipeline, false))
                .unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content\n");
        assert_eq!(result.applied.len(), 1);
        assert_eq!(result.rejected.len(), 0);
    }

    #[test]
    fn test_dry_run_no_write() {
        let (dir, mut index, cache, pipeline) = setup();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, "original\n").unwrap();

        let abs_str = file_path.to_string_lossy().to_string();
        index.allocate_fid(&abs_str, &abs_str);
        if let Some(e) = index.files.get_mut(&abs_str) {
            e.current_hash = crate::index::compute_hash("original\n");
        }

        let blocks = vec![ScannedBlock::Replace {
            fid: 1,
            generation: 1,
            body: "new content\n".to_string(),
        }];

        execute_apply(&blocks, &mut index, apply_context(&cache, &pipeline, &pipeline, true))
            .unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "original\n");
    }

    #[test]
    fn test_unknown_fid_skipped() {
        let (_dir, mut index, cache, pipeline) = setup();
        let blocks =
            vec![ScannedBlock::Replace { fid: 999, generation: 0, body: "content".to_string() }];
        let result =
            execute_apply(&blocks, &mut index, apply_context(&cache, &pipeline, &pipeline, false))
                .unwrap();
        assert_eq!(result.applied.len(), 0);
        assert_eq!(result.rejected.len(), 0);
    }

    #[test]
    fn test_patch_apply_uses_content_lines_without_anchor_margin() {
        let (dir, mut index, cache, _pipeline) = setup();
        let mut pipeline = Pipeline::new();
        pipeline.add_stage(IndentEncoder::new(4));
        pipeline.add_stage(AnchorEncoder::new(10));
        let mut content_pipeline = Pipeline::new();
        content_pipeline.add_stage(IndentEncoder::new(4));

        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, "fn main() {\n    old();\n}\n").unwrap();

        let abs_str = file_path.to_string_lossy().to_string();
        let fid = index.allocate_fid(&abs_str, &abs_str);
        if let Some(e) = index.files.get_mut(&abs_str) {
            e.current_hash = crate::index::compute_hash("fn main() {\n    old();\n}\n");
        }

        let body = "@@ anchor:1 @@\n [0]fn main() {\n-[4]old();\n+[4]new();\n [0]}\n".to_string();
        let blocks = vec![ScannedBlock::Patch { fid, generation: 0, pid: 1, body }];

        let result = execute_apply(
            &blocks,
            &mut index,
            apply_context(&cache, &pipeline, &content_pipeline, false),
        )
        .unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "fn main() {\n    new();\n}\n");
        assert_eq!(result.applied.len(), 1);
        assert_eq!(result.rejected.len(), 0);
    }

    #[test]
    fn test_reject_write_failure_does_not_abort_batch() {
        let (dir, mut index, cache, pipeline) = setup();

        // fid 1: real path in a directory that does not exist, so both the file
        // read and the .rej write fail.
        let missing = dir.path().join("missing/nested/gone.rs");
        let missing_str = missing.to_string_lossy().to_string();
        let fid1 = index.allocate_fid(&missing_str, &missing_str);

        // fid 2: a valid file that must still be applied afterwards.
        let ok_path = dir.path().join("ok.rs");
        std::fs::write(&ok_path, "old\n").unwrap();
        let ok_str = ok_path.to_string_lossy().to_string();
        let fid2 = index.allocate_fid(&ok_str, &ok_str);
        if let Some(e) = index.files.get_mut(&ok_str) {
            e.current_hash = crate::index::compute_hash("old\n");
        }

        let blocks = vec![
            ScannedBlock::Patch {
                fid: fid1,
                generation: 0,
                pid: 1,
                body: "@@ anchor:1 @@\n-old\n+new\n".to_string(),
            },
            ScannedBlock::Replace { fid: fid2, generation: 1, body: "new\n".to_string() },
        ];

        let result =
            execute_apply(&blocks, &mut index, apply_context(&cache, &pipeline, &pipeline, false))
                .unwrap();

        assert_eq!(result.rejected.len(), 1);
        assert!(result.rejected[0].rej_file.is_none());
        assert_eq!(result.applied.len(), 1);
        assert_eq!(std::fs::read_to_string(&ok_path).unwrap(), "new\n");
    }

    #[test]
    fn test_stale_patch_pid_rejected() {
        let (dir, mut index, cache, pipeline) = setup();
        let file_path = dir.path().join("test.rs");
        std::fs::write(&file_path, "old\n").unwrap();

        let abs_str = file_path.to_string_lossy().to_string();
        let fid = index.allocate_fid(&abs_str, &abs_str);
        if let Some(e) = index.files.get_mut(&abs_str) {
            e.current_hash = crate::index::compute_hash("old\n");
            e.current_pid = 1;
        }

        let body = "@@ anchor:1 @@\n-old\n+new\n".to_string();
        let blocks = vec![ScannedBlock::Patch { fid, generation: 0, pid: 1, body }];

        let result =
            execute_apply(&blocks, &mut index, apply_context(&cache, &pipeline, &pipeline, false))
                .unwrap();
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "old\n");
        assert_eq!(result.applied.len(), 0);
        assert_eq!(result.rejected.len(), 1);
        assert!(result.rejected[0].reason.contains("stale patch pid"));
    }
}
