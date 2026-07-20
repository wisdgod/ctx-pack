use anyhow::Result;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;

pub fn discover_stdin_from<R: Read>(reader: R) -> Result<Vec<PathBuf>> {
    let reader = BufReader::new(reader);
    let mut results = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let path = PathBuf::from(trimmed);
        if !path.exists() {
            tracing::warn!("stdin path does not exist: {}", path.display());
        } else {
            results.push(path);
        }
    }

    Ok(results)
}

pub fn discover_stdin() -> Result<Vec<PathBuf>> {
    discover_stdin_from(std::io::stdin())
}
