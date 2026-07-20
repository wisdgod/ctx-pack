use anyhow::{Context, Result};
use std::path::Path;

pub fn read_to_utf8(path: &Path, detect_encoding: bool) -> Result<String> {
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;

    if let Ok(s) = std::str::from_utf8(&bytes) {
        return Ok(s.to_string());
    }

    if !detect_encoding {
        return String::from_utf8(bytes).with_context(|| {
            format!(
                "failed to decode {} as UTF-8; enable encoding_detection to convert legacy encodings",
                path.display()
            )
        });
    }

    // Try to detect and convert encoding using encoding_rs
    let (encoding, _bom_stripped) = encoding_rs::Encoding::for_bom(&bytes)
        .map(|(enc, _len)| (enc, true))
        .unwrap_or_else(|| {
            // Use a heuristic: try UTF-16, fall back to windows-1252
            let enc = encoding_rs::UTF_8;
            (enc, false)
        });

    let (decoded, actual_encoding, had_errors) = encoding.decode(&bytes);
    if had_errors {
        // Try windows-1252 as last resort
        let (decoded2, _, had_errors2) = encoding_rs::WINDOWS_1252.decode(&bytes);
        if !had_errors2 {
            return Ok(decoded2.into_owned());
        }
        anyhow::bail!(
            "failed to decode {} as {} (had errors)",
            path.display(),
            actual_encoding.name()
        );
    }
    Ok(decoded.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_utf8_file() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "hello UTF-8 world").unwrap();
        let content = read_to_utf8(f.path(), true).unwrap();
        assert_eq!(content, "hello UTF-8 world\n");
    }

    #[test]
    fn test_latin1_file() {
        let mut f = NamedTempFile::new().unwrap();
        // Latin-1 encoded string with non-UTF-8 bytes
        f.write_all(&[b'h', b'e', b'l', b'l', b'o', 0xe9, b'\n']).unwrap();
        // Should succeed (converted to UTF-8)
        let content = read_to_utf8(f.path(), true).unwrap();
        assert!(content.starts_with("hello"));
    }

    #[test]
    fn test_non_utf8_rejected_when_detection_disabled() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(&[b'h', b'e', b'l', b'l', b'o', 0xe9, b'\n']).unwrap();
        let err = read_to_utf8(f.path(), false).unwrap_err();
        assert!(err.to_string().contains("enable encoding_detection"));
    }
}
