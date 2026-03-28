use crate::services::linker;
use crate::state::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileNode>>,
}

#[tauri::command]
pub async fn note_list(notebook: String, state: State<'_, AppState>) -> Result<Vec<FileNode>, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let notebook_path = vault_path.join(&notebook);
    if !notebook_path.exists() || !notebook_path.is_dir() {
        return Err(format!("Notebook '{}' does not exist", notebook));
    }

    build_file_tree(&vault_path, &notebook_path, &notebook)
}

fn build_file_tree(vault_path: &PathBuf, notebook_path: &PathBuf, _notebook: &str) -> Result<Vec<FileNode>, String> {
    let mut root_nodes: Vec<FileNode> = Vec::new();
    let mut dir_map: std::collections::HashMap<PathBuf, Vec<FileNode>> = std::collections::HashMap::new();

    for entry in WalkDir::new(notebook_path)
        .min_depth(1)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') // Skip hidden files and .trash
        })
    {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();

        // Build path relative to vault (including notebook name)
        let relative_path = path
            .strip_prefix(vault_path)
            .unwrap()
            .to_string_lossy()
            .to_string();

        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().is_dir();

        let node = FileNode {
            name,
            path: relative_path.clone(),
            is_dir,
            children: if is_dir { Some(Vec::new()) } else { None },
        };

        if let Some(parent) = path.parent() {
            if parent == notebook_path {
                root_nodes.push(node);
            } else {
                dir_map
                    .entry(parent.to_path_buf())
                    .or_insert_with(Vec::new)
                    .push(node);
            }
        }
    }

    // Build tree structure
    fn populate_children(
        nodes: &mut Vec<FileNode>,
        dir_map: &std::collections::HashMap<PathBuf, Vec<FileNode>>,
        vault_path: &PathBuf,
    ) {
        for node in nodes.iter_mut() {
            if node.is_dir {
                let full_path = vault_path.join(&node.path);
                if let Some(children) = dir_map.get(&full_path) {
                    node.children = Some(children.clone());
                    if let Some(ref mut children) = node.children {
                        populate_children(children, dir_map, vault_path);
                    }
                }
            }
        }
    }

    populate_children(&mut root_nodes, &dir_map, vault_path);

    Ok(root_nodes)
}

#[tauri::command]
pub async fn note_exists(path: String, state: State<'_, AppState>) -> Result<bool, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let full_path = vault_path.join(&path);
    Ok(full_path.exists() && full_path.is_file())
}

#[tauri::command]
pub async fn note_read(path: String, state: State<'_, AppState>) -> Result<String, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let full_path = vault_path.join(&path);
    fs::read_to_string(&full_path).map_err(|e| format!("Failed to read note: {}", e))
}

#[tauri::command]
pub async fn note_write(
    path: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let full_path = vault_path.join(&path);
    fs::write(&full_path, &content).map_err(|e| format!("Failed to write note: {}", e))?;

    // Update metadata in database
    update_note_metadata(&state, &path, &content)?;

    Ok(())
}

#[tauri::command]
pub async fn note_create(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let full_path = vault_path.join(&path);

    // Create parent directories if needed
    if let Some(parent) = full_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directories: {}", e))?;
    }

    // Extract title from filename
    let title = full_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Create file with title as heading
    let initial_content = format!("# {}\n\n", title);
    fs::write(&full_path, &initial_content)
        .map_err(|e| format!("Failed to create note: {}", e))?;

    // Update metadata
    update_note_metadata(&state, &path, &initial_content)?;

    Ok(())
}

#[tauri::command]
pub async fn note_delete(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    // Extract notebook from path
    let notebook = path
        .split('/')
        .next()
        .ok_or("Invalid path: missing notebook")?;

    let full_path = vault_path.join(&path);
    let trash_path = vault_path.join(notebook).join(".trash");

    // Create .trash if it doesn't exist
    fs::create_dir_all(&trash_path)
        .map_err(|e| format!("Failed to create trash folder: {}", e))?;

    // Move to trash instead of deleting
    let file_name = full_path
        .file_name()
        .ok_or("Invalid file path")?
        .to_string_lossy();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let trash_file = trash_path.join(format!("{}_{}", timestamp, file_name));

    fs::rename(&full_path, trash_file).map_err(|e| format!("Failed to delete note: {}", e))?;

    // Remove from database
    let db = state.db.lock().unwrap();
    db.execute("DELETE FROM notes WHERE path = ?1", params![path])
        .map_err(|e| format!("Failed to remove from db: {}", e))?;

    db.execute("DELETE FROM notes_fts WHERE path = ?1", params![path])
        .map_err(|e| format!("Failed to remove from FTS: {}", e))?;

    Ok(())
}

