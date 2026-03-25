use crate::services::indexer;
use crate::state::AppState;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn vault_init(
    path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let vault_path = PathBuf::from(&path);

    // Create vault directory if it doesn't exist
    if !vault_path.exists() {
        fs::create_dir_all(&vault_path).map_err(|e| format!("Failed to create vault: {}", e))?;
    }

    if !vault_path.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    // Create .trash directory
    let trash_path = vault_path.join(".trash");
    if !trash_path.exists() {
        fs::create_dir(&trash_path).map_err(|e| format!("Failed to create .trash: {}", e))?;
    }

    // Check for .md files in vault root - move them to "Default" notebook
    let mut loose_files = Vec::new();
    if let Ok(entries) = fs::read_dir(&vault_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                loose_files.push(path);
            }
        }
    }

    if !loose_files.is_empty() {
        // Create Default notebook
        let default_notebook = vault_path.join("Default");
        fs::create_dir_all(&default_notebook)
            .map_err(|e| format!("Failed to create Default notebook: {}", e))?;
        fs::create_dir(default_notebook.join(".trash"))
            .map_err(|e| format!("Failed to create Default/.trash: {}", e))?;

        // Move loose files to Default
        for file_path in loose_files {
            let file_name = file_path.file_name().unwrap();
            let target_path = default_notebook.join(file_name);
            fs::rename(&file_path, &target_path)
                .map_err(|e| format!("Failed to move file to Default: {}", e))?;
        }
    }

    // Scan vault for first-level folders (notebooks) and create DB entries
    if let Ok(entries) = fs::read_dir(&vault_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden folders and non-directories
            if path.is_dir() && !name.starts_with('.') {
                // Create .trash subfolder if it doesn't exist
                let notebook_trash = path.join(".trash");
                if !notebook_trash.exists() {
                    fs::create_dir(&notebook_trash).ok();
                }

                // Insert into database
                let db = state.db.lock().unwrap();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                db.execute(
                    "INSERT OR IGNORE INTO notebooks (name, created_at, note_count) VALUES (?1, ?2, 0)",
                    rusqlite::params![name, now],
                )
                .ok();
            }
        }
    }

    // Update vault path in state
    {
        let mut vault = state.vault_path.write().unwrap();
        *vault = Some(vault_path.clone());
    }

    // Update config
    {
        let mut config = state.config.write().unwrap();
        config.vault.path = Some(path.clone());
    }

    // Save config to file
    let config = state.config.read().unwrap().clone();
    save_config_to_file(&config)?;

    // Check if database is empty and needs indexing
    let needs_indexing = {
        let db = state.db.lock().unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))
            .unwrap_or(0);
        count == 0
    };

    // Run initial indexing if database is empty
    if needs_indexing {
        app_handle
            .emit("indexing-started", serde_json::json!({}))
            .ok();

        let vault_path_clone = vault_path.clone();
        let state_clone = state.inner().clone();

        // Run indexing in background
        tauri::async_runtime::spawn_blocking(move || {
            let db = state_clone.db.lock().unwrap();
            match indexer::reindex_vault(&db, &vault_path_clone) {
                Ok(count) => {
                    app_handle
                        .emit("indexing-complete", serde_json::json!({ "count": count }))
                        .ok();
                }
                Err(e) => {
                    app_handle
                        .emit("indexing-error", serde_json::json!({ "error": e.to_string() }))
                        .ok();
                }
            }
        });
    }

    Ok(path)
}

#[tauri::command]
pub async fn vault_get_path(state: State<'_, AppState>) -> Result<String, String> {
    let vault = state.vault_path.read().unwrap();
    match &*vault {
        Some(path) => Ok(path.to_string_lossy().to_string()),
        None => Err("No vault initialized".to_string()),
    }
}

fn save_config_to_file(config: &crate::state::AppConfig) -> Result<(), String> {
    let config_dir = dirs::home_dir()
        .ok_or("Cannot find home directory")?
        .join(".noteforge");

    fs::create_dir_all(&config_dir).map_err(|e| format!("Failed to create config dir: {}", e))?;

    let config_path = config_dir.join("config.toml");
    let toml_str = toml::to_string(config).map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(config_path, toml_str).map_err(|e| format!("Failed to write config: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn reindex_vault(state: State<'_, AppState>) -> Result<usize, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let db = state.db.lock().unwrap();
    let count = indexer::reindex_vault(&db, &vault_path)
        .map_err(|e| format!("Failed to reindex vault: {}", e))?;

    Ok(count)
}

#[tauri::command]
pub async fn select_vault_folder(app_handle: AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let folder = app_handle
        .dialog()
        .file()
        .set_title("Select Vault Folder")
        .blocking_pick_folder();

    Ok(folder.map(|path| path.as_path().unwrap().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn change_vault(
    new_path: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Initialize new vault (this will create necessary directories and update config)
    vault_init(new_path.clone(), app_handle.clone(), state.clone()).await?;

    // Clear current database (notes will be reindexed)
    {
        let db = state.db.lock().unwrap();
        db.execute("DELETE FROM notes", [])
            .map_err(|e| format!("Failed to clear notes: {}", e))?;
        db.execute("DELETE FROM links", [])
            .map_err(|e| format!("Failed to clear links: {}", e))?;
    }

    Ok(())
}
