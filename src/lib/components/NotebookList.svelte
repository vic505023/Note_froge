<script lang="ts">
  import { notebooksStore } from '../stores/notebooks.svelte';
  import { notesStore } from '../stores/notes.svelte';
  import { uiStore } from '../stores/ui.svelte';
  import { onMount } from 'svelte';
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  let contextMenu = $state<{ x: number; y: number; name: string } | null>(null);
  let renamePrompt = $state<{ name: string; value: string } | null>(null);
  let isReindexing = $state(false);

  onMount(() => {
    notebooksStore.loadNotebooks();
  });

  async function handleNotebookClick(name: string) {
    await notebooksStore.selectNotebook(name);
    uiStore.setSidebarView('files');
    uiStore.setSidebarTab('notes');
  }

  async function handleCreate() {
    const name = prompt('Notebook name:');
    if (!name) return;

    try {
      await notebooksStore.createNotebook(name);
    } catch (err) {
      // Error already handled in store
    }
  }

  function handleContextMenu(e: MouseEvent, name: string) {
    e.preventDefault();
    e.stopPropagation();
    contextMenu = { x: e.clientX, y: e.clientY, name };
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  async function handleRename(name: string) {
    closeContextMenu();
    renamePrompt = { name, value: name };
  }

  async function confirmRename() {
    if (!renamePrompt) return;

    const { name, value } = renamePrompt;
    if (!value || value === name) {
      renamePrompt = null;
      return;
    }

    try {
      await notebooksStore.renameNotebook(name, value);
      renamePrompt = null;
    } catch (err) {
      // Error already handled in store
    }
  }

  function cancelRename() {
    renamePrompt = null;
  }

  async function handleDelete(name: string) {
    closeContextMenu();

    const confirmed = await confirm(`Delete notebook "${name}"? Files will be moved to trash.`, {
      title: 'Delete Notebook',
      kind: 'warning'
    });

    if (!confirmed) return;

    try {
      await notebooksStore.deleteNotebook(name);
    } catch (err) {
      // Error already handled in store
    }
  }

  async function reindexVault() {
    if (isReindexing) return;

    isReindexing = true;
    try {
      const count = await invoke<number>('reindex_vault');
      alert(`Indexed ${count} notes successfully`);
      await notebooksStore.loadNotebooks();
    } catch (err) {
      console.error('Reindex failed:', err);
      alert('Reindex failed: ' + err);
    } finally {
      isReindexing = false;
    }
  }

  async function changeVault() {
    try {
      const newPath = await invoke<string | null>('select_vault_folder');
      if (!newPath) return;

      const confirmed = await confirm(`Change vault to:\n${newPath}\n\nCurrent files will be closed.`, {
        title: 'Change Vault',
        kind: 'warning'
      });
      if (!confirmed) return;

      notesStore.closeFile();
      notebooksStore.deselectNotebook();

      await invoke('change_vault', { newPath });

      await notebooksStore.loadNotebooks();
      uiStore.setSidebarView('notebooks');

      alert('Vault changed successfully');
    } catch (err) {
      console.error('Change vault failed:', err);
      alert('Failed to change vault: ' + err);
    }
  }
</script>

<svelte:window onclick={closeContextMenu} />

<div class="notebook-list">
  <div class="content">
    {#if notebooksStore.isLoading}
      <div class="empty-state">Loading...</div>
    {:else if notebooksStore.error}
      <div class="error">{notebooksStore.error}</div>
    {:else if notebooksStore.notebooks.length === 0}
      <div class="empty-state">
        <p>Create your first notebook</p>
        <button class="create-first-btn" onclick={handleCreate}>
          + New Notebook
        </button>
      </div>
    {:else}
      {#each notebooksStore.notebooks as notebook (notebook.name)}
        <div
          class="notebook-item"
          onclick={() => handleNotebookClick(notebook.name)}
          oncontextmenu={(e) => handleContextMenu(e, notebook.name)}
          role="button"
          tabindex="0"
          onkeydown={(e) => e.key === 'Enter' && handleNotebookClick(notebook.name)}
        >
          <div class="notebook-info">
            <div class="notebook-name">{notebook.name}</div>
            <div class="notebook-stats">
              {notebook.note_count} notes
              {#if notebook.document_count > 0}
                · {notebook.document_count} sources
              {/if}
            </div>
          </div>
        </div>
      {/each}
    {/if}
  </div>

  <div class="footer">
    <button
      class="footer-btn change-dir-btn"
      onclick={changeVault}
      title="Change vault"
    >
      change dir
    </button>
    <button
      class="footer-btn icon-btn"
      onclick={reindexVault}
      disabled={isReindexing}
      title="Reindex vault"
    >
      <svg width="16" height="16" viewBox="0 0 16 16" fill="none" class:spinning={isReindexing}>
        <path d="M13 8C13 10.7614 10.7614 13 8 13C5.23858 13 3 10.7614 3 8C3 5.23858 5.23858 3 8 3C9.38071 3 10.6193 3.52679 11.5355 4.39645" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        <path d="M11 3V5H13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>
  </div>
</div>

{#if contextMenu}
  <div
    class="context-menu"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px;"
  >
    <button onclick={() => handleRename(contextMenu.name)}>Rename</button>
    <button class="danger" onclick={() => handleDelete(contextMenu.name)}>Delete</button>
  </div>
{/if}

{#if renamePrompt}
  <div class="modal-overlay" onclick={cancelRename}>
    <div class="modal" onclick={(e) => e.stopPropagation()}>
      <h3>Rename Notebook</h3>
      <input
        type="text"
        bind:value={renamePrompt.value}
        onkeydown={(e) => {
          if (e.key === 'Enter') confirmRename();
          if (e.key === 'Escape') cancelRename();
        }}
        autofocus
      />
      <div class="modal-buttons">
        <button onclick={cancelRename}>Cancel</button>
        <button class="primary" onclick={confirmRename}>Rename</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .notebook-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-secondary);
    color: var(--text-primary);
  }

  .content {
    flex: 1;
    overflow-y: auto;
    padding: 4px;
  }

  .notebook-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 16px;
    cursor: pointer;
    border-left: 2px solid transparent;
    transition: all 0.15s ease;
  }

  .notebook-item:hover {
    background: var(--bg-hover);
    border-left-color: var(--accent);
  }

  .notebook-info {
    flex: 1;
    min-width: 0;
  }

  .notebook-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 4px;
  }

  .notebook-stats {
    font-size: 11px;
    color: var(--text-muted);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px 20px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
  }

  .empty-state p {
    margin-bottom: 16px;
  }

  .create-first-btn {
    padding: 8px 16px;
    background: var(--accent);
    color: var(--bg-primary);
    border: none;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .create-first-btn:hover {
    background: var(--accent-hover);
  }

  .footer {
    padding: 10px 12px;
    border-top: 1px solid var(--border);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .footer-btn {
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0;
    transition: color 0.15s ease;
  }

  .footer-btn.icon-btn {
    width: 28px;
    flex-shrink: 0;
  }

  .footer-btn.change-dir-btn {
    padding: 0 8px;
    font-size: 0.8125rem;
    font-weight: 500;
    flex: 1;
    text-align: center;
  }

  .footer-btn:hover:not(:disabled) {
    color: var(--text-primary);
  }

  .footer-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .spinning {
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  .error {
    padding: 12px;
    margin: 8px;
    background: rgba(247, 118, 142, 0.1);
    border: 1px solid var(--error);
    border-radius: 6px;
    color: var(--error);
    font-size: 12px;
  }

  .context-menu {
    position: fixed;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 4px;
    z-index: 1000;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  }

  .context-menu button {
    display: block;
    width: 100%;
    padding: 8px 12px;
    text-align: left;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 13px;
    cursor: pointer;
    border-radius: 4px;
    transition: background 0.15s ease;
  }

  .context-menu button:hover {
    background: var(--bg-hover);
  }

  .context-menu button.danger {
    color: var(--error);
  }

  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }

  .modal {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 20px;
    min-width: 300px;
  }

  .modal h3 {
    margin: 0 0 12px 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .modal input {
    width: 100%;
    padding: 8px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
    margin-bottom: 16px;
  }

  .modal input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .modal-buttons {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
  }

  .modal-buttons button {
    padding: 6px 12px;
    background: var(--bg-hover);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .modal-buttons button:hover {
    background: var(--bg-secondary);
  }

  .modal-buttons button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--bg-primary);
  }

  .modal-buttons button.primary:hover {
    background: var(--accent-hover);
    border-color: var(--accent-hover);
  }
</style>
