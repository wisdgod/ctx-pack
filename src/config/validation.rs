use super::schema::{Config, ExtractionMode};

#[derive(Debug, Clone)]
pub struct ConfigWarning {
    pub field: String,
    pub message: String,
}

impl std::fmt::Display for ConfigWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.field, self.message)
    }
}

pub fn validate_config(config: &Config) -> Vec<ConfigWarning> {
    let mut warnings = Vec::new();

    if config.profiles.is_empty() {
        warnings.push(ConfigWarning {
            field: "profiles".to_string(),
            message: "no profiles configured".to_string(),
        });
    }

    for (name, profile) in &config.profiles {
        if name.is_empty() {
            warnings.push(ConfigWarning {
                field: "profiles".to_string(),
                message: "profile name is empty".to_string(),
            });
        }

        for root in &profile.roots {
            let p = std::path::Path::new(&root.path);
            if !p.exists() {
                warnings.push(ConfigWarning {
                    field: format!("profiles.{}.roots.{}", name, root.path),
                    message: format!("path '{}' does not exist", root.path),
                });
            }
        }

        let v = &profile.versioning;
        if v.replace_threshold < 0.0 || v.replace_threshold > 1.0 {
            warnings.push(ConfigWarning {
                field: format!("profiles.{}.versioning.replace_threshold", name),
                message: format!(
                    "replace_threshold {} is out of range [0.0, 1.0]",
                    v.replace_threshold
                ),
            });
        }

        let has_partial_extraction = profile.extraction.default_mode != ExtractionMode::Full
            || profile.extraction.rules.iter().any(|rule| rule.mode != ExtractionMode::Full);
        if has_partial_extraction {
            warnings.push(ConfigWarning {
                field: format!("profiles.{}.extraction", name),
                message: "partial extraction output is read-only for apply; use full extraction for files expected to receive LLM edits".to_string(),
            });
        }

        for (i, include) in profile.discovery.include.iter().enumerate() {
            if let Err(e) = globset::Glob::new(include) {
                warnings.push(ConfigWarning {
                    field: format!("profiles.{}.discovery.include[{}]", name, i),
                    message: format!("invalid glob pattern '{}': {}", include, e),
                });
            }
        }

        for (i, exclude) in profile.discovery.exclude.iter().enumerate() {
            if let Err(e) = globset::Glob::new(exclude) {
                warnings.push(ConfigWarning {
                    field: format!("profiles.{}.discovery.exclude[{}]", name, i),
                    message: format!("invalid glob pattern '{}': {}", exclude, e),
                });
            }
        }

        for (i, rule) in profile.extraction.rules.iter().enumerate() {
            if let Err(e) = globset::Glob::new(&rule.match_glob) {
                warnings.push(ConfigWarning {
                    field: format!("profiles.{}.extraction.rules[{}].match", name, i),
                    message: format!("invalid glob pattern '{}': {}", rule.match_glob, e),
                });
            }

            if rule.mode == ExtractionMode::Regex {
                if let Some(pattern) = &rule.pattern {
                    if let Err(e) = regex::Regex::new(pattern) {
                        warnings.push(ConfigWarning {
                            field: format!("profiles.{}.extraction.rules[{}].pattern", name, i),
                            message: format!("invalid regex '{}': {}", pattern, e),
                        });
                    }
                } else {
                    warnings.push(ConfigWarning {
                        field: format!("profiles.{}.extraction.rules[{}]", name, i),
                        message: "mode=regex but no pattern provided".to_string(),
                    });
                }
            }
        }
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{ExtractionMode, ExtractionRule};

    #[test]
    fn test_invalid_regex_warning() {
        let mut config = Config::default();
        let profile = config.profiles.get_mut("default").unwrap();
        profile.extraction.rules.push(ExtractionRule {
            match_glob: "**/*.rs".to_string(),
            mode: ExtractionMode::Regex,
            pattern: Some("[invalid".to_string()),
            ranges: None,
            context_lines: 0,
        });
        let warnings = validate_config(&config);
        assert!(warnings.iter().any(|w| w.message.contains("invalid regex")));
    }

    #[test]
    fn test_threshold_out_of_range() {
        let mut config = Config::default();
        let profile = config.profiles.get_mut("default").unwrap();
        profile.versioning.replace_threshold = 1.5;
        let warnings = validate_config(&config);
        assert!(warnings.iter().any(|w| w.message.contains("out of range")));
    }

    #[test]
    fn test_partial_extraction_warning() {
        let mut config = Config::default();
        let profile = config.profiles.get_mut("default").unwrap();
        profile.extraction.rules.push(ExtractionRule {
            match_glob: "**/*.rs".to_string(),
            mode: ExtractionMode::Lines,
            pattern: None,
            ranges: Some("1-10".to_string()),
            context_lines: 0,
        });
        let warnings = validate_config(&config);
        assert!(warnings.iter().any(|w| w.message.contains("read-only for apply")));
    }

    #[test]
    fn test_valid_config_no_warnings_except_path() {
        let config = Config::default();
        let warnings = validate_config(&config);
        // Only root path might warn if "." doesn't exist, otherwise clean
        for w in &warnings {
            assert!(w.field.contains("roots"), "unexpected warning: {}", w);
        }
    }
}
