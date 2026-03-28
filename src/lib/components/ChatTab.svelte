<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { aiStore } from '../stores/ai.svelte';
  import { notesStore } from '../stores/notes.svelte';
  import { settingsStore } from '../stores/settings.svelte';
  import ChatMessage from './ChatMessage.svelte';
  import DiffView from './DiffView.svelte';
  import ConfirmModal from './ConfirmModal.svelte';
  import { noteWrite } from '../utils/tauri';

  let inputText = $state('');
  let chatContainer: HTMLDivElement;
  let textareaEl: HTMLTextAreaElement;
  let diffViewEl: HTMLDivElement | null = null;

  let proposedEdit = $state<{ oldContent: string; newContent: string } | null>(null);
  let showClearHistoryModal = $state(false);
  let selectedModel = $state('');
  let showModelDropdown = $state(false);
  let useSources = $state(false);
  let webSearch = $state(false);

  // Watch proposedEdit changes and scroll to it
  $effect(() => {
    console.log('proposedEdit changed:', proposedEdit ? 'SET' : 'NULL', proposedEdit);
    if (proposedEdit && diffViewEl) {
      setTimeout(() => {
        diffViewEl?.scrollIntoView({ behavior: 'smooth', block: 'end' });
      }, 100);
    }
  });

  // Get models from active agent
  const availableModels = $derived(() => {
    const activeAgent = settingsStore.config?.agents.find(
      a => a.id === settingsStore.config?.active_agent_id
    );
    return activeAgent?.models || [];
  });

  // Restore selected model from settings or use first available
  $effect(() => {
    const models = availableModels();
    if (models.length > 0) {
      const savedModel = settingsStore.config?.ui.selected_model;
      if (savedModel && models.includes(savedModel)) {
        selectedModel = savedModel;
      } else if (!models.includes(selectedModel)) {
        selectedModel = models[0];
      }
    }
  });

  function toggleModelDropdown() {
    showModelDropdown = !showModelDropdown;
  }

  async function selectModel(model: string) {
    selectedModel = model;
    showModelDropdown = false;

    // Save selected model to settings
    if (settingsStore.config) {
      try {
        const updatedConfig = {
          ...settingsStore.config,
          ui: {
            ...settingsStore.config.ui,
            selected_model: model
          }
        };
        await settingsStore.saveSettings(updatedConfig);
      } catch (err) {
        console.error('Failed to save selected model:', err);
      }
    }
  }

  // Auto-scroll to bottom on new messages or proposed edits
  $effect(() => {
    if (chatContainer && (aiStore.messages.length > 0 || aiStore.currentStreamContent || proposedEdit)) {
      setTimeout(() => {
        chatContainer.scrollTop = chatContainer.scrollHeight;
      }, 0);
    }
  });

  // Check for <noteforge-edit> tags in assistant responses
  $effect(() => {
    if (aiStore.currentStreamContent) {
      const editMatch = aiStore.currentStreamContent.match(/<noteforge-edit>([\s\S]*?)<\/noteforge-edit>/);
      if (editMatch) {
        const newContent = editMatch[1].trim();
        proposedEdit = {
          oldContent: notesStore.currentContent,
          newContent
        };
      }
    }
  });

  onMount(async () => {
    // aiStore.init() is called in Layout.svelte, no need to call again
    if (notesStore.currentFile) {
      await aiStore.loadHistory(notesStore.currentFile);
    }
  });

  // Reload history when note changes
  $effect(() => {
    if (notesStore.currentFile) {
      aiStore.loadHistory(notesStore.currentFile);
    }
  });

  async function handleSend() {
    console.log('handleSend called, editMode:', aiStore.editMode);
    if (!inputText.trim() || aiStore.isEditingNote) return;
    if (!notesStore.currentFile || !selectedModel || aiStore.isStreaming) return;

    const content = inputText.trim();
    inputText = '';

    console.log('About to send, editMode:', aiStore.editMode, 'content:', content.substring(0, 50));

    try {
      if (aiStore.editMode) {
        console.log('EDIT MODE - calling aiStore.editNote');
        console.log('Current note content length:', notesStore.currentContent.length);
        const newContent = await aiStore.editNote(
          content,
          notesStore.currentContent,
          notesStore.currentFile,
          selectedModel || undefined,
          false  // Never use sources in edit mode - only chat history + current note
        );

        console.log('AI edit response length:', newContent.length);
        console.log('AI edit response (first 200 chars):', newContent.substring(0, 200));
        console.log('Old content length:', notesStore.currentContent.length);

        proposedEdit = {
          oldContent: notesStore.currentContent,
          newContent
        };

        console.log('proposedEdit set:', proposedEdit);
      } else {
        await aiStore.sendMessage(
          content,
          notesStore.currentContent,
          notesStore.currentFile,
          selectedModel || undefined,
          useSources,
          webSearch
        );
      }
    } catch (err) {
      console.error('Failed to send message:', err);
    }

    if (textareaEl) {
      textareaEl.style.height = 'auto';
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  function handleInput() {
    if (textareaEl) {
      textareaEl.style.height = 'auto';
      textareaEl.style.height = Math.min(textareaEl.scrollHeight, 150) + 'px';
    }
  }

  async function applyEdit() {
    if (!proposedEdit || !notesStore.currentFile) return;

    try {
      await noteWrite(notesStore.currentFile, proposedEdit.newContent);
      notesStore.applyAIEdit(proposedEdit.newContent);
      await aiStore.confirmEdit(notesStore.currentFile);
      proposedEdit = null;
    } catch (err) {
      console.error('Failed to apply edit:', err);
    }
  }

  async function rejectEdit() {
    if (!notesStore.currentFile) return;

    try {
      await aiStore.rejectEdit(notesStore.currentFile);
      proposedEdit = null;
    } catch (err) {
      console.error('Failed to reject edit:', err);
    }
  }

  async function handleClear() {
    if (!notesStore.currentFile) return;
    showClearHistoryModal = true;
  }

  async function confirmClearHistory() {
    if (!notesStore.currentFile) return;

    try {
      await aiStore.clearHistory(notesStore.currentFile);
    } catch (err) {
      console.error('Failed to clear chat history:', err);
      alert('Failed to clear chat history: ' + err);
    }
  }

  function handleStop() {
    aiStore.stopStreaming();
  }

  async function handleOpenDocument(filepath: string) {
    try {
      await invoke('open_document', { filepath });
    } catch (err) {
      console.error('Failed to open document:', err);
      alert(`Failed to open document: ${err}`);
    }
  }
</script>

<div class="chat-container">
  <div class="chat-header">
    <button
      class="clear-chat-btn"
      onclick={handleClear}
      title="Clear chat history"
      disabled={aiStore.messages.length === 0 && !aiStore.currentStreamContent}
    >
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none">
        <path d="M3 4H13M5 4V3C5 2.448 5.448 2 6 2H10C10.552 2 11 2.448 11 3V4M6 7V11M10 7V11M4 4H12V13C12 13.552 11.552 14 11 14H5C4.448 14 4 13.552 4 13V4Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      Clear
    </button>
  </div>
  <div class="chat-messages" bind:this={chatContainer}>
    {#if aiStore.messages.length === 0 && !aiStore.currentStreamContent}
      <div class="empty-state">
        <svg width="44" height="44" viewBox="0 0 44 44" fill="none">
          <circle cx="22" cy="22" r="20" stroke="currentColor" stroke-width="2" opacity="0.1"/>
          <path d="M28 17C28 14.239 25.761 12 23 12H18C15.239 12 13 14.239 13 17V22C13 24.761 15.239 27 18 27H19.5L22 30L24.5 27H23C25.761 27 28 24.761 28 22V17Z" stroke="currentColor" stroke-width="1.5" fill="none"/>
        </svg>
        <p class="empty-title">AI Assistant</p>
        <p class="empty-description">
          {#if !notesStore.currentFile}
            Open a note to start chatting
          {:else if aiStore.editMode}
            Enter edit instructions for the current note
          {:else}
            Ask questions or request changes to your note
          {/if}
        </p>
      </div>
    {:else}
      {#each aiStore.messages as message, index}
        <ChatMessage role={message.role} content={message.content} />

        {#if message.role === 'assistant' && message.sources && message.sources.length > 0}
          <div class="sources-section">
            <div class="sources-title">
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none" style="display: inline-block; vertical-align: middle; margin-right: 4px;">
                <path d="M7.5 4.5L10.5 1.5C11.328 0.672 12.672 0.672 13.5 1.5C14.328 2.328 14.328 3.672 13.5 4.5L10.5 7.5M7.5 4.5L4.5 7.5M7.5 4.5L8.5 5.5M4.5 7.5L1.5 10.5C0.672 11.328 0.672 12.672 1.5 13.5C2.328 14.328 3.672 14.328 4.5 13.5L7.5 10.5M4.5 7.5L5.5 8.5M8.5 5.5L5.5 8.5M8.5 5.5L10.5 7.5M5.5 8.5L7.5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              Sources used:
            </div>
            <div class="sources-list">
              {#each message.sources as source}
                <div class="source-item" role="button" tabindex="0" onclick={() => handleOpenDocument(source.filepath)}>
                  <span class="source-relevance">{source.relevance}%</span>
                  <span class="source-name" title={source.filename}>
                    {source.filename}
                  </span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      {/each}

      {#if aiStore.currentStreamContent}
        <ChatMessage role="assistant" content={aiStore.currentStreamContent} isStreaming={true} />

        {#if aiStore.currentSources.length > 0}
          <div class="sources-section">
            <div class="sources-title">
              <svg width="14" height="14" viewBox="0 0 16 16" fill="none" style="display: inline-block; vertical-align: middle; margin-right: 4px;">
                <path d="M7.5 4.5L10.5 1.5C11.328 0.672 12.672 0.672 13.5 1.5C14.328 2.328 14.328 3.672 13.5 4.5L10.5 7.5M7.5 4.5L4.5 7.5M7.5 4.5L8.5 5.5M4.5 7.5L1.5 10.5C0.672 11.328 0.672 12.672 1.5 13.5C2.328 14.328 3.672 14.328 4.5 13.5L7.5 10.5M4.5 7.5L5.5 8.5M8.5 5.5L5.5 8.5M8.5 5.5L10.5 7.5M5.5 8.5L7.5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
              Sources used:
            </div>
            <div class="sources-list">
              {#each aiStore.currentSources as source}
                <div class="source-item" role="button" tabindex="0" onclick={() => handleOpenDocument(source.filepath)}>
                  <span class="source-relevance">{source.relevance}%</span>
                  <span class="source-name" title={source.filename}>
                    {source.filename}
                  </span>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      {/if}


      {#if aiStore.error}
        <div class="error-message">
          <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
            <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
            <path d="M8 4V8M8 11H8.01" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          {aiStore.error}
        </div>
      {/if}

      {#if proposedEdit}
        <div bind:this={diffViewEl} class="diff-view-wrapper">
          <DiffView
            oldText={proposedEdit.oldContent}
            newText={proposedEdit.newContent}
            onApply={applyEdit}
            onReject={rejectEdit}
          />
        </div>
      {/if}
    {/if}
  </div>

  <div class="chat-input-wrapper">
    <div class="unified-input-container" class:has-text={inputText.trim().length > 0}>
      <textarea
        bind:this={textareaEl}
        bind:value={inputText}
        onkeydown={handleKeyDown}
        oninput={handleInput}
        placeholder={aiStore.editMode ? "Edit instruction..." : "Ask about this note..."}
        disabled={aiStore.isEditingNote || !notesStore.currentFile}
        rows="1"
      ></textarea>

      <div class="divider-line"></div>

      <div class="bottom-controls">
        <button
          class="control-btn"
          class:active={aiStore.editMode}
          onclick={() => aiStore.toggleEditMode()}
          title="Edit mode"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M10 1L13 4L5 12H2V9L10 1Z" stroke="currentColor" stroke-width="1.2" fill="none"/>
          </svg>
        </button>

        <button
          class="control-btn"
          class:active={useSources && !webSearch}
          onclick={() => {
            useSources = !useSources;
            if (useSources) webSearch = false;
          }}
          title="Use sources from this notebook"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M3 2H11V10H9V12H1V4H3V2Z" stroke="currentColor" stroke-width="1.2" fill="none"/>
            <path d="M5 4H9V8" stroke="currentColor" stroke-width="1.2" fill="none"/>
          </svg>
        </button>

        <button
          class="control-btn"
          class:active={webSearch}
          onclick={() => {
            webSearch = !webSearch;
            if (webSearch) useSources = false;
          }}
          title="Web search mode"
        >
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <circle cx="7" cy="7" r="5.5" stroke="currentColor" stroke-width="1.2" fill="none"/>
            <path d="M7 1.5C7 1.5 9 3 9 7C9 11 7 12.5 7 12.5M7 1.5C7 1.5 5 3 5 7C5 11 7 12.5 7 12.5M2 7H12" stroke="currentColor" stroke-width="1.2"/>
          </svg>
        </button>

        <div class="model-select-wrapper">
          <button
            class="model-select"
            onclick={toggleModelDropdown}
            type="button"
            disabled={availableModels().length === 0}
          >
            <span class="model-name">{selectedModel || 'No models'}</span>
            <svg class="dropdown-arrow" width="12" height="12" viewBox="0 0 12 12" fill="none">
              <path d="M3 5L6 8L9 5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </button>

          {#if showModelDropdown}
            <div class="model-dropdown">
              {#each availableModels() as model}
                <button
                  class="model-option"
                  class:selected={selectedModel === model}
                  onclick={() => selectModel(model)}
                  type="button"
                >
                  {model}
                </button>
              {/each}
            </div>
          {/if}
        </div>

        {#if aiStore.isStreaming || aiStore.isEditingNote}
          <button class="send-btn stop" onclick={handleStop} title="Stop" disabled={aiStore.isEditingNote}>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none">
              {#if aiStore.isEditingNote}
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
              {:else}
                <rect x="4" y="4" width="8" height="8" fill="currentColor"/>
              {/if}
            </svg>
          </button>
        {:else}
          <button
            class="send-btn"
            onclick={handleSend}
            disabled={!inputText.trim() || !notesStore.currentFile || !selectedModel || aiStore.isStreaming}
            title="Send (Enter)"
          >
            ↑
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>

<ConfirmModal
  bind:isOpen={showClearHistoryModal}
  title="Clear Chat History"
  message="Clear all chat history for this note?"
  confirmText="Clear"
  onConfirm={confirmClearHistory}
/>

<style>
  .chat-container {
    display: flex;
    flex-direction: column;
    height: 100%;
    position: relative;
  }

  .chat-header {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    padding: 8px 12px;
    background: linear-gradient(to bottom, var(--bg-primary) 0%, transparent 100%);
    backdrop-filter: blur(4px);
    -webkit-backdrop-filter: blur(4px);
    border-bottom: 1px solid var(--border);
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    z-index: 10;
  }

  .clear-chat-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .clear-chat-btn:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
    border-color: var(--error);
  }

  .clear-chat-btn:active:not(:disabled) {
    background: var(--error);
    color: #fff;
  }

  .clear-chat-btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  .chat-messages {
    flex: 1;
    overflow-y: auto;
    overflow-x: auto;
    padding: 46px 16px 16px 16px;
    display: flex;
    flex-direction: column;
  }

  /* Scrollbar */
  .chat-messages::-webkit-scrollbar {
    width: 8px;
    height: 8px;
  }

  .chat-messages::-webkit-scrollbar-track {
    background: transparent;
  }

  .chat-messages::-webkit-scrollbar-thumb {
    background: var(--bg-hover);
    border-radius: 4px;
  }

  .chat-messages::-webkit-scrollbar-thumb:hover {
    background: var(--border);
  }

  .chat-messages::-webkit-scrollbar-corner {
    background: var(--bg-primary);
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
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

  .error-message {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    margin: 12px 0;
    background: rgba(247, 118, 142, 0.15);
    border-left: 2px solid var(--error);
    border-radius: 4px;
    color: var(--error);
    font-size: 12px;
  }

  .diff-view-wrapper {
    margin-top: 12px;
  }

  .chat-input-wrapper {
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
    background: var(--error);
  }

  .send-btn.stop:hover {
    opacity: 0.85;
  }

  .send-btn.stop:disabled {
    cursor: default;
    background: var(--accent);
  }

  .spinner {
    animation: spin 1s linear infinite;
    transform-origin: center;
  }

  @keyframes spin {
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

  .model-select-wrapper {
    flex: 1 1 auto;
    max-width: 140px;
    min-width: 0;
    position: relative;
    overflow: visible;
  }

  .model-select {
    width: 100%;
    min-width: 0;
    height: 28px;
    padding: 0 10px;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: rgba(255, 255, 255, 0.5);
    font-size: 11px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
    text-align: left;
    display: flex;
    align-items: center;
    gap: 4px;
    position: relative;
    overflow: hidden;
  }

  .model-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-select:hover:not(:disabled) {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .model-select:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .dropdown-arrow {
    flex-shrink: 0;
    color: var(--text-muted);
    pointer-events: none;
  }

  .model-dropdown {
    position: absolute;
    bottom: 100%;
    left: 0;
    right: 0;
    margin-bottom: 4px;
    background: rgba(22, 27, 34, 0.85);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    z-index: 1000;
  }

  .model-option {
    width: 100%;
    padding: 8px 10px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 0.75rem;
    text-align: left;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .model-option:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .model-option.selected {
    color: var(--accent);
    background: rgba(122, 162, 247, 0.12);
  }

  .sources-section {
    margin-top: 12px;
    margin-bottom: 24px;
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