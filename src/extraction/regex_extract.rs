use anyhow::Result;
use regex::Regex;

use super::{ExtractionResult, Fragment};

pub fn extract_regex(content: &str, pattern: &str, context_lines: u32) -> Result<ExtractionResult> {
    let re = Regex::new(pattern)?;
    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len() as u32;

    let mut match_ranges: Vec<(u32, u32)> = Vec::new();

    for (i, line) in all_lines.iter().enumerate() {
        if re.is_match(line) {
            let line_num = (i + 1) as u32;
            let start = line_num.saturating_sub(context_lines).max(1);
            let end = (line_num + context_lines).min(total_lines);
            match_ranges.push((start, end));
        }
    }

    // Merge overlapping ranges
    let merged = merge_ranges(match_ranges);

    let mut fragments = Vec::new();
    for (start, end) in merged {
        let mut frag_content = join_lines(&all_lines[(start - 1) as usize..end as usize]);
        frag_content.push('\n');
        fragments.push(Fragment { line_start: start, line_end: end, content: frag_content });
    }

    Ok(ExtractionResult { fragments, total_lines, is_partial: true })
}

fn join_lines(lines: &[&str]) -> String {
    let len = lines.iter().map(|line| line.len()).sum::<usize>() + lines.len().saturating_sub(1);
    let mut out = String::with_capacity(len);
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(line);
    }
    out
}

fn merge_ranges(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    if ranges.is_empty() {
        return ranges;
    }
    ranges.sort_by_key(|r| r.0);
    let mut merged: Vec<(u32, u32)> = Vec::new();
    for (start, end) in ranges {
        if let Some(last) = merged.last_mut()
            && start <= last.1 + 1
        {
            last.1 = last.1.max(end);
            continue;
        }
        merged.push((start, end));
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_content(lines: &[&str]) -> String {
        lines.join("\n") + "\n"
    }

    #[test]
    fn test_regex_no_context() {
        let content = make_content(&[
            "fn main() {",
            "    let x = 1;",
            "    println!(\"{}\", x);",
            "}",
            "fn helper() {",
            "    todo!()",
            "}",
        ]);
        let result = extract_regex(&content, "fn ", 0).unwrap();
        assert_eq!(result.fragments.len(), 2);
        assert_eq!(result.fragments[0].line_start, 1);
        assert_eq!(result.fragments[1].line_start, 5);
    }

    #[test]
    fn test_regex_with_context_merge() {
        let content: String = (1..=10).map(|i| format!("line{}\n", i)).collect();
        // Match lines 3 and 5 with context_lines=2 -> ranges [1,5] and [3,7] -> merged [1,7]
        let result = extract_regex(&content, "line3|line5", 2).unwrap();
        assert_eq!(result.fragments.len(), 1);
        assert_eq!(result.fragments[0].line_start, 1);
        assert_eq!(result.fragments[0].line_end, 7);
    }

    #[test]
    fn test_regex_no_match() {
        let content = make_content(&["hello", "world"]);
        let result = extract_regex(&content, "nomatch", 0).unwrap();
        assert_eq!(result.fragments.len(), 0);
    }
}
