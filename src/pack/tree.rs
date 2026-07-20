use std::fmt::Write as _;

pub struct FileVersion {
    pub fid: u32,
    pub display_path: String,
    pub generation: u32,
    pub pid: u32,
}

pub fn generate_tree(files: &[FileVersion]) -> String {
    let mut out = String::with_capacity(files.len() * 48);
    for f in files {
        if f.pid > 0 {
            let _ =
                writeln!(out, "[{}] {} (gen{}.pid{})", f.fid, f.display_path, f.generation, f.pid);
        } else {
            let _ = writeln!(out, "[{}] {} (gen{})", f.fid, f.display_path, f.generation);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_gen0() {
        let files = vec![
            FileVersion { fid: 1, display_path: "src/main.rs".to_string(), generation: 0, pid: 0 },
            FileVersion { fid: 2, display_path: "src/lib.rs".to_string(), generation: 0, pid: 0 },
        ];
        let tree = generate_tree(&files);
        assert_eq!(tree, "[1] src/main.rs (gen0)\n[2] src/lib.rs (gen0)\n");
    }

    #[test]
    fn test_tree_with_patches() {
        let files = vec![FileVersion {
            fid: 1,
            display_path: "src/main.rs".to_string(),
            generation: 1,
            pid: 2,
        }];
        let tree = generate_tree(&files);
        assert_eq!(tree, "[1] src/main.rs (gen1.pid2)\n");
    }
}
