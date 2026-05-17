use graphyn_core::ir::ReExportEntry;

pub fn extract_all_exports(source: &str) -> Vec<ReExportEntry> {
    let mut out = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("__all__") && trimmed.contains('[') && trimmed.contains(']') {
            let inside = trimmed
                .split('[')
                .nth(1)
                .and_then(|s| s.split(']').next())
                .unwrap_or("");
            for part in inside.split(',') {
                let name = part.trim().trim_matches('"').trim_matches('\'');
                if !name.is_empty() {
                    out.push(ReExportEntry {
                        exported_name: name.to_string(),
                        source_module: ".".to_string(),
                    });
                }
            }
        }
    }
    out
}