fn update_note_metadata(state: &AppState, path: &str, content: &str) -> Result<(), String> {
    let db = state.db.lock().unwrap();

    // Extract notebook from path (first component before /)
    let notebook = path
        .split('/')
        .next()
        .ok_or("Invalid path: missing notebook")?
        .to_string();

    // Extract title (first heading or filename)
    let title = extract_title(content, path);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let size = content.len() as i64;

    // Update notes table with notebook
    db.execute(
        "INSERT OR REPLACE INTO notes (path, notebook, title, created_at, modified_at, size)
         VALUES (?1, ?2, ?3, COALESCE((SELECT created_at FROM notes WHERE path = ?1), ?4), ?4, ?5)",
        params![path, notebook, title, now, size],
    )
    .map_err(|e| format!("Failed to update notes: {}", e))?;

    // Update FTS table
    db.execute(
        "INSERT OR REPLACE INTO notes_fts (path, notebook, title, content) VALUES (?1, ?2, ?3, ?4)",
        params![path, notebook, title, content],
    )
    .map_err(|e| format!("Failed to update FTS: {}", e))?;

    // Parse and update links
    let links = linker::parse_links(content);
    linker::update_links(&db, path, &links)
        .map_err(|e| format!("Failed to update links: {}", e))?;

    Ok(())
}

