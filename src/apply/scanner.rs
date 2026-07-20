use regex::Regex;

#[derive(Debug, Clone)]
pub enum ScannedBlock {
    Patch { fid: u32, generation: u32, pid: u32, body: String },
    Replace { fid: u32, generation: u32, body: String },
}

/// The protocol writes `<tag>\ncontent</tag>`; the newline after the open tag
/// belongs to the tag, not the content.
fn strip_tag_newline(body: &str) -> &str {
    body.strip_prefix("\r\n").or_else(|| body.strip_prefix('\n')).unwrap_or(body)
}

pub fn scan_blocks(input: &str, prefix: &str) -> Vec<ScannedBlock> {
    let mut blocks = Vec::new();
    let block_pattern = format!(
        r#"<{p}:patch\s+fid="(\d+)"\s+gen="(\d+)"\s+pid="(\d+)">([\s\S]*?)</{p}:patch>|<{p}:replace\s+fid="(\d+)"\s+gen="(\d+)">([\s\S]*?)</{p}:replace>"#,
        p = regex::escape(prefix)
    );

    if let Ok(re) = Regex::new(&block_pattern) {
        for cap in re.captures_iter(input) {
            if let (Some(fid), Some(generation), Some(pid), Some(body)) =
                (cap.get(1), cap.get(2), cap.get(3), cap.get(4))
            {
                match (
                    fid.as_str().parse::<u32>(),
                    generation.as_str().parse::<u32>(),
                    pid.as_str().parse::<u32>(),
                ) {
                    (Ok(fid), Ok(generation), Ok(pid)) => {
                        blocks.push(ScannedBlock::Patch {
                            fid,
                            generation,
                            pid,
                            body: strip_tag_newline(body.as_str()).to_string(),
                        });
                    }
                    _ => tracing::warn!(
                        "failed to parse patch block attributes: fid={} gen={} pid={}",
                        fid.as_str(),
                        generation.as_str(),
                        pid.as_str()
                    ),
                }
            } else if let (Some(fid), Some(generation), Some(body)) =
                (cap.get(5), cap.get(6), cap.get(7))
            {
                match (fid.as_str().parse::<u32>(), generation.as_str().parse::<u32>()) {
                    (Ok(fid), Ok(generation)) => {
                        blocks.push(ScannedBlock::Replace {
                            fid,
                            generation,
                            body: strip_tag_newline(body.as_str()).to_string(),
                        });
                    }
                    _ => tracing::warn!(
                        "failed to parse replace block attributes: fid={} gen={}",
                        fid.as_str(),
                        generation.as_str()
                    ),
                }
            }
        }
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_patch() {
        let input = r#"Some free text here.

<ctx:patch fid="1" gen="0" pid="1">
@@ anchor:1 @@
 context
-old line
+new line
</ctx:patch>

More text.
"#;
        let blocks = scan_blocks(input, "ctx");
        assert_eq!(blocks.len(), 1);
        if let ScannedBlock::Patch { fid, generation, pid, body } = &blocks[0] {
            assert_eq!(*fid, 1);
            assert_eq!(*generation, 0);
            assert_eq!(*pid, 1);
            assert!(body.contains("@@ anchor:1 @@"));
        } else {
            panic!("expected patch block");
        }
    }

    #[test]
    fn test_scan_replace() {
        let input = r#"<ctx:replace fid="2" gen="1">
[0]fn main() {}
</ctx:replace>"#;
        let blocks = scan_blocks(input, "ctx");
        assert_eq!(blocks.len(), 1);
        if let ScannedBlock::Replace { fid, generation, body } = &blocks[0] {
            assert_eq!(*fid, 2);
            assert_eq!(*generation, 1);
            assert!(body.contains("fn main()"));
        } else {
            panic!("expected replace block");
        }
    }

    #[test]
    fn test_scan_multiple_mixed() {
        let input = r#"
<ctx:patch fid="1" gen="0" pid="1">
-old
+new
</ctx:patch>
<ctx:replace fid="2" gen="1">
content
</ctx:replace>
<ctx:patch fid="3" gen="0" pid="2">
-a
+b
</ctx:patch>
"#;
        let blocks = scan_blocks(input, "ctx");
        assert_eq!(blocks.len(), 3);
    }

    #[test]
    fn test_scan_preserves_text_order() {
        let input = r#"
<ctx:replace fid="2" gen="1">
content
</ctx:replace>
<ctx:patch fid="1" gen="0" pid="1">
-old
+new
</ctx:patch>
"#;
        let blocks = scan_blocks(input, "ctx");
        assert_eq!(blocks.len(), 2);
        assert!(matches!(blocks[0], ScannedBlock::Replace { fid: 2, .. }));
        assert!(matches!(blocks[1], ScannedBlock::Patch { fid: 1, .. }));
    }

    #[test]
    fn test_scan_malformed_skipped() {
        let input = r#"
<ctx:patch fid="abc" gen="0" pid="1">
body
</ctx:patch>
<ctx:patch fid="1" gen="0" pid="2">
valid
</ctx:patch>
"#;
        // "abc" won't parse as u32, should be skipped
        let blocks = scan_blocks(input, "ctx");
        // Valid one should be present
        assert!(blocks.iter().any(|b| matches!(b, ScannedBlock::Patch { fid: 1, .. })));
    }

    #[test]
    fn test_scan_different_prefix() {
        let input = r#"<myprefix:replace fid="5" gen="2">
content
</myprefix:replace>"#;
        let blocks = scan_blocks(input, "myprefix");
        assert_eq!(blocks.len(), 1);
        if let ScannedBlock::Replace { fid, generation, .. } = &blocks[0] {
            assert_eq!(*fid, 5);
            assert_eq!(*generation, 2);
        }
    }
}
