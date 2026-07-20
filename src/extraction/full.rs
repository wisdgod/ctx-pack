use super::{ExtractionResult, Fragment};

pub fn extract_full(content: &str) -> ExtractionResult {
    let total_lines = content.lines().count() as u32;
    let total_lines = if total_lines == 0 && !content.is_empty() { 1 } else { total_lines };

    ExtractionResult {
        fragments: vec![Fragment {
            line_start: 1,
            line_end: total_lines.max(1),
            content: content.to_string(),
        }],
        total_lines,
        is_partial: false,
    }
}
