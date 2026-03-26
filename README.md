# NoteForge

A powerful desktop note-taking application with AI assistance, built with Tauri 2, Svelte 5, and Rust.

## Overview

NoteForge is a markdown-based note-taking app organized by **notebooks**. Each notebook is an isolated workspace with its own notes and document sources. An integrated AI assistant helps you search, chat with your sources, and edit notes.

### Key Features

- **Notebook organization** — Vault contains multiple notebooks, each with its own notes and sources
- **Live markdown rendering** — Headings, lists, bold, code render directly in the editor; markdown syntax hides when cursor moves away
- **Wiki-links** — `[[Note Name]]` links between notes with auto-completion and backlinks
- **Document sources** — Attach PDF, DOCX, PPTX files to notebooks as reference material
- **AI chat** — Ask questions about your sources, search through documents with RAG (embeddings)
- **Full-text search** — Fast FTS5 search across all notes
- **File-based** — All notes are `.md` files on disk; SQLite is just a cache
- **Dark theme** — Clean, minimal Tokyo Night inspired design

## Architecture

```
~/notes/                          ← vault
├── Physics/                      ← notebook
│   ├── Interference.md
│   └── Diffraction.md
├── Programming/                  ← notebook
│   ├── Ownership.md
│   └── Lifetimes.md
└── Personal/                     ← notebook
    └── Goals.md

~/.noteforge/
├── config.toml                   ← settings
├── noteforge.db                  ← metadata, FTS, embeddings
└── documents/                    ← centralized document storage
    ├── a1b2c3_textbook.pdf
    └── d4e5f6_lecture.docx
```

### Tech Stack

- **Backend:** Rust, Tauri 2, SQLite (with FTS5 and embeddings)
- **Frontend:** Svelte 5 (runes), Vite 6, Tailwind CSS 4
- **Editor:** CodeMirror 6 with custom live markdown plugin
- **AI:** OpenAI-compatible API (configurable endpoint)
- **Document parsing:** pdf-extract, quick-xml (DOCX/PPTX)

## Prerequisites

- **Rust** 1.70+ ([rustup.rs](https://rustup.rs/))
- **Node.js** 18+ and npm
- **System dependencies** for Tauri ([docs](https://tauri.app/v2/guides/prerequisites/))

### Linux (Arch)
```bash
sudo pacman -S webkit2gtk-4.1 base-devel curl wget file openssl gtk3 librsvg
```

### macOS
```bash
xcode-select --install
```

### Windows
Install [Microsoft Visual C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

## Getting Started

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/noteforge.git
cd noteforge

# Install dependencies
npm install

# Run in development mode (first run compiles Rust, takes a few minutes)
npm run tauri dev
```

### First Launch

1. Select a vault directory (e.g., `~/notes`)
2. Create your first notebook
3. Start writing notes in markdown

### Configuration

Settings are stored in `~/.noteforge/config.toml`:

```toml
[vault]
path = "/home/user/notes"

[ai]
base_url = "https://api.openai.com/v1"  # or compatible endpoint
api_key = "sk-..."
model = "gpt-4o"
embedding_model = "text-embedding-3-small"

[editor]
font_size = 14
font_family = "JetBrains Mono"
autosave_ms = 1000
```

## Features Guide

### Live Markdown

Markdown formatting renders directly in the editor:
- **Headings** — `## Heading` displays large, `##` hides when cursor moves away
- **Lists** — `- item` shows bullet point, `-` marker hides
- **Bold** — `**text**` renders bold, `**` markers hide
- **Code** — `` `code` `` renders monospaced with background, backticks hide
- **Horizontal lines** — `---` renders as visual divider

### Wiki-Links

Link notes with `[[Note Name]]`:
- Click to navigate
- Auto-completion while typing
- Backlinks panel shows which notes link to current note
- Works across notes in the same notebook

### Document Sources

1. Click "Sources" in sidebar
2. Upload PDF, DOCX, or PPTX files
3. Documents are indexed and embedded for AI search
4. One document can be attached to multiple notebooks

### AI Chat

**Chat mode:**
- Ask questions about your document sources
- AI retrieves relevant chunks via embeddings (RAG)
- Toggle "Web search" for unrestricted answers

**Edit mode:**
- AI edits the current note based on your instructions
- Uses both note content and sources as context
- Shows diff before applying changes

### Search

- **Full-text search** — Search note content with FTS5
- **Global search** — Search across all notebooks
- **RAG search** — Semantic search through embeddings

## Keyboard Shortcuts

- **Ctrl+S** — Save note (auto-save also enabled)
- **Ctrl+P** — Quick open (fuzzy file search)
- **Ctrl+/** — Toggle sidebar
- **Ctrl+\\** — Toggle AI panel
- **[[** — Trigger wiki-link autocomplete

## Development

### Project Structure

```
noteforge/
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── commands/       # Tauri commands (vault, notes, AI, etc.)
│   │   ├── db/             # SQLite migrations
│   │   ├── services/       # Indexer, parser, linker, watcher
│   │   └── utils/          # Chunking, embeddings, AI client
├── src/                    # Svelte frontend
│   ├── lib/
│   │   ├── components/     # UI components
│   │   ├── stores/         # Svelte 5 state (runes)
│   │   └── utils/          # Markdown, CodeMirror, diff
└── CLAUDE.md               # Full development instructions

```

### Key Conventions

- **Tauri 2 API only** — `#[tauri::command]`, `tauri::State<>`
- **Svelte 5 runes** — `$state()`, `$derived()`, `$effect()` (no legacy `$:`)
- **All FS/network in Rust** — Frontend calls backend via `invoke()`
- **File-first** — Notes are `.md` files, DB is just cache
- **Error handling** — `Result<T, String>` in Rust, `try/catch` in frontend

### Building for Production

```bash
npm run tauri build
```

Binary will be in `src-tauri/target/release/`.

## Database Schema

SQLite with WAL mode, foreign keys enabled:
- `notebooks` — Notebook metadata
- `notes` — Note metadata and file paths
- `notes_fts` — Full-text search (FTS5 virtual table)
- `links` — Wiki-links and markdown links between notes
- `documents` — Uploaded source documents
- `notebook_documents` — Many-to-many: notebooks ↔ documents
- `embeddings` — Vector embeddings for RAG search
- `chat_history` — AI conversation history per notebook

## Contributing

1. Read `CLAUDE.md` for full architectural details
2. Follow existing code style (Rust: `cargo fmt`, TypeScript: Prettier)
3. Test changes before submitting
4. Do not add features not in CLAUDE.md without discussion

## Roadmap

- [ ] Light theme
- [ ] Multiple editor tabs
- [ ] File watcher for external changes
- [ ] Drag & drop file upload
- [ ] Export to PDF/HTML
- [ ] Mobile companion app
- [ ] Plugin system

## License

MIT

---

**Built with:**
- [Tauri](https://tauri.app/) — Native desktop framework
- [Svelte](https://svelte.dev/) — Reactive UI framework
- [CodeMirror](https://codemirror.net/) — Extensible code editor
- [Tailwind CSS](https://tailwindcss.com/) — Utility-first CSS
