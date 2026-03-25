<script lang="ts">
  import { documentsStore } from '../stores/documents.svelte';
  import { notebooksStore } from '../stores/notebooks.svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { DocumentInfo } from '../types';

  let selectedDoc = $state<DocumentInfo | null>(null);
  let showContextMenu = $state(false);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);

  async function uploadDocument() {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: 'PDF Documents',
            extensions: ['pdf']
          },
          {
            name: 'Word Documents',
            extensions: ['docx']
          },
          {
            name: 'PowerPoint Presentations',
            extensions: ['pptx']
          },
          {
            name: 'Text Files',
            extensions: ['txt']
          },
          {
            name: 'All Supported',
            extensions: ['pdf', 'docx', 'pptx', 'txt']
          }
        ]
      });

      if (selected && notebooksStore.currentNotebook) {
        // selected is already a string path, not an object
        await documentsStore.upload(selected, notebooksStore.currentNotebook);
      }
    } catch (err) {
      console.error('Failed to upload document:', err);
      alert('Failed to upload document: ' + err);
    }
  }

  async function deleteDocument(doc: DocumentInfo) {
    if (confirm(`Delete "${doc.filename}"?`)) {
      try {
        await documentsStore.remove(doc.id);
      } catch (err) {
        console.error('Failed to delete document:', err);
        alert('Failed to delete: ' + err);
      }
    }
  }

  function showDocContextMenu(doc: DocumentInfo, event: MouseEvent) {
    event.preventDefault();
    selectedDoc = doc;
    contextMenuX = event.clientX;
    contextMenuY = event.clientY;
    showContextMenu = true;
  }

  function closeContextMenu() {
    showContextMenu = false;
    selectedDoc = null;
  }

  function getPageInfo(doc: DocumentInfo): string {
    if (doc.file_type === 'pptx' && doc.page_count) {
      return `${doc.page_count} slides`;
    } else if (doc.page_count) {
      return `${doc.page_count}p`;
    }
    return '';
  }

  function getStatusTooltip(doc: DocumentInfo): string {
    const status = documentsStore.getDocumentStatus(doc);
    switch (status) {
      case 'indexing':
        return 'Creating embeddings...';
      case 'indexed':
        return 'Ready for AI search';
      case 'error':
        return 'Indexing failed';
      case 'pending':
        return 'Waiting to index';
      default:
        return status;
    }
  }

  function getFileIcon(fileType: string) {
    switch (fileType.toLowerCase()) {
      case 'pdf':
        return `<svg width="24" height="24" viewBox="0 0 24 24" fill="none">
          <path d="M14 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V8L14 2Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M14 2V8H20" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M10 13H8V17M8 13V17M8 13C8 13 8.5 13 9 13C9.5 13 10 13.5 10 14C10 14.5 9.5 15 9 15H8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M12 13H13C13.5 13 14 13.5 14 14V16C14 16.5 13.5 17 13 17H12V13Z" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M16 13V17M16 13H18M16 15H17.5" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>`;
      case 'docx':
        return `<svg width="24" height="24" viewBox="0 0 24 24" fill="none">
          <path d="M14 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V8L14 2Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M14 2V8H20" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M8 13L9.5 17L11 13M11 13L12.5 17L14 13M14 13L15.5 17L17 13" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>`;
      case 'pptx':
        return `<svg width="24" height="24" viewBox="0 0 24 24" fill="none">
          <path d="M14 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V8L14 2Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M14 2V8H20" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <rect x="8" y="12" width="8" height="6" rx="1" stroke="currentColor" stroke-width="1.2"/>
          <line x1="8" y1="15" x2="16" y2="15" stroke="currentColor" stroke-width="1.2"/>
        </svg>`;
      case 'txt':
        return `<svg width="24" height="24" viewBox="0 0 24 24" fill="none">
          <path d="M14 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V8L14 2Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M14 2V8H20" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M8 13H16M8 16H16" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>`;
      default:
        return `<svg width="24" height="24" viewBox="0 0 24 24" fill="none">
          <path d="M14 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V8L14 2Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M14 2V8H20" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>`;
    }
  }
</script>

<svelte:window onclick={closeContextMenu} />

