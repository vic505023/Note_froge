<script lang="ts">
  import { fuzzySearch, type FuzzyMatch } from '../utils/fuzzy';
  import { notesStore } from '../stores/notes.svelte';
  import type { FileNode } from '../types';

  let { isOpen = $bindable(false) } = $props<{ isOpen: boolean }>();

  let query = $state('');
  let selectedIndex = $state(0);
  let inputElement = $state<HTMLInputElement | null>(null);

  // Flatten file tree to get all files
  function flattenFiles(nodes: FileNode[]): FileNode[] {
    const files: FileNode[] = [];
    for (const node of nodes) {
      if (!node.is_dir) {
        files.push(node);
      }
      if (node.children) {
        files.push(...flattenFiles(node.children));
      }
    }
    return files;
  }

  // Get all files from notes store
  const allFiles = $derived(flattenFiles(notesStore.files));

  // Fuzzy search results
  const searchResults = $derived.by(() => {
    if (!query.trim()) {
      return allFiles.slice(0, 15).map(file => ({ item: file, match: { score: 0, indices: [] } }));
    }
    return fuzzySearch(query, allFiles, file => file.name, 15);
  });

  // Reset state when opened
  $effect(() => {
    if (isOpen) {
      query = '';
      selectedIndex = 0;
      setTimeout(() => inputElement?.focus(), 50);
    }
  });

  // Update selected index when results change
  $effect(() => {
    if (selectedIndex >= searchResults.length) {
      selectedIndex = Math.max(0, searchResults.length - 1);
    }
  });

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      isOpen = false;
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, searchResults.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      if (searchResults[selectedIndex]) {
        selectFile(searchResults[selectedIndex].item);
      }
    }
  }

  function selectFile(file: FileNode) {
    notesStore.openFile(file.path);
    isOpen = false;
  }

  function highlightMatch(text: string, indices: number[]): string {
    if (indices.length === 0) return text;

    const chars = text.split('');
    const indicesSet = new Set(indices);

    return chars
      .map((char, i) => {
        if (indicesSet.has(i)) {
          return `<mark>${char}</mark>`;
        }
        return char;
      })
      .join('');
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      isOpen = false;
    }
  }
</script>

{#if isOpen}
  <div class="quick-open-backdrop" onclick={handleBackdropClick} role="dialog" aria-modal="true">
    <div class="quick-open-modal">
      <input
        bind:this={inputElement}
        bind:value={query}
        type="text"
        class="search-input"
        placeholder="Search files..."
        onkeydown={handleKeyDown}
      />

      <div class="results-list">
        {#each searchResults as { item, match }, i}
          <button
            class="result-item"
            class:selected={i === selectedIndex}
            onclick={() => selectFile(item)}
            onmouseenter={() => selectedIndex = i}
          >
            <svg class="file-icon" width="14" height="14" viewBox="0 0 14 14" fill="none">
              <rect x="2" y="2" width="10" height="10" rx="1.5" stroke="currentColor" stroke-width="1.2" fill="none"/>
              <path d="M5 6H9M5 8H7" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
            </svg>
            <span class="file-name">{@html highlightMatch(item.name, match.indices)}</span>
            <span class="file-path">{item.path}</span>
          </button>
        {:else}
          <div class="no-results">No files found</div>
        {/each}
      </div>

      <div class="quick-open-footer">
        <span class="hint">↑↓ Navigate</span>
        <span class="hint">↵ Open</span>
        <span class="hint">Esc Close</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .quick-open-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 15vh;
    z-index: 1000;
  }

  .quick-open-modal {
    width: 500px;
    max-width: 90vw;
    background: rgba(26, 27, 38, 0.7);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    box-shadow: 0 12px 32px rgba(0, 0, 0, 0.4);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .search-input {
    width: 100%;
    padding: 16px;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--text-primary);
    font-size: 1rem;
    outline: none;
  }

  .search-input::placeholder {
    color: var(--text-muted);
  }

  .results-list {
    max-height: 400px;
    overflow-y: auto;
  }

  .result-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 16px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    transition: background var(--transition-fast);
    border-bottom: 1px solid transparent;
  }

  .result-item:hover,
  .result-item.selected {
    background: var(--bg-hover);
  }

  .result-item.selected {
    border-bottom-color: var(--accent);
  }

  .file-icon {
    flex-shrink: 0;
    color: var(--text-muted);
  }

  .file-name {
    flex-shrink: 0;
    font-weight: 500;
  }

  .file-name :global(mark) {
    background: transparent;
    color: var(--accent);
    font-weight: 600;
  }

  .file-path {
    flex: 1;
    font-size: 0.8125rem;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin-left: auto;
    padding-left: 16px;
  }

  .no-results {
    padding: 32px 16px;
    text-align: center;
    color: var(--text-muted);
  }

  .quick-open-footer {
    display: flex;
    gap: 16px;
    padding: 10px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-secondary);
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .hint {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  /* Scrollbar */
  .results-list::-webkit-scrollbar {
    width: 6px;
  }

  .results-list::-webkit-scrollbar-track {
    background: transparent;
  }

  .results-list::-webkit-scrollbar-thumb {
    background: var(--scrollbar-thumb);
    border-radius: 3px;
  }

  .results-list::-webkit-scrollbar-thumb:hover {
    background: var(--text-muted);
  }
</style>
