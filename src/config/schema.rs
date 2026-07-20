use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

fn deserialize_size_string<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    let s = serde_yaml::Value::deserialize(deserializer)?;
    match s {
        serde_yaml::Value::Number(n) => n.as_u64().ok_or_else(|| Error::custom("invalid number")),
        serde_yaml::Value::String(s) => parse_size_string(&s).map_err(Error::custom),
        _ => Err(Error::custom("expected string or number for size")),
    }
}

pub fn parse_size_string(s: &str) -> Result<u64, String> {
    let s = s.trim();
    let lower = s.to_ascii_lowercase();
    if let Some(rest) = lower.strip_suffix("kb") {
        let n: u64 = rest.trim().parse().map_err(|e| format!("parse error: {e}"))?;
        Ok(n * 1024)
    } else if let Some(rest) = lower.strip_suffix("mb") {
        let n: u64 = rest.trim().parse().map_err(|e| format!("parse error: {e}"))?;
        Ok(n * 1024 * 1024)
    } else if let Some(rest) = lower.strip_suffix("gb") {
        let n: u64 = rest.trim().parse().map_err(|e| format!("parse error: {e}"))?;
        Ok(n * 1024 * 1024 * 1024)
    } else {
        s.parse::<u64>().map_err(|e| format!("parse error: {e}"))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BinaryPolicy {
    #[default]
    Skip,
    Warn,
    Abort,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SizePolicy {
    #[default]
    Warn,
    Abort,
    Ignore,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMode {
    #[default]
    Full,
    Lines,
    Regex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    #[serde(default = "default_prefix")]
    pub prefix: String,
    #[serde(default = "default_anchor_interval")]
    pub anchor_interval: u32,
    #[serde(default = "default_indent_encoding")]
    pub indent_encoding: bool,
    #[serde(default = "default_tab_width")]
    pub tab_width: u32,
    #[serde(default)]
    pub binary_policy: BinaryPolicy,
    #[serde(default = "default_encoding_detection")]
    pub encoding_detection: bool,
    #[serde(default = "default_max_content_size", deserialize_with = "deserialize_size_string")]
    pub max_content_size: u64,
    #[serde(default = "default_max_file_size", deserialize_with = "deserialize_size_string")]
    pub max_file_size: u64,
    #[serde(default)]
    pub size_policy: SizePolicy,
    #[serde(default = "default_index_file")]
    pub index_file: String,
    #[serde(default = "default_cache_dir")]
    pub cache_dir: String,
    #[serde(default = "default_cache_retention")]
    pub cache_retention: u32,
    #[serde(default = "default_manifest")]
    pub manifest: bool,
    #[serde(default = "default_prompt_generation")]
    pub prompt_generation: bool,
}

fn default_prefix() -> String {
    "ctx".to_string()
}
fn default_anchor_interval() -> u32 {
    10
}
fn default_indent_encoding() -> bool {
    true
}
fn default_tab_width() -> u32 {
    4
}
fn default_encoding_detection() -> bool {
    true
}
fn default_max_content_size() -> u64 {
    500 * 1024
}
fn default_max_file_size() -> u64 {
    100 * 1024
}
fn default_index_file() -> String {
    ".ctx-index.yaml".to_string()
}
fn default_cache_dir() -> String {
    ".ctx-cache".to_string()
}
fn default_cache_retention() -> u32 {
    5
}
fn default_manifest() -> bool {
    true
}
fn default_prompt_generation() -> bool {
    true
}

impl Default for GlobalConfig {
    fn default() -> Self {
        GlobalConfig {
            prefix: default_prefix(),
            anchor_interval: default_anchor_interval(),
            indent_encoding: default_indent_encoding(),
            tab_width: default_tab_width(),
            binary_policy: BinaryPolicy::default(),
            encoding_detection: default_encoding_detection(),
            max_content_size: default_max_content_size(),
            max_file_size: default_max_file_size(),
            size_policy: SizePolicy::default(),
            index_file: default_index_file(),
            cache_dir: default_cache_dir(),
            cache_retention: default_cache_retention(),
            manifest: default_manifest(),
            prompt_generation: default_prompt_generation(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootEntry {
    pub path: String,
    #[serde(default = "default_label")]
    pub label: String,
}

fn default_label() -> String {
    "project".to_string()
}

impl Default for RootEntry {
    fn default() -> Self {
        RootEntry { path: ".".to_string(), label: default_label() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoveryConfig {
    #[serde(default = "default_use_gitignore")]
    pub use_gitignore: bool,
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default)]
    pub stdin_merge: bool,
}

fn default_use_gitignore() -> bool {
    true
}

impl DiscoveryConfig {
    pub fn with_defaults() -> Self {
        DiscoveryConfig {
            use_gitignore: true,
            include: vec![],
            exclude: vec![],
            stdin_merge: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRule {
    #[serde(rename = "match")]
    pub match_glob: String,
    pub mode: ExtractionMode,
    pub ranges: Option<String>,
    pub pattern: Option<String>,
    #[serde(default)]
    pub context_lines: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractionConfig {
    #[serde(default)]
    pub default_mode: ExtractionMode,
    #[serde(default)]
    pub rules: Vec<ExtractionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersioningConfig {
    #[serde(default = "default_auto_diff")]
    pub auto_diff: bool,
    #[serde(default = "default_replace_threshold")]
    pub replace_threshold: f64,
    #[serde(default = "default_max_patches_before_replace")]
    pub max_patches_before_replace: u32,
}

fn default_auto_diff() -> bool {
    true
}
fn default_replace_threshold() -> f64 {
    0.5
}
fn default_max_patches_before_replace() -> u32 {
    5
}

impl Default for VersioningConfig {
    fn default() -> Self {
        VersioningConfig {
            auto_diff: default_auto_diff(),
            replace_threshold: default_replace_threshold(),
            max_patches_before_replace: default_max_patches_before_replace(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    #[serde(default = "default_output_file")]
    pub file: String,
    #[serde(default = "default_manifest_file")]
    pub manifest: String,
}

fn default_output_file() -> String {
    "context.ctx".to_string()
}
fn default_manifest_file() -> String {
    "context.ctx.manifest".to_string()
}

impl Default for OutputConfig {
    fn default() -> Self {
        OutputConfig { file: default_output_file(), manifest: default_manifest_file() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub roots: Vec<RootEntry>,
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    #[serde(default)]
    pub extraction: ExtractionConfig,
    #[serde(default)]
    pub versioning: VersioningConfig,
    #[serde(default)]
    pub output: OutputConfig,
}

impl Default for Profile {
    fn default() -> Self {
        Profile {
            roots: vec![RootEntry::default()],
            discovery: DiscoveryConfig::with_defaults(),
            extraction: ExtractionConfig::default(),
            versioning: VersioningConfig::default(),
            output: OutputConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default = "default_profiles")]
    pub profiles: HashMap<String, Profile>,
}

fn default_profiles() -> HashMap<String, Profile> {
    let mut profiles = HashMap::new();
    profiles.insert("default".to_string(), Profile::default());
    profiles
}

impl Default for Config {
    fn default() -> Self {
        Config { global: GlobalConfig::default(), profiles: default_profiles() }
    }
}

impl Config {
    pub fn get_profile(&self, name: &str) -> anyhow::Result<&Profile> {
        self.profiles.get(name).ok_or_else(|| {
            let mut available: Vec<&str> = self.profiles.keys().map(String::as_str).collect();
            available.sort_unstable();
            let available =
                if available.is_empty() { "<none>".to_string() } else { available.join(", ") };
            anyhow::anyhow!("profile '{}' not found (available: {})", name, available)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_round_trip() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let _: Config = serde_yaml::from_str(&yaml).unwrap();
    }

    #[test]
    fn test_size_string_parse() {
        assert_eq!(parse_size_string("500KB").unwrap(), 512000);
        assert_eq!(parse_size_string("10MB").unwrap(), 10485760);
        assert_eq!(parse_size_string("1234").unwrap(), 1234);
        assert_eq!(parse_size_string("1GB").unwrap(), 1073741824);
        assert_eq!(parse_size_string("500kb").unwrap(), 512000);
    }

    #[test]
    fn test_size_string_deserialize() {
        let yaml = r#"
global:
  max_content_size: "500KB"
  max_file_size: "10MB"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.global.max_content_size, 512000);
        assert_eq!(config.global.max_file_size, 10485760);
    }

    #[test]
    fn test_size_string_numeric() {
        let yaml = r#"
global:
  max_content_size: 1234
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.global.max_content_size, 1234);
    }

    #[test]
    fn test_missing_profiles_uses_default_profile() {
        let yaml = r#"
global:
  prefix: "ctx"
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();
        assert!(config.get_profile("default").is_ok());
    }

    #[test]
    fn test_missing_profile_reports_available_profiles() {
        let config = Config::default();
        let err = config.get_profile("missing").unwrap_err();
        assert!(err.to_string().contains("profile 'missing' not found"));
        assert!(err.to_string().contains("default"));
    }
}