fn extract_title(content: &str, path: &str) -> String {
    // Try to find first markdown heading
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            return trimmed.trim_start_matches('#').trim().to_string();
        }
    }

    // Fallback to filename
    PathBuf::from(path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BacklinkResult {
    pub source_path: String,
    pub source_title: String,
}

#[tauri::command]
pub async fn resolve_wiki_link(
    target: String,
    notebook: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let notebook_path = vault_path.join(&notebook);

    // 1. Check exact match within notebook: {notebook}/{target}.md
    let exact_relative = if target.contains('/') {
        // Target has subdirectory
        if target.ends_with(".md") {
            format!("{}/{}", notebook, target)
        } else {
            format!("{}/{}.md", notebook, target)
        }
    } else {
        if target.ends_with(".md") {
            format!("{}/{}", notebook, target)
        } else {
            format!("{}/{}.md", notebook, target)
        }
    };

    let full_exact_path = vault_path.join(&exact_relative);
    if full_exact_path.exists() {
        return Ok(Some(exact_relative));
    }

    // 2. Case-insensitive search by filename within notebook
    let target_lower = target.to_lowercase();
    let target_filename = if target_lower.ends_with(".md") {
        target_lower.clone()
    } else {
        format!("{}.md", target_lower)
    };

    for entry in WalkDir::new(&notebook_path)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.')
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if let Some(filename) = path.file_name() {
            let filename_str = filename.to_string_lossy().to_lowercase();
            if filename_str == target_filename {
                let relative_path = path
                    .strip_prefix(&vault_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                return Ok(Some(relative_path));
            }
        }
    }

    Ok(None)
}

#[tauri::command]
pub async fn get_backlinks(
    path: String,
    state: State<'_, AppState>,
) -> Result<Vec<BacklinkResult>, String> {
    let db = state.db.lock().unwrap();

    // Get filename without extension for matching wiki-links
    let filename = PathBuf::from(&path)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    // Query links table for any references to this note
    let mut stmt = db
        .prepare(
            "SELECT DISTINCT l.source, n.title
             FROM links l
             LEFT JOIN notes n ON l.source = n.path
             WHERE l.target = ?1 OR l.target = ?2
             ORDER BY n.title",
        )
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let results = stmt
        .query_map(params![path, filename], |row| {
            Ok(BacklinkResult {
                source_path: row.get(0)?,
                source_title: row.get(1).unwrap_or_else(|_| "Untitled".to_string()),
            })
        })
        .map_err(|e| format!("Failed to query backlinks: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    Ok(results)
}

#[tauri::command]
pub async fn open_external_url(url: String) -> Result<(), String> {
    // Open URL in default browser
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", &url])
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn move_file(
    source: String,
    target_dir: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let source_path = vault_path.join(&source);
    let target_dir_path = if target_dir.is_empty() {
        vault_path.clone()
    } else {
        vault_path.join(&target_dir)
    };

    // Validate source exists and is a file
    if !source_path.exists() {
        return Err("Source file does not exist".to_string());
    }
    if !source_path.is_file() {
        return Err("Source is not a file".to_string());
    }

    // Validate target is a directory
    if !target_dir_path.is_dir() {
        return Err("Target is not a directory".to_string());
    }

    // Get filename
    let filename = source_path
        .file_name()
        .ok_or("Invalid source path")?
        .to_string_lossy();

    let target_path = target_dir_path.join(filename.as_ref());

    // Check if target already exists
    if target_path.exists() {
        return Err("File already exists in target directory".to_string());
    }

    // Move file
    fs::rename(&source_path, &target_path)
        .map_err(|e| format!("Failed to move file: {}", e))?;

    // Calculate new relative path
    let new_relative_path = target_path
        .strip_prefix(&vault_path)
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Update database
    let db = state.db.lock().unwrap();
    db.execute(
        "UPDATE notes SET path = ?1 WHERE path = ?2",
        params![new_relative_path, source],
    )
    .map_err(|e| format!("Failed to update database: {}", e))?;

    // Update links that reference this file
    db.execute(
        "UPDATE links SET source = ?1 WHERE source = ?2",
        params![new_relative_path, source],
    )
    .map_err(|e| format!("Failed to update links: {}", e))?;

    db.execute(
        "UPDATE links SET target = ?1 WHERE target = ?2",
        params![new_relative_path, source],
    )
    .map_err(|e| format!("Failed to update link targets: {}", e))?;

    Ok(new_relative_path)
}

#[tauri::command]
pub async fn copy_file(
    source: String,
    target_dir: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let source_path = vault_path.join(&source);
    let target_dir_path = if target_dir.is_empty() {
        vault_path.clone()
    } else {
        vault_path.join(&target_dir)
    };

    // Validate source exists and is a file
    if !source_path.exists() {
        return Err("Source file does not exist".to_string());
    }
    if !source_path.is_file() {
        return Err("Source is not a file".to_string());
    }

    // Validate target is a directory
    if !target_dir_path.is_dir() {
        return Err("Target is not a directory".to_string());
    }

    // Get filename
    let filename = source_path
        .file_name()
        .ok_or("Invalid source path")?
        .to_string_lossy();

    let target_path = target_dir_path.join(filename.as_ref());

    // Handle name conflicts by adding (n)
    let final_target_path = if target_path.exists() {
        let stem = target_path.file_stem().unwrap().to_string_lossy().to_string();
        let ext = target_path.extension().unwrap_or_default().to_string_lossy().to_string();

        let mut counter = 1;
        loop {
            let new_name = if ext.is_empty() {
                format!("{} ({})", stem, counter)
            } else {
                format!("{} ({}).{}", stem, counter, ext)
            };
            let new_path = target_dir_path.join(&new_name);
            if !new_path.exists() {
                break new_path;
            }
            counter += 1;
        }
    } else {
        target_path
    };

    // Copy file
    fs::copy(&source_path, &final_target_path)
        .map_err(|e| format!("Failed to copy file: {}", e))?;

    // Calculate new relative path
    let new_relative_path = final_target_path
        .strip_prefix(&vault_path)
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Read content and update database
    let content = fs::read_to_string(&final_target_path)
        .map_err(|e| format!("Failed to read copied file: {}", e))?;

    let db = state.db.lock().unwrap();
    let title = extract_title(&content, &new_relative_path);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let size = content.len() as i64;

    db.execute(
        "INSERT INTO notes (path, title, created_at, modified_at, size) VALUES (?1, ?2, ?3, ?3, ?4)",
        params![new_relative_path, title, now, size],
    )
    .map_err(|e| format!("Failed to update database: {}", e))?;

    Ok(new_relative_path)
}

#[tauri::command]
pub async fn copy_folder(
    source: String,
    target_dir: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let source_path = vault_path.join(&source);
    let target_dir_path = if target_dir.is_empty() {
        vault_path.clone()
    } else {
        vault_path.join(&target_dir)
    };

    // Validate source exists and is a directory
    if !source_path.exists() {
        return Err("Source folder does not exist".to_string());
    }
    if !source_path.is_dir() {
        return Err("Source is not a directory".to_string());
    }

    // Validate target is a directory
    if !target_dir_path.is_dir() {
        return Err("Target is not a directory".to_string());
    }

    // Check if trying to copy into itself or its subdirectory
    if target_dir_path.starts_with(&source_path) {
        return Err("Cannot copy folder into itself or its subdirectory".to_string());
    }

    // Get folder name
    let folder_name = source_path
        .file_name()
        .ok_or("Invalid source path")?
        .to_string_lossy()
        .to_string();

    let target_path = target_dir_path.join(&folder_name);

    // Handle name conflicts
    let final_target_path = if target_path.exists() {
        let mut counter = 1;
        loop {
            let new_name = format!("{} ({})", folder_name, counter);
            let new_path = target_dir_path.join(&new_name);
            if !new_path.exists() {
                break new_path;
            }
            counter += 1;
        }
    } else {
        target_path
    };

    // Copy folder recursively
    copy_dir_all(&source_path, &final_target_path)
        .map_err(|e| format!("Failed to copy folder: {}", e))?;

    // Calculate new relative path
    let new_relative_path = final_target_path
        .strip_prefix(&vault_path)
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Reindex the copied files
    // (We could optimize this by reading and inserting only new files, but reindexing is simpler)

    Ok(new_relative_path)
}

#[tauri::command]
pub async fn rename_item(
    old_path: String,
    new_name: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let source_path = vault_path.join(&old_path);
    if !source_path.exists() {
        return Err("Source does not exist".to_string());
    }

    // Get parent directory
    let parent = source_path.parent().ok_or("Invalid path")?;
    let new_path = parent.join(&new_name);

    if new_path.exists() {
        return Err("File or folder with this name already exists".to_string());
    }

    // Rename
    fs::rename(&source_path, &new_path)
        .map_err(|e| format!("Failed to rename: {}", e))?;

    // Calculate new relative path
    let new_relative_path = new_path
        .strip_prefix(&vault_path)
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Update database
    let is_dir = new_path.is_dir();
    let db = state.db.lock().unwrap();

    if is_dir {
        // Update all files in folder
        let old_prefix = if old_path.ends_with('/') {
            old_path.clone()
        } else {
            format!("{}/", old_path)
        };

        let new_prefix = if new_relative_path.ends_with('/') {
            new_relative_path.clone()
        } else {
            format!("{}/", new_relative_path)
        };

        db.execute(
            "UPDATE notes SET path = ? || substr(path, ?) WHERE path LIKE ?",
            params![new_prefix, old_prefix.len() + 1, format!("{}%", old_prefix)],
        )
        .map_err(|e| format!("Failed to update notes: {}", e))?;

        db.execute(
            "UPDATE links SET source = ? || substr(source, ?) WHERE source LIKE ?",
            params![new_prefix, old_prefix.len() + 1, format!("{}%", old_prefix)],
        )
        .map_err(|e| format!("Failed to update links: {}", e))?;

        db.execute(
            "UPDATE links SET target = ? || substr(target, ?) WHERE target LIKE ?",
            params![new_prefix, old_prefix.len() + 1, format!("{}%", old_prefix)],
        )
        .map_err(|e| format!("Failed to update link targets: {}", e))?;
    } else {
        // Update single file
        db.execute(
            "UPDATE notes SET path = ? WHERE path = ?",
            params![new_relative_path, old_path],
        )
        .map_err(|e| format!("Failed to update database: {}", e))?;

        db.execute(
            "UPDATE links SET source = ? WHERE source = ?",
            params![new_relative_path, old_path],
        )
        .map_err(|e| format!("Failed to update links: {}", e))?;

        db.execute(
            "UPDATE links SET target = ? WHERE target = ?",
            params![new_relative_path, old_path],
        )
        .map_err(|e| format!("Failed to update link targets: {}", e))?;

        // Update link content in files
        update_links_in_files(&db, &vault_path, &old_path, &new_relative_path)?;
    }

    drop(db);

    Ok(new_relative_path)
}

/// Update links in files that reference the renamed file
fn update_links_in_files(
    db: &rusqlite::Connection,
    vault_path: &PathBuf,
    old_path: &str,
    new_path: &str,
) -> Result<(), String> {
    // Extract old and new names for [[wikilink]] replacement
    let old_name = old_path.split('/').last().unwrap_or(old_path);
    let new_name = new_path.split('/').last().unwrap_or(new_path);

    // Remove .md extension from names for wikilinks
    let old_wiki_name = old_name.trim_end_matches(".md");
    let new_wiki_name = new_name.trim_end_matches(".md");

    // Find all files that link to the renamed file
    let mut stmt = db
        .prepare("SELECT DISTINCT source FROM links WHERE target = ? OR target = ?")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let sources: Vec<String> = stmt
        .query_map(params![old_wiki_name, old_path], |row| row.get(0))
        .map_err(|e| format!("Query failed: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect sources: {}", e))?;

    // Update each file
    for source_path in sources {
        let full_path = vault_path.join(&source_path);

        // Read file content
        let content = match fs::read_to_string(&full_path) {
            Ok(c) => c,
            Err(_) => continue, // Skip if file doesn't exist or can't be read
        };

        // Replace wikilinks [[old_name]] -> [[new_name]]
        let mut updated_content = content.clone();

        // Replace [[old_name]]
        updated_content = updated_content.replace(
            &format!("[[{}]]", old_wiki_name),
            &format!("[[{}]]", new_wiki_name),
        );

        // Replace [text](old_path)
        updated_content = updated_content.replace(
            &format!("]({})", old_path),
            &format!("]({})", new_path),
        );

        // Also replace relative markdown links
        updated_content = updated_content.replace(
            &format!("]({})", old_name),
            &format!("]({})", new_name),
        );

        // Write updated content if changed
        if updated_content != content {
            fs::write(&full_path, updated_content)
                .map_err(|e| format!("Failed to update file {}: {}", source_path, e))?;
        }
    }

    Ok(())
}

// Helper function to copy directory recursively
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn move_folder(
    source: String,
    target_dir: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let vault_path = {
        let vault = state.vault_path.read().unwrap();
        vault.clone().ok_or("No vault initialized")?
    };

    let source_path = vault_path.join(&source);
    let target_dir_path = if target_dir.is_empty() {
        vault_path.clone()
    } else {
        vault_path.join(&target_dir)
    };

    // Validate source exists and is a directory
    if !source_path.exists() {
        return Err("Source folder does not exist".to_string());
    }
    if !source_path.is_dir() {
        return Err("Source is not a directory".to_string());
    }

    // Validate target is a directory
    if !target_dir_path.is_dir() {
        return Err("Target is not a directory".to_string());
    }

    // Check if trying to move into itself or its subdirectory
    if target_dir_path.starts_with(&source_path) {
        return Err("Cannot move folder into itself or its subdirectory".to_string());
    }

    // Get folder name
    let folder_name = source_path
        .file_name()
        .ok_or("Invalid source path")?
        .to_string_lossy();

    let target_path = target_dir_path.join(folder_name.as_ref());

    // Check if target already exists
    if target_path.exists() {
        return Err("Folder already exists in target directory".to_string());
    }

    // Move folder
    fs::rename(&source_path, &target_path)
        .map_err(|e| format!("Failed to move folder: {}", e))?;

    // Calculate new relative path
    let new_relative_path = target_path
        .strip_prefix(&vault_path)
        .unwrap()
        .to_string_lossy()
        .to_string();

    // Update all files in database that were in this folder
    let db = state.db.lock().unwrap();
    let source_prefix = if source.ends_with('/') {
        source.clone()
    } else {
        format!("{}/", source)
    };

    let new_prefix = if new_relative_path.ends_with('/') {
        new_relative_path.clone()
    } else {
        format!("{}/", new_relative_path)
    };

    // Update notes paths
    db.execute(
        "UPDATE notes SET path = ? || substr(path, ?) WHERE path LIKE ?",
        params![new_prefix, source_prefix.len() + 1, format!("{}%", source_prefix)],
    )
    .map_err(|e| format!("Failed to update notes in database: {}", e))?;

    // Update links
    db.execute(
        "UPDATE links SET source = ? || substr(source, ?) WHERE source LIKE ?",
        params![new_prefix, source_prefix.len() + 1, format!("{}%", source_prefix)],
    )
    .map_err(|e| format!("Failed to update link sources: {}", e))?;

    db.execute(
        "UPDATE links SET target = ? || substr(target, ?) WHERE target LIKE ?",
        params![new_prefix, source_prefix.len() + 1, format!("{}%", source_prefix)],
    )
    .map_err(|e| format!("Failed to update link targets: {}", e))?;

    Ok(new_relative_path)
}
