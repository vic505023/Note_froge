<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { notebooksStore } from '../stores/notebooks.svelte';
  import { notesStore } from '../stores/notes.svelte';
  import ChatMessage from './ChatMessage.svelte';

  interface Source {
    path: string;
    relevance: number;
  }

  let query = $state('');
  let searchScope = $state<'current' | 'all'>('current');
  let messages = $state<Array<{ role: string; content: string }>>([]);
  let isSearching = $state(false);
  let currentResponse = $state('');
  let sources = $state<Source[]>([]);

  let unlistenChunk: (() => void) | null = null;
  let unlistenDone: (() => void) | null = null;
  let unlistenError: (() => void) | null = null;
  let unlistenSources: (() => void) | null = null;

  async function setupListeners() {
    // Clean up old listeners
    if (unlistenChunk) unlistenChunk();
    if (unlistenDone) unlistenDone();
    if (unlistenError) unlistenError();
    if (unlistenSources) unlistenSources();

    unlistenChunk = await listen<{ content: string }>('rag-chunk', (event) => {
      currentResponse += event.payload.content;
    });

    unlistenDone = await listen('rag-done', () => {
      if (currentResponse) {
        messages = [...messages, { role: 'assistant', content: currentResponse }];
        currentResponse = '';
      }
      isSearching = false;
    });

    unlistenError = await listen<{ error: string }>('rag-error', (event) => {
      messages = [
        ...messages,
        { role: 'assistant', content: `Error: ${event.payload.error}` }
      ];
      currentResponse = '';
      isSearching = false;
    });

    unlistenSources = await listen<{ sources: Source[] }>('rag-sources', (event) => {
      sources = event.payload.sources;
    });
  }

  // Setup listeners on mount
  setupListeners();

  async function handleSearch() {
    if (!query.trim() || isSearching) return;

    const notebook = searchScope === 'current' ? notebooksStore.currentNotebook || '' : '';

    // Add user message
    messages = [...messages, { role: 'user', content: query }];
    currentResponse = '';
    sources = [];
    isSearching = true;

    try {
      await invoke('rag_search_notes', { query, notebook });
    } catch (err) {
      console.error('Search failed:', err);
      messages = [...messages, { role: 'assistant', content: `Error: ${err}` }];
      isSearching = false;
    }

    query = '';
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      handleSearch();
    }
  }

  function clearHistory() {
    messages = [];
    sources = [];
    currentResponse = '';
  }

  function openNote(path: string) {
    notesStore.openFile(path);
  }
</script>

