use std::collections::HashSet;
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use graphyn_adapter_ts::analyze_repo;
use graphyn_store::RocksGraphStore;
use notify::{RecursiveMode, Watcher};

use crate::output;

const DEBOUNCE: Duration = Duration::from_millis(300);

pub fn run(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let root = std::fs::canonicalize(path)
        .map_err(|e| format!("cannot access '{}': {}", path, e))?;

    output::banner("watch");
    output::info(&format!(
        "Watching {}",
        output::file_path(&root.display().to_string())
    ));
    output::dim_line("Press Ctrl+C to stop.");
    output::blank();

    // ── initial analysis ─────────────────────────────────────
    let start = Instant::now();
    let repo_ir = analyze_repo(&root)
        .map_err(|e| format!("initial analysis failed: {e}"))?;
    let (graph, stats) = super::analyze::build_graph(&repo_ir);

    let db = super::db_path(&root);
    if let Some(parent) = db.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let store = RocksGraphStore::open(&db)
        .map_err(|e| format!("failed to open store: {e}"))?;
    store
        .save_graph(&graph)
        .map_err(|e| format!("failed to persist graph: {e}"))?;

    output::success(&format!(
        "Initial analysis complete — {} symbols, {} edges ({:.0?})",
        stats.symbols,
        stats.relationships,
        start.elapsed(),
    ));
    output::blank();

    // ── start watcher ────────────────────────────────────────
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(tx)
        .map_err(|e| format!("failed to create file watcher: {e}"))?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|e| format!("failed to start watching: {e}"))?;

    let mut pending: HashSet<String> = HashSet::new();
    let mut last_event = Instant::now();

    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(event)) => {
                for path in &event.paths {
                    if is_watchable(path, &root) {
                        if let Some(rel) = path
                            .strip_prefix(&root)
                            .ok()
                            .map(|p| p.display().to_string())
                        {
                            pending.insert(rel);
                        }
                        last_event = Instant::now();
                    }
                }
            }
            Ok(Err(e)) => {
                output::warning(&format!("Watch error: {e}"));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // check debounce window
                if !pending.is_empty() && last_event.elapsed() >= DEBOUNCE {
                    let files: Vec<String> = pending.drain().collect();
                    handle_change(&root, &store, &files);
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                output::info("Watcher disconnected.");
                break;
            }
        }
    }

    Ok(())
}

fn handle_change(root: &Path, store: &RocksGraphStore, files: &[String]) {
    let file_list = files
        .iter()
        .map(|f| output::file_path(f))
        .collect::<Vec<_>>()
        .join(", ");
    let ts = output::dim(&format!("[{}]", output::timestamp()));
    println!("  {ts} Changed: {file_list}");
    print!("  {}      Re-analyzing … ", output::dim(""));

    let start = Instant::now();
    match analyze_repo(root) {
        Ok(repo_ir) => {
            let (graph, stats) = super::analyze::build_graph(&repo_ir);
            match store.save_graph(&graph) {
                Ok(()) => {
                    println!(
                        "{} ({} symbols, {} edges, {:.0?})",
                        output::green("done"),
                        stats.symbols,
                        stats.relationships,
                        start.elapsed(),
                    );
                }
                Err(e) => {
                    println!("{}", output::red(&format!("persist failed: {e}")));
                }
            }
        }
        Err(e) => {
            println!("{}", output::red(&format!("analysis failed: {e}")));
        }
    }
    println!();
}

fn is_watchable(path: &Path, root: &Path) -> bool {
    // skip .graphyn/ directory and hidden files
    if let Ok(relative) = path.strip_prefix(root) {
        let rel_str = relative.display().to_string();
        if rel_str.starts_with(".graphyn")
            || rel_str.contains("/node_modules/")
            || rel_str.contains("/.git/")
        {
            return false;
        }
    }

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default();

    matches!(ext, "ts" | "tsx" | "js" | "jsx")
}
