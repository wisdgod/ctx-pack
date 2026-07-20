pub mod schema;
pub mod validation;

pub use schema::*;
pub use validation::validate_config;

use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn load_config(path: Option<&Path>) -> Result<Config> {
    let config_path = match path {
        Some(p) => Some(p.to_path_buf()),
        None => find_config_upward()?,
    };

    match config_path {
        Some(p) => {
            let content = std::fs::read_to_string(&p)?;
            let config: Config = serde_yaml::from_str(&content)
                .map_err(|e| anyhow::anyhow!("failed to parse config at {}: {}", p.display(), e))?;
            Ok(config)
        }
        None => Ok(Config::default()),
    }
}

fn find_config_upward() -> Result<Option<PathBuf>> {
    let mut dir = std::env::current_dir()?;
    loop {
        let candidate = dir.join("ctx-pack.yaml");
        if candidate.exists() {
            return Ok(Some(candidate));
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return Ok(None),
        }
    }
}
