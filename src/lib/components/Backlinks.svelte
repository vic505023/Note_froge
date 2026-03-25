<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { notesStore } from '../stores/notes.svelte';

  let { currentFile } = $props<{ currentFile: string | null }>();

  interface BacklinkResult {
    source_path: string;
    source_title: string;
  }

  let backlinks = $state<BacklinkResult[]>([]);
  let isExpanded = $state(false); // Скрыто по умолчанию
  let isLoading = $state(false);

  // Load backlinks when current file changes
  $effect(() => {
    if (currentFile) {
      loadBacklinks(currentFile);
    } else {
      backlinks = [];
    }
  });

  async function loadBacklinks(path: string) {
    isLoading = true;
    try {
      const results = await invoke<BacklinkResult[]>('get_backlinks', { path });
      backlinks = results;
    } catch (err) {
      console.error('Failed to load backlinks:', err);
      backlinks = [];
    } finally {
      isLoading = false;
    }
  }

  function openBacklink(path: string) {
    notesStore.openFile(path);
  }

  function toggleExpanded() {
    isExpanded = !isExpanded;
  }
</script>

{#if currentFile && backlinks.length > 0}
  <div class="backlinks-section">
    <button class="backlinks-header" onclick={toggleExpanded}>
      <svg class="expand-icon" class:expanded={isExpanded} width="12" height="12" viewBox="0 0 12 12" fill="none">
        <path d="M3 4.5L6 7.5L9 4.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      <span class="header-title">Backlinks ({backlinks.length})</span>
    </button>

    {#if isExpanded}
      <div class="backlinks-list">
        {#if isLoading}
          <div class="loading">Loading...</div>
        {:else}
          {#each backlinks as backlink}
            <button class="backlink-item" onclick={() => openBacklink(backlink.source_path)}>
              <svg class="link-icon" width="12" height="12" viewBox="0 0 12 12" fill="none">
                <path d="M4 6H8M6 4V8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
                <path d="M2 2H10V10H2V2Z" stroke="currentColor" stroke-width="1.2" fill="none"/>
              </svg>
              <span class="backlink-title">{backlink.source_title}</span>
            </button>
          {/each}
        {/if}
      </div>
    {/if}
  </div>
{/if}

<style>
  .backlinks-section {
    border-top: 1px solid var(--border-soft);
    background: var(--bg-secondary);
  }

  .backlinks-header {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 0.8125rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .backlinks-header:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .expand-icon {
    flex-shrink: 0;
    transition: transform var(--transition-fast);
  }

  .expand-icon.expanded {
    transform: rotate(0deg);
  }

  .expand-icon:not(.expanded) {
    transform: rotate(-90deg);
  }

  .header-title {
    flex: 1;
    text-align: left;
  }

  .backlinks-list {
    display: flex;
    flex-direction: column;
  }

  .backlink-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px 8px 36px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 0.875rem;
    text-align: left;
    cursor: pointer;
    transition: all var(--transition-fast);
  }

  .backlink-item:hover {
    background: var(--bg-hover);
    color: var(--accent);
  }

  .link-icon {
    flex-shrink: 0;
    opacity: 0.6;
  }

  .backlink-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .loading {
    padding: 16px;
    text-align: center;
    color: var(--text-muted);
    font-size: 0.875rem;
  }
</style>
