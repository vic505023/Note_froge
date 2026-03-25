use crate::services::doc_indexer::{index_document, index_all_notes};
use crate::services::doc_parser::{parse_document, parse_document_with_vision};
use crate::state::AppState;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentInfo {
    pub id: i64,
    pub filename: String,
    pub filepath: String,
    pub file_type: String,
    pub title: String,
    pub page_count: Option<u32>,
    pub size_bytes: i64,
    pub indexed_at: Option<i64>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingStats {
    pub total_notes: i64,
    pub indexed_notes: i64,
    pub total_documents: i64,
    pub indexed_documents: i64,
    pub total_chunks: i64,
}

/// Upload a document from source path to .noteforge/documents/ and index it
#[tauri::command]
pub async fn document_upload(
    source_path: String,
    notebook: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<DocumentInfo, String> {
    let source = Path::new(&source_path);

    if !source.exists() {
        return Err("Source file does not exist".to_string());
    }

    // Determine file type
    let extension = source
        .extension()
        .and_then(|s| s.to_str())
        .ok_or("File has no extension")?
        .to_lowercase();

    let file_type = match extension.as_str() {
        "pdf" => "pdf",
        "docx" => "docx",
        "pptx" => "pptx",
        "txt" => "txt",
        _ => return Err(format!("Unsupported file type: {}", extension)),
    };

    // Get original filename early
    let original_filename = source
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?
        .to_string();

    // Emit parsing start event immediately
    let _ = app_handle.emit(
        "document-parsing-start",
        serde_json::json!({ "filename": original_filename }),
    );

    // Parse document to get title and page count (with vision OCR support)
    let source_path_for_parsing = source_path.clone();
    let state_for_parsing = state.inner().clone();

    eprintln!("========================================");
    eprintln!("DOCUMENT UPLOAD: Starting to parse {}", source_path);

    let parsed = tokio::task::spawn_blocking(move || {
        let source = Path::new(&source_path_for_parsing);
        let client = reqwest::Client::new();
        let (vision_config, vision_base_url, vision_api_key) = {
            let cfg = state_for_parsing.config.read().unwrap();
            (
                cfg.vision.clone(),
                cfg.get_vision_base_url(),
                cfg.get_vision_api_key(),
            )
        };

        eprintln!("Vision enabled: {}", vision_config.enabled);
        eprintln!("Vision API key present: {}", !vision_api_key.is_empty());
        eprintln!("Vision base URL: {}", vision_base_url);
        eprintln!("Vision model: {}", vision_config.model);

        let runtime = tokio::runtime::Handle::current();

        if vision_config.enabled && !vision_api_key.is_empty() {
            eprintln!("Using Vision OCR for parsing");
            runtime.block_on(parse_document_with_vision(
                source,
                &client,
                true,
                &vision_base_url,
                &vision_api_key,
                &vision_config.model,
            ))
        } else {
            eprintln!("Vision OCR disabled or no API key, using regular parsing");
            // Fall back to regular parsing (may fail on scanned PDFs)
            parse_document(source).map_err(|e| {
                // If it's a scanned PDF and vision is disabled, provide helpful message
                if e.contains("Scanned PDF") {
                    format!("{} Enable Vision OCR in Settings to process scanned documents.", e)
                } else {
                    e
                }
            })
        }
    })
    .await
    .map_err(|e| format!("Parsing task failed: {}", e))??;

    eprintln!("Parsing successful! Title: {}", parsed.title);
    eprintln!("========================================");

    // Generate unique filename
    let uuid = Uuid::new_v4();
    let original_filename = source
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or("Invalid filename")?;
    let unique_filename = format!("{}_{}", uuid.as_simple(), original_filename);

    // Copy to documents directory
    let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
    let documents_dir = home_dir.join(".noteforge").join("documents");
    fs::create_dir_all(&documents_dir).map_err(|e| format!("Failed to create documents directory: {}", e))?;

    let dest_path = documents_dir.join(&unique_filename);
    fs::copy(source, &dest_path).map_err(|e| format!("Failed to copy file: {}", e))?;

    let dest_path_str = dest_path
        .to_str()
        .ok_or("Invalid destination path")?
        .to_string();

    // Get file size
    let metadata = fs::metadata(&dest_path).map_err(|e| format!("Failed to get file metadata: {}", e))?;
    let size_bytes = metadata.len() as i64;

    // Insert into database
    let now = chrono::Utc::now().timestamp();

    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let document_id = db
        .query_row(
            "INSERT INTO documents (filename, filepath, file_type, title, page_count, size_bytes, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             RETURNING id",
            params![
                original_filename,
                dest_path_str,
                file_type,
                parsed.title,
                parsed.page_count,
                size_bytes,
                now,
            ],
            |row| row.get(0),
        )
        .map_err(|e| format!("Failed to insert document: {}", e))?;

    // Associate with notebook
    db.execute(
        "INSERT INTO notebook_documents (notebook, document_id, added_at) VALUES (?, ?, ?)",
        params![notebook, document_id, now],
    )
    .map_err(|e| format!("Failed to associate document with notebook: {}", e))?;

    drop(db);

    let doc_info = DocumentInfo {
        id: document_id,
        filename: original_filename.to_string(),
        filepath: dest_path_str.clone(),
        file_type: file_type.to_string(),
        title: parsed.title.clone(),
        page_count: parsed.page_count,
        size_bytes,
        indexed_at: None,
        created_at: now,
    };

    // Emit indexing start event
    let _ = app_handle.emit(
        "document-indexing-start",
        serde_json::json!({ "id": document_id, "filename": original_filename }),
    );

    // Start background indexing
    let state_clone = state.inner().clone();
    let app_handle_clone = app_handle.clone();
    let dest_path_clone = dest_path_str.clone();

    tokio::spawn(async move {
        // Get AI config and vision config
        let (embedding_base_url, embedding_api_key, embedding_model, vision_config, vision_base_url, vision_api_key) = {
            let cfg = state_clone.config.read().unwrap();
            let agent = cfg.get_active_agent_or_legacy();
            (
                agent.get_embedding_base_url(),
                agent.get_embedding_api_key(),
                agent.embedding_model.clone(),
                cfg.vision.clone(),
                cfg.get_vision_base_url(),
                cfg.get_vision_api_key(),
            )
        };

        // Check if API key is configured
        if embedding_api_key.is_empty() {
            eprintln!("Skipping document indexing: embedding API key not configured");
            return;
        }

        // Build config for indexer
        let config = crate::state::AIConfig {
            base_url: embedding_base_url,
            api_key: embedding_api_key,
            model: String::new(), // Not used for embeddings
            embedding_model,
        };

        // Index document
        let client = reqwest::Client::new();

        // Use spawn_blocking for SQLite operations
        let state_for_indexing = state_clone.clone();
        let dest_path_for_indexing = dest_path_clone.clone();
        let config_for_indexing = config.clone();
        let vision_config_for_indexing = vision_config.clone();
        let vision_base_url_for_indexing = vision_base_url.clone();
        let vision_api_key_for_indexing = vision_api_key.clone();

        let result = tokio::task::spawn_blocking(move || {
            let db = state_for_indexing.db.lock().unwrap();
            let runtime = tokio::runtime::Handle::current();
            runtime.block_on(index_document(
                &*db,
                &client,
                &config_for_indexing,
                &vision_config_for_indexing,
                &vision_base_url_for_indexing,
                &vision_api_key_for_indexing,
                &dest_path_for_indexing
            ))
        }).await;

        match result {
            Ok(Ok(())) => {
                let _ = app_handle_clone.emit(
                    "document-indexed",
                    serde_json::json!({ "id": document_id }),
                );
            }
            Ok(Err(e)) => {
                eprintln!("Failed to index document {}: {}", document_id, e);
                let _ = app_handle_clone.emit(
                    "document-indexing-error",
                    serde_json::json!({ "id": document_id, "error": e }),
                );
            }
            Err(e) => {
                eprintln!("Indexing task panicked: {}", e);
            }
        }
    });

    Ok(doc_info)
}

/// List all documents associated with a notebook
#[tauri::command]
pub async fn document_list_for_notebook(
    notebook: String,
    state: State<'_, AppState>,
) -> Result<Vec<DocumentInfo>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let mut stmt = db
        .prepare(
            "SELECT d.id, d.filename, d.filepath, d.file_type, d.title, d.page_count,
                    d.size_bytes, d.indexed_at, d.created_at
             FROM documents d
             INNER JOIN notebook_documents nd ON nd.document_id = d.id
             WHERE nd.notebook = ?
             ORDER BY d.created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let docs = stmt
        .query_map([&notebook], |row| {
            Ok(DocumentInfo {
                id: row.get(0)?,
                filename: row.get(1)?,
                filepath: row.get(2)?,
                file_type: row.get(3)?,
                title: row.get(4)?,
                page_count: row.get(5)?,
                size_bytes: row.get(6)?,
                indexed_at: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    Ok(docs)
}

/// List all documents in the system
#[tauri::command]
pub async fn document_list_all(state: State<'_, AppState>) -> Result<Vec<DocumentInfo>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let mut stmt = db
        .prepare(
            "SELECT id, filename, filepath, file_type, title, page_count,
                    size_bytes, indexed_at, created_at
             FROM documents
             ORDER BY created_at DESC",
        )
        .map_err(|e| format!("Failed to prepare statement: {}", e))?;

    let docs = stmt
        .query_map([], |row| {
            Ok(DocumentInfo {
                id: row.get(0)?,
                filename: row.get(1)?,
                filepath: row.get(2)?,
                file_type: row.get(3)?,
                title: row.get(4)?,
                page_count: row.get(5)?,
                size_bytes: row.get(6)?,
                indexed_at: row.get(7)?,
                created_at: row.get(8)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to collect results: {}", e))?;

    Ok(docs)
}

/// Delete a document and its file
#[tauri::command]
pub async fn document_delete(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    // Get filepath before deleting
    let filepath: String = db
        .query_row("SELECT filepath FROM documents WHERE id = ?", [id], |row| {
            row.get(0)
        })
        .map_err(|e| format!("Document not found: {}", e))?;

    // Delete embeddings
    db.execute(
        "DELETE FROM embeddings WHERE source_type = 'document' AND source_id = ?",
        [&filepath],
    )
    .map_err(|e| format!("Failed to delete embeddings: {}", e))?;

    // Delete associations
    db.execute("DELETE FROM notebook_documents WHERE document_id = ?", [id])
        .map_err(|e| format!("Failed to delete associations: {}", e))?;

    // Delete document record
    db.execute("DELETE FROM documents WHERE id = ?", [id])
        .map_err(|e| format!("Failed to delete document: {}", e))?;

    drop(db);

    // Delete file
    let _ = fs::remove_file(&filepath); // Ignore error if file doesn't exist

    Ok(())
}

/// Add existing document to a notebook
#[tauri::command]
pub async fn document_add_to_notebook(
    document_id: i64,
    notebook: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let now = chrono::Utc::now().timestamp();

    db.execute(
        "INSERT OR IGNORE INTO notebook_documents (notebook, document_id, added_at) VALUES (?, ?, ?)",
        params![notebook, document_id, now],
    )
    .map_err(|e| format!("Failed to add document to notebook: {}", e))?;

    Ok(())
}

/// Remove document from notebook (doesn't delete the document)
#[tauri::command]
pub async fn document_remove_from_notebook(
    document_id: i64,
    notebook: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    db.execute(
        "DELETE FROM notebook_documents WHERE notebook = ? AND document_id = ?",
        params![notebook, document_id],
    )
    .map_err(|e| format!("Failed to remove document from notebook: {}", e))?;

    Ok(())
}

/// Get indexing statistics
#[tauri::command]
pub async fn get_indexing_stats(state: State<'_, AppState>) -> Result<IndexingStats, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let total_notes: i64 = db
        .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))
        .unwrap_or(0);

    let indexed_notes: i64 = db
        .query_row(
            "SELECT COUNT(DISTINCT source_id) FROM embeddings WHERE source_type = 'note'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_documents: i64 = db
        .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        .unwrap_or(0);

    let indexed_documents: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM documents WHERE indexed_at IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let total_chunks: i64 = db
        .query_row("SELECT COUNT(*) FROM embeddings", [], |row| row.get(0))
        .unwrap_or(0);

    Ok(IndexingStats {
        total_notes,
        indexed_notes,
        total_documents,
        indexed_documents,
        total_chunks,
    })
}

/// Index all notes in the vault for RAG search
#[tauri::command]
pub async fn index_all_notes_embeddings(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    // Get vault path
    let vault_path = {
        let vault_path_guard = state.vault_path.read()
            .map_err(|e| format!("Vault path lock error: {}", e))?;
        vault_path_guard.as_ref()
            .ok_or("Vault not initialized")?
            .clone()
    };

    // Get AI config
    let (embedding_base_url, embedding_api_key, embedding_model) = {
        let cfg = state.config.read()
            .map_err(|e| format!("Config lock error: {}", e))?;
        let agent = cfg.get_active_agent_or_legacy();
        (
            agent.get_embedding_base_url(),
            agent.get_embedding_api_key(),
            agent.embedding_model.clone()
        )
    };

    // Check if API key is configured
    if embedding_api_key.is_empty() {
        return Err("Embedding API key not configured".to_string());
    }

    // Build config for indexer
    let config = crate::state::AIConfig {
        base_url: embedding_base_url,
        api_key: embedding_api_key,
        model: String::new(),
        embedding_model,
    };

    // Clone state for background task
    let state_clone = state.inner().clone();

    // Run indexing in background
    let indexed_count = tokio::task::spawn_blocking(move || {
        let db = state_clone.db.lock().unwrap();
        let client = reqwest::Client::new();
        let runtime = tokio::runtime::Handle::current();
        runtime.block_on(index_all_notes(&*db, &client, &config, &vault_path))
    })
    .await
    .map_err(|e| format!("Indexing task failed: {}", e))??;

    // Emit completion event
    let _ = app_handle.emit("notes-indexed", serde_json::json!({ "count": indexed_count }));

    Ok(indexed_count)
}
