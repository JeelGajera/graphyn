pub mod analyze;
pub mod query;
pub mod serve;
pub mod status;
pub mod watch;

use std::path::{Path, PathBuf};

/// Normalize canonicalized paths for Graphyn command usage.
///
/// On Windows, `canonicalize` can return extended-length paths (`\\?\...` or
/// `\\?\UNC\...`) that are problematic for downstream path consumers.
pub fn normalize_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        let stripped = if s.starts_with(r"\\?\UNC\") {
            format!(r"\\{}", &s[8..])
        } else if s.starts_with(r"\\?\") {
            s[4..].to_string()
        } else {
            s.into_owned()
        };
        PathBuf::from(stripped.replace('/', "\\"))
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

/// Convention: graph database lives at `<repo_root>/.graphyn/db`
pub fn db_path(repo_root: &Path) -> PathBuf {
    normalize_path(repo_root).join(".graphyn").join("db")
}
