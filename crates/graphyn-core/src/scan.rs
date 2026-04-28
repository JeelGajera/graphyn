use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Default)]
pub struct ScanConfig {
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub respect_gitignore: bool,
}

impl ScanConfig {
    pub fn default_enabled() -> Self {
        Self {
            include_patterns: Vec::new(),
            exclude_patterns: Vec::new(),
            respect_gitignore: true,
        }
    }
}

pub fn parse_csv_patterns(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|p| p.replace('\\', "/"))
        .collect()
}

pub fn path_matches_any(path: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| pattern_matches(path, p))
}

pub fn pattern_matches(path: &str, pattern: &str) -> bool {
    let path = normalize(path);
    let pattern = normalize(pattern);

    if pattern.is_empty() {
        return false;
    }

    if !pattern.contains('/') {
        if wildcard_match(&path, &pattern) {
            return true;
        }
        return path.split('/').any(|seg| wildcard_match(seg, &pattern));
    }

    if let Some(tail) = pattern.strip_prefix("**/") {
        return path.split('/').enumerate().any(|(idx, _)| {
            wildcard_match(
                &path.split('/').skip(idx).collect::<Vec<_>>().join("/"),
                tail,
            )
        });
    }

    if wildcard_match(&path, &pattern) {
        return true;
    }

    if path.len() >= pattern.len() {
        return path.ends_with(&pattern);
    }

    false
}

fn wildcard_match(text: &str, pattern: &str) -> bool {
    let t = text.as_bytes();
    let p = pattern.as_bytes();

    let (mut ti, mut pi) = (0usize, 0usize);
    let mut star = None::<usize>;
    let mut match_i = 0usize;

    while ti < t.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == t[ti]) {
            ti += 1;
            pi += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            pi += 1;
            match_i = ti;
        } else if let Some(star_pos) = star {
            pi = star_pos + 1;
            match_i += 1;
            ti = match_i;
        } else {
            return false;
        }
    }

    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }

    pi == p.len()
}

fn normalize(input: &str) -> String {
    input
        .trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_matches('/')
        .to_string()
}

#[derive(Debug, Clone)]
pub struct GitignoreRule {
    pub pattern: String,
    pub negated: bool,
    pub directory_only: bool,
}

pub fn load_root_gitignore_rules(root: &Path) -> Vec<GitignoreRule> {
    let path = root.join(".gitignore");
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let negated = line.starts_with('!');
            let mut body = if negated { &line[1..] } else { line };
            body = body.trim();
            if body.is_empty() {
                return None;
            }

            let directory_only = body.ends_with('/');
            let pattern = body.trim_end_matches('/').replace('\\', "/");
            Some(GitignoreRule {
                pattern,
                negated,
                directory_only,
            })
        })
        .collect()
}

pub fn is_ignored_by_rules(rel_path: &str, _is_dir: bool, rules: &[GitignoreRule]) -> bool {
    let path = normalize(rel_path);
    if path.is_empty() {
        return false;
    }

    let mut ignored = false;

    for rule in rules {
        let anchored = rule.pattern.starts_with('/');
        let rule_pattern = rule.pattern.trim_start_matches('/');
        let dir_prefix = format!("{rule_pattern}/");

        let matches_candidate = |candidate: &str| {
            if rule.directory_only {
                candidate == rule_pattern || candidate.starts_with(&dir_prefix)
            } else {
                pattern_matches(candidate, rule_pattern)
            }
        };

        let matched = if anchored {
            matches_candidate(&path)
        } else {
            matches_candidate(&path)
                || path.split('/').enumerate().any(|(idx, _)| {
                    matches_candidate(&path.split('/').skip(idx).collect::<Vec<_>>().join("/"))
                })
        };

        if matched {
            ignored = !rule.negated;
        }
    }

    ignored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_patterns() {
        let patterns = parse_csv_patterns(Some("src/**,  tests/*.ts , ,node_modules/**"));
        assert_eq!(
            patterns,
            vec![
                "src/**".to_string(),
                "tests/*.ts".to_string(),
                "node_modules/**".to_string()
            ]
        );
    }

    #[test]
    fn test_pattern_matches_globs_and_suffix() {
        assert!(pattern_matches("src/a/b/file.ts", "src/**"));
        assert!(pattern_matches("src/a/b/file.ts", "**/*.ts"));
        assert!(pattern_matches("src/a/b/file.ts", "*.ts"));
        assert!(pattern_matches("src/a/b/file.ts", "a/b/file.ts"));
        assert!(!pattern_matches("src/a/b/file.ts", "*.tsx"));
    }

    #[test]
    fn test_gitignore_rule_evaluation_with_negation() {
        let rules = vec![
            GitignoreRule {
                pattern: "dist".to_string(),
                negated: false,
                directory_only: true,
            },
            GitignoreRule {
                pattern: "dist/keep.ts".to_string(),
                negated: true,
                directory_only: false,
            },
        ];

        assert!(is_ignored_by_rules("dist", true, &rules));
        assert!(is_ignored_by_rules("dist/a.ts", false, &rules));
        assert!(!is_ignored_by_rules("dist/keep.ts", false, &rules));
    }
}