<div class="search-tab">
  <div class="search-header">
    <div class="scope-selector">
      <button
        class="scope-btn"
        class:active={searchScope === 'current'}
        onclick={() => (searchScope = 'current')}
        disabled={!notebooksStore.currentNotebook}
        title={notebooksStore.currentNotebook || 'No notebook selected'}
      >
        Current notebook
      </button>
      <button
        class="scope-btn"
        class:active={searchScope === 'all'}
        onclick={() => (searchScope = 'all')}
      >
        All notebooks
      </button>
    </div>
    {#if messages.length > 0}
      <button class="clear-btn" onclick={clearHistory} title="Clear history">
        🗑
      </button>
    {/if}
  </div>

  <div class="messages-container">
    {#if messages.length === 0 && !currentResponse}
      <div class="empty-state">
        <div class="empty-icon">🔍</div>
        <div class="empty-title">Search your notes</div>
        <div class="empty-text">
          Ask questions about your notes.<br />
          AI will find and cite relevant notes.
        </div>
      </div>
    {:else}
      {#each messages as message, index (index)}
        <div class="chat-message" class:user={message.role === 'user'} class:assistant={message.role === 'assistant'}>
          <div class="message-content">
            {@html message.content.replace(/\n/g, '<br>')}
          </div>
        </div>

        {#if message.role === 'assistant' && sources.length > 0 && index === messages.length - 1}
          <div class="sources-section">
            <div class="sources-title">📎 Sources:</div>
            <div class="sources-list">
              {#each sources as source}
                <button class="source-item" onclick={() => openNote(source.path)}>
                  <span class="source-name">{source.path.split('/').pop()}</span>
                  <span class="source-relevance">{source.relevance}%</span>
                </button>
              {/each}
            </div>
          </div>
        {/if}
      {/each}

      {#if currentResponse}
        <div class="chat-message assistant streaming">
          <div class="message-content">
            {@html currentResponse.replace(/\n/g, '<br>')}
          </div>
        </div>
      {/if}

      {#if isSearching && !currentResponse}
        <div class="loading-indicator">
          <div class="loading-spinner"></div>
          <span>Searching notes...</span>
        </div>
      {/if}
    {/if}
  </div>

  <div class="search-input-container">
    <textarea
      class="search-input"
      bind:value={query}
      onkeydown={handleKeydown}
      placeholder="Ask a question about your notes..."
      rows="2"
      disabled={isSearching}
    ></textarea>
    <button
      class="search-btn"
      onclick={handleSearch}
      disabled={!query.trim() || isSearching}
      title="Search (Enter)"
    >
      {isSearching ? '⏳' : '🔍'}
    </button>
  </div>
</div>

<style>
  .search-tab {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
  }

  .search-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px;
    border-bottom: 1px solid var(--border);
    gap: 8px;
  }

  .scope-selector {
    display: flex;
    gap: 4px;
    flex: 1;
  }

  .scope-btn {
    flex: 1;
    padding: 6px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .scope-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .scope-btn.active {
    background: var(--accent);
    color: #fff;
    border-color: var(--accent);
  }

  .scope-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .clear-btn {
    padding: 6px 10px;
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
    transition: color 0.15s ease;
  }

  .clear-btn:hover {
    color: var(--text-primary);
  }

  .messages-container {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    text-align: center;
    padding: 40px 20px;
  }

  .empty-icon {
    font-size: 64px;
    margin-bottom: 16px;
    opacity: 0.5;
  }

  .empty-title {
    font-size: 18px;
    font-weight: 600;
    color: var(--text-primary);
    margin-bottom: 8px;
  }

  .empty-text {
    font-size: 14px;
    color: var(--text-muted);
    line-height: 1.6;
  }

  .sources-section {
    margin: 0;
    padding: 0;
  }

  .sources-title {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .sources-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .source-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .source-item:hover {
    background: var(--bg-hover);
    border-color: var(--accent);
  }

  .source-name {
    flex: 1;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .source-relevance {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--accent);
    margin-left: 8px;
  }

  .loading-indicator {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px;
    color: var(--text-muted);
    font-size: 13px;
  }

  .loading-spinner {
    width: 16px;
    height: 16px;
    border: 2px solid var(--text-muted);
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 0.6s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .search-input-container {
    display: flex;
    gap: 8px;
    padding: 12px;
    border-top: 1px solid var(--border);
  }

  .search-input {
    flex: 1;
    padding: 10px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    resize: none;
    transition: border-color 0.15s ease;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .search-input:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .search-btn {
    padding: 10px 16px;
    background: var(--accent);
    border: none;
    border-radius: 6px;
    color: #fff;
    font-size: 16px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .search-btn:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .search-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .chat-message {
    display: flex;
    margin-bottom: 14px;
    animation: fadeIn 0.15s ease;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .chat-message.user {
    justify-content: flex-end;
  }

  .chat-message.assistant {
    justify-content: flex-start;
  }

  .message-content {
    max-width: 85%;
    padding: 8px 12px;
    font-size: 12px;
    line-height: 1.6;
    color: var(--text-primary);
    word-wrap: break-word;
  }

  .chat-message.user .message-content {
    background: var(--bg-elevated);
    border-radius: 12px 12px 2px 12px;
  }

  .chat-message.assistant .message-content {
    background: transparent;
    border-left: 2px solid var(--accent);
    padding-left: 10px;
  }

  .message-content :global(code) {
    background: var(--bg-elevated);
    padding: 2px 4px;
    border-radius: 3px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 11px;
  }

  .message-content :global(pre) {
    background: var(--bg-elevated);
    padding: 10px;
    border-radius: 4px;
    overflow-x: auto;
    margin: 6px 0;
    font-size: 11px;
  }

  .message-content :global(pre code) {
    background: none;
    padding: 0;
  }
</style>
