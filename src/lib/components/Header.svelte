<script lang="ts">
  import { uiStore } from '../stores/ui.svelte';
  import { notesStore } from '../stores/notes.svelte';
  import { notebooksStore } from '../stores/notebooks.svelte';
  import InputModal from './InputModal.svelte';

  interface Props {
    sidebarWidth: number;
    aiPanelWidth: number;
  }

  let { sidebarWidth, aiPanelWidth }: Props = $props();

  let showNotebookModal = $state(false);
  let showNoteModal = $state(false);

  async function handleCreateNotebook() {
    showNotebookModal = true;
  }

  async function handleCreateNote() {
    showNoteModal = true;
  }

  async function createNotebook(name: string) {
    try {
      await notebooksStore.createNotebook(name);
    } catch (err) {
      console.error('Failed to create notebook:', err);
      alert('Failed to create notebook: ' + err);
    }
  }

  async function createNote(name: string) {
    const currentNotebook = notebooksStore.currentNotebook;
    if (!currentNotebook) return;

    const fileName = name.endsWith('.md') ? name : `${name}.md`;
    const path = `${currentNotebook}/${fileName}`;

    try {
      await notesStore.createFile(path);
      // Reload file tree after creation
      await notesStore.loadFiles(currentNotebook);
    } catch (err) {
      console.error('Failed to create note:', err);
      alert('Failed to create note: ' + err);
    }
  }

  function handleBack() {
    notebooksStore.deselectNotebook();
    uiStore.setSidebarView('notebooks');
  }
</script>

<header class="app-header" style="--sidebar-width: {sidebarWidth}px; --ai-width: {aiPanelWidth}px">
  <!-- Sidebar section -->
  <div class="header-section sidebar-section" class:open={uiStore.sidebarOpen}>
    {#if uiStore.sidebarOpen}
      {#if uiStore.sidebarView === 'files'}
        <button class="icon-btn" onclick={handleCreateNote} title="New note">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M8 3V13M3 8H13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      {:else}
        <button class="icon-btn" onclick={handleCreateNotebook} title="New notebook">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M8 3V13M3 8H13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </button>
      {/if}
      <div class="spacer"></div>
      <h1 class="section-title">{uiStore.sidebarView === 'notebooks' ? 'Notebooks' : notebooksStore.currentNotebook || 'Notes'}</h1>
      <div class="spacer"></div>
      {#if uiStore.sidebarView === 'files'}
        <button class="icon-btn" onclick={handleBack} title="Back to notebooks">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M10 4L6 8L10 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      {:else}
        <button class="icon-btn" onclick={() => uiStore.toggleSidebar()} title="Close sidebar">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <path d="M10 4L6 8L10 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </button>
      {/if}
    {:else}
      <button class="icon-btn toggle-btn" onclick={() => uiStore.toggleSidebar()} title="Open sidebar">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M6 4L10 8L6 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    {/if}
  </div>

  <!-- Editor section -->
  <div class="header-section editor-section">
    {#if notesStore.currentFile}
      <div class="editor-left"></div>
      <div class="current-file">
        <svg class="file-icon" width="14" height="14" viewBox="0 0 14 14" fill="none">
          <rect x="2" y="2" width="10" height="10" rx="1.5" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <path d="M5 6H9M5 8H7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>
        <span class="file-name">{notesStore.currentFile.split('/').pop()}</span>
      </div>
      <div class="editor-right">
        <button
          class="mode-switch-btn"
          onclick={() => uiStore.cycleViewMode()}
          title="Switch to {uiStore.viewMode === 'edit' ? 'Preview' : 'Edit'} mode"
        >
          {uiStore.viewMode === 'edit' ? 'Preview' : 'Edit'}
        </button>
      </div>
    {/if}
  </div>

  <!-- AI Panel section -->
  <div class="header-section ai-section" class:open={uiStore.aiPanelOpen}>
    {#if uiStore.aiPanelOpen}
      <button class="icon-btn" onclick={() => uiStore.toggleAIPanel()} title="Close panel">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M6 4L10 8L6 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
      <div class="spacer"></div>
      <h1 class="section-title">AI Assistant</h1>
      <div class="spacer"></div>
    {:else}
      <button class="icon-btn toggle-btn" onclick={() => uiStore.toggleAIPanel()} title="Open AI panel">
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M10 4L6 8L10 12" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
      </button>
    {/if}
  </div>
</header>

<InputModal
  bind:isOpen={showNotebookModal}
  title="Create Notebook"
  placeholder="Notebook name"
  onSubmit={createNotebook}
/>

<InputModal
  bind:isOpen={showNoteModal}
  title="Create Note"
  placeholder="Note name"
  onSubmit={createNote}
/>

<style>
  .app-header {
    display: flex;
    height: 48px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .header-section {
    display: flex;
    align-items: center;
    padding: 0 16px;
    gap: 8px;
  }

  /* Sidebar section */
  .sidebar-section {
    justify-content: space-between;
    border-right: 1px solid var(--border);
  }

  .sidebar-section.open {
    width: var(--sidebar-width);
  }

  /* Editor section */
  .editor-section {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
  }

  .editor-left {
    /* Пустая колонка слева для баланса */
  }

  .editor-right {
    display: flex;
    justify-content: flex-end;
    padding-right: 0;
  }

  /* AI section */
  .ai-section {
    justify-content: space-between;
    border-left: 1px solid var(--border);
  }

  .ai-section.open {
    width: var(--ai-width);
  }

  /* When panels are closed */
  .sidebar-section:has(.toggle-btn),
  .ai-section:has(.toggle-btn) {
    width: auto;
    min-width: 48px;
    justify-content: center;
  }

  .section-title {
    font-size: 0.8125rem;
    font-weight: 600;
    color: var(--text-primary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .section-actions {
    display: flex;
    gap: 4px;
  }

  /* Spacer для балансировки */
  .spacer {
    flex: 1;
  }

  .icon-btn,
  .icon-btn-placeholder {
    width: 28px;
    height: 28px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-secondary);
    flex-shrink: 0;
  }

  .icon-btn {
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .icon-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .icon-btn:active {
    background: var(--bg-hover);
    color: var(--accent);
  }

  /* Current file display */
  .current-file {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--text-primary);
    font-size: 0.875rem;
    font-weight: 500;
  }

  .file-icon {
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Mode switch button */
  .mode-switch-btn {
    padding: 4px 12px;
    font-size: 0.8125rem;
    font-weight: 500;
    background: transparent;
    border: none;
    border-radius: 6px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  .mode-switch-btn:hover {
    color: var(--text-primary);
  }

  .mode-switch-btn:active {
    color: var(--accent);
  }
</style>
