mod commands;
mod db;
mod services;
mod state;
mod utils;

use commands::*;
use state::{AppConfig, AppState};
use std::sync::{Arc, Mutex, RwLock};
use tauri::{Emitter, Manager};

// Import services for background indexing
use services::doc_indexer;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Get or create config directory
            let config_dir = dirs::home_dir()
                .expect("Cannot find home directory")
                .join(".noteforge");
            std::fs::create_dir_all(&config_dir).expect("Failed to create config directory");

            // Create documents directory
            let documents_dir = config_dir.join("documents");
            std::fs::create_dir_all(&documents_dir).expect("Failed to create documents directory");

            // Load or create config
            let config_path = config_dir.join("config.toml");
            let config = if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)
                    .expect("Failed to read config file");
                toml::from_str(&content).unwrap_or_default()
            } else {
                AppConfig::default()
            };

            // Open or create database
            let db_path = config_dir.join("noteforge.db");
            let conn = rusqlite::Connection::open(&db_path)
                .expect("Failed to open database");

            // Run migrations
            db::run_migrations(&conn).expect("Failed to run migrations");

            // Initialize vault path from config
            let vault_path = config
                .vault
                .path
                .as_ref()
                .map(|p| std::path::PathBuf::from(p));

            // Create and manage state
            let state = AppState {
                db: Arc::new(Mutex::new(conn)),
                config: Arc::new(RwLock::new(config.clone())),
                vault_path: Arc::new(RwLock::new(vault_path.clone())),
            };

            app.manage(state.clone());

            // Start background note indexing after app starts
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Wait a bit for app to fully start
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

                // Check if vault is initialized
                let vault_path = {
                    let vault_path_guard = state.vault_path.read().unwrap();
                    vault_path_guard.clone()
                };

                if let Some(vault_path) = vault_path {
                    // Get AI config
                    let (embedding_base_url, embedding_api_key, embedding_model) = {
                        let cfg = state.config.read().unwrap();
                        let agent = cfg.get_active_agent_or_legacy();
                        (
                            agent.get_embedding_base_url(),
                            agent.get_embedding_api_key(),
                            agent.embedding_model.clone()
                        )
                    };

                    // Only index if API key is configured
                    if !embedding_api_key.is_empty() && !embedding_model.is_empty() {
                        eprintln!("Starting background note indexing...");

                        let ai_config = state::AIConfig {
                            base_url: embedding_base_url,
                            api_key: embedding_api_key,
                            model: String::new(),
                            embedding_model,
                        };

                        let state_clone = state.clone();
                        let result = tokio::task::spawn_blocking(move || {
                            let db = state_clone.db.lock().unwrap();
                            let client = reqwest::Client::new();
                            let runtime = tokio::runtime::Handle::current();
                            runtime.block_on(doc_indexer::index_all_notes(
                                &*db,
                                &client,
                                &ai_config,
                                &vault_path
                            ))
                        }).await;

                        match result {
                            Ok(Ok(count)) => {
                                eprintln!("Background indexing complete: {} notes indexed", count);
                                let _ = app_handle.emit("notes-indexed", serde_json::json!({ "count": count }));
                            }
                            Ok(Err(e)) => eprintln!("Background indexing failed: {}", e),
                            Err(e) => eprintln!("Background indexing task panicked: {}", e),
                        }
                    } else {
                        eprintln!("Skipping background note indexing: embedding API not configured");
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            vault_init,
            vault_get_path,
            reindex_vault,
            select_vault_folder,
            change_vault,
            notebook_list,
            notebook_create,
            notebook_delete,
            notebook_rename,
            note_list,
            note_exists,
            note_read,
            note_write,
            note_create,
            note_delete,
            copy_file,
            copy_folder,
            move_file,
            move_folder,
            rename_item,
            resolve_wiki_link,
            get_backlinks,
            open_external_url,
            get_settings,
            update_settings,
            ai_chat,
            ai_test_connection,
            ai_edit_note,
            save_chat_message,
            get_chat_history,
            clear_chat_history,
            document_upload,
            document_list_for_notebook,
            document_list_all,
            document_delete,
            document_add_to_notebook,
            document_remove_from_notebook,
            get_indexing_stats,
            index_all_notes_embeddings,
            fts_search,
            rag_search_notes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
