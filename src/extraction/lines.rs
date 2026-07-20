use anyhow::Result;

use super::{ExtractionResult, Fragment};

#[derive(Debug, Clone)]
struct LineRange {
    start: u32,
    end: Option<u32>,
}

fn parse_ranges(ranges: &str) -> Result<Vec<LineRange>> {
    let mut result = Vec::new();
    for part in ranges.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(dash_pos) = part.find('-') {
            let start_str = &part[..dash_pos];
            let end_str = &part[dash_pos + 1..];
            let start: u32 = start_str
                .trim()
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid line range start: {}", start_str))?;
            let end = if end_str.trim().is_empty() {
                None
            } else {
                Some(
                    end_str
                        .trim()
                        .parse::<u32>()
                        .map_err(|_| anyhow::anyhow!("invalid line range end: {}", end_str))?,
                )
            };
            result.push(LineRange { start, end });
        } else {
            let n: u32 =
                part.parse().map_err(|_| anyhow::anyhow!("invalid line number: {}", part))?;
            result.push(LineRange { start: n, end: Some(n) });
        }
    }
    Ok(result)
}

pub fn extract_lines(content: &str, ranges: &str) -> Result<ExtractionResult> {
    let all_lines: Vec<&str> = content.lines().collect();
    let total_lines = all_lines.len() as u32;

    let parsed_ranges = parse_ranges(ranges)?;
    let mut fragments = Vec::new();

    for range in &parsed_ranges {
        let start = range.start.max(1);
        let end = range.end.unwrap_or(total_lines).min(total_lines);
        if start > total_lines || start > end {
            continue;
        }
        let mut frag_content = join_lines(&all_lines[(start - 1) as usize..end as usize]);
        if content.ends_with('\n') || range.end.is_none() {
            frag_content.push('\n');
        }
        fragments.push(Fragment { line_start: start, line_end: end, content: frag_content });
    }

    let covers_all = parsed_ranges.len() == 1
        && parsed_ranges[0].start == 1
        && parsed_ranges[0].end.map(|e| e >= total_lines).unwrap_or(true);

    Ok(ExtractionResult { fragments, total_lines, is_partial: !covers_all })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_content(n: u32) -> String {
        (1..=n).map(|i| format!("line{}", i)).collect::<Vec<_>>().join("\n") + "\n"
    }

    #[test]
    fn test_lines_1_5() {
        let content = make_content(10);
        let result = extract_lines(&content, "1-5").unwrap();
        assert_eq!(result.fragments.len(), 1);
        assert_eq!(result.fragments[0].line_start, 1);
        assert_eq!(result.fragments[0].line_end, 5);
        assert!(result.is_partial);
    }

    #[test]
    fn test_lines_two_ranges() {
        let content = make_content(10);
        let result = extract_lines(&content, "3-5,8-10").unwrap();
        assert_eq!(result.fragments.len(), 2);
        assert_eq!(result.fragments[0].line_start, 3);
        assert_eq!(result.fragments[0].line_end, 5);
        assert_eq!(result.fragments[1].line_start, 8);
        assert_eq!(result.fragments[1].line_end, 10);
    }

    #[test]
    fn test_lines_to_end() {
        let content = make_content(10);
        let result = extract_lines(&content, "5-").unwrap();
        assert_eq!(result.fragments.len(), 1);
        assert_eq!(result.fragments[0].line_start, 5);
        assert_eq!(result.fragments[0].line_end, 10);
    }

    #[test]
    fn test_parse_single_line() {
        let content = make_content(10);
        let result = extract_lines(&content, "3").unwrap();
        assert_eq!(result.fragments.len(), 1);
        assert_eq!(result.fragments[0].line_start, 3);
        assert_eq!(result.fragments[0].line_end, 3);
    }
}
