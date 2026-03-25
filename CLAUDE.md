# CLAUDE.md — Инструкции для разработки NoteForge

Ты разрабатываешь **NoteForge** — десктопное приложение для заметок в формате Markdown с AI-ассистентом, организованное по ноутбукам (notebooks).

## Критические правила

1. **Язык кода:** Rust (бэкенд Tauri), TypeScript (фронтенд Svelte). Комментарии в коде — на английском.
2. **Не выдумывай API.** Если не уверен в сигнатуре Tauri 2 / Svelte 5 / CodeMirror 6 — проверь документацию перед написанием.
3. **Tauri 2, не Tauri 1.** Ключевые отличия: `#[tauri::command]`, state через `tauri::State<>`, плагины через `tauri::Builder::default().plugin()`, фронтенд-invoke через `@tauri-apps/api/core`.
4. **Svelte 5 рунический синтаксис.** Используй `$state()`, `$derived()`, `$effect()`, `$props()`. НЕ используй устаревшие `$:`, `export let`, `<script context="module">`.
5. **Заметки = .md файлы на диске.** SQLite — только кеш/индекс. Потеря БД не должна означать потерю данных.
6. **Все FS/сеть/БД операции — только в Rust.** Фронтенд вызывает Rust через `invoke()`. Фронтенд НИКОГДА не обращается к FS или сети напрямую.
7. **Ошибки — Result<T, String> в Tauri commands.** На фронте — try/catch с отображением пользователю.

---

## Концепция Notebooks

Vault — это корневая папка. Внутри — ноутбуки (папки первого уровня). Каждый ноутбук содержит заметки (.md) и может ссылаться на документы-источники.

**Ноутбук** — изолированное пространство: свои заметки, свои источники, AI работает в контексте текущего ноутбука.

**Источники (documents)** — PDF, DOCX, PPTX файлы. Хранятся централизованно в `~/.noteforge/documents/`. Привязка к ноутбукам через таблицу `notebook_documents` в БД. Один документ может быть в нескольких ноутбуках. Эмбеддинги считаются один раз.

**Вложенность ноутбуков — НЕТ.** Только один уровень. Внутри ноутбука могут быть подпапки для организации заметок, но это не вложенные ноутбуки.

### Структура на диске

```
~/notes/                          ← vault
├── Физика/                       ← notebook
│   ├── Интерференция.md
│   ├── Дифракция.md
│   ├── formulas/                 ← подпапка (просто организация)
│   │   └── Формулы Максвелла.md
│   └── .trash/
├── Программирование/             ← notebook
│   ├── Ownership.md
│   └── Lifetimes.md
└── Личное/                       ← notebook без источников
    └── Цели на год.md

~/.noteforge/
├── config.toml
├── noteforge.db
└── documents/                    ← централизованное хранилище источников
    ├── a1b2c3_quantum_textbook.pdf
    ├── d4e5f6_lecture_notes.docx
    └── g7h8i9_presentation.pptx
```

### Разделение Search и Chat

**Search (вкладка)** — поиск по ЗАМЕТКАМ текущего ноутбука. FTS (полнотекстовый) + опционально RAG по заметкам. Кнопка "Search all notebooks" для глобального поиска.

**Chat (вкладка)** — AI-ассистент. Контекст: текущая заметка + источники (documents) текущего ноутбука. AI достаёт релевантные фрагменты из источников через RAG (эмбеддинги). Тогл "Web search" — AI отвечает без ограничения на локальные источники.

**Edit mode (в Chat)** — AI редактирует текущую заметку. Использует и заметку, и источники как контекст.

---

## Стек и версии

- **Tauri**: 2.x (НЕ 1.x)
- **Svelte**: 5.x (руны, НЕ legacy reactive)
- **Vite**: 6.x
- **Tailwind CSS**: 4.x (CSS-first конфиг)
- **CodeMirror**: 6.x
- **Rust edition**: 2021
- **rusqlite**: 0.31+ с features = ["bundled", "vtab"] (для FTS5)
- **reqwest**: 0.12+ с features = ["stream", "json"]
- **notify**: 6.x (file watcher)
- **tokio**: 1.x с features = ["full"]
- **pdf-extract**: 0.7 (парсинг PDF)
- **zip**: 2.x (распаковка DOCX/PPTX)
- **quick-xml**: 0.36 (парсинг XML внутри DOCX/PPTX)

