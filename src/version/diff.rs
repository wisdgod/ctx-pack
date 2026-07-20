use similar::{ChangeTag, TextDiff};

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub old_start: u32,
    pub removes: Vec<String>,
    pub adds: Vec<String>,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

pub fn compute_change_ratio(old: &str, new: &str) -> f64 {
    if old == new {
        return 0.0;
    }
    let diff = TextDiff::from_lines(old, new);
    let total = diff.iter_all_changes().count();
    if total == 0 {
        return 0.0;
    }
    let changed = diff.iter_all_changes().filter(|c| c.tag() != ChangeTag::Equal).count();
    changed as f64 / total as f64
}

pub fn compute_line_diff(old: &str, new: &str) -> Vec<DiffHunk> {
    let diff = TextDiff::from_lines(old, new);
    let mut hunks = Vec::new();

    for group in diff.grouped_ops(3) {
        let mut removes = Vec::new();
        let mut adds = Vec::new();
        let mut context_before = Vec::new();
        let mut context_after = Vec::new();
        let mut old_start = 0u32;
        let mut in_context_before = true;
        let mut first = true;

        for op in &group {
            for change in diff.iter_changes(op) {
                match change.tag() {
                    ChangeTag::Equal => {
                        let line = change.value().trim_end_matches('\n').to_string();
                        if first {
                            old_start = change.old_index().map(|i| i as u32 + 1).unwrap_or(0);
                        }
                        if in_context_before {
                            context_before.push(line);
                        } else {
                            context_after.push(line);
                        }
                    }
                    ChangeTag::Delete => {
                        if first {
                            old_start = change.old_index().map(|i| i as u32 + 1).unwrap_or(0);
                            first = false;
                        }
                        in_context_before = false;
                        context_after.clear();
                        let value = change.value().trim_end_matches('\n');
                        let mut line = String::with_capacity(value.len() + 1);
                        line.push('-');
                        line.push_str(value);
                        removes.push(line);
                    }
                    ChangeTag::Insert => {
                        if first {
                            old_start = change.old_index().map(|i| i as u32 + 1).unwrap_or(0);
                            first = false;
                        }
                        in_context_before = false;
                        context_after.clear();
                        let value = change.value().trim_end_matches('\n');
                        let mut line = String::with_capacity(value.len() + 1);
                        line.push('+');
                        line.push_str(value);
                        adds.push(line);
                    }
                }
            }
        }

        if !removes.is_empty() || !adds.is_empty() {
            hunks.push(DiffHunk { old_start, removes, adds, context_before, context_after });
        }
    }

    hunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratio_identical() {
        assert_eq!(compute_change_ratio("hello\n", "hello\n"), 0.0);
    }

    #[test]
    fn test_ratio_completely_different() {
        let ratio = compute_change_ratio("aaa\nbbb\nccc\n", "xxx\nyyy\nzzz\n");
        assert!(ratio > 0.9, "ratio should be high: {}", ratio);
    }

    #[test]
    fn test_ratio_small_change() {
        let old = "line1\nline2\nline3\nline4\nline5\n";
        let new = "line1\nline2\nchanged\nline4\nline5\n";
        let ratio = compute_change_ratio(old, new);
        assert!(ratio < 0.5, "ratio should be low for small change: {}", ratio);
    }

    #[test]
    fn test_diff_hunks() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nchanged\nline3\n";
        let hunks = compute_line_diff(old, new);
        assert!(!hunks.is_empty());
        assert!(hunks[0].removes.iter().any(|r| r.contains("line2")));
        assert!(hunks[0].adds.iter().any(|a| a.contains("changed")));
    }
}
