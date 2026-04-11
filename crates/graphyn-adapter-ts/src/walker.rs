use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::language::is_supported_source_file;

pub fn walk_source_files(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut out = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| !is_ignored_dir(e.path()))
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && is_supported_source_file(path) {
            out.push(path.to_path_buf());
        }
    }

    out.sort();
    Ok(out)
}

fn is_ignored_dir(path: &Path) -> bool {
    path.file_name()
        .map(|name| {
            let name = name.to_string_lossy();
            name == "node_modules" || name == ".git" || name == "target"
        })
        .unwrap_or(false)
}
