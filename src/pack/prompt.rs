use crate::config::GlobalConfig;
use std::fmt::Write as _;

pub fn generate_prompt(config: &GlobalConfig) -> String {
    let mut prompt = String::with_capacity(1_600);
    let prefix = &config.prefix;

    let _ = writeln!(prompt, "This file uses the ctx-pack protocol with prefix \"{prefix}\".");
    prompt.push('\n');
    prompt.push_str("## Tags\n");
    let _ = writeln!(
        prompt,
        "- <{p}:file id=\"N\" gen=\"G\" path=\"...\"> ... </{p}:file>  File content at generation G. The id value is the file fid.",
        p = prefix
    );
    let _ = writeln!(
        prompt,
        "- <{p}:patch fid=\"N\" gen=\"G\" pid=\"P\"> ... </{p}:patch>  Incremental patch, patch P on gen G.",
        p = prefix
    );
    let _ = writeln!(
        prompt,
        "- <{p}:replace fid=\"N\" gen=\"G\"> ... </{p}:replace>  Full replacement, new generation G.",
        p = prefix
    );
    let _ =
        writeln!(prompt, "- <{p}:tree> ... </{p}:tree>  File index with version info.", p = prefix);

    if config.indent_encoding {
        prompt.push('\n');
        prompt.push_str("## Indent Encoding\n");
        prompt.push_str(
            "Leading indentation is encoded as [N] prefix, where N is the number of spaces.\n",
        );
        prompt.push_str("Examples:\n");
        prompt.push_str("  [0]fn main() {   →  fn main() {\n");
        prompt.push_str("  [4]let x = 1;   →      let x = 1;\n");
        prompt.push_str("  [8]inner();      →          inner();\n");
        prompt.push_str("When writing patch/replace blocks, preserve the [N] encoding exactly.\n");
    }

    if config.anchor_interval > 0 {
        prompt.push('\n');
        prompt.push_str("## Anchor Line Numbers\n");
        let _ = writeln!(
            prompt,
            "Every {} lines, a line number is shown in the left margin followed by \" | \".",
            config.anchor_interval
        );
        prompt.push_str("Lines without anchors show blank space. Example:\n");
        prompt.push_str("   1 | [0]fn main() {\n");
        prompt.push_str("     | [4]let x = 1;\n");
        let _ = writeln!(prompt, "  {} | [0]}}", config.anchor_interval);
        prompt.push_str("Use the anchor number in @@ anchor:N @@ headers in patch blocks, but do not copy the left line-number margin into hunk lines.\n");
    }

    prompt.push('\n');
    prompt.push_str("## Version Model\n");
    prompt.push_str("Each file has (fid, gen, pid). fid is permanent. gen increments on replace. pid increments on patch.\n");
    prompt.push_str("When a file changes slightly, output a patch block. When it changes significantly, output a replace block.\n");
    prompt.push_str("Files marked extraction=\"partial\" are read-only context views; do not emit patch/replace blocks for them.\n");

    prompt.push('\n');
    prompt.push_str("## Patch Format\n");
    let _ = writeln!(prompt, "<{p}:patch fid=\"N\" gen=\"G\" pid=\"P\">", p = prefix);

    if config.anchor_interval > 0 {
        prompt.push_str("@@ anchor:LINE_NUM @@\n");
    } else {
        prompt.push_str("@@ anchor:LINE_NUM @@  (use nearest line number)\n");
    }

    prompt.push_str(
        "Hunk lines contain only file content after removing the left line-number margin.\n",
    );
    if config.indent_encoding {
        prompt.push_str(" [4]context line (unchanged, space prefix)\n");
        prompt.push_str("-[4]removed line\n");
        prompt.push_str("+[4]added line\n");
    } else {
        prompt.push_str(" context line (unchanged, space prefix)\n");
        prompt.push_str("-removed line\n");
        prompt.push_str("+added line\n");
    }
    let _ = write!(prompt, "</{p}:patch>", p = prefix);

    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GlobalConfig;

    #[test]
    fn test_prompt_with_all_features() {
        let config = GlobalConfig::default();
        let prompt = generate_prompt(&config);
        assert!(prompt.contains("ctx-pack protocol"));
        assert!(prompt.contains("Indent Encoding"));
        assert!(prompt.contains("Anchor Line Numbers"));
        assert!(prompt.contains("Version Model"));
        assert!(prompt.contains("Patch Format"));
    }

    #[test]
    fn test_prompt_no_indent_encoding() {
        let config = GlobalConfig { indent_encoding: false, ..GlobalConfig::default() };
        let prompt = generate_prompt(&config);
        assert!(!prompt.contains("Indent Encoding"));
        assert!(prompt.contains("Anchor Line Numbers"));
    }

    #[test]
    fn test_prompt_no_anchor() {
        let config = GlobalConfig { anchor_interval: 0, ..GlobalConfig::default() };
        let prompt = generate_prompt(&config);
        assert!(!prompt.contains("Anchor Line Numbers"));
        assert!(prompt.contains("Indent Encoding"));
    }

    #[test]
    fn test_prompt_minimal() {
        let config =
            GlobalConfig { indent_encoding: false, anchor_interval: 0, ..GlobalConfig::default() };
        let prompt = generate_prompt(&config);
        assert!(!prompt.contains("Indent Encoding"));
        assert!(!prompt.contains("Anchor Line Numbers"));
        assert!(prompt.contains("Version Model"));
    }
}
