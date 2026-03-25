use rusqlite::{Connection, Result as SqliteResult, params};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

/// Извлекает заголовок из markdown контента (первый # heading)
fn extract_title_from_content(content: &str, fallback_path: &Path) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            return trimmed.trim_start_matches('#').trim().to_string();
        }
    }

    // Fallback to filename
    fallback_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

/// Полная переиндексация всех .md файлов в vault
pub fn reindex_vault(db: &Connection, vault_path: &Path) -> SqliteResult<usize> {
    let mut indexed_count = 0;

    // Clear existing data
    db.execute("DELETE FROM notes", [])?;
    db.execute("DELETE FROM links", [])?;

    // Walk through vault and index all .md files
    for entry in WalkDir::new(vault_path)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') // Skip hidden files and .trash
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Only process .md files
        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            if let Ok(content) = fs::read_to_string(path) {
                let relative_path = path
                    .strip_prefix(vault_path)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                let title = extract_title_from_content(&content, path);

                // Get file metadata
                let metadata = fs::metadata(path).ok();
                let created_at = metadata
                    .as_ref()
                    .and_then(|m| m.created().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                let modified_at = metadata
                    .as_ref()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);

                let size = content.len() as i64;

                // Extract notebook from path (first component before /)
                let notebook = relative_path
                    .split('/')
                    .next()
                    .unwrap_or("Default")
                    .to_string();

                // Insert into notes table with notebook
                db.execute(
                    "INSERT INTO notes (path, notebook, title, created_at, modified_at, size)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![relative_path, notebook, title, created_at, modified_at, size],
                )?;

                // Insert into FTS table
                db.execute(
                    "INSERT INTO notes_fts (path, notebook, title, content)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![relative_path, notebook, title, content],
                )?;

                // Parse and index links
                let links = crate::services::linker::parse_links(&content);
                crate::services::linker::update_links(db, &relative_path, &links)?;

                indexed_count += 1;
            }
        }
    }

    Ok(indexed_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_extract_title() {
        let content = "# My Title\n\nSome content";
        let title = extract_title_from_content(content, Path::new("test.md"));
        assert_eq!(title, "My Title");
    }

    #[test]
    fn test_extract_title_fallback() {
        let content = "No heading here";
        let title = extract_title_from_content(content, Path::new("/path/to/test.md"));
        assert_eq!(title, "test");
    }
}
