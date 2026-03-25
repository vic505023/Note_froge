#!/bin/bash

# Fix Sidebar.svelte
cat > src/lib/components/Sidebar.svelte << 'EOF'
<script lang="ts">
  import FileTree from './FileTree.svelte';
  import NotebookList from './NotebookList.svelte';
  import { notesStore } from '../stores/notes.svelte';
  import { notebooksStore } from '../stores/notebooks.svelte';
  import { uiStore } from '../stores/ui.svelte';
  import { invoke } from '@tauri-apps/api/core';

  let isReindexing = $state(false);

  function setTab(tab: 'notes' | 'sources') {
    uiStore.setSidebarTab(tab);
  }

  async function reindexVault() {
    if (isReindexing) return;

    isReindexing = true;
    try {
      const count = await invoke<number>('reindex_vault');
      alert(\`Indexed \${count} notes successfully\`);
      // Reload notebooks and files
      await notebooksStore.loadNotebooks();
      if (notebooksStore.currentNotebook) {
        await notesStore.loadFiles(notebooksStore.currentNotebook);
      }
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

      const confirmed = confirm(\`Change vault to:\n\${newPath}\n\nCurrent files will be closed.\`);
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

<div class="sidebar-container">
  {#if uiStore.sidebarView === 'notebooks'}
    <!-- Notebooks view -->
    <NotebookList />
  {:else if uiStore.sidebarView === 'files' && notebooksStore.currentNotebook}
    <!-- Files view with tabs -->
    <div class="tabs">
      <button
        class="tab"
        class:active={uiStore.sidebarTab === 'notes'}
        onclick={() => setTab('notes')}
      >
        Notes
      </button>
      <button
        class="tab"
        class:active={uiStore.sidebarTab === 'sources'}
        onclick={() => setTab('sources')}
      >
        Sources
      </button>
    </div>

    <div class="tab-content">
      {#if uiStore.sidebarTab === 'notes'}
        <div class="file-list">
          <FileTree nodes={notesStore.files} />
        </div>
      {:else}
        <div class="sources-placeholder">
          <div class="upload-box">
            <button class="upload-btn" disabled title="Available in Phase 4">
              + Upload source
            </button>
          </div>
          <div class="placeholder-text">
            Sources will appear here<br/>after Phase 4
          </div>
        </div>
      {/if}
    </div>

    <div class="sidebar-footer">
      <button
        class="footer-btn"
        onclick={changeVault}
        title="Change vault"
      >
        <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
          <path d="M2 5L2 13L14 13L14 5" stroke="currentColor" stroke-width="1.2" fill="none"/>
          <path d="M1 5L3 3L7 3L8 5L15 5" stroke="currentColor" stroke-width="1.2" fill="none"/>
        </svg>
      </button>
      <button
        class="footer-btn"
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
  {/if}
</div>

<style>
  .sidebar-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-secondary);
  }

  .tabs {
    display: flex;
    width: 100%;
    border-bottom: 1px solid var(--border);
  }

  .tab {
    flex: 1;
    padding: 10px 0;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 400;
    text-align: center;
    cursor: pointer;
    position: relative;
    transition: all 0.15s ease;
  }

  .tab:hover {
    color: var(--text-primary);
  }

  .tab.active {
    color: var(--text-primary);
    font-weight: 600;
  }

  .tab.active::after {
    content: '';
    position: absolute;
    bottom: -1px;
    left: 0;
    right: 0;
    height: 2px;
    background: var(--accent);
  }

  .tab-content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px 8px 8px 16px;
  }

  .sources-placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 20px;
    gap: 16px;
  }

  .upload-box {
    width: 100%;
    max-width: 200px;
  }

  .upload-btn {
    width: 100%;
    padding: 10px;
    background: transparent;
    border: 1px dashed var(--text-muted);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 12px;
    cursor: not-allowed;
    transition: all 0.15s ease;
  }

  .placeholder-text {
    text-align: center;
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.6;
    margin-top: 20px;
  }

  .sidebar-footer {
    display: flex;
    justify-content: flex-end;
    gap: 4px;
    padding: 10px 12px;
    border-top: 1px solid var(--border);
  }

  .footer-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    padding: 0;
    background: transparent;
    border: none;
    color: var(--text-muted);
    cursor: pointer;
    transition: color 0.15s ease;
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
</style>
EOF

echo "Fixed Sidebar.svelte"
