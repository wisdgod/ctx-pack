use super::traits::TransformStage;
use std::fmt::Write as _;

pub struct AnchorEncoder {
    interval: u32,
}

impl AnchorEncoder {
    pub fn new(interval: u32) -> Self {
        AnchorEncoder { interval }
    }

    fn compute_width(total_lines: usize) -> usize {
        let mut digits = 1;
        let mut n = total_lines;
        while n >= 10 {
            digits += 1;
            n /= 10;
        }
        digits.max(3) + 1
    }
}

impl TransformStage for AnchorEncoder {
    fn encode(&self, input: &str) -> String {
        if self.interval == 0 {
            return input.to_string();
        }

        let (body, trailing_newline) = if let Some(stripped) = input.strip_suffix('\n') {
            (stripped, true)
        } else {
            (input, false)
        };
        let total = body.split('\n').count();
        let width = Self::compute_width(total);

        let mut out = String::with_capacity(input.len() + total * (width + 3));
        for (i, line) in body.split('\n').enumerate() {
            if i > 0 {
                out.push('\n');
            }
            let line_num = i + 1;
            let should_anchor = line_num == 1 || (line_num % self.interval as usize == 0);
            if should_anchor {
                let _ = write!(out, "{line_num:>width$} | {line}");
            } else {
                for _ in 0..width {
                    out.push(' ');
                }
                out.push_str(" | ");
                out.push_str(line);
            }
        }

        if trailing_newline {
            out.push('\n');
        }
        out
    }

    fn decode(&self, input: &str) -> String {
        if self.interval == 0 {
            return input.to_string();
        }

        let (body, trailing_newline) = if let Some(stripped) = input.strip_suffix('\n') {
            (stripped, true)
        } else {
            (input, false)
        };

        let mut out = String::with_capacity(input.len());
        for (i, line) in body.split('\n').enumerate() {
            if i > 0 {
                out.push('\n');
            }
            // Strip prefix of form: `\s*\d*\s* | `
            if let Some(rest) = strip_anchor_prefix(line) {
                out.push_str(rest);
            } else {
                out.push_str(line);
            }
        }

        if trailing_newline {
            out.push('\n');
        }
        out
    }
}

pub fn strip_anchor_prefix(line: &str) -> Option<&str> {
    // Pattern: optional digits and spaces, then " | " separator
    let trimmed = line.trim_start();
    // Try to find " | " after digits/spaces from start
    // The format is: `{spaces_or_digits:width} | {content}`
    // We need to find the first " | " occurrence
    if let Some(pos) = line.find(" | ") {
        // Verify everything before pos is whitespace or digits
        let prefix = &line[..pos];
        if prefix.bytes().all(|b| b == b' ' || b.is_ascii_digit()) {
            return Some(&line[pos + 3..]);
        }
    }
    // Fallback: if trimmed starts with digits then " | "
    let _ = trimmed;
    None
}

pub fn strip_anchor_prefixes(input: &str) -> String {
    let (body, trailing_newline) = if let Some(stripped) = input.strip_suffix('\n') {
        (stripped, true)
    } else {
        (input, false)
    };

    let mut out = String::with_capacity(input.len());
    for (i, line) in body.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(strip_anchor_prefix(line).unwrap_or(line));
    }

    if trailing_newline {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_zero_identity() {
        let enc = AnchorEncoder::new(0);
        let text = "hello\nworld\n";
        assert_eq!(enc.encode(text), text);
        assert_eq!(enc.decode(text), text);
    }

    #[test]
    fn test_anchor_first_line_always() {
        let enc = AnchorEncoder::new(5);
        let text = "line1\nline2\nline3\n";
        let encoded = enc.encode(text);
        let lines: Vec<&str> = encoded.lines().collect();
        // First line should have "1" in it
        assert!(
            lines[0].contains(" 1 | ") || lines[0].trim_start().starts_with("1 | "),
            "first line should have line number: {}",
            lines[0]
        );
        // Second and third lines should not have numbers (interval=5, only line 1 and 5 get anchors)
        assert!(
            lines[1].starts_with("  ") || lines[1].starts_with("   "),
            "second line should have blank prefix: {}",
            lines[1]
        );
    }

    #[test]
    fn test_anchor_interval_5_12_lines() {
        let enc = AnchorEncoder::new(5);
        let lines_input: Vec<String> = (1..=12).map(|i| format!("line{}", i)).collect();
        let text = lines_input.join("\n") + "\n";
        let encoded = enc.encode(&text);
        let encoded_lines: Vec<&str> = encoded.lines().collect();

        // Line 1 anchored
        assert!(encoded_lines[0].contains("1 | "));
        // Line 2 not anchored (space prefix)
        assert!(!encoded_lines[1].trim_start_matches(' ').starts_with("2 | "));
        // Line 5 anchored
        assert!(encoded_lines[4].contains("5 | "));
        // Line 10 anchored
        assert!(encoded_lines[9].contains("10 | "));
    }

    #[test]
    fn test_round_trip() {
        let enc = AnchorEncoder::new(10);
        let inputs = vec![
            "fn main() {\n    let x = 1;\n}\n",
            "single line\n",
            "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10\nline11\n",
        ];
        for text in inputs {
            let encoded = enc.encode(text);
            let decoded = enc.decode(&encoded);
            assert_eq!(decoded, text, "round-trip failed for: {:?}", text);
        }
    }

    #[test]
    fn test_large_file_round_trip() {
        let enc = AnchorEncoder::new(10);
        let lines: Vec<String> = (1..=150).map(|i| format!("    line number {}", i)).collect();
        let text = lines.join("\n") + "\n";
        let encoded = enc.encode(&text);
        let decoded = enc.decode(&encoded);
        assert_eq!(decoded, text);
    }

    #[test]
    fn test_strip_anchor_prefixes_preserves_content_projection() {
        let input = "   1 | [0]fn main() {\n     | [4]old();\n  10 | [0]}\n";
        let stripped = strip_anchor_prefixes(input);
        assert_eq!(stripped, "[0]fn main() {\n[4]old();\n[0]}\n");
    }
}
