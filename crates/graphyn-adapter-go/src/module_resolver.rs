use std::path::{Path, PathBuf};

pub struct GoModule {
    pub module_path: String,
    pub root: PathBuf,
}

impl GoModule {
    pub fn load(root: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(root.join("go.mod")).ok()?;
        let module_path = content
            .lines()
            .find(|l| l.trim_start().starts_with("module "))?
            .trim_start()
            .trim_start_matches("module ")
            .trim()
            .to_string();
        Some(Self {
            module_path,
            root: root.to_path_buf(),
        })
    }
}
