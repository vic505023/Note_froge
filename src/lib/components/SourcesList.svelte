<script lang="ts">
  import { documentsStore } from '../stores/documents.svelte';
  import { notebooksStore } from '../stores/notebooks.svelte';
  import { open, ask } from '@tauri-apps/plugin-dialog';
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
    console.log('🔵 deleteDocument STARTED for:', doc.filename, 'id:', doc.id);
    console.log('🔵 Documents BEFORE confirm:', documentsStore.documents.map(d => ({ id: d.id, name: d.filename })));

    const confirmed = confirm(`Delete "${doc.filename}"?`);

    console.log('🔵 User confirmed:', confirmed);
    console.log('🔵 Documents AFTER confirm:', documentsStore.documents.map(d => ({ id: d.id, name: d.filename })));

    if (confirmed) {
      try {
        console.log('🔵 Calling documentsStore.remove()');
        await documentsStore.remove(doc.id);
        console.log('🔵 Document removed successfully');
      } catch (err) {
        console.error('Failed to delete document:', err);
        alert('Failed to delete: ' + err);
      }
    } else {
      console.log('🔵 Delete cancelled by user');
    }
  }

  function showDocContextMenu(doc: DocumentInfo, event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    console.log('🟡 Context menu opened for:', doc.filename, 'id:', doc.id);
    selectedDoc = doc;
    // Смещаем меню чтобы курсор не был над кнопкой Delete
    contextMenuX = event.clientX + 5;
    contextMenuY = event.clientY + 5;
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

  function getFileIcon() {
    return `<svg width="24" height="24" viewBox="0 0 24 24" fill="none">
      <path d="M14 2H6C5.46957 2 4.96086 2.21071 4.58579 2.58579C4.21071 2.96086 4 3.46957 4 4V20C4 20.5304 4.21071 21.0391 4.58579 21.4142C4.96086 21.7893 5.46957 22 6 22H18C18.5304 22 19.0391 21.7893 19.4142 21.4142C19.7893 21.0391 20 20.5304 20 20V8L14 2Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M14 2V8H20" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      <path d="M8 13H16M8 16H16" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
    </svg>`;
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
        <div class="empty-icon">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none">
            <path d="M4 19.5C4 18.837 4.26339 18.2011 4.73223 17.7322C5.20107 17.2634 5.83696 17 6.5 17H20" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M6.5 2H20V22H6.5C5.83696 22 5.20107 21.7366 4.73223 21.2678C4.26339 20.7989 4 20.163 4 19.5V4.5C4 3.83696 4.26339 3.20107 4.73223 2.73223C5.20107 2.26339 5.83696 2 6.5 2Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
            <path d="M8 6H16M8 10H16M8 14H12" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          </svg>
        </div>
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
            {@html getFileIcon()}
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
    onclick={(e) => e.stopPropagation()}
  >
    <button
      class="context-menu-item"
      onclick={async (e) => {
        console.log('🔴 BUTTON CLICKED - START');
        console.log('🔴 Event:', e);
        console.log('🔴 Documents in store:', documentsStore.documents.length);
        console.log('🔴 Document IDs:', documentsStore.documents.map(d => d.id));

        e.preventDefault();
        e.stopPropagation();

        console.log('🔴 After preventDefault/stopPropagation');
        console.log('🔴 selectedDoc:', selectedDoc);

        if (selectedDoc) {
          const docId = selectedDoc.id;
          const docFilename = selectedDoc.filename;

          // Close menu first for clean UX
          closeContextMenu();
          console.log('🔴 Menu closed');

          console.log('🔴 BEFORE CONFIRM');
          console.log('🔴 Will show confirm for:', docFilename, 'id:', docId);

          const confirmed = await ask(`Delete "${docFilename}"?`, {
            title: 'Confirm Delete',
            kind: 'warning'
          });

          console.log('🔴 AFTER CONFIRM, result:', confirmed);
          console.log('🔴 Documents count after confirm:', documentsStore.documents.length);

          if (confirmed) {
            console.log('🔴 User confirmed, calling remove');
            try {
              await documentsStore.remove(docId);
              console.log('🔴 Remove complete');
            } catch (err) {
              console.error('🔴 Remove failed:', err);
              alert('Failed to delete: ' + err);
            }
          } else {
            console.log('🔴 User cancelled');
          }
        } else {
          console.log('🔴 No selectedDoc, just closing menu');
          closeContextMenu();
        }

        console.log('🔴 BUTTON CLICKED - END');
      }}
    >
      <svg width="14" height="14" viewBox="0 0 16 16" fill="none" style="display: inline-block; vertical-align: middle; margin-right: 6px;">
        <path d="M3 4H13M5 4V3C5 2.44772 5.44772 2 6 2H10C10.5523 2 11 2.44772 11 3V4M6 7V11M10 7V11M4 4L4.5 13C4.5 13.5523 4.94772 14 5.5 14H10.5C11.0523 14 11.5 13.5523 11.5 13L12 4" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      Delete
    </button>
    {#if documentsStore.indexingErrors.has(selectedDoc.id)}
      <button
        class="context-menu-item error"
        onclick={closeContextMenu}
        title={documentsStore.indexingErrors.get(selectedDoc.id)}
      >
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" style="display: inline-block; vertical-align: middle; margin-right: 6px;">
          <circle cx="8" cy="8" r="7" stroke="currentColor" stroke-width="1.5"/>
          <path d="M6 6L10 10M10 6L6 10" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        Indexing error
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
    color: var(--accent);
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
