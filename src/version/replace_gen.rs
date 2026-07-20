use std::fmt::Write as _;

pub fn generate_replace(fid: u32, generation: u32, encoded_content: &str, prefix: &str) -> String {
    let mut out = String::with_capacity(encoded_content.len() + prefix.len() * 2 + 48);
    let _ = writeln!(out, "<{prefix}:replace fid=\"{fid}\" gen=\"{generation}\">");
    out.push_str(encoded_content);
    let _ = write!(out, "</{prefix}:replace>");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_replace() {
        let content = "[0]fn main() {}\n";
        let block = generate_replace(3, 2, content, "ctx");
        assert!(block.starts_with("<ctx:replace fid=\"3\" gen=\"2\">"));
        assert!(block.contains(content));
        assert!(block.ends_with("</ctx:replace>"));
    }
}
