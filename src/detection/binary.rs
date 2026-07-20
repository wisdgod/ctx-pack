use anyhow::Result;
use std::io::Read;
use std::path::Path;

pub fn is_binary(path: &Path) -> Result<bool> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = [0u8; 8192];
    let n = file.read(&mut buf)?;
    let sample = &buf[..n];
    Ok(content_inspector::inspect(sample).is_binary())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_text_file_not_binary() {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "fn main() {{\n    println!(\"hello\");\n}}\n").unwrap();
        assert!(!is_binary(f.path()).unwrap());
    }

    #[test]
    fn test_binary_file_is_binary() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(&[0u8, 1, 2, 3, 0, 255, 254, 253]).unwrap();
        assert!(is_binary(f.path()).unwrap());
    }
}
