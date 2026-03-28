<script lang="ts">
  import { documentsStore } from '../stores/documents.svelte';
  import { notebooksStore } from '../stores/notebooks.svelte';
  import { notesStore } from '../stores/notes.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { invoke } from '@tauri-apps/api/core';

  let { onClose } = $props<{ onClose: () => void }>();

  let sourceType = $state<'file' | 'youtube' | 'url' | 'text'>('file');
  let youtubeUrl = $state('');
  let youtubeConvertToNote = $state(false);
  let webUrl = $state('');
  let textTitle = $state('');
  let textContent = $state('');
  let isLoading = $state(false);
  let error = $state<string | null>(null);

  async function handleFileUpload() {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: 'PDF Documents', extensions: ['pdf'] },
          { name: 'Word Documents', extensions: ['docx'] },
          { name: 'PowerPoint Presentations', extensions: ['pptx'] },
          { name: 'Text Files', extensions: ['txt'] },
          { name: 'All Supported', extensions: ['pdf', 'docx', 'pptx', 'txt'] }
        ]
      });

      if (selected && notebooksStore.currentNotebook) {
        isLoading = true;
        error = null;
        await documentsStore.upload(selected, notebooksStore.currentNotebook);
        onClose();
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      isLoading = false;
    }
  }

  async function handleYoutubeUpload() {
    if (!youtubeUrl.trim() || !notebooksStore.currentNotebook) return;

    const url = youtubeUrl.trim();
    const notebook = notebooksStore.currentNotebook;
    const convertToNote = youtubeConvertToNote;

    // Close modal immediately
    onClose();

    // Run upload in background
    try {
      if (convertToNote) {
        console.log('Converting YouTube to note + adding as source...');

        // First, add as document source (will show with loading indicator)
        await invoke('document_upload_youtube', {
          url,
          notebook
        });
        await documentsStore.loadForNotebook(notebook);
        console.log('Source added, now converting to note...');

        // Then, convert to structured markdown note
        const markdown = await invoke<string>('youtube_to_note', {
          url,
          notebook
        });
        console.log('Received markdown, creating note...');

        // Create note from markdown
        const videoId = url.split('v=')[1]?.split('&')[0] || 'video';
        const notePath = `${notebook}/YouTube_${videoId}.md`;

        await invoke('note_create', { path: notePath });
        await invoke('note_write', { path: notePath, content: markdown });

        // Refresh and open note
        await notesStore.loadFiles(notebook);
        await notesStore.openFile(notePath);
        console.log('Note opened successfully');
      } else {
        // Add as document source only
        await invoke('document_upload_youtube', {
          url,
          notebook
        });
        await documentsStore.loadForNotebook(notebook);
      }
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      console.error('YouTube upload error:', errorMsg);
      // Show error to user (could add a toast notification here)
      alert(`Failed to process YouTube video: ${errorMsg}`);
    }
  }

  async function handleUrlUpload() {
    if (!webUrl.trim() || !notebooksStore.currentNotebook) return;

    isLoading = true;
    error = null;

    try {
      await invoke('document_upload_url', {
        url: webUrl.trim(),
        notebook: notebooksStore.currentNotebook
      });
      await documentsStore.loadForNotebook(notebooksStore.currentNotebook);
      onClose();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      isLoading = false;
    }
  }

  async function handleTextUpload() {
    if (!textContent.trim() || !notebooksStore.currentNotebook) return;

    isLoading = true;
    error = null;

    try {
      await invoke('document_upload_text', {
        title: textTitle.trim() || 'Text snippet',
        content: textContent.trim(),
        notebook: notebooksStore.currentNotebook
      });
      await documentsStore.loadForNotebook(notebooksStore.currentNotebook);
      onClose();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      isLoading = false;
    }
  }

  function handleSubmit() {
    switch (sourceType) {
      case 'file':
        handleFileUpload();
        break;
      case 'youtube':
        handleYoutubeUpload();
        break;
      case 'url':
        handleUrlUpload();
        break;
      case 'text':
        handleTextUpload();
        break;
    }
  }
</script>

