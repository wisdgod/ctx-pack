use super::traits::TransformStage;
use std::fmt::Write as _;

pub struct IndentEncoder {
    tab_width: u32,
}

impl IndentEncoder {
    pub fn new(tab_width: u32) -> Self {
        IndentEncoder { tab_width }
    }

    fn count_leading_spaces<'a>(&self, line: &'a str) -> (u32, &'a str) {
        let mut count = 0u32;
        let bytes = line.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b' ' => {
                    count += 1;
                    i += 1;
                }
                b'\t' => {
                    count += self.tab_width;
                    i += 1;
                }
                _ => break,
            }
        }
        (count, &line[i..])
    }
}

impl TransformStage for IndentEncoder {
    fn encode(&self, input: &str) -> String {
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
            if line.is_empty() {
                // empty line stays empty
            } else {
                let (n, rest) = self.count_leading_spaces(line);
                if rest.is_empty() {
                    // pure whitespace line
                    let _ = write!(out, "[{n}]");
                } else {
                    let _ = write!(out, "[{n}]{rest}");
                }
            }
        }

        if trailing_newline {
            out.push('\n');
        }
        out
    }

    fn decode(&self, input: &str) -> String {
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
            if line.is_empty() {
                // empty line stays empty
            } else if let Some(rest) = line.strip_prefix('[') {
                if let Some(bracket_end) = rest.find(']') {
                    if let Ok(n) = rest[..bracket_end].parse::<u32>() {
                        for _ in 0..n {
                            out.push(' ');
                        }
                        out.push_str(&rest[bracket_end + 1..]);
                    } else {
                        out.push_str(line);
                    }
                } else {
                    out.push_str(line);
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn enc(s: &str) -> String {
        IndentEncoder::new(4).encode(s)
    }

    fn dec(s: &str) -> String {
        IndentEncoder::new(4).decode(s)
    }

    #[test]
    fn test_encode_empty() {
        assert_eq!(enc(""), "");
    }

    #[test]
    fn test_encode_no_indent() {
        assert_eq!(enc("hello"), "[0]hello");
    }

    #[test]
    fn test_encode_four_spaces() {
        assert_eq!(enc("    let x = 1;"), "[4]let x = 1;");
    }

    #[test]
    fn test_encode_eight_spaces() {
        assert_eq!(enc("        deep();"), "[8]deep();");
    }

    #[test]
    fn test_encode_tabs() {
        assert_eq!(enc("\t\tcode"), "[8]code");
    }

    #[test]
    fn test_encode_pure_whitespace() {
        assert_eq!(enc("    "), "[4]");
    }

    #[test]
    fn test_decode_four_spaces() {
        assert_eq!(dec("[4]let x = 1;"), "    let x = 1;");
    }

    #[test]
    fn test_decode_no_indent() {
        assert_eq!(dec("[0]hello"), "hello");
    }

    #[test]
    fn test_decode_empty() {
        assert_eq!(dec(""), "");
    }

    #[test]
    fn test_decode_pure_whitespace() {
        assert_eq!(dec("[4]"), "    ");
    }

    #[test]
    fn test_round_trip_multiline() {
        let encoder = IndentEncoder::new(4);
        let inputs = vec![
            "fn main() {\n    let x = 1;\n}\n",
            "no indent\n    indented\n        deeply\n",
            "\n\n\n",
            "    pure whitespace\n",
            "mixed\n    indented\n\n    back\n",
        ];
        for text in inputs {
            let encoded = encoder.encode(text);
            let decoded = encoder.decode(&encoded);
            assert_eq!(decoded, text, "round-trip failed for: {:?}", text);
        }
    }

    #[test]
    fn test_trailing_newline_preserved() {
        let encoder = IndentEncoder::new(4);
        let with_nl = "hello\n";
        let without_nl = "hello";
        assert!(encoder.encode(with_nl).ends_with('\n'));
        assert!(!encoder.encode(without_nl).ends_with('\n'));
    }
}
