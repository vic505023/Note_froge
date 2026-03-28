<script lang="ts">
  import { notesStore } from '../stores/notes.svelte';
  import { notebooksStore } from '../stores/notebooks.svelte';

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
    if (notebooksStore.currentNotebook) {
      const event = new CustomEvent('new-note');
      window.dispatchEvent(event);
    }
  }
</script>

<div class="tab-bar">
    <div class="tabs-container">
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

    <!-- New tab button - only show when there are open tabs -->
    {#if notesStore.tabs.length > 0}
      <button class="new-tab-btn" onclick={handleNewTab} title="New tab">
        <svg width="12" height="12" viewBox="0 0 12 12" fill="none">
          <path d="M6 1V11M1 6H11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      </button>
    {/if}
</div>

<style>
  .tab-bar {
    display: flex;
    align-items: flex-end;
    height: 36px;
    background: var(--bg-secondary);
    padding: 0 8px 0 16px;
    flex-shrink: 0;
    position: relative;
  }

  /* Border line with gap for active tab */
  .tab-bar::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 1px;
    background: var(--border);
    z-index: 0;
  }

  .tabs-container {
    display: flex;
    align-items: flex-end;
    gap: 2px;
    flex: 1;
    overflow-x: scroll !important;
    overflow-y: hidden;
    height: 100%;
    flex-wrap: nowrap;
  }

  .tabs-container::-webkit-scrollbar {
    height: 4px;
  }

  .tabs-container::-webkit-scrollbar-track {
    background: transparent;
  }

  .tabs-container::-webkit-scrollbar-thumb {
    background: var(--border);
    border-radius: 2px;
  }

  .tabs-container::-webkit-scrollbar-thumb:hover {
    background: var(--text-muted);
  }

  /* Tab styling - Obsidian style */
  .tab {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    background: transparent;
    border: none;
    border-radius: 6px 6px 0 0;
    font-size: 13px;
    font-weight: 400;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 150ms ease;
    white-space: nowrap;
    flex-shrink: 0;
    min-width: fit-content;
  }

  .tab:hover:not(.active) {
    background: var(--bg-hover);
    color: var(--text-secondary);
  }

  /* Active tab - extends down to cover border */
  .tab.active {
    background: var(--bg-primary);
    color: var(--text-primary);
    z-index: 1;
    position: relative;
    bottom: -1px;
    padding-bottom: 9px;
  }

  .tab-name {
    width: 140px;
    min-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    font-weight: 400;
  }

  .tab.active .tab-name {
    font-weight: 500;
  }

  /* Modified indicator */
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

  /* Close button */
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
    color: var(--text-muted);
    cursor: pointer;
    opacity: 0;
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

  /* New tab button */
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
    color: var(--text-muted);
    cursor: pointer;
    transition: all 150ms ease;
    flex-shrink: 0;
    margin-left: 4px;
  }

  .new-tab-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

</style>
