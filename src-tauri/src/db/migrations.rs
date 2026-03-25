use rusqlite::{Connection, Result};

pub fn run_migrations(conn: &Connection) -> Result<()> {
    // Enable WAL mode for better concurrency
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;

    let user_version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if user_version < 1 {
        create_initial_schema(conn)?;
        conn.pragma_update(None, "user_version", 1)?;
    }

    if user_version < 2 {
        migrate_to_v2(conn)?;
        conn.pragma_update(None, "user_version", 2)?;
    }

    if user_version < 3 {
        migrate_to_v3(conn)?;
        conn.pragma_update(None, "user_version", 3)?;
    }

    Ok(())
}

fn create_initial_schema(conn: &Connection) -> Result<()> {
    // Notes metadata table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS notes (
            path TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            modified_at INTEGER NOT NULL,
            size INTEGER NOT NULL
        )",
        [],
    )?;

    // Full-text search index
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
            path, title, content,
            tokenize='unicode61'
        )",
        [],
    )?;

    // Links between notes
    conn.execute(
        "CREATE TABLE IF NOT EXISTS links (
            source TEXT NOT NULL,
            target TEXT NOT NULL,
            link_type TEXT NOT NULL CHECK(link_type IN ('wiki', 'markdown')),
            PRIMARY KEY (source, target)
        )",
        [],
    )?;

    // Embeddings for RAG
    conn.execute(
        "CREATE TABLE IF NOT EXISTS embeddings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            note_path TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            chunk_text TEXT NOT NULL,
            vector BLOB NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(note_path, chunk_index)
        )",
        [],
    )?;

    // Chat history
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            note_path TEXT,
            role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
            content TEXT NOT NULL,
            mode TEXT NOT NULL CHECK(mode IN ('chat', 'search')),
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    Ok(())
}

fn migrate_to_v2(conn: &Connection) -> Result<()> {
    // Create notebooks table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS notebooks (
            name TEXT PRIMARY KEY,
            created_at INTEGER NOT NULL,
            note_count INTEGER DEFAULT 0
        )",
        [],
    )?;

    // Create documents table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS documents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            filename TEXT NOT NULL,
            filepath TEXT NOT NULL UNIQUE,
            file_type TEXT NOT NULL CHECK(file_type IN ('pdf', 'docx', 'pptx', 'txt')),
            title TEXT NOT NULL,
            page_count INTEGER,
            size_bytes INTEGER NOT NULL,
            indexed_at INTEGER,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    // Create notebook_documents junction table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS notebook_documents (
            notebook TEXT NOT NULL,
            document_id INTEGER NOT NULL,
            added_at INTEGER NOT NULL,
            PRIMARY KEY (notebook, document_id),
            FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Add notebook column to notes table
    conn.execute("ALTER TABLE notes ADD COLUMN notebook TEXT NOT NULL DEFAULT ''", [])?;

    // Add notebook column to chat_history table
    conn.execute("ALTER TABLE chat_history ADD COLUMN notebook TEXT NOT NULL DEFAULT ''", [])?;

    // Drop and recreate FTS table with notebook column
    conn.execute("DROP TABLE IF EXISTS notes_fts", [])?;
    conn.execute(
        "CREATE VIRTUAL TABLE notes_fts USING fts5(
            path, notebook, title, content,
            tokenize='unicode61'
        )",
        [],
    )?;

    // Recreate embeddings table with source_type and source_id
    conn.execute("DROP TABLE IF EXISTS embeddings", [])?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS embeddings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_type TEXT NOT NULL CHECK(source_type IN ('note', 'document')),
            source_id TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            chunk_text TEXT NOT NULL,
            vector BLOB NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE(source_id, chunk_index)
        )",
        [],
    )?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_embeddings_source ON embeddings(source_id)", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_embeddings_type ON embeddings(source_type)", [])?;

    // Migrate existing notes data - extract notebook from path
    // Path format: "Notebook/subfolder/note.md" -> notebook = "Notebook"
    // If no slash, set notebook to "Default"
    conn.execute(
        "UPDATE notes
         SET notebook = CASE
             WHEN instr(path, '/') > 0 THEN substr(path, 1, instr(path, '/') - 1)
             ELSE 'Default'
         END",
        [],
    )?;

    // Migrate chat_history - use notebook from associated note or Default
    conn.execute(
        "UPDATE chat_history
         SET notebook = COALESCE(
             (SELECT notebook FROM notes WHERE notes.path = chat_history.note_path LIMIT 1),
             'Default'
         )",
        [],
    )?;

    Ok(())
}

fn migrate_to_v3(conn: &Connection) -> Result<()> {
    // Add page column to embeddings table to track which page/slide a chunk came from
    conn.execute("ALTER TABLE embeddings ADD COLUMN page INTEGER", [])?;
    eprintln!("Migration v3: Added page column to embeddings table");
    Ok(())
}
