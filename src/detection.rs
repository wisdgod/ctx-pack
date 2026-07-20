pub mod binary;
pub mod encoding;

use anyhow::Result;
use std::path::Path;

use crate::config::{BinaryPolicy, GlobalConfig};

pub fn load_file_content(path: &Path, config: &GlobalConfig) -> Result<Option<String>> {
    let bin = binary::is_binary(path)?;
    if bin {
        match config.binary_policy {
            BinaryPolicy::Skip => return Ok(None),
            BinaryPolicy::Warn => {
                tracing::warn!("binary file skipped: {}", path.display());
                return Ok(None);
            }
            BinaryPolicy::Abort => {
                anyhow::bail!("binary file encountered: {}", path.display());
            }
        }
    }
    let content = encoding::read_to_utf8(path, config.encoding_detection)?;
    Ok(Some(content))
}
