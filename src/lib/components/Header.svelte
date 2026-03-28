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

  // Tab functions
  function handleTabClick(path: string) {
    notesStore.switchTab(path);
  }

  function handleCloseTab(event: MouseEvent, path: string) {
    event.stopPropagation();
    notesStore.closeTab(path);
  }

  function handleMiddleClick(event: MouseEvent, path: string) {
    if (event.button === 1) {
      event.preventDefault();
      notesStore.closeTab(path);
    }
  }

  function handleNewTab() {
    // Open empty tab
    notesStore.openEmptyTab();
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

  <!-- Tabs section -->
  <div class="header-section tabs-section">
    {#each notesStore.tabs as tab (tab.path)}
      <button
        class="tab"
        class:active={tab.path === notesStore.activeTab}
        onclick={() => handleTabClick(tab.path)}
        onmousedown={(e) => handleMiddleClick(e, tab.path)}
        title={tab.path}
      >
        <span class="tab-name">{tab.name}</span>
        {#if tab.modified}
          <span class="modified-dot"></span>
        {/if}
        <span
          class="close-btn"
          onclick={(e) => handleCloseTab(e, tab.path)}
          role="button"
          tabindex="0"
          title="Close"
        >
          <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
            <path d="M1 1L9 9M9 1L1 9" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
        </span>
      </button>
    {/each}

    <!-- New tab button -->
    {#if notesStore.tabs.length > 0}
      <button class="new-tab-btn" onclick={handleNewTab} title="New tab">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <path d="M6 1V11M1 6H11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
    {/if}
  </div>

  <!-- Editor controls -->
  {#if notesStore.currentFile}
    <div class="header-section editor-controls">
      <button
        class="mode-switch-btn"
        onclick={() => uiStore.cycleViewMode()}
        title="Switch to {uiStore.viewMode === 'edit' ? 'Preview' : 'Edit'} mode"
      >
        {uiStore.viewMode === 'edit' ? 'Edit' : 'Preview'}
      </button>
    </div>
  {/if}

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
    align-items: flex-end;
    height: 44px;
    background: var(--bg-secondary);
    flex-shrink: 0;
    position: relative;
  }

  /* Border line with gap for active tab */
  .app-header::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: var(--border);
    z-index: 0;
  }

  .header-section {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 100%;
  }

  /* Sidebar section */
  .sidebar-section {
    justify-content: space-between;
    padding: 0 14px;
    align-items: center;
  }

  .sidebar-section.open {
    width: var(--sidebar-width);
    border-right: 1px solid var(--border-subtle);
  }

  /* AI section */
  .ai-section {
    justify-content: space-between;
    align-items: center;
  }

  .ai-section.open {
    width: var(--ai-width);
    padding: 0 14px;
    border-left: 1px solid var(--border-subtle);
  }

  /* When panels are closed */
  .sidebar-section:has(.toggle-btn),
  .ai-section:has(.toggle-btn) {
    width: auto;
    min-width: 44px;
    justify-content: center;
  }

  .section-title {
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-primary);
    letter-spacing: 0;
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
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    opacity: 0.4;
    flex-shrink: 0;
  }

  .icon-btn svg {
    width: 14px;
    height: 14px;
  }

  .icon-btn {
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .icon-btn:hover {
    background: var(--bg-hover);
    opacity: 0.8;
  }

  .icon-btn:active {
    background: var(--bg-active);
    opacity: 1;
  }


  /* Mode switch button */
  .mode-switch-btn {
    padding: 4px 10px;
    font-size: 0.75rem;
    font-weight: 400;
    background: transparent;
    border: none;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    opacity: 0.5;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .mode-switch-btn:hover {
    opacity: 0.8;
  }

  .mode-switch-btn:active {
    opacity: 1;
  }

  /* Tabs section */
  .tabs-section {
    flex: 1;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
    -ms-overflow-style: none;
    padding: 0 8px;
  }

  .tabs-section::-webkit-scrollbar {
    display: none;
  }

  /* Tab styling */
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    gap: 24px;
    padding: 8px 14px;
    background: transparent;
    border: none;
    border-radius: 6px 6px 0 0;
    font-size: 13px;
    font-weight: 400;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 150ms ease;
    white-space: nowrap;
    flex-shrink: 0;
  }

  /* Separator between non-active tabs */
  .tab:not(.active)::after {
    content: '';
    position: absolute;
    right: 0;
    top: 50%;
    transform: translateY(-50%);
    height: 16px;
    width: 1px;
    background: var(--border);
    opacity: 0.4;
  }

  /* Hide separator if next tab is active or if it's the last tab */
  .tab:not(.active):last-child::after,
  .tab:not(.active):has(+ .tab.active)::after {
    display: none;
  }

  .tab:hover:not(.active) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .tab.active {
    background: var(--bg-primary);
    color: var(--text-primary);
    z-index: 1;
    position: relative;
    bottom: -1px;
    padding-bottom: 9px;
  }

  .tab-name {
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 400;
  }

  .tab.active .tab-name {
    font-weight: 500;
  }

  .modified-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-muted);
    flex-shrink: 0;
    margin-left: -4px;
    transition: background 150ms ease;
  }

  .tab.active .modified-dot {
    background: var(--accent);
  }

  .close-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 16px;
    height: 16px;
    padding: 0;
    background: none;
    border: none;
    border-radius: 3px;
    color: var(--text-secondary);
    cursor: pointer;
    opacity: 0.8;
    transition: all 150ms ease;
    flex-shrink: 0;
  }

  .tab:hover .close-btn {
    opacity: 1;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .tab.active .close-btn {
    color: var(--text-secondary);
  }

  .tab.active:hover .close-btn {
    opacity: 1;
  }

  .tab.active .close-btn:hover {
    background: var(--bg-elevated);
    color: var(--text-primary);
  }

  .new-tab-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    margin-bottom: 3px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 150ms ease;
    flex-shrink: 0;
    margin-left: 4px;
  }

  .new-tab-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  /* Editor controls */
  .editor-controls {
    padding: 0 8px;
    align-items: center;
  }
</style>
