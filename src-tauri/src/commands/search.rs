use crate::services::ai_client::{AiClient, ChatMessage};
use crate::state::AppState;
use crate::utils::embeddings::{blob_to_vec, find_top_k, get_embeddings};
use futures::StreamExt;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub notebook: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

/// Full-text search using FTS5
#[tauri::command]
pub async fn fts_search(
    query: String,
    notebook: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<SearchResult>, String> {
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let sql = if notebook.is_some() {
        "SELECT path, notebook, title, snippet(notes_fts, 3, '<mark>', '</mark>', '...', 64)
         FROM notes_fts
         WHERE notes_fts MATCH ? AND notebook = ?
         ORDER BY rank
         LIMIT 50"
    } else {
        "SELECT path, notebook, title, snippet(notes_fts, 3, '<mark>', '</mark>', '...', 64)
         FROM notes_fts
         WHERE notes_fts MATCH ?
         ORDER BY rank
         LIMIT 50"
    };

    let mut stmt = db
        .prepare(sql)
        .map_err(|e| format!("Failed to prepare FTS query: {}", e))?;

    let mut results = Vec::new();

    if let Some(ref nb) = notebook {
        let rows = stmt.query_map(params![query, nb], |row| {
            Ok(SearchResult {
                path: row.get(0)?,
                notebook: row.get(1)?,
                title: row.get(2)?,
                snippet: row.get(3)?,
                score: 1.0,
            })
        })
        .map_err(|e| format!("FTS query failed: {}", e))?;

        for row in rows {
            results.push(row.map_err(|e| format!("Failed to collect results: {}", e))?);
        }
    } else {
        let rows = stmt.query_map([query], |row| {
            Ok(SearchResult {
                path: row.get(0)?,
                notebook: row.get(1)?,
                title: row.get(2)?,
                snippet: row.get(3)?,
                score: 1.0,
            })
        })
        .map_err(|e| format!("FTS query failed: {}", e))?;

        for row in rows {
            results.push(row.map_err(|e| format!("Failed to collect results: {}", e))?);
        }
    }

    Ok(results)
}

/// RAG search through notes using embeddings
#[tauri::command]
pub async fn rag_search_notes(
    query: String,
    notebook: String,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Get AI config
    let (embedding_base_url, embedding_api_key, embedding_model, chat_base_url, chat_api_key, chat_model) = {
        let cfg = state.config.read().map_err(|e| format!("Config lock error: {}", e))?;
        let agent = cfg.get_active_agent_or_legacy();
        (
            agent.get_embedding_base_url(),
            agent.get_embedding_api_key(),
            agent.embedding_model.clone(),
            agent.base_url.clone(),
            agent.api_key.clone(),
            agent.models.first().cloned().unwrap_or_default()
        )
    };

    // Check API key
    if embedding_api_key.is_empty() {
        return Err("Embedding API key not configured".to_string());
    }

    // Get query embedding
    let client = reqwest::Client::new();
    let query_embeddings = get_embeddings(
        &client,
        &embedding_base_url,
        &embedding_api_key,
        &embedding_model,
        vec![query.clone()],
    )
    .await?;

    let query_vec = query_embeddings
        .first()
        .ok_or("No embedding returned for query")?;

    // Fetch note embeddings from database
    let db = state.db.lock().map_err(|e| format!("DB lock error: {}", e))?;

    let sql = if notebook.is_empty() {
        // Global search
        "SELECT id, source_id, chunk_text, vector, page FROM embeddings WHERE source_type = 'note'"
    } else {
        // Notebook-specific search
        "SELECT id, source_id, chunk_text, vector, page FROM embeddings
         WHERE source_type = 'note' AND source_id LIKE ?"
    };

    let mut stmt = db.prepare(sql).map_err(|e| format!("Failed to prepare query: {}", e))?;

    let mut embeddings: Vec<(i64, String, String, Vec<f32>, Option<i64>)> = Vec::new();

    if notebook.is_empty() {
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let source_id: String = row.get(1)?;
            let chunk_text: String = row.get(2)?;
            let blob: Vec<u8> = row.get(3)?;
            let page: Option<i64> = row.get(4)?;
            let vec = blob_to_vec(&blob);
            Ok((id, source_id, chunk_text, vec, page))
        })
        .map_err(|e| format!("Query failed: {}", e))?;

        for row in rows {
            embeddings.push(row.map_err(|e| format!("Failed to collect embeddings: {}", e))?);
        }
    } else {
        let pattern = format!("{}/%", notebook);
        let rows = stmt.query_map([pattern], |row| {
            let id: i64 = row.get(0)?;
            let source_id: String = row.get(1)?;
            let chunk_text: String = row.get(2)?;
            let blob: Vec<u8> = row.get(3)?;
            let page: Option<i64> = row.get(4)?;
            let vec = blob_to_vec(&blob);
            Ok((id, source_id, chunk_text, vec, page))
        })
        .map_err(|e| format!("Query failed: {}", e))?;

        for row in rows {
            embeddings.push(row.map_err(|e| format!("Failed to collect embeddings: {}", e))?);
        }
    }

    // db will be dropped here

    if embeddings.is_empty() {
        return Err("No indexed notes found. Notes must be indexed first.".to_string());
    }

    // Find top-k similar chunks
    const MIN_RELEVANCE: f32 = 0.3; // 30% minimum relevance threshold
    let top_chunks = find_top_k(query_vec, &embeddings, 10)
        .into_iter()
        .filter(|(_, _, _, score, _)| *score >= MIN_RELEVANCE)
        .collect::<Vec<_>>();

    // Build sources list for emission
    let mut sources_map: HashMap<String, f32> = HashMap::new();
    for (_, source_id, _, score, _) in &top_chunks {
        sources_map
            .entry(source_id.clone())
            .and_modify(|s| *s = s.max(*score))
            .or_insert(*score);
    }

    let sources: Vec<serde_json::Value> = sources_map
        .into_iter()
        .map(|(path, relevance)| {
            serde_json::json!({
                "path": path,
                "relevance": (relevance * 100.0) as u32,
            })
        })
        .collect();

    // Emit sources
    let _ = app_handle.emit("rag-sources", serde_json::json!({ "sources": sources }));

    // Check if we have relevant sources
    if top_chunks.is_empty() {
        let _ = app_handle.emit("rag-chunk", serde_json::json!({
            "content": "⚠️ No relevant sources found (all sources below 30% relevance threshold). Answering without source context.\n\n"
        }));
    }

    // Build context from top chunks
    let context = top_chunks
        .iter()
        .map(|(_, source_id, chunk_text, _, _)| {
            // Notes don't have page numbers, so the 5th element will always be None
            format!("[Note: {}]\n{}\n", source_id, chunk_text)
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Build system prompt
    let system_prompt = format!(
        "You are a helpful assistant that answers questions ONLY based on the user's notes. \
        If the answer is not in the provided context, say so clearly.\n\n\
        Reference materials from notes:\n\n{}",
        context
    );

    // Prepare messages
    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: system_prompt,
        },
        ChatMessage {
            role: "user".to_string(),
            content: query,
        },
    ];

    // Check chat configuration
    if chat_api_key.is_empty() {
        let _ = app_handle.emit("rag-error", serde_json::json!({
            "error": "Chat API key not configured. Please configure AI provider in Settings."
        }));
        return Ok(());
    }

    if chat_model.is_empty() {
        let _ = app_handle.emit("rag-error", serde_json::json!({
            "error": "Chat model not configured. Please add a model to your AI provider in Settings."
        }));
        return Ok(());
    }

    // Stream response
    let app_handle_clone = app_handle.clone();

    tokio::spawn(async move {
        let ai_client = AiClient::new(
            &chat_base_url,
            &chat_api_key,
            &chat_model,
        );

        match ai_client.chat_stream(messages).await {
            Ok(mut stream) => {
                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(chunk) => {
                            let _ = app_handle_clone.emit(
                                "rag-chunk",
                                serde_json::json!({ "content": chunk }),
                            );
                        }
                        Err(e) => {
                            let _ = app_handle_clone.emit(
                                "rag-error",
                                serde_json::json!({ "error": e }),
                            );
                            break;
                        }
                    }
                }
                let _ = app_handle_clone.emit("rag-done", serde_json::json!({}));
            }
            Err(e) => {
                let _ = app_handle_clone.emit(
                    "rag-error",
                    serde_json::json!({ "error": e }),
                );
            }
        }
    });

    Ok(())
}
