pub mod full;
pub mod lines;
pub mod regex_extract;

use anyhow::Result;
use globset::{Glob, GlobMatcher};

use crate::config::{ExtractionMode, ExtractionRule};

#[derive(Debug, Clone)]
pub struct Fragment {
    pub line_start: u32,
    pub line_end: u32,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ExtractionResult {
    pub fragments: Vec<Fragment>,
    pub total_lines: u32,
    pub is_partial: bool,
}

pub fn extract(content: &str, rule: &ExtractionRule) -> Result<ExtractionResult> {
    match rule.mode {
        ExtractionMode::Full => Ok(full::extract_full(content)),
        ExtractionMode::Lines => {
            let ranges = rule.ranges.as_deref().unwrap_or("1-");
            lines::extract_lines(content, ranges)
        }
        ExtractionMode::Regex => {
            let pattern = rule.pattern.as_deref().unwrap_or(".*");
            regex_extract::extract_regex(content, pattern, rule.context_lines)
        }
    }
}

static DEFAULT_FULL_RULE: std::sync::OnceLock<ExtractionRule> = std::sync::OnceLock::new();
static DEFAULT_LINES_RULE: std::sync::OnceLock<ExtractionRule> = std::sync::OnceLock::new();
static DEFAULT_REGEX_RULE: std::sync::OnceLock<ExtractionRule> = std::sync::OnceLock::new();

fn get_default_rule(mode: &ExtractionMode) -> &'static ExtractionRule {
    match mode {
        ExtractionMode::Full => DEFAULT_FULL_RULE.get_or_init(|| ExtractionRule {
            match_glob: "*".to_string(),
            mode: ExtractionMode::Full,
            ranges: None,
            pattern: None,
            context_lines: 0,
        }),
        ExtractionMode::Lines => DEFAULT_LINES_RULE.get_or_init(|| ExtractionRule {
            match_glob: "*".to_string(),
            mode: ExtractionMode::Lines,
            ranges: Some("1-".to_string()),
            pattern: None,
            context_lines: 0,
        }),
        ExtractionMode::Regex => DEFAULT_REGEX_RULE.get_or_init(|| ExtractionRule {
            match_glob: "*".to_string(),
            mode: ExtractionMode::Regex,
            ranges: None,
            pattern: Some(".*".to_string()),
            context_lines: 0,
        }),
    }
}

pub fn match_rule<'a>(
    path: &str,
    rules: &'a [ExtractionRule],
    default_mode: &'a ExtractionMode,
) -> &'a ExtractionRule {
    let matchers: Vec<GlobMatcher> = rules
        .iter()
        .map(|r| {
            Glob::new(&r.match_glob).unwrap_or_else(|_| Glob::new("*").unwrap()).compile_matcher()
        })
        .collect();

    for (i, matcher) in matchers.iter().enumerate() {
        if matcher.is_match(path) {
            return &rules[i];
        }
    }

    get_default_rule(default_mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_match_wins() {
        let rules = vec![
            ExtractionRule {
                match_glob: "**/*.rs".to_string(),
                mode: ExtractionMode::Full,
                ranges: None,
                pattern: None,
                context_lines: 0,
            },
            ExtractionRule {
                match_glob: "**/*.rs".to_string(),
                mode: ExtractionMode::Lines,
                ranges: Some("1-5".to_string()),
                pattern: None,
                context_lines: 0,
            },
        ];
        let rule = match_rule("src/main.rs", &rules, &ExtractionMode::Full);
        assert_eq!(rule.mode, ExtractionMode::Full);
    }

    #[test]
    fn test_no_match_uses_default() {
        let rules = vec![ExtractionRule {
            match_glob: "**/*.py".to_string(),
            mode: ExtractionMode::Lines,
            ranges: Some("1-5".to_string()),
            pattern: None,
            context_lines: 0,
        }];
        let rule = match_rule("src/main.rs", &rules, &ExtractionMode::Full);
        assert_eq!(rule.mode, ExtractionMode::Full);
    }
}
