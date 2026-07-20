use anyhow::Result;

use crate::config::Config;

pub fn migrate_prefix(old: &str, new: &str, config: &Config) -> Result<()> {
    let mut total_replacements = 0usize;
    let patterns = PrefixPatterns::new(old, new);

    for profile in config.profiles.values() {
        let output_file = &profile.output.file;
        if std::path::Path::new(output_file).exists() {
            let content = std::fs::read_to_string(output_file)?;
            let replaced = patterns.replace_prefix(&content);
            let count = patterns.count_replacements(&content);
            if count > 0 {
                std::fs::write(output_file, replaced)?;
                total_replacements += count;
                tracing::info!("updated {} ({} replacements)", output_file, count);
            }
        }

        let manifest_file = &profile.output.manifest;
        if std::path::Path::new(manifest_file).exists() {
            let content = std::fs::read_to_string(manifest_file)?;
            // Replace tag_occurrences keys in manifest
            let replaced = patterns.replace_prefix(&content);
            std::fs::write(manifest_file, replaced)?;
        }
    }

    // Update config file prefix
    let config_file = "ctx-pack.yaml";
    if std::path::Path::new(config_file).exists() {
        let content = std::fs::read_to_string(config_file)?;
        let replaced =
            content.replace(&format!("prefix: \"{}\"", old), &format!("prefix: \"{}\"", new));
        std::fs::write(config_file, replaced)?;
    }

    println!("Migrated prefix '{}' → '{}' ({} replacements)", old, new, total_replacements);
    Ok(())
}

struct PrefixPatterns {
    old_open: String,
    old_close: String,
    new_open: String,
    new_close: String,
}

impl PrefixPatterns {
    fn new(old: &str, new: &str) -> Self {
        PrefixPatterns {
            old_open: format!("<{}:", old),
            old_close: format!("</{}:", old),
            new_open: format!("<{}:", new),
            new_close: format!("</{}:", new),
        }
    }

    fn replace_prefix(&self, content: &str) -> String {
        content.replace(&self.old_open, &self.new_open).replace(&self.old_close, &self.new_close)
    }

    fn count_replacements(&self, content: &str) -> usize {
        content.matches(&self.old_open).count() + content.matches(&self.old_close).count()
    }
}
