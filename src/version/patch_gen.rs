use super::diff::DiffHunk;
use std::fmt::Write as _;

pub fn generate_patch(
    fid: u32,
    generation: u32,
    pid: u32,
    hunks: &[DiffHunk],
    prefix: &str,
    anchor_interval: u32,
) -> String {
    let mut out = String::with_capacity(hunks.len() * 96 + 64);
    let _ =
        writeln!(out, "<{p}:patch fid=\"{fid}\" gen=\"{generation}\" pid=\"{pid}\">", p = prefix);

    for hunk in hunks {
        let anchor_line = nearest_anchor_line(hunk.old_start, anchor_interval);

        let _ = writeln!(out, "@@ anchor:{anchor_line} @@");

        for ctx in &hunk.context_before {
            let _ = writeln!(out, " {ctx}");
        }
        for rem in &hunk.removes {
            out.push_str(rem);
            out.push('\n');
        }
        for add in &hunk.adds {
            out.push_str(add);
            out.push('\n');
        }
        for ctx in &hunk.context_after {
            let _ = writeln!(out, " {ctx}");
        }
    }

    let _ = write!(out, "</{p}:patch>", p = prefix);
    out
}

fn nearest_anchor_line(line: u32, anchor_interval: u32) -> u32 {
    if line == 0 {
        return 1;
    }
    if anchor_interval == 0 {
        return line;
    }

    let base = (line / anchor_interval) * anchor_interval;

    base.max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::version::diff::{DiffHunk, compute_line_diff};

    #[test]
    fn test_generate_patch_basic() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nchanged\nline3\n";
        let hunks = compute_line_diff(old, new);
        let patch = generate_patch(1, 0, 1, &hunks, "ctx", 10);
        assert!(patch.contains("<ctx:patch fid=\"1\" gen=\"0\" pid=\"1\">"));
        assert!(patch.contains("@@ anchor:"));
        assert!(patch.contains("-line2") || patch.contains("line2"));
        assert!(patch.contains("+changed") || patch.contains("changed"));
        assert!(patch.contains("</ctx:patch>"));
    }

    #[test]
    fn test_generate_patch_no_anchor() {
        let hunks = vec![DiffHunk {
            old_start: 5,
            removes: vec!["-old line".to_string()],
            adds: vec!["+new line".to_string()],
            context_before: vec!["ctx".to_string()],
            context_after: vec![],
        }];
        let patch = generate_patch(2, 1, 3, &hunks, "ctx", 0);
        assert!(patch.contains("fid=\"2\" gen=\"1\" pid=\"3\""));
        assert!(patch.contains("@@ anchor:5 @@"));
    }

    #[test]
    fn test_anchor_line_boundaries() {
        assert_eq!(nearest_anchor_line(1, 10), 1);
        assert_eq!(nearest_anchor_line(9, 10), 1);
        assert_eq!(nearest_anchor_line(10, 10), 10);
        assert_eq!(nearest_anchor_line(11, 10), 10);
        assert_eq!(nearest_anchor_line(20, 10), 20);
        assert_eq!(nearest_anchor_line(20, 0), 20);
    }
}
