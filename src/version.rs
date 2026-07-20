pub mod diff;
pub mod patch_gen;
pub mod replace_gen;

use crate::config::VersioningConfig;

#[derive(Debug)]
pub enum VersionAction {
    Unchanged,
    Patch { pid: u32, content: String },
    Replace { generation: u32, content: String },
}

pub struct VersionContext<'a> {
    pub current_gen: u32,
    pub current_pid: u32,
    pub config: &'a VersioningConfig,
    pub prefix: &'a str,
    pub anchor_interval: u32,
}

pub fn determine_action(
    fid: u32,
    old_encoded: &str,
    new_encoded: &str,
    context: VersionContext<'_>,
) -> VersionAction {
    let old_patch_view = patch_view(old_encoded, context.anchor_interval);
    let new_patch_view = patch_view(new_encoded, context.anchor_interval);

    if old_patch_view == new_patch_view {
        return VersionAction::Unchanged;
    }

    let ratio = diff::compute_change_ratio(&old_patch_view, &new_patch_view);

    if ratio > context.config.replace_threshold
        || context.current_pid >= context.config.max_patches_before_replace
    {
        let new_gen = context.current_gen + 1;
        let content = replace_gen::generate_replace(fid, new_gen, new_encoded, context.prefix);
        return VersionAction::Replace { generation: new_gen, content };
    }

    let hunks = diff::compute_line_diff(&old_patch_view, &new_patch_view);
    let new_pid = context.current_pid + 1;
    let content = patch_gen::generate_patch(
        fid,
        context.current_gen,
        new_pid,
        &hunks,
        context.prefix,
        context.anchor_interval,
    );
    VersionAction::Patch { pid: new_pid, content }
}

fn patch_view(encoded: &str, anchor_interval: u32) -> String {
    if anchor_interval > 0 {
        crate::encoding_layer::anchor::strip_anchor_prefixes(encoded)
    } else {
        encoded.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VersioningConfig;

    fn config() -> VersioningConfig {
        VersioningConfig::default()
    }

    #[test]
    fn test_unchanged() {
        let cfg = config();
        let action = determine_action(
            1,
            "same\n",
            "same\n",
            VersionContext {
                current_gen: 0,
                current_pid: 0,
                config: &cfg,
                prefix: "ctx",
                anchor_interval: 10,
            },
        );
        assert!(matches!(action, VersionAction::Unchanged));
    }

    #[test]
    fn test_small_change_produces_patch() {
        let old = "line1\nline2\nline3\nline4\nline5\n";
        let new = "line1\nmodified\nline3\nline4\nline5\n";
        let cfg = config();
        let action = determine_action(
            1,
            old,
            new,
            VersionContext {
                current_gen: 0,
                current_pid: 0,
                config: &cfg,
                prefix: "ctx",
                anchor_interval: 10,
            },
        );
        assert!(matches!(action, VersionAction::Patch { .. }));
    }

    #[test]
    fn test_large_change_produces_replace() {
        let old = "a\nb\nc\nd\ne\n";
        let new = "x\ny\nz\nw\nv\n";
        let mut cfg = config();
        cfg.replace_threshold = 0.3;
        let action = determine_action(
            1,
            old,
            new,
            VersionContext {
                current_gen: 0,
                current_pid: 0,
                config: &cfg,
                prefix: "ctx",
                anchor_interval: 10,
            },
        );
        assert!(matches!(action, VersionAction::Replace { .. }));
    }

    #[test]
    fn test_max_patches_triggers_replace() {
        let old = "line1\nline2\nline3\nline4\nline5\n";
        let new = "line1\nmodified\nline3\nline4\nline5\n";
        let mut cfg = config();
        cfg.max_patches_before_replace = 3;
        let action = determine_action(
            1,
            old,
            new,
            VersionContext {
                current_gen: 0,
                current_pid: 3,
                config: &cfg,
                prefix: "ctx",
                anchor_interval: 10,
            },
        );
        assert!(matches!(action, VersionAction::Replace { .. }));
    }

    #[test]
    fn test_patch_uses_content_lines_without_anchor_margin() {
        let old = "   1 | [0]fn main() {\n     | [4]old();\n     | [0]}\n";
        let new = "   1 | [0]fn main() {\n     | [4]new();\n     | [0]}\n";
        let mut cfg = config();
        cfg.replace_threshold = 1.0;

        let action = determine_action(
            1,
            old,
            new,
            VersionContext {
                current_gen: 0,
                current_pid: 0,
                config: &cfg,
                prefix: "ctx",
                anchor_interval: 10,
            },
        );

        match action {
            VersionAction::Patch { content, .. } => {
                assert!(content.contains("-[4]old();"));
                assert!(content.contains("+[4]new();"));
                assert!(!content.contains("| [4]old();"));
                assert!(!content.contains("| [4]new();"));
            }
            _ => panic!("expected patch"),
        }
    }
}