pub fn walk_source_files_with_config<F>(
    root: &Path,
    config: &ScanConfig,
    is_supported: F,
) -> Result<Vec<PathBuf>, std::io::Error>
where
    F: Fn(&Path) -> bool,
{
    let mut out = Vec::new();
    let rules = if config.respect_gitignore {
        load_root_gitignore_rules(root)
    } else {
        Vec::new()
    };

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| should_descend(root, e.path(), config, &rules))
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !is_supported(path) {
            continue;
        }

        if should_include_file(root, path, config, &rules) {
            out.push(path.to_path_buf());
        }
    }

    out.sort();
    Ok(out)
}

const DEFAULT_EXCLUDE_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".output",
    "coverage",
    ".cache",
    ".turbo",
    ".parcel-cache",
    ".graphyn",
    ".git",
    "target",
];

const DEFAULT_EXCLUDE_SUFFIXES: &[&str] = &[
    ".d.ts", ".d.mts", ".d.cts", // TypeScript declarations
    ".min.js", ".min.mjs", // Minified JS
    ".min.css", // Minified CSS
    ".map",     // Source maps
];

pub fn should_include_relative_path(
    relative_path: &str,
    is_dir: bool,
    config: &ScanConfig,
    rules: &[GitignoreRule],
) -> bool {
    let rel = relative_path.replace('\\', "/");

    if rel.is_empty() || rel == "." {
        return true;
    }

    // Check directory segments against default excludes
    for segment in rel.split('/') {
        if DEFAULT_EXCLUDE_DIRS.contains(&segment) {
            return false;
        }
    }

    // Check file suffixes (compound extensions like .d.ts)
    if !is_dir {
        for suffix in DEFAULT_EXCLUDE_SUFFIXES {
            if rel.ends_with(suffix) {
                return false;
            }
        }
    }

    if config.respect_gitignore && is_ignored_by_rules(&rel, is_dir, rules) {
        return false;
    }

    if !config.exclude_patterns.is_empty() && path_matches_any(&rel, &config.exclude_patterns) {
        return false;
    }

    if config.include_patterns.is_empty() {
        return true;
    }

    path_matches_any(&rel, &config.include_patterns)
}

fn should_descend(root: &Path, path: &Path, config: &ScanConfig, rules: &[GitignoreRule]) -> bool {
    if !path.is_dir() {
        return true;
    }

    let Ok(relative) = path.strip_prefix(root) else {
        return true;
    };
    let rel = relative.to_string_lossy().replace('\\', "/");

    should_include_relative_path(&rel, true, config, rules)
}

fn should_include_file(
    root: &Path,
    path: &Path,
    config: &ScanConfig,
    rules: &[GitignoreRule],
) -> bool {
    let Ok(relative) = path.strip_prefix(root) else {
        return false;
    };

    let rel = relative.to_string_lossy().replace('\\', "/");
    should_include_relative_path(&rel, false, config, rules)
}
