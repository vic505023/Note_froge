use crate::state::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookInfo {
    pub name: String,
    pub note_count: i64,
    pub document_count: i64,
    pub created_at: i64,
}

#[tauri::command]
pub async fn notebook_list(state: State<'_, AppState>) -> Result<Vec<NotebookInfo>, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    // Scan vault directory for first-level folders (notebooks)
    let mut notebooks = Vec::new();

    for entry in fs::read_dir(&vault_path).map_err(|e| format!("Failed to read vault: {}", e))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        // Skip hidden files and non-directories
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || !path.is_dir() {
            continue;
        }

        // Count .md files recursively in this notebook
        let note_count = WalkDir::new(&path)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.')
            })
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_type().is_file() &&
                e.path().extension().and_then(|s| s.to_str()) == Some("md")
            })
            .count() as i64;

        // Get document count from database
        let db = state.db.lock().unwrap();
        let document_count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM notebook_documents WHERE notebook = ?1",
                params![name],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Get created_at from database, or use current time if not found
        let created_at: i64 = db
            .query_row(
                "SELECT created_at FROM notebooks WHERE name = ?1",
                params![name],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| {
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64
            });

        notebooks.push(NotebookInfo {
            name,
            note_count,
            document_count,
            created_at,
        });

        // Sync notebooks table with filesystem
        db.execute(
            "INSERT OR REPLACE INTO notebooks (name, created_at, note_count) VALUES (?1, ?2, ?3)",
            params![notebooks.last().unwrap().name, created_at, note_count],
        )
        .map_err(|e| format!("Failed to sync notebook: {}", e))?;
    }

    // Sort by name
    notebooks.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(notebooks)
}

#[tauri::command]
pub async fn notebook_create(name: String, state: State<'_, AppState>) -> Result<(), String> {
    // Validate name - no special characters
    if name.is_empty() {
        return Err("Notebook name cannot be empty".to_string());
    }

    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    if name.chars().any(|c| invalid_chars.contains(&c)) {
        return Err("Notebook name contains invalid characters".to_string());
    }

    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let notebook_path = vault_path.join(&name);

    // Check if already exists
    if notebook_path.exists() {
        return Err("Notebook already exists".to_string());
    }

    // Create notebook folder
    fs::create_dir(&notebook_path)
        .map_err(|e| format!("Failed to create notebook folder: {}", e))?;

    // Create .trash subfolder
    fs::create_dir(notebook_path.join(".trash"))
        .map_err(|e| format!("Failed to create trash folder: {}", e))?;

    // Insert into database
    let db = state.db.lock().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    db.execute(
        "INSERT INTO notebooks (name, created_at, note_count) VALUES (?1, ?2, 0)",
        params![name, now],
    )
    .map_err(|e| format!("Failed to insert notebook into database: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn notebook_delete(name: String, state: State<'_, AppState>) -> Result<(), String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let notebook_path = vault_path.join(&name);

    if !notebook_path.exists() {
        return Err("Notebook does not exist".to_string());
    }

    // Create vault-level .trash if it doesn't exist
    let vault_trash = vault_path.join(".trash");
    fs::create_dir_all(&vault_trash)
        .map_err(|e| format!("Failed to create trash folder: {}", e))?;

    // Move to trash with timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let trash_path = vault_trash.join(format!("{}_{}", name, timestamp));

    fs::rename(&notebook_path, &trash_path)
        .map_err(|e| format!("Failed to move notebook to trash: {}", e))?;

    // Delete from database
    let db = state.db.lock().unwrap();

    db.execute("DELETE FROM notebooks WHERE name = ?1", params![name])
        .map_err(|e| format!("Failed to delete notebook from db: {}", e))?;

    db.execute("DELETE FROM notebook_documents WHERE notebook = ?1", params![name])
        .map_err(|e| format!("Failed to delete notebook documents: {}", e))?;

    db.execute("DELETE FROM notes WHERE notebook = ?1", params![name])
        .map_err(|e| format!("Failed to delete notes: {}", e))?;

    db.execute("DELETE FROM notes_fts WHERE notebook = ?1", params![name])
        .map_err(|e| format!("Failed to delete from FTS: {}", e))?;

    db.execute("DELETE FROM chat_history WHERE notebook = ?1", params![name])
        .map_err(|e| format!("Failed to delete chat history: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn notebook_rename(
    old_name: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Validate new name
    if new_name.is_empty() {
        return Err("Notebook name cannot be empty".to_string());
    }

    let invalid_chars = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
    if new_name.chars().any(|c| invalid_chars.contains(&c)) {
        return Err("Notebook name contains invalid characters".to_string());
    }

    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let old_path = vault_path.join(&old_name);
    let new_path = vault_path.join(&new_name);

    if !old_path.exists() {
        return Err("Notebook does not exist".to_string());
    }

    if new_path.exists() {
        return Err("A notebook with this name already exists".to_string());
    }

    // Rename folder
    fs::rename(&old_path, &new_path)
        .map_err(|e| format!("Failed to rename notebook folder: {}", e))?;

    // Update database - all tables that reference notebook name
    let db = state.db.lock().unwrap();

    db.execute(
        "UPDATE notebooks SET name = ?1 WHERE name = ?2",
        params![new_name, old_name],
    )
    .map_err(|e| format!("Failed to update notebooks table: {}", e))?;

    db.execute(
        "UPDATE notes SET notebook = ?1, path = ?1 || substr(path, ?2) WHERE notebook = ?3",
        params![new_name, old_name.len() + 1, old_name],
    )
    .map_err(|e| format!("Failed to update notes: {}", e))?;

    db.execute(
        "UPDATE notes_fts SET notebook = ?1, path = ?1 || substr(path, ?2) WHERE notebook = ?3",
        params![new_name, old_name.len() + 1, old_name],
    )
    .map_err(|e| format!("Failed to update FTS: {}", e))?;

    db.execute(
        "UPDATE notebook_documents SET notebook = ?1 WHERE notebook = ?2",
        params![new_name, old_name],
    )
    .map_err(|e| format!("Failed to update notebook_documents: {}", e))?;

    db.execute(
        "UPDATE chat_history SET notebook = ?1 WHERE notebook = ?2",
        params![new_name, old_name],
    )
    .map_err(|e| format!("Failed to update chat_history: {}", e))?;

    db.execute(
        "UPDATE links SET source = ?1 || substr(source, ?2) WHERE source LIKE ?3",
        params![new_name, old_name.len() + 1, format!("{}/%", old_name)],
    )
    .map_err(|e| format!("Failed to update link sources: {}", e))?;

    db.execute(
        "UPDATE links SET target = ?1 || substr(target, ?2) WHERE target LIKE ?3",
        params![new_name, old_name.len() + 1, format!("{}/%", old_name)],
    )
    .map_err(|e| format!("Failed to update link targets: {}", e))?;

    Ok(())
}