<div class="modal-backdrop" onclick={onClose}>
  <div class="modal" onclick={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <h2>Add Source</h2>
    </div>

    <div class="modal-body">
      <!-- Source Type Selector -->
      <div class="source-type-selector">
        <button
          class="type-btn"
          class:active={sourceType === 'file'}
          onclick={() => (sourceType = 'file')}
        >
          <svg width="16" height="16" viewBox="0 0 20 20" fill="none">
            <path d="M12 2H5C4.44772 2 4 2.44772 4 3V17C4 17.5523 4.44772 18 5 18H15C15.5523 18 16 17.5523 16 17V6L12 2Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M12 2V6H16" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M7 10H13M7 13H13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          File
        </button>
        <button
          class="type-btn"
          class:active={sourceType === 'youtube'}
          onclick={() => (sourceType = 'youtube')}
        >
          <svg width="16" height="16" viewBox="0 0 20 20" fill="none">
            <rect x="2" y="4" width="16" height="12" rx="2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M8 7L13 10L8 13V7Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
          YouTube
        </button>
        <button
          class="type-btn"
          class:active={sourceType === 'url'}
          onclick={() => (sourceType = 'url')}
        >
          <svg width="16" height="16" viewBox="0 0 20 20" fill="none">
            <path d="M9 6L6.5 8.5C5.5 9.5 5.5 11 6.5 12C7.5 13 9 13 10 12L11.5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M11 14L13.5 11.5C14.5 10.5 14.5 9 13.5 8C12.5 7 11 7 10 8L8.5 9.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.5"/>
          </svg>
          Web Page
        </button>
        <button
          class="type-btn"
          class:active={sourceType === 'text'}
          onclick={() => (sourceType = 'text')}
        >
          <svg width="16" height="16" viewBox="0 0 20 20" fill="none">
            <rect x="3" y="3" width="14" height="14" rx="2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M6 7H14M6 10H14M6 13H11" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          </svg>
          Text
        </button>
      </div>

      <!-- Content based on selected type -->
      {#if sourceType === 'file'}
        <div class="form-section">
          <p class="hint">Click the button below to select a PDF, DOCX, PPTX, or TXT file.</p>
        </div>
      {:else if sourceType === 'youtube'}
        <div class="form-section">
          <label for="youtube-url">YouTube URL</label>
          <input
            id="youtube-url"
            type="text"
            placeholder="https://www.youtube.com/watch?v=..."
            bind:value={youtubeUrl}
          />
          <p class="hint">Transcript will be extracted automatically</p>

          <label class="checkbox-label" style="margin-top: 12px;">
            <input
              type="checkbox"
              bind:checked={youtubeConvertToNote}
            />
            <span>Convert to structured markdown note (uses AI)</span>
          </label>
          <p class="hint" style="margin-top: 4px;">
            Creates a well-formatted note instead of adding as raw source. Takes ~10 seconds.
          </p>
        </div>
      {:else if sourceType === 'url'}
        <div class="form-section">
          <label for="web-url">Web Page URL</label>
          <input
            id="web-url"
            type="text"
            placeholder="https://example.com/article"
            bind:value={webUrl}
          />
          <p class="hint">Main content will be extracted from the page</p>
        </div>
      {:else if sourceType === 'text'}
        <div class="form-section">
          <label for="text-title">Title (optional)</label>
          <input
            id="text-title"
            type="text"
            placeholder="My notes"
            bind:value={textTitle}
          />
          <label for="text-content">Content</label>
          <textarea
            id="text-content"
            placeholder="Paste or type text here..."
            bind:value={textContent}
            rows="10"
          ></textarea>
        </div>
      {/if}

      {#if error}
        <div class="error-message">{error}</div>
      {/if}
    </div>

    <div class="modal-footer">
      <button class="btn-secondary" onclick={onClose} disabled={isLoading}>
        Cancel
      </button>
      <button class="btn-primary" onclick={handleSubmit} disabled={isLoading}>
        {#if isLoading}
          Adding...
        {:else}
          Add Source
        {/if}
      </button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
  }

  .modal {
    background: rgba(22, 27, 34, 0.85);
    backdrop-filter: blur(20px);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: 90%;
    max-width: 500px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 16px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: none;
    border: none;
    font-size: 24px;
    color: var(--text-muted);
    cursor: pointer;
    padding: 0;
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    transition: all 0.15s ease;
  }

  .close-btn:hover {
    background: var(--bg-hover);
    color: var(--text-primary);
  }

  .modal-body {
    padding: 20px;
    overflow-y: auto;
    flex: 1;
  }

  .source-type-selector {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 8px;
    margin-bottom: 20px;
  }

  .type-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 10px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .type-btn svg {
    flex-shrink: 0;
    opacity: 0.7;
  }

  .type-btn:hover {
    background: var(--bg-hover);
    border-color: var(--accent);
  }

  .type-btn:hover svg {
    opacity: 1;
  }

  .type-btn.active {
    background: var(--bg-elevated);
    border-color: var(--accent);
    color: var(--text-primary);
  }

  .type-btn.active svg {
    opacity: 1;
  }

  .form-section {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    margin-top: 4px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    color: var(--text-primary);
    cursor: pointer;
    margin-top: 0;
    font-weight: normal;
  }

  .checkbox-label input[type="checkbox"] {
    width: auto;
    cursor: pointer;
  }

  .checkbox-label span {
    user-select: none;
  }

  input,
  textarea {
    width: 100%;
    padding: 10px 12px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    font-family: inherit;
    transition: all 0.15s ease;
  }

  input:focus,
  textarea:focus {
    outline: none;
    border-color: var(--accent);
  }

  textarea {
    resize: vertical;
    font-family: 'JetBrains Mono', monospace;
  }

  .hint {
    font-size: 12px;
    color: var(--text-muted);
    margin: 0;
  }

  .error-message {
    padding: 10px 12px;
    background: rgba(247, 118, 142, 0.1);
    border: 1px solid var(--error);
    border-radius: 6px;
    color: var(--error);
    font-size: 12px;
    margin-top: 12px;
  }

  .modal-footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 8px;
    padding: 16px 20px;
    border-top: 1px solid var(--border);
  }

  .btn-secondary,
  .btn-primary {
    padding: 8px 16px;
    border: none;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .btn-secondary {
    background: var(--bg-secondary);
    color: var(--text-primary);
    border: 1px solid transparent;
  }

  .btn-secondary:hover:not(:disabled) {
    background: var(--bg-hover);
    border-color: var(--error);
  }

  .btn-secondary:active:not(:disabled) {
    background: var(--error);
    color: white;
    border-color: var(--error);
  }

  .btn-primary {
    background: var(--accent);
    color: var(--bg-primary);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
  }

  .btn-secondary:disabled,
  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
