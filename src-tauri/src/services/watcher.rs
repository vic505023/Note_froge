use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

use crate::state::AppState;

/// Start watching the vault for file changes
pub fn start_watcher(
    vault_path: PathBuf,
    app_handle: AppHandle,
    _state: Arc<AppState>,
) -> Result<(), String> {
    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        move |res: Result<Event, notify::Error>| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default()
            .with_poll_interval(Duration::from_millis(500)),
    )
    .map_err(|e| format!("Failed to create watcher: {}", e))?;

    watcher
        .watch(&vault_path, RecursiveMode::Recursive)
        .map_err(|e| format!("Failed to watch vault: {}", e))?;

    // Spawn background task to handle events
    std::thread::spawn(move || {
        // Keep watcher alive
        let _watcher = watcher;

        for event in rx {
            if should_ignore_event(&event) {
                continue;
            }

            match event.kind {
                EventKind::Create(_) => {
                    for path in &event.paths {
                        if is_markdown_file(path) {
                            emit_file_changed(&app_handle, path, "created");
                        } else if path.is_dir() {
                            emit_file_changed(&app_handle, path, "folder");
                        }
                    }
                }
                EventKind::Modify(_) => {
                    for path in &event.paths {
                        if is_markdown_file(path) {
                            emit_file_changed(&app_handle, path, "modified");
                        }
                    }
                }
                EventKind::Remove(_) => {
                    for path in &event.paths {
                        emit_file_changed(&app_handle, path, "deleted");
                    }
                }
                _ => {}
            }
        }
    });

    Ok(())
}

/// Check if event should be ignored
fn should_ignore_event(event: &Event) -> bool {
    for path in &event.paths {
        let path_str = path.to_string_lossy();

        // Ignore hidden files and directories
        if path_str.contains("/.") {
            return true;
        }

        // Ignore .trash directories
        if path_str.contains("/.trash") || path_str.contains("\\.trash") {
            return true;
        }

        // Ignore non-markdown files (except for directory events)
        if path.is_file() && !is_markdown_file(path) {
            return true;
        }
    }

    false
}

/// Check if path is a markdown file
fn is_markdown_file(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("md"))
        .unwrap_or(false)
}

/// Emit file-changed event
fn emit_file_changed(app_handle: &AppHandle, path: &PathBuf, kind: &str) {
    let _ = app_handle.emit(
        "file-changed",
        serde_json::json!({
            "path": path.to_string_lossy(),
            "kind": kind
        }),
    );
}
