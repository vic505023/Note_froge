# NoteForge

A modern note-taking application built with Tauri 2, Svelte 5, and Rust.

## Features (Phase 1 - MVP)

✅ **Implemented:**
- File-based note storage (markdown files on disk)
- Hierarchical folder structure with recursive file tree
- CodeMirror 6 editor with markdown syntax highlighting
- Auto-save functionality (1 second debounce)
- SQLite database for metadata and indexing
- Tokyo Night dark theme
- Three-panel layout (sidebar, editor, AI panel placeholder)

🚧 **Coming in future phases:**
- Markdown preview and wiki-links (Phase 2)
- AI chat integration (Phase 3)
- RAG-based search (Phase 4)
- Multiple tabs, themes, and more (Phase 5)

## Prerequisites

- **Rust** 1.70+ (install from [rustup.rs](https://rustup.rs/))
- **Node.js** 18+ and npm
- **System dependencies** for Tauri (see [Tauri Prerequisites](https://tauri.app/v2/guides/prerequisites/))

### Linux (Arch)
```bash
sudo pacman -S webkit2gtk-4.1 base-devel curl wget file openssl appmenu-gtk-module gtk3 libappindicator-gtk3 librsvg libvips
```

## Development

### First-time setup

```bash
# Install npm dependencies
npm install

# The first run will take several minutes to compile Rust dependencies
npm run tauri dev
```

### Running the app

```bash
npm run tauri dev
```

On first launch, you'll be prompted to select a vault directory where your notes will be stored.

### Project structure

```
noteforge/
├── src-tauri/          # Rust backend (Tauri)
│   ├── src/
│   │   ├── commands/   # Tauri commands (API)
│   │   ├── db/         # SQLite migrations
│   │   └── state.rs    # Application state
├── src/                # Svelte frontend
│   ├── lib/
│   │   ├── components/ # Svelte components
│   │   ├── stores/     # Svelte 5 rune stores
│   │   └── utils/      # Utilities and helpers
│   └── routes/         # SvelteKit routes
└── CLAUDE.md           # Full development instructions
```

## Testing

A test vault has been created at `~/noteforge-test-vault/` with example notes.

To use it:
1. Launch the app
2. When prompted, select `/home/vic/noteforge-test-vault/`
3. Explore the example notes

## Building for production

```bash
npm run tauri build
```

The compiled application will be in `src-tauri/target/release/`.

## Configuration

Configuration is stored in `~/.noteforge/`:
- `config.toml` - Application settings
- `noteforge.db` - SQLite database (metadata, FTS index)

## Keyboard shortcuts

- **Ctrl+S** - Save (auto-save also active)
- **Ctrl+/** - Toggle sidebar
- **Ctrl+\\** - Toggle AI panel (coming in Phase 3)

## Architecture

```
Svelte Frontend ──invoke()──▶ Rust Tauri Commands
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
              FS (notes/)     SQLite DB      [AI API - Phase 3]
```

- **Frontend (Svelte 5):** UI components, editor integration, state management
- **Backend (Rust/Tauri):** File system operations, database queries, API integration
- **Storage:** Markdown files on disk (primary), SQLite for indexing (cache)

## Development notes

- This project uses **Tauri 2** (not v1) - check API differences in [Tauri docs](https://tauri.app/v2/)
- **Svelte 5 runes** are used throughout (no legacy `$:` reactivity)
- **Tailwind CSS 4** with new CSS-first configuration
- All file system operations go through Rust backend for security

## License

MIT

## Phase 1 Status

✅ **Completed:**
1. Project initialization with Tauri 2 + Svelte 5
2. Rust backend with SQLite database and migrations
3. Vault management (init, file tree)
4. Note CRUD operations (create, read, write, delete)
5. CodeMirror 6 integration with markdown support
6. Auto-save with debouncing
7. Three-panel layout with collapsible sidebars
8. Tokyo Night theme styling
9. Settings management

**Next:** Phase 2 - Markdown preview and wiki-links