<div class="sources-list">
  <div class="upload-section">
    <button class="upload-btn" onclick={uploadDocument}>
      <span class="upload-icon">+</span>
      Upload source
    </button>
  </div>

  <div class="documents-container">
    {#if documentsStore.isLoading}
      <div class="loading">Loading sources...</div>
    {:else if documentsStore.error}
      <div class="error">{documentsStore.error}</div>
    {:else if documentsStore.documents.length === 0}
      <div class="empty-state">
        <div class="empty-icon">📚</div>
        <div class="empty-text">No sources yet</div>
        <div class="empty-hint">Upload PDF, DOCX, PPTX, or TXT files</div>
      </div>
    {:else}
      {#each documentsStore.documents as doc (doc.id)}
        <div
          class="document-item"
          oncontextmenu={(e) => showDocContextMenu(doc, e)}
        >
          <div class="doc-icon">
            {@html getFileIcon(doc.file_type)}
          </div>
          <div class="doc-info">
            <div class="doc-name" title={doc.title}>
              {doc.filename}
            </div>
            <div class="doc-meta">
              {#if getPageInfo(doc)}
                <span>{getPageInfo(doc)}</span>
                <span class="separator">·</span>
              {/if}
              <span>{documentsStore.formatFileSize(doc.size_bytes)}</span>
            </div>
          </div>
          <div class="doc-status" title={getStatusTooltip(doc)}>
            {#if documentsStore.getDocumentStatus(doc) === 'indexing'}
              <svg class="hourglass-icon" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <path d="M3 2H13V4L8 8L13 12V14H3V12L8 8L3 4V2Z" stroke="currentColor" stroke-width="1.2" fill="none"/>
                <path class="sand-top" d="M5 3H11V5L8 7L5 5V3Z" fill="currentColor" opacity="0.5"/>
                <path class="sand-bottom" d="M5 13H11V11L8 9L5 11V13Z" fill="currentColor" opacity="0.5"/>
              </svg>
            {:else if documentsStore.getDocumentStatus(doc) === 'indexed'}
              <svg class="status-icon success" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
                <path d="M5 8L7 10L11 6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            {:else if documentsStore.getDocumentStatus(doc) === 'error'}
              <svg class="status-icon error" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
                <path d="M6 6L10 10M10 6L6 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
            {:else}
              <svg class="status-icon pending" width="16" height="16" viewBox="0 0 16 16" fill="none">
                <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
                <circle cx="8" cy="8" r="2" fill="currentColor"/>
              </svg>
            {/if}
          </div>
        </div>
      {/each}
    {/if}
  </div>
</div>

{#if showContextMenu && selectedDoc}
  <div
    class="context-menu"
    style="left: {contextMenuX}px; top: {contextMenuY}px;"
  >
    <button
      class="context-menu-item"
      onclick={() => {
        if (selectedDoc) deleteDocument(selectedDoc);
        closeContextMenu();
      }}
    >
      🗑 Delete
    </button>
    {#if documentsStore.indexingErrors.has(selectedDoc.id)}
      <button
        class="context-menu-item error"
        onclick={closeContextMenu}
        title={documentsStore.indexingErrors.get(selectedDoc.id)}
      >
        ❌ Indexing error
      </button>
    {/if}
  </div>
{/if}

<style>
  .sources-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .upload-section {
    padding: 12px;
    border-bottom: 1px solid var(--border);
  }

  .upload-btn {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .upload-btn:hover {
    background: var(--bg-hover);
    border-color: var(--accent);
  }

  .upload-icon {
    font-size: 16px;
    font-weight: 300;
  }

  .documents-container {
    flex: 1;
    overflow-y: auto;
    padding: 4px;
  }

  .loading,
  .error,
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

  .error {
    color: var(--error);
  }

  .empty-icon {
    font-size: 48px;
    margin-bottom: 12px;
    opacity: 0.3;
    filter: grayscale(1);
  }

  .empty-text {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 6px;
  }

  .empty-hint {
    font-size: 12px;
    color: var(--text-muted);
  }

  .document-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px;
    margin: 2px 0;
    border-radius: 4px;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .document-item:hover {
    background: var(--bg-hover);
  }

  .doc-icon {
    flex-shrink: 0;
    width: 24px;
    height: 24px;
    color: var(--text-secondary);
  }

  .doc-info {
    flex: 1;
    min-width: 0;
  }

  .doc-name {
    font-size: 13px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .doc-meta {
    font-size: 11px;
    color: var(--text-muted);
    margin-top: 2px;
  }

  .separator {
    margin: 0 4px;
  }

  .doc-status {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .status-icon {
    color: var(--text-muted);
  }

  .status-icon.success {
    color: var(--success);
  }

  .status-icon.error {
    color: var(--error);
  }

  .status-icon.pending {
    color: var(--text-muted);
    opacity: 0.4;
  }

  .hourglass-icon {
    color: var(--accent);
    animation: hourglass-flip 2s ease-in-out infinite;
  }

  @keyframes hourglass-flip {
    0%, 40% {
      transform: rotate(0deg);
    }
    50%, 90% {
      transform: rotate(180deg);
    }
    100% {
      transform: rotate(180deg);
    }
  }

  .sand-top {
    animation: sand-drain 2s ease-in-out infinite;
  }

  .sand-bottom {
    animation: sand-fill 2s ease-in-out infinite;
  }

  @keyframes sand-drain {
    0% {
      opacity: 0.7;
    }
    40% {
      opacity: 0.2;
    }
    50%, 100% {
      opacity: 0.2;
    }
  }

  @keyframes sand-fill {
    0%, 50% {
      opacity: 0.2;
    }
    90% {
      opacity: 0.7;
    }
    100% {
      opacity: 0.7;
    }
  }

  .context-menu {
    position: fixed;
    z-index: 1000;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    padding: 4px;
    min-width: 150px;
  }

  .context-menu-item {
    display: block;
    width: 100%;
    padding: 8px 12px;
    background: transparent;
    border: none;
    border-radius: 4px;
    color: var(--text-primary);
    font-size: 13px;
    text-align: left;
    cursor: pointer;
    transition: background 0.15s ease;
  }

  .context-menu-item:hover {
    background: var(--bg-hover);
  }

  .context-menu-item.error {
    color: var(--error);
  }
</style>
