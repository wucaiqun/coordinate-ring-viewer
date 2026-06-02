use std::path::PathBuf;

pub const RING_JOIN_SEPARATOR: &str = "===============";

/// Directory with bundled sample coordinate files (`examples/`).
pub fn examples_dir() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        let dir = cwd.join("examples");
        if dir.is_dir() {
            return dir;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let dir = parent.join("examples");
            if dir.is_dir() {
                return dir;
            }
        }
    }
    PathBuf::from("examples")
}

/// Relative paths for sample files (UI display only).
pub const EXAMPLES_SAMPLE_PATHS: &str =
    "examples/beijing_2d.txt, examples/beijing_3d.txt";

/// Open a native file picker (multi-select). Returns chosen paths, if any.
pub fn pick_coordinate_files(dialog_title: &str) -> Option<Vec<PathBuf>> {
    let start = examples_dir();
    let mut dialog = rfd::FileDialog::new()
        .set_title(dialog_title)
        .add_filter("Coordinate text", &["txt", "csv", "dat", "geo"])
        .add_filter("All files", &["*"]);
    if start.is_dir() {
        dialog = dialog.set_directory(&start);
    }
    dialog.pick_files()
}

/// Read one or more text files and merge them for the input box.
///
/// - One file: file contents become the input (trimmed).
/// - Multiple files: each file is one block, blocks are joined with `===============`
///   (same rule as rings inside a single file).
pub fn merge_files(paths: &[PathBuf]) -> Result<String, String> {
    if paths.is_empty() {
        return Err("No files selected.".to_string());
    }

    let mut blocks = Vec::new();
    for path in paths {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
        let trimmed = content.trim();
        if !trimmed.is_empty() {
            blocks.push(trimmed.to_string());
        }
    }

    if blocks.is_empty() {
        return Err("Selected file(s) contain no coordinate data.".to_string());
    }

    Ok(blocks.join(&format!("\n{RING_JOIN_SEPARATOR}\n")))
}

pub fn format_import_status(paths: &[PathBuf], merged_from_multiple: bool) -> String {
    if paths.len() == 1 {
        format!("Imported: {}", paths[0].display())
    } else if merged_from_multiple {
        format!(
            "Imported {} files (merged with {RING_JOIN_SEPARATOR}): {}",
            paths.len(),
            paths
                .iter()
                .map(|p| p.file_name().unwrap_or_default().to_string_lossy())
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        format!("Imported {} files.", paths.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_paths_are_relative() {
        assert!(!EXAMPLES_SAMPLE_PATHS.starts_with('/'));
        assert!(EXAMPLES_SAMPLE_PATHS.contains("examples/beijing_2d.txt"));
        assert!(EXAMPLES_SAMPLE_PATHS.contains("examples/beijing_3d.txt"));
    }

    #[test]
    fn merge_single_file() {
        let dir = std::env::temp_dir().join("geo_ring_viewer_test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("a.txt");
        std::fs::write(&path, "1.0 2.0\n3.0 4.0\n").unwrap();
        let merged = merge_files(&[path]).unwrap();
        assert!(merged.contains("1.0 2.0"));
    }

    #[test]
    fn merge_multiple_files_adds_separator() {
        let dir = std::env::temp_dir().join("geo_ring_viewer_test2");
        let _ = std::fs::create_dir_all(&dir);
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        std::fs::write(&a, "1.0 2.0").unwrap();
        std::fs::write(&b, "3.0 4.0").unwrap();
        let merged = merge_files(&[a, b]).unwrap();
        assert!(merged.contains(RING_JOIN_SEPARATOR));
    }
}
