use rusqlite::Connection;
use reqwest::Client;
use std::path::Path;
use std::fs;
use crate::utils::chunker::{chunk_text, chunk_pages};
use crate::utils::embeddings::{get_embeddings, vec_to_blob};
use crate::services::doc_parser::{parse_document, parse_document_with_vision};
use crate::state::{AiConfig, VisionConfig};

/// Index a document: parse, chunk, embed, and store in database
pub async fn index_document(
    db: &Connection,
    client: &Client,
    config: &AiConfig,
    vision_config: &VisionConfig,
    vision_base_url: &str,
    vision_api_key: &str,
    doc_filepath: &str,
) -> Result<(), String> {
    let path = Path::new(doc_filepath);

    // 1. Parse document (with vision fallback if enabled)
    eprintln!("Parsing document: {}", doc_filepath);
    let parsed = if vision_config.enabled && !vision_api_key.is_empty() {
        parse_document_with_vision(
            path,
            client,
            true,
            vision_base_url,
            vision_api_key,
            &vision_config.model,
        ).await?
    } else {
        // Fall back to regular parsing without vision
        parse_document(path)?
    };

    // 2. Chunk text with page numbers
    const CHUNK_SIZE: usize = 1500;
    const OVERLAP: usize = 200;

    let chunks = chunk_pages(&parsed.pages, CHUNK_SIZE, OVERLAP);

    if chunks.is_empty() {
        return Err("Document has no text chunks (text too short or empty)".to_string());
    }

    eprintln!("Split into {} chunks", chunks.len());

    // 3. Get embeddings
    let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
    let embeddings = get_embeddings(
        client,
        &config.base_url,
        &config.api_key,
        &config.embedding_model,
        texts,
    ).await?;

    if embeddings.len() != chunks.len() {
        return Err(format!(
            "Embedding count mismatch: got {}, expected {}",
            embeddings.len(),
            chunks.len()
        ));
    }

    // 4. Delete old embeddings for this document
    db.execute(
        "DELETE FROM embeddings WHERE source_type = 'document' AND source_id = ?",
        [doc_filepath],
    )
    .map_err(|e| format!("Failed to delete old embeddings: {}", e))?;

    // 5. Insert new embeddings
    let mut stmt = db.prepare(
        "INSERT INTO embeddings (source_type, source_id, chunk_index, chunk_text, vector, updated_at, page)
         VALUES ('document', ?, ?, ?, ?, ?, ?)"
    )
    .map_err(|e| format!("Failed to prepare insert statement: {}", e))?;

    let now = chrono::Utc::now().timestamp();

    for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
        let blob = vec_to_blob(embedding);
        stmt.execute(rusqlite::params![
            doc_filepath,
            chunk.index,
            chunk.text,
            blob,
            now,
            chunk.page,  // Option<u32> - will be stored as INTEGER or NULL
        ])
        .map_err(|e| format!("Failed to insert embedding: {}", e))?;
    }

    // 6. Update indexed_at timestamp
    db.execute(
        "UPDATE documents SET indexed_at = ? WHERE filepath = ?",
        rusqlite::params![now, doc_filepath],
    )
    .map_err(|e| format!("Failed to update indexed_at: {}", e))?;

    eprintln!("Successfully indexed document: {} ({} chunks)", doc_filepath, chunks.len());

    Ok(())
}

/// Index note embeddings (for RAG search in notes)
pub async fn index_note_embeddings(
    db: &Connection,
    client: &Client,
    config: &AiConfig,
    note_path: &str,
    note_content: &str,
) -> Result<(), String> {
    // 1. Chunk text
    const CHUNK_SIZE: usize = 1500;
    const OVERLAP: usize = 200;

    let chunks = chunk_text(note_content, CHUNK_SIZE, OVERLAP);

    if chunks.is_empty() {
        // Note too short - delete embeddings and return
        db.execute(
            "DELETE FROM embeddings WHERE source_type = 'note' AND source_id = ?",
            [note_path],
        )
        .map_err(|e| format!("Failed to delete embeddings: {}", e))?;
        return Ok(());
    }

    // 2. Get embeddings
    let texts: Vec<String> = chunks.iter().map(|c| c.text.clone()).collect();
    let embeddings = get_embeddings(
        client,
        &config.base_url,
        &config.api_key,
        &config.embedding_model,
        texts,
    ).await?;

    if embeddings.len() != chunks.len() {
        return Err(format!(
            "Embedding count mismatch: got {}, expected {}",
            embeddings.len(),
            chunks.len()
        ));
    }

    // 3. Delete old embeddings
    db.execute(
        "DELETE FROM embeddings WHERE source_type = 'note' AND source_id = ?",
        [note_path],
    )
    .map_err(|e| format!("Failed to delete old embeddings: {}", e))?;

    // 4. Insert new embeddings
    let mut stmt = db.prepare(
        "INSERT INTO embeddings (source_type, source_id, chunk_index, chunk_text, vector, updated_at, page)
         VALUES ('note', ?, ?, ?, ?, ?, ?)"
    )
    .map_err(|e| format!("Failed to prepare insert statement: {}", e))?;

    let now = chrono::Utc::now().timestamp();

    for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
        let blob = vec_to_blob(embedding);
        stmt.execute(rusqlite::params![
            note_path,
            chunk.index,
            chunk.text,
            blob,
            now,
            chunk.page,  // Will be None for notes (no pages in markdown)
        ])
        .map_err(|e| format!("Failed to insert embedding: {}", e))?;
    }

    Ok(())
}

/// Index all notes in the vault for RAG search (only modified notes)
pub async fn index_all_notes(
    db: &Connection,
    client: &Client,
    config: &AiConfig,
    vault_path: &Path,
) -> Result<usize, String> {
    // Get all notes with their modification times
    let mut stmt = db.prepare("SELECT path, modified_at FROM notes")
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let notes: Vec<(String, i64)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| format!("Query failed: {}", e))?
        .filter_map(Result::ok)
        .collect();

    eprintln!("Found {} notes, checking which need indexing...", notes.len());

    let mut indexed_count = 0;
    let mut skipped_count = 0;
    let mut error_count = 0;

    for (i, (note_path, file_modified_at)) in notes.iter().enumerate() {
        // Check if note is already indexed and up to date
        let last_indexed: Option<i64> = db
            .query_row(
                "SELECT updated_at FROM embeddings WHERE source_type = 'note' AND source_id = ? LIMIT 1",
                [note_path],
                |row| row.get(0),
            )
            .ok();

        // Skip if already indexed and file hasn't changed
        if let Some(indexed_at) = last_indexed {
            if indexed_at >= *file_modified_at {
                skipped_count += 1;
                continue;
            }
        }

        // Need to index
        let full_path = vault_path.join(note_path);

        match fs::read_to_string(&full_path) {
            Ok(content) => {
                eprintln!("Indexing note {}/{}: {} (modified)", i + 1, notes.len(), note_path);

                match index_note_embeddings(db, client, config, note_path, &content).await {
                    Ok(()) => {
                        indexed_count += 1;
                    }
                    Err(e) => {
                        eprintln!("Failed to index note {}: {}", note_path, e);
                        error_count += 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read note {}: {}", note_path, e);
                error_count += 1;
            }
        }
    }

    eprintln!("Indexing complete: {} indexed, {} skipped (up-to-date), {} errors",
              indexed_count, skipped_count, error_count);

    Ok(indexed_count)
}
