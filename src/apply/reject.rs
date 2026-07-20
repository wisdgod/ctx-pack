use anyhow::Result;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::scanner::ScannedBlock;

pub fn reject_path(path: &Path) -> PathBuf {
    match path.extension() {
        Some(extension) if !extension.is_empty() => {
            let mut reject_extension = OsString::from(extension);
            reject_extension.push(".rej");
            path.with_extension(reject_extension)
        }
        _ => path.with_extension("rej"),
    }
}

pub fn write_reject(
    path: &Path,
    block: &ScannedBlock,
    reason: &str,
    prefix: &str,
) -> Result<PathBuf> {
    let rej_path = reject_path(path);
    let header = format!("# Reject reason: {}\n# Original block:\n", reason);
    let block_text = match block {
        ScannedBlock::Patch { fid, generation, pid, body } => {
            format!(
                "<{prefix}:patch fid=\"{}\" gen=\"{}\" pid=\"{}\">\n{}</{prefix}:patch>",
                fid, generation, pid, body,
            )
        }
        ScannedBlock::Replace { fid, generation, body } => {
            format!(
                "<{prefix}:replace fid=\"{}\" gen=\"{}\">\n{}</{prefix}:replace>",
                fid, generation, body,
            )
        }
    };

    std::fs::write(&rej_path, format!("{}{}\n", header, block_text))?;
    Ok(rej_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reject_path_appends_to_existing_extension() {
        assert_eq!(reject_path(Path::new("src/main.rs")), PathBuf::from("src/main.rs.rej"));
    }

    #[test]
    fn reject_path_handles_missing_extension() {
        assert_eq!(reject_path(Path::new("Makefile")), PathBuf::from("Makefile.rej"));
    }
}
