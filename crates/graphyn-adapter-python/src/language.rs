use std::path::Path;

use graphyn_core::ir::Language;

pub fn detect_language(path: &Path) -> Option<Language> {
    match path.extension()?.to_string_lossy().to_ascii_lowercase().as_str() {
        "py" | "pyi" => Some(Language::Python),
        _ => None,
    }
}

pub fn is_supported_source_file(path: &Path) -> bool {
    detect_language(path).is_some()
}
