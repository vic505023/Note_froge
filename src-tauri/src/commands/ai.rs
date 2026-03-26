use crate::services::ai_client::{AiClient, ChatMessage};
use crate::state::AppState;
use crate::utils::embeddings::{blob_to_vec, find_top_k, get_embeddings};
use futures::StreamExt;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn ai_chat(
    messages: Vec<ChatMessage>,
    notebook: String,
    note_context: Option<String>,
    use_sources: bool,
    web_search: bool,
    model: Option<String>,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Read config
    let (api_key, base_url, selected_model) = {
        let config = state.config.read().unwrap();

        // Get active agent
        let agent = config.get_active_agent_or_legacy();

        if agent.api_key.is_empty() {
            return Err("API key not configured. Open Settings (Ctrl+,)".to_string());
        }

        // Use provided model or default to first model from agent
        let model_to_use = model.or_else(|| agent.models.first().cloned())
            .ok_or_else(|| "No model selected. Add models in Settings (Ctrl+,)".to_string())?;

        (
            agent.api_key.clone(),
            agent.base_url.clone(),
            model_to_use,
        )
    };

    // Create AI client
    let client = AiClient::new(&base_url, &api_key, &selected_model);

    // RAG: Get sources context if enabled
    let mut sources_context = String::new();
    if use_sources && !web_search && !api_key.is_empty() {
        // Get last user message for embedding
        if let Some(last_msg) = messages.last() {
            let query = &last_msg.content;

            // Get embedding for query
            let reqwest_client = reqwest::Client::new();
            let (embedding_base_url, embedding_api_key, embedding_model) = {
                let config = state.config.read().unwrap();
                let agent = config.get_active_agent_or_legacy();
                (
                    agent.get_embedding_base_url(),
                    agent.get_embedding_api_key(),
                    agent.embedding_model.clone()
                )
            };

            match get_embeddings(
                &reqwest_client,
                &embedding_base_url,
                &embedding_api_key,
                &embedding_model,
                vec![query.clone()],
            )
            .await
            {
                Ok(embeddings) if !embeddings.is_empty() => {
                    let query_vec = &embeddings[0];

                    // Fetch document embeddings for this notebook
                    let db = state.db.lock().unwrap();

                    let stmt = db
                        .prepare(
                            "SELECT e.id, e.source_id, e.chunk_text, e.vector, e.page, d.filename, d.file_type
                             FROM embeddings e
                             INNER JOIN documents d ON d.filepath = e.source_id
                             INNER JOIN notebook_documents nd ON nd.document_id = d.id
                             WHERE e.source_type = 'document' AND nd.notebook = ?",
                        )
                        .ok();

                    if let Some(mut stmt) = stmt {
                        let doc_embeddings: Vec<(i64, String, String, Vec<f32>, Option<i64>, String, String)> = stmt
                            .query_map([&notebook], |row| {
                                let id: i64 = row.get(0)?;
                                let source_id: String = row.get(1)?;
                                let chunk_text: String = row.get(2)?;
                                let blob: Vec<u8> = row.get(3)?;
                                let page: Option<i64> = row.get(4)?;
                                let filename: String = row.get(5)?;
                                let file_type: String = row.get(6)?;
                                let vec = blob_to_vec(&blob);
                                Ok((id, source_id, chunk_text, vec, page, filename, file_type))
                            })
                            .ok()
                            .map(|rows| rows.filter_map(Result::ok).collect())
                            .unwrap_or_default();

                        // db will be dropped here

                        if !doc_embeddings.is_empty() {
                            // Convert to format for find_top_k
                            let embeddings_for_search: Vec<(i64, String, String, Vec<f32>, Option<i64>)> =
                                doc_embeddings
                                    .iter()
                                    .map(|(id, sid, text, vec, page, _, _)| {
                                        (*id, sid.clone(), text.clone(), vec.clone(), *page)
                                    })
                                    .collect();

                            // Find top 10 chunks with minimum relevance threshold
                            const MIN_RELEVANCE: f32 = 0.3; // 30% minimum relevance
                            let top_chunks = find_top_k(query_vec, &embeddings_for_search, 10)
                                .into_iter()
                                .filter(|(_, _, _, score, _)| *score >= MIN_RELEVANCE)
                                .collect::<Vec<_>>();

                            // Check if we have relevant sources after filtering
                            if top_chunks.is_empty() {
                                eprintln!("No sources above 30% relevance threshold found");
                            }

                            // Build sources map for emission - collect ALL pages used
                            let mut sources_map: HashMap<String, (f32, String, Vec<i64>, String)> = HashMap::new();
                            for (_, source_id, _, score, page) in &top_chunks {
                                // Find filename and file_type for this source_id
                                if let Some((_, _, _, _, _, filename, file_type)) = doc_embeddings
                                    .iter()
                                    .find(|(_, sid, _, _, _, _, _)| sid == source_id)
                                {
                                    sources_map
                                        .entry(filename.clone())
                                        .and_modify(|(s, _, pages, _)| {
                                            // Keep max score
                                            if *score > *s {
                                                *s = *score;
                                            }
                                            // Add page if not already present
                                            if let Some(p) = page {
                                                if !pages.contains(p) {
                                                    pages.push(*p);
                                                }
                                            }
                                        })
                                        .or_insert((*score, source_id.clone(), page.map_or(vec![], |p| vec![p]), file_type.clone()));
                                }
                            }

                            // Emit sources
                            let sources: Vec<serde_json::Value> = sources_map
                                .iter()
                                .map(|(filename, (relevance, filepath, pages, file_type))| {
                                    let mut source = serde_json::json!({
                                        "filename": filename,
                                        "filepath": filepath,
                                        "relevance": (relevance * 100.0) as u32,
                                        "file_type": file_type,
                                    });
                                    if !pages.is_empty() {
                                        // Sort pages in ascending order
                                        let mut sorted_pages = pages.clone();
                                        sorted_pages.sort();
                                        source["pages"] = serde_json::json!(sorted_pages);
                                    }
                                    source
                                })
                                .collect();

                            let _ = app_handle.emit(
                                "ai-sources",
                                serde_json::json!({ "sources": sources }),
                            );

                            // Build context from top chunks
                            let mut chunks_by_file: HashMap<String, Vec<(String, Option<i64>)>> = HashMap::new();
                            for (_, source_id, chunk_text, _, page) in &top_chunks {
                                if let Some((_, _, _, _, _, filename, _)) = doc_embeddings
                                    .iter()
                                    .find(|(_, sid, _, _, _, _, _)| sid == source_id)
                                {
                                    chunks_by_file
                                        .entry(filename.clone())
                                        .or_default()
                                        .push((chunk_text.clone(), *page));
                                }
                            }

                            sources_context = chunks_by_file
                                .into_iter()
                                .map(|(filename, chunks)| {
                                    let chunks_text = chunks
                                        .iter()
                                        .map(|(text, page)| {
                                            if let Some(p) = page {
                                                format!("[p.{}] {}", p, text)
                                            } else {
                                                text.clone()
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                        .join("\n\n");
                                    format!(
                                        "[Source: {}]\n{}\n",
                                        filename,
                                        chunks_text
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to get embeddings for query: {}", e);
                }
                _ => {}
            }
        }
    }

    // Build messages for API
    let mut api_messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: r#"YOU HAVE NO FUNCTIONS. YOU HAVE NO TOOLS. YOU CANNOT CALL FUNCTIONS.
DO NOT generate XML tags like <function_calls>, <invoke>, <parameter>.
YOU ARE A TEXT-ONLY ASSISTANT."#.to_string(),
        },
        ChatMessage {
        role: "system".to_string(),
        content: format!(
            r#"You are a helpful AI assistant integrated into a note-taking app called NoteForge.
You help the user with their notes and answer questions.
Be concise and helpful. Respond in the same language the user writes in.

{}

═══════════════════════════════════════════════
CRITICAL: YOU ARE IN CHAT MODE
═══════════════════════════════════════════════

You CANNOT directly edit the note. You can only ANSWER QUESTIONS.

If the user asks to ADD/REMOVE/EDIT the note (examples):
- "добавь таблицу в заметку"
- "удали из заметки"
- "add to note"
- "insert into note"

→ YOU MUST respond:
"Я не могу редактировать заметку в режиме чата. Переключитесь на режим редактирования (кнопка с карандашом ✏️) и повторите команду."

═══════════════════════════════════════════════
ABSOLUTELY FORBIDDEN IN CHAT MODE:
═══════════════════════════════════════════════

❌ XML tags: <noteforge-edit>, <function_calls>, <invoke>, <parameter>
❌ Function calling syntax
❌ JSON structures with "function" or "tool"
❌ Pretending you edited: "Я добавил", "Done!", "✅ Готово"
❌ Status updates: "Удаляю...", "Updating...", "Adding..."

ONLY USE PLAIN TEXT RESPONSES.

═══════════════════════════════════════════════
WHAT YOU CAN DO:
═══════════════════════════════════════════════

✓ Answer questions about the note
✓ Generate content IN THE CHAT (tables, lists, summaries)
✓ Explain concepts
✓ Have conversations

YOU ARE IN CHAT MODE. YOU CANNOT EDIT FILES."#,
            if !sources_context.is_empty() {
                format!(
                    "You have access to the following reference materials from the user's sources:\n\n{}\n\
                    Use these sources to answer questions when relevant. Cite which source you used.\n\
                    Note: Only sources with 30%+ relevance to the query are shown.",
                    sources_context
                )
            } else if use_sources {
                "No sources with sufficient relevance (30%+) were found for this query. \
                Answer based on your general knowledge, but inform the user that no relevant sources were available.".to_string()
            } else {
                String::new()
            }
        ),
    }];

    // Add note context if provided
    if let Some(context) = note_context {
        api_messages.push(ChatMessage {
            role: "system".to_string(),
            content: format!(
                "Current note content:\n---\n{}\n---",
                context
            ),
        });
    }

    // Detect user language from last message
    let user_lang = messages
        .last()
        .map(|m| {
            if m.content.chars().any(|c| {
                ('\u{0400}'..='\u{04FF}').contains(&c) || c == 'а' || c == 'я' || c == 'ё'
            }) {
                "ru"
            } else {
                "en"
            }
        })
        .unwrap_or("en");

    // Add user messages
    api_messages.extend(messages);

    // Start streaming
    match client.chat_stream(api_messages).await {
        Ok(mut stream) => {
            let mut accumulated = String::new();
            let mut sent_count = 0;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        accumulated.push_str(&chunk);

                        // Check for forbidden patterns (buffer first 100 chars before streaming)
                        if accumulated.len() < 100 || sent_count == 0 {
                            if accumulated.contains("<function")
                                || accumulated.contains("<invoke")
                                || accumulated.contains("<parameter")
                                || accumulated.contains("<notebook")
                                || accumulated.contains("function_calls")
                            {
                                // Blocked! Send error message
                                let error_msg = if user_lang == "ru" {
                                    "Я не могу редактировать заметку в режиме чата. \
                                    Переключитесь на режим редактирования (кнопка с карандашом ✏️) и повторите команду."
                                } else {
                                    "I cannot edit the note in chat mode. \
                                    Switch to edit mode (pencil button ✏️) and try again."
                                };
                                let _ = app_handle.emit(
                                    "ai-chunk",
                                    serde_json::json!({ "content": error_msg }),
                                );
                                let _ = app_handle.emit("ai-done", serde_json::json!({}));
                                return Ok(());
                            }
                        }

                        // After 100 chars buffered, start streaming
                        if accumulated.len() >= 100 && sent_count == 0 {
                            // Send all buffered content
                            let _ = app_handle.emit(
                                "ai-chunk",
                                serde_json::json!({ "content": accumulated.clone() }),
                            );
                            sent_count = accumulated.len();
                        } else if sent_count > 0 {
                            // Stream normally
                            if let Err(e) = app_handle.emit(
                                "ai-chunk",
                                serde_json::json!({ "content": chunk }),
                            ) {
                                eprintln!("Failed to emit ai-chunk: {}", e);
                            }
                            sent_count += chunk.len();
                        }
                    }
                    Err(e) => {
                        let _ = app_handle.emit(
                            "ai-error",
                            serde_json::json!({ "error": e.to_string() }),
                        );
                        return Err(e);
                    }
                }
            }

            // Send remaining buffered content if any
            if accumulated.len() > 0 && sent_count == 0 {
                let _ = app_handle.emit(
                    "ai-chunk",
                    serde_json::json!({ "content": accumulated }),
                );
            }

            let _ = app_handle.emit("ai-done", serde_json::json!({}));
            Ok(())
        }
        Err(e) => {
            let _ = app_handle.emit("ai-error", serde_json::json!({ "error": e.clone() }));
            Err(e)
        }
    }
}

#[tauri::command]
pub async fn ai_test_connection(state: State<'_, AppState>) -> Result<String, String> {
    let (api_key, base_url, model) = {
        let config = state.config.read().unwrap();

        // Get active agent
        let agent = config.get_active_agent_or_legacy();

        if agent.api_key.is_empty() {
            return Err("API key not configured".to_string());
        }

        let model_to_use = agent.models.first().cloned()
            .ok_or_else(|| "No model configured. Add models in Settings".to_string())?;

        (
            agent.api_key.clone(),
            agent.base_url.clone(),
            model_to_use,
        )
    };

    let client = AiClient::new(&base_url, &api_key, &model);

    client.test_connection().await
}

#[tauri::command]
pub async fn ai_edit_note(
    instruction: String,
    current_content: String,
    notebook: String,
    previous_messages: Vec<ChatMessage>,
    model: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let (api_key, base_url, selected_model, embedding_model) = {
        let config = state.config.read().unwrap();

        // Get active agent
        let agent = config.get_active_agent_or_legacy();

        if agent.api_key.is_empty() {
            return Err("API key not configured".to_string());
        }

        // Use provided model or default to first model from agent
        let model_to_use = model.or_else(|| agent.models.first().cloned())
            .ok_or_else(|| "No model selected. Add models in Settings".to_string())?;

        (
            agent.api_key.clone(),
            agent.base_url.clone(),
            model_to_use,
            agent.embedding_model.clone(),
        )
    };

    let client = AiClient::new(&base_url, &api_key, &selected_model);

    // RAG: Get sources context for editing
    let mut sources_context = String::new();
    if !notebook.is_empty() {
        // Get embedding for instruction
        let reqwest_client = reqwest::Client::new();

        // Get embedding config
        let (embedding_base_url, embedding_api_key) = {
            let config = state.config.read().unwrap();
            let agent = config.get_active_agent_or_legacy();
            (
                agent.get_embedding_base_url(),
                agent.get_embedding_api_key()
            )
        };

        if embedding_api_key.is_empty() {
            eprintln!("Skipping RAG for edit: embedding API key not configured");
        } else {
            match get_embeddings(
                &reqwest_client,
                &embedding_base_url,
                &embedding_api_key,
                &embedding_model,
                vec![instruction.clone()],
            )
            .await
        {
            Ok(embeddings) if !embeddings.is_empty() => {
                let query_vec = &embeddings[0];

                // Fetch document embeddings for this notebook
                let db = state.db.lock().unwrap();

                let stmt = db
                    .prepare(
                        "SELECT e.id, e.source_id, e.chunk_text, e.vector, e.page, d.filename, d.file_type
                         FROM embeddings e
                         INNER JOIN documents d ON d.filepath = e.source_id
                         INNER JOIN notebook_documents nd ON nd.document_id = d.id
                         WHERE e.source_type = 'document' AND nd.notebook = ?",
                    )
                    .ok();

                if let Some(mut stmt) = stmt {
                    let doc_embeddings: Vec<(i64, String, String, Vec<f32>, Option<i64>, String, String)> = stmt
                        .query_map([&notebook], |row| {
                            let id: i64 = row.get(0)?;
                            let source_id: String = row.get(1)?;
                            let chunk_text: String = row.get(2)?;
                            let blob: Vec<u8> = row.get(3)?;
                            let page: Option<i64> = row.get(4)?;
                            let filename: String = row.get(5)?;
                            let file_type: String = row.get(6)?;
                            let vec = blob_to_vec(&blob);
                            Ok((id, source_id, chunk_text, vec, page, filename, file_type))
                        })
                        .ok()
                        .map(|rows| rows.filter_map(Result::ok).collect())
                        .unwrap_or_default();

                    // db will be dropped here

                    if !doc_embeddings.is_empty() {
                        // Convert to format for find_top_k
                        let embeddings_for_search: Vec<(i64, String, String, Vec<f32>, Option<i64>)> =
                            doc_embeddings
                                .iter()
                                .map(|(id, sid, text, vec, page, _, _)| {
                                    (*id, sid.clone(), text.clone(), vec.clone(), *page)
                                })
                                .collect();

                        // Find top 10 chunks with minimum relevance threshold
                        const MIN_RELEVANCE: f32 = 0.3; // 30% minimum relevance
                        let top_chunks = find_top_k(query_vec, &embeddings_for_search, 10)
                            .into_iter()
                            .filter(|(_, _, _, score, _)| *score >= MIN_RELEVANCE)
                            .collect::<Vec<_>>();

                        if top_chunks.is_empty() {
                            eprintln!("No sources above 30% relevance threshold found for edit instruction");
                        }

                        // Build context from top chunks
                        let mut chunks_by_file: HashMap<String, Vec<(String, Option<i64>)>> = HashMap::new();
                        for (_, source_id, chunk_text, _, page) in &top_chunks {
                            if let Some((_, _, _, _, _, filename, _)) = doc_embeddings
                                .iter()
                                .find(|(_, sid, _, _, _, _, _)| sid == source_id)
                            {
                                chunks_by_file
                                    .entry(filename.clone())
                                    .or_default()
                                    .push((chunk_text.clone(), *page));
                            }
                        }

                        sources_context = chunks_by_file
                            .into_iter()
                            .map(|(filename, chunks)| {
                                let chunks_text = chunks
                                    .iter()
                                    .map(|(text, page)| {
                                        if let Some(p) = page {
                                            format!("[p.{}] {}", p, text)
                                        } else {
                                            text.clone()
                                        }
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n\n");
                                format!(
                                    "[Source: {}]\n{}\n",
                                    filename,
                                    chunks_text
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get embeddings for instruction: {}", e);
            }
            _ => {}
            }
        }
    }

    // Build messages with conversation history
    let mut messages = vec![ChatMessage {
        role: "system".to_string(),
        content: format!(
            r#"You are a helpful AI assistant integrated into a note-taking app called NoteForge.
You help the user with their notes, answer questions, and can edit the current note when asked.
Be concise and helpful. Respond in the same language the user writes in.

{}"#,
            if !sources_context.is_empty() {
                format!(
                    "You have access to the following reference materials from the user's sources:\n\n{}\n\
                    Use these sources when editing the note if relevant.\n\
                    Note: Only sources with 30%+ relevance to the instruction are shown.",
                    sources_context
                )
            } else if !notebook.is_empty() {
                "Note: No sources with sufficient relevance (30%+) were found for this instruction.".to_string()
            } else {
                String::new()
            }
        ),
    }];

    // Add conversation history (excluding system messages to avoid duplication)
    messages.extend(
        previous_messages
            .into_iter()
            .filter(|m| m.role != "system")
    );

    // Add current note content and edit instruction rules
    messages.push(ChatMessage {
        role: "system".to_string(),
        content: format!(
            r#"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
END OF CONVERSATION HISTORY
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

CURRENT NOTE FILE CONTENT:
────────────────────────────────────────
{}
────────────────────────────────────────

IMPORTANT CONTEXT:
- Everything above the first line was our CONVERSATION - we discussed ideas, generated examples, etc.
- The content between the lines above is the ACTUAL NOTE FILE - this is what currently exists in the file
- These are SEPARATE things - do NOT confuse what we discussed with what's actually in the file

The user will now give you an instruction to edit THE FILE (not the conversation).

EDIT RULES:
1. ADD/INSERT instruction → Take content from conversation + add to file
   Example: "add the table we discussed"
   → Find table in conversation → Insert into file → Keep all original content

2. MODIFY instruction → Edit only the specified part
   Example: "make the introduction shorter"
   → Modify only introduction → Keep everything else unchanged

3. REPLACE/REWRITE instruction → Replace entire file content
   Example: "rewrite this note completely"
   → Generate new content

4. DELETE + WRITE instruction → Clear file and write new content
   Example: "write only X, delete everything else" OR "replace the note with just Y"
   → Clear file → Write only the requested content

5. When in doubt → Minimal changes only

PROCESS:
- Read conversation history to find what user references ("this table", "that list", etc.)
- Read current file content to see what exists
- Analyze instruction keywords:
  * "add", "insert", "append" → Keep existing + add new
  * "modify", "change", "edit" → Change specific part
  * "delete everything and", "replace with", "only write" → Clear + write new
  * "rewrite", "start over" → Replace entire content
- Apply the change
- Return the updated file

═══════════════════════════════════════════════
CRITICAL OUTPUT REQUIREMENTS
═══════════════════════════════════════════════

YOU MUST return ONLY the raw markdown file content. Nothing else.

Your response must START with the first character of the markdown file.
Your response must END with the last character of the markdown file.

DO NOT include:
❌ ANY text before the markdown ("Удаляю...", "Here's...", "Вот обновлённая...")
❌ ANY text after the markdown ("Готово!", "Done!", "Changes applied...")
❌ Status messages ("✅", "I deleted...", "Я удалил...")
❌ XML tags (<function_calls>, <invoke>, <parameter>)
❌ JSON structures
❌ Code fences (```)
❌ Explanations

CORRECT OUTPUT EXAMPLE:
# Title
Content here...

INCORRECT OUTPUT EXAMPLES:
❌ "Удаляю таблицу. # Title..."
❌ "<function_calls>...</function_calls>"
❌ "```markdown\n# Title\n```"
❌ "Готово! # Title..."

YOU ARE PROPOSING CHANGES, NOT APPLYING THEM.
The user will review a diff and click Apply."#,
            current_content
        ),
    });

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: instruction,
    });

    client.chat_complete(messages).await
}

#[derive(Serialize, Deserialize)]
pub struct ChatHistoryEntry {
    pub id: i64,
    pub note_path: Option<String>,
    pub role: String,
    pub content: String,
    pub mode: String,
    pub created_at: i64,
}

#[tauri::command]
pub async fn save_chat_message(
    notebook: String,
    note_path: Option<String>,
    role: String,
    content: String,
    mode: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    db.execute(
        "INSERT INTO chat_history (notebook, note_path, role, content, mode, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![notebook, note_path, role, content, mode, now],
    )
    .map_err(|e| format!("Failed to save chat message: {}", e))?;

    Ok(())
}

#[tauri::command]
pub async fn get_chat_history(
    notebook: String,
    note_path: Option<String>,
    mode: String,
    limit: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<ChatHistoryEntry>, String> {
    let db = state.db.lock().unwrap();
    let limit_val = limit.unwrap_or(50);

    let mut results = Vec::new();

    if let Some(path) = note_path {
        let mut stmt = db
            .prepare(
                "SELECT id, note_path, role, content, mode, created_at
                 FROM chat_history
                 WHERE notebook = ?1 AND note_path = ?2 AND mode = ?3
                 ORDER BY created_at ASC
                 LIMIT ?4",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![notebook, path, mode, limit_val], |row| {
                Ok(ChatHistoryEntry {
                    id: row.get(0)?,
                    note_path: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    mode: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;

        for row in rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
    } else {
        let mut stmt = db
            .prepare(
                "SELECT id, note_path, role, content, mode, created_at
                 FROM chat_history
                 WHERE notebook = ?1 AND note_path IS NULL AND mode = ?2
                 ORDER BY created_at ASC
                 LIMIT ?3",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![notebook, mode, limit_val], |row| {
                Ok(ChatHistoryEntry {
                    id: row.get(0)?,
                    note_path: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    mode: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
            .map_err(|e| e.to_string())?;

        for row in rows {
            results.push(row.map_err(|e| e.to_string())?);
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn clear_chat_history(
    notebook: String,
    note_path: Option<String>,
    mode: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.db.lock().unwrap();

    if let Some(path) = note_path {
        db.execute(
            "DELETE FROM chat_history WHERE notebook = ?1 AND note_path = ?2 AND mode = ?3",
            params![notebook, path, mode],
        )
        .map_err(|e| format!("Failed to clear chat history: {}", e))?;
    } else {
        db.execute(
            "DELETE FROM chat_history WHERE notebook = ?1 AND note_path IS NULL AND mode = ?2",
            params![notebook, mode],
        )
        .map_err(|e| format!("Failed to clear chat history: {}", e))?;
    }

    Ok(())
}