---

## Архитектура

```
Svelte Frontend ──invoke()──▶ Rust Tauri Commands
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
              FS (vault/)     SQLite DB      HTTP (AI API)
                                │
                    ┌───────────┼───────────┐
                    ▼           ▼           ▼
              notes/FTS    embeddings   documents
              links        chat_history  notebook_documents
```

### config.toml

```toml
[vault]
path = "/home/user/notes"

[ai]
base_url = "https://api.ranvik.ru/v1"
api_key = "sk-..."
model = "gpt-4o"
embedding_model = "text-embedding-3-small"

[editor]
font_size = 14
font_family = "JetBrains Mono"
theme = "dark"
autosave_ms = 1000
tab_size = 4

[ui]
sidebar_width = 250
ai_panel_width = 350
ai_panel_open = true
sidebar_open = true
```

---

## Схема БД (SQLite)

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS notebooks (
    name TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL,
    note_count INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS notes (
    path TEXT PRIMARY KEY,
    notebook TEXT NOT NULL,
    title TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    size INTEGER NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS notes_fts USING fts5(
    path, notebook, title, content,
    tokenize='unicode61'
);

CREATE TABLE IF NOT EXISTS links (
    source TEXT NOT NULL,
    target TEXT NOT NULL,
    link_type TEXT NOT NULL CHECK(link_type IN ('wiki', 'markdown')),
    PRIMARY KEY (source, target)
);

CREATE TABLE IF NOT EXISTS documents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    filename TEXT NOT NULL,
    filepath TEXT NOT NULL UNIQUE,
    file_type TEXT NOT NULL CHECK(file_type IN ('pdf', 'docx', 'pptx', 'txt')),
    title TEXT NOT NULL,
    page_count INTEGER,
    size_bytes INTEGER NOT NULL,
    indexed_at INTEGER,
    created_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS notebook_documents (
    notebook TEXT NOT NULL,
    document_id INTEGER NOT NULL,
    added_at INTEGER NOT NULL,
    PRIMARY KEY (notebook, document_id),
    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS embeddings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_type TEXT NOT NULL CHECK(source_type IN ('note', 'document')),
    source_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    chunk_text TEXT NOT NULL,
    vector BLOB NOT NULL,
    updated_at INTEGER NOT NULL,
    UNIQUE(source_id, chunk_index)
);
CREATE INDEX IF NOT EXISTS idx_embeddings_source ON embeddings(source_id);
CREATE INDEX IF NOT EXISTS idx_embeddings_type ON embeddings(source_type);

CREATE TABLE IF NOT EXISTS chat_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    notebook TEXT NOT NULL,
    note_path TEXT,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    mode TEXT NOT NULL CHECK(mode IN ('chat', 'search')),
    created_at INTEGER NOT NULL
);
```

---

## Структура проекта

```
noteforge/
├── CLAUDE.md
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/default.json
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── state.rs
│       ├── commands/
│       │   ├── mod.rs
│       │   ├── vault.rs
│       │   ├── notebooks.rs
│       │   ├── notes.rs
│       │   ├── documents.rs
│       │   ├── search.rs
│       │   ├── ai.rs
│       │   └── settings.rs
│       ├── db/
│       │   ├── mod.rs
│       │   └── migrations.rs
│       ├── services/
│       │   ├── mod.rs
│       │   ├── indexer.rs
│       │   ├── doc_indexer.rs
│       │   ├── doc_parser.rs
│       │   ├── linker.rs
│       │   └── watcher.rs
│       └── utils/
│           ├── mod.rs
│           ├── chunker.rs
│           ├── embeddings.rs
│           └── ai_client.rs
├── src/
│   ├── app.html
│   ├── app.css
│   ├── lib/
│   │   ├── components/
│   │   │   ├── Layout.svelte
│   │   │   ├── Sidebar.svelte
│   │   │   ├── NotebookList.svelte
│   │   │   ├── FileTree.svelte
│   │   │   ├── SourcesList.svelte
│   │   │   ├── Editor.svelte
│   │   │   ├── Preview.svelte
│   │   │   ├── EditorPane.svelte
│   │   │   ├── TabBar.svelte
│   │   │   ├── AIPanel.svelte
│   │   │   ├── ChatTab.svelte
│   │   │   ├── SearchTab.svelte
│   │   │   ├── ChatMessage.svelte
│   │   │   ├── DiffView.svelte
│   │   │   ├── Backlinks.svelte
│   │   │   ├── QuickOpen.svelte
│   │   │   ├── SearchPanel.svelte
│   │   │   ├── IndexingStatus.svelte
│   │   │   └── Settings.svelte
│   │   ├── stores/
│   │   │   ├── notebooks.svelte.ts
│   │   │   ├── notes.svelte.ts
│   │   │   ├── documents.svelte.ts
│   │   │   ├── editor.svelte.ts
│   │   │   ├── ui.svelte.ts
│   │   │   └── ai.svelte.ts
│   │   ├── utils/
│   │   │   ├── markdown.ts
│   │   │   ├── codemirror.ts
│   │   │   ├── wikilinks-cm.ts
│   │   │   ├── fuzzy.ts
│   │   │   ├── diff.ts
│   │   │   └── tauri.ts
│   │   └── types.ts
│   └── routes/
│       └── +page.svelte
├── package.json
├── svelte.config.js
├── vite.config.ts
└── tsconfig.json
```

---

## Tauri Commands (контракт Rust ↔ Frontend)

### Vault
```rust
#[tauri::command] async fn vault_init(path: String, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn vault_get_path(state: State<'_, AppState>) -> Result<String, String>
```

### Notebooks
```rust
#[tauri::command] async fn notebook_list(state: State<'_, AppState>) -> Result<Vec<NotebookInfo>, String>
#[tauri::command] async fn notebook_create(name: String, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn notebook_delete(name: String, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn notebook_rename(old_name: String, new_name: String, state: State<'_, AppState>) -> Result<(), String>
```

### Notes
```rust
#[tauri::command] async fn note_list(notebook: String, state: State<'_, AppState>) -> Result<Vec<FileNode>, String>
#[tauri::command] async fn note_read(path: String, state: State<'_, AppState>) -> Result<String, String>
#[tauri::command] async fn note_write(path: String, content: String, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn note_create(path: String, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn note_delete(path: String, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn note_rename(old_path: String, new_path: String, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn resolve_wiki_link(target: String, notebook: String, state: State<'_, AppState>) -> Result<Option<String>, String>
#[tauri::command] async fn get_backlinks(path: String, state: State<'_, AppState>) -> Result<Vec<BacklinkResult>, String>
```

### Documents
```rust
#[tauri::command] async fn document_upload(source_path: String, notebook: String, app_handle: AppHandle, state: State<'_, AppState>) -> Result<DocumentInfo, String>
#[tauri::command] async fn document_list_for_notebook(notebook: String, state: State<'_, AppState>) -> Result<Vec<DocumentInfo>, String>
#[tauri::command] async fn document_list_all(state: State<'_, AppState>) -> Result<Vec<DocumentInfo>, String>
#[tauri::command] async fn document_delete(id: i64, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn document_add_to_notebook(document_id: i64, notebook: String, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn document_remove_from_notebook(document_id: i64, notebook: String, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn get_indexing_stats(state: State<'_, AppState>) -> Result<IndexingStats, String>
```

### Search (по заметкам)
```rust
#[tauri::command] async fn fts_search(query: String, notebook: Option<String>, state: State<'_, AppState>) -> Result<Vec<SearchResult>, String>
#[tauri::command] async fn rag_search_notes(query: String, notebook: String, app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String>
```

### AI (чат с источниками)
```rust
#[tauri::command] async fn ai_chat(messages: Vec<ChatMessage>, notebook: String, note_context: Option<String>, use_sources: bool, web_search: bool, app_handle: AppHandle, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn ai_edit_note(instruction: String, current_content: String, notebook: String, state: State<'_, AppState>) -> Result<String, String>
#[tauri::command] async fn ai_test_connection(state: State<'_, AppState>) -> Result<String, String>
#[tauri::command] async fn save_chat_message(notebook: String, note_path: Option<String>, role: String, content: String, mode: String, state: State<'_, AppState>) -> Result<(), String>
#[tauri::command] async fn get_chat_history(notebook: String, note_path: Option<String>, mode: String, limit: Option<i64>, state: State<'_, AppState>) -> Result<Vec<ChatHistoryEntry>, String>
#[tauri::command] async fn clear_chat_history(notebook: String, note_path: Option<String>, mode: String, state: State<'_, AppState>) -> Result<(), String>
```

### Settings
```rust
#[tauri::command] async fn get_settings(state: State<'_, AppState>) -> Result<AppConfig, String>
#[tauri::command] async fn update_settings(config: AppConfig, state: State<'_, AppState>) -> Result<(), String>
```

---

## Парсеры документов

### PDF → текст
- Crate: `pdf-extract`. Извлечь текст + page_count. Обработка: пароль → ошибка, скан → предупреждение.

### DOCX → текст
- ZIP-архив → `word/document.xml` → извлечь `<w:t>` через `quick-xml` → склеить `\n` между `<w:p>`.

### PPTX → текст
- ZIP-архив → `ppt/slides/slide*.xml` → извлечь `<a:t>` из `<p:sp>` → разделить `\n\n--- Slide N ---\n\n`.

### TXT → текст
- `fs::read_to_string`.

---

## Дизайн (Tokyo Night)

```css
:root {
    --bg-primary: #1a1b26;    --bg-secondary: #16161e;
    --bg-elevated: #1f2335;   --bg-hover: #292e42;
    --text-primary: #a9b1d6;  --text-secondary: #565f89;  --text-muted: #3b4261;
    --accent: #7aa2f7;        --accent-hover: #89b4fa;
    --link-wiki: #bb9af7;     --link-url: #7dcfff;
    --border: #292e42;
    --success: #9ece6a;       --warning: #e0af68;         --error: #f7768e;
}
```

Минимализм: без теней, без градиентов, border-radius max 6px, transition 200ms ease.

---

## Стриминг (Tauri events)

- `ai-chunk` { content } / `ai-done` {} / `ai-error` { error } / `ai-sources` { sources }
- `indexing-progress` { current, total, phase } / `indexing-complete` {} / `document-indexed` { id }

---

## Фазы разработки

### Фаза 1 — Скелет
Tauri 2 + Svelte 5. Vault, ноутбуки CRUD, заметки CRUD, CodeMirror, дерево файлов, автосохранение, тема.

### Фаза 2 — Markdown и ссылки
Превью, wiki-links, режимы edit/split/preview, backlinks, FTS5 поиск, Ctrl+P.

### Фаза 3 — AI-интеграция
Settings UI, AI HTTP-клиент, чат со стримингом, контекст заметки, AI-редактирование, история чата.

### Фаза 4 — Источники и RAG
Парсеры PDF/DOCX/PPTX, загрузка источников, хранилище, чанкинг, эмбеддинги, RAG в чате, Web search тогл, фоновая индексация.

### Фаза 5 — Полировка
Светлая тема, вкладки, переименование ссылок, file watcher, горячие клавиши, drag & drop.

---

## Частые ошибки — НЕ ДОПУСКАЙ

1. Tauri 1 API в Tauri 2 проекте.
2. Svelte 4 синтаксис. Только руны.
3. Прямой FS-доступ из фронта.
4. Блокирующий SQLite в async — `spawn_blocking` или Mutex.
5. Забыть WAL: `PRAGMA journal_mode = WAL;`
6. Hardcoded пути — `dirs::home_dir()`.
7. Индексация блокирует UI — всегда `tokio::spawn`.
8. Краш при отсутствии API-ключа — graceful degradation.
9. Удаление ноутбука удаляет документы — только отвязывать.
