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
  let searchAllNotebooks = $state(false);
  let messages = $state<Array<{ role: string; content: string }>>([]);
  let isSearching = $state(false);
  let currentResponse = $state('');
  let sources = $state<Source[]>([]);
  let textareaEl: HTMLTextAreaElement;

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

    const notebook = searchAllNotebooks ? '' : (notebooksStore.currentNotebook || '');

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

    if (textareaEl) {
      textareaEl.style.height = 'auto';
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      handleSearch();
    }
  }

  function handleInput() {
    if (textareaEl) {
      textareaEl.style.height = 'auto';
      textareaEl.style.height = Math.min(textareaEl.scrollHeight, 150) + 'px';
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

<div class="search-container">
  <div class="search-messages">
    {#if messages.length === 0 && !currentResponse}
      <div class="empty-state">
        <svg width="44" height="44" viewBox="0 0 44 44" fill="none">
          <circle cx="22" cy="22" r="20" stroke="currentColor" stroke-width="2" opacity="0.1"/>
          <circle cx="22" cy="22" r="14" stroke="currentColor" stroke-width="1.5"/>
          <path d="M29 29L35 35" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <p class="empty-title">Search Notes</p>
        <p class="empty-description">
          Ask questions about your notes.<br />
          AI will find and cite relevant passages.
        </p>
      </div>
    {:else}
      {#each messages as message, index (index)}
        <ChatMessage role={message.role} content={message.content} />

        {#if message.role === 'assistant' && sources.length > 0 && index === messages.length - 1}
          <div class="sources-section">
            <div class="sources-title">
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none" style="display: inline-block; vertical-align: middle; margin-right: 4px;">
                <path d="M7.5 4.5L10.5 1.5C11.328 0.672 12.672 0.672 13.5 1.5C14.328 2.328 14.328 3.672 13.5 4.5L10.5 7.5M7.5 4.5L4.5 7.5M7.5 4.5L8.5 5.5M4.5 7.5L1.5 10.5C0.672 11.328 0.672 12.672 1.5 13.5C2.328 14.328 3.672 14.328 4.5 13.5L7.5 10.5M4.5 7.5L5.5 8.5M8.5 5.5L5.5 8.5M8.5 5.5L10.5 7.5M5.5 8.5L7.5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              Sources used:
            </div>
            <div class="sources-list">
              {#each sources as source}
                <div class="source-item" onclick={() => openNote(source.path)}>
                  <span class="source-relevance">{source.relevance}%</span>
                  <span class="source-name" title={source.path}>
                    {source.path.split('/').pop()}
                  </span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      {/each}

      {#if currentResponse}
        <ChatMessage role="assistant" content={currentResponse} isStreaming={true} />
      {/if}
    {/if}
  </div>

  <div class="search-input-wrapper">
    <div class="unified-input-container" class:has-text={query.trim().length > 0}>
      <textarea
        bind:this={textareaEl}
        bind:value={query}
        onkeydown={handleKeydown}
        oninput={handleInput}
        placeholder="Ask about your notes..."
        disabled={isSearching}
        rows="1"
      ></textarea>

      <div class="divider-line"></div>

      <div class="bottom-controls">
        <button
          class="control-btn"
          class:active={searchAllNotebooks}
          onclick={() => searchAllNotebooks = !searchAllNotebooks}
          title={searchAllNotebooks ? "Searching all notebooks" : "Searching current notebook"}
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M2 2H12V10H10V12H2V4H2V2Z" stroke="currentColor" stroke-width="1.2" fill="none"/>
            <path d="M4 4H8V6" stroke="currentColor" stroke-width="1.2" fill="none"/>
          </svg>
        </button>

        <span class="scope-label">
          {searchAllNotebooks ? 'All notebooks' : 'Current notebook'}
        </span>

        {#if isSearching}
          <button class="send-btn stop" disabled title="Searching...">
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              <circle cx="8" cy="8" r="6" stroke="currentColor" stroke-width="2" fill="none" opacity="0.3"/>
              <circle
                cx="8"
                cy="8"
                r="6"
                stroke="currentColor"
                stroke-width="2"
                fill="none"
                stroke-dasharray="37.7"
                stroke-dashoffset="9.425"
                class="spinner"
              />
            </svg>
          </button>
        {:else}
          <button
            class="send-btn"
            onclick={handleSearch}
            disabled={!query.trim() || isSearching}
            title="Search (Enter)"
          >
            ↑
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .search-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    position: relative;
  }

  .search-messages {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  /* Scrollbar */
  .search-messages::-webkit-scrollbar {
    width: 8px;
  }

  .search-messages::-webkit-scrollbar-track {
    background: transparent;
  }

  .search-messages::-webkit-scrollbar-thumb {
    background: var(--bg-hover);
    border-radius: 4px;
  }

  .search-messages::-webkit-scrollbar-thumb:hover {
    background: var(--border);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 12px;
    color: var(--text-muted);
  }

  .empty-title {
    font-size: 0.875rem;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .empty-description {
    font-size: 0.8125rem;
    text-align: center;
    max-width: 250px;
    line-height: 1.4;
  }

  .search-input-wrapper {
    padding: 10px 12px 12px 12px;
  }

  .unified-input-container {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-radius: 10px;
    padding: 10px 14px;
    position: relative;
    overflow: visible;
    transition: border-color var(--transition-fast);
  }

  .unified-input-container:focus-within {
    border-color: var(--accent);
  }

  .unified-input-container.has-text {
    border-color: var(--accent);
  }

  textarea {
    width: 100%;
    min-height: 20px;
    max-height: 150px;
    padding: 0;
    margin-bottom: 8px;
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-family: inherit;
    font-size: 0.8125rem;
    line-height: 1.5;
    resize: none;
    overflow-y: auto;
  }

  .divider-line {
    width: calc(100% + 28px);
    height: 1px;
    background: rgba(255, 255, 255, 0.06);
    margin: 8px -14px 8px -14px;
  }

  textarea:focus {
    outline: none;
  }

  textarea:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  textarea::placeholder {
    color: rgba(255, 255, 255, 0.3);
  }

  textarea::-webkit-scrollbar {
    width: 3px;
  }

  textarea::-webkit-scrollbar-track {
    background: transparent;
  }

  textarea::-webkit-scrollbar-thumb {
    background: var(--bg-hover);
    border-radius: 2px;
  }

  .send-btn {
    width: 28px;
    height: 28px;
    background: var(--accent);
    border: none;
    border-radius: 8px;
    color: #fff;
    font-size: 16px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all var(--transition-fast);
    margin-left: auto;
    flex-shrink: 0;
  }

  .send-btn:hover:not(:disabled) {
    opacity: 0.85;
    transform: scale(0.98);
  }

  .send-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .send-btn.stop {
    background: var(--accent);
    cursor: default;
  }

  .spinner {
    animation: spin-stroke 1s linear infinite;
    transform-origin: center;
  }

  @keyframes spin-stroke {
    from {
      stroke-dashoffset: 37.7;
    }
    to {
      stroke-dashoffset: 0;
    }
  }

  /* Bottom controls */
  .bottom-controls {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding-top: 8px;
  }

  .control-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 4px;
    width: 28px;
    height: 28px;
    padding: 0;
    background: rgba(255, 255, 255, 0.05);
    border: 1px solid rgba(255, 255, 255, 0.06);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 0.6875rem;
    font-weight: 400;
    cursor: pointer;
    transition: all var(--transition-fast);
    flex-shrink: 0;
  }

  .control-btn:hover:not(:disabled) {
    background: rgba(255, 255, 255, 0.08);
  }

  .control-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .control-btn.active {
    background: var(--accent);
    border-color: transparent;
    color: var(--text-primary);
  }

  .control-btn.active:hover:not(:disabled) {
    background: var(--accent);
    opacity: 0.85;
  }

  .scope-label {
    flex: 1;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* Sources section */
  .sources-section {
    margin-top: 12px;
    padding: 12px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .sources-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .sources-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .source-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 10px;
    background: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 12px;
    transition: background 0.15s ease;
    cursor: pointer;
  }

  .source-item:hover {
    background: var(--bg-hover);
  }

  .source-name {
    flex: 1;
    min-width: 0;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .source-relevance {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--text-muted);
    font-weight: 500;
    margin-right: 8px;
  }
</style>
