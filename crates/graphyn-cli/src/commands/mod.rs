pub mod analyze;
pub mod query;
pub mod serve;
pub mod status;
pub mod watch;

use std::path::{Path, PathBuf};

/// Convention: graph database lives at `<repo_root>/.graphyn/db`
pub fn db_path(repo_root: &Path) -> PathBuf {
    repo_root.join(".graphyn").join("db")
}
