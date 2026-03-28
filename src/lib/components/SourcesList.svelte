<script lang="ts">
  import { documentsStore } from '../stores/documents.svelte';
  import { notebooksStore } from '../stores/notebooks.svelte';
  import { uiStore } from '../stores/ui.svelte';
  import { ask } from '@tauri-apps/plugin-dialog';
  import type { DocumentInfo } from '../types';

  let selectedDoc = $state<DocumentInfo | null>(null);
  let showContextMenu = $state(false);
  let contextMenuX = $state(0);
  let contextMenuY = $state(0);
  let isDragging = $state(false);
  let dragCounter = $state(0);

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

  function getFileIconType(doc: DocumentInfo): 'youtube' | 'web' | 'file' {
    // YouTube videos
    if (doc.filename.startsWith('YouTube:')) {
      return 'youtube';
    }
    // Web URLs
    if (doc.filename.startsWith('http://') || doc.filename.startsWith('https://')) {
      return 'web';
    }
    // Regular files and text snippets
    return 'file';
  }

  // Drag & Drop handlers
  function handleDragEnter(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    dragCounter++;
    if (dragCounter === 1) {
      isDragging = true;
    }
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
  }

  function handleDragLeave(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    dragCounter--;
    if (dragCounter === 0) {
      isDragging = false;
    }
  }

  async function handleDrop(e: DragEvent) {
    e.preventDefault();
    e.stopPropagation();
    isDragging = false;
    dragCounter = 0;

    const files = e.dataTransfer?.files;
    if (!files || files.length === 0) return;

    const notebook = notebooksStore.currentNotebook;
    if (!notebook) {
      alert('Please select a notebook first');
      return;
    }

    const supportedExtensions = ['.pdf', '.docx', '.pptx', '.txt'];
    const validFiles: File[] = [];
    const invalidFiles: string[] = [];

    for (let i = 0; i < files.length; i++) {
      const file = files[i];
      const ext = file.name.toLowerCase().substring(file.name.lastIndexOf('.'));

      if (supportedExtensions.includes(ext)) {
        validFiles.push(file);
      } else {
        invalidFiles.push(file.name);
      }
    }

    if (invalidFiles.length > 0) {
      alert(`Unsupported file types:\n${invalidFiles.join('\n')}\n\nSupported: PDF, DOCX, PPTX, TXT`);
    }

    // Upload valid files
    for (const file of validFiles) {
      try {
        await documentsStore.upload(file.path, notebook);
      } catch (err) {
        console.error('Failed to upload file:', file.name, err);
        alert(`Failed to upload ${file.name}: ${err}`);
      }
    }
  }
</script>

<svelte:window onclick={closeContextMenu} />

<div class="sources-list">
  <div class="upload-section">
    <button class="upload-btn" onclick={() => uiStore.openAddSourceModal()}>
      <span class="upload-icon">+</span>
      Add source
    </button>
  </div>

  <div
    class="documents-container"
    class:dragging={isDragging}
    ondragenter={handleDragEnter}
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
  >
    {#if isDragging}
      <div class="drop-overlay">
        <div class="drop-icon">📁</div>
        <div class="drop-text">Drop files to upload</div>
        <div class="drop-hint">PDF, DOCX, PPTX, TXT</div>
      </div>
    {/if}

    {#if documentsStore.isLoading}
      <div class="loading">Loading sources...</div>
    {:else if documentsStore.error}
      <div class="error">{documentsStore.error}</div>
    {:else if documentsStore.documents.length === 0}
      <div class="empty-state">
        <svg width="48" height="48" viewBox="0 0 48 48" fill="none">
          <circle cx="24" cy="24" r="22" stroke="currentColor" stroke-width="2" opacity="0.1"/>
          <path d="M18 28C18 27.448 18.224 26.92 18.624 26.52C19.024 26.12 19.552 25.9 20.1 25.9H30" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M20.1 16H30V32H20.1C19.552 32 19.024 31.78 18.624 31.38C18.224 30.98 18 30.452 18 29.9V18.1C18 17.548 18.224 17.02 18.624 16.62C19.024 16.22 19.552 16 20.1 16Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M21 19H28M21 22H28M21 25H25" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
        </svg>
        <div class="empty-text">No sources yet</div>
        <div class="empty-hint">Drop files here or click "Add source"</div>
      </div>
    {:else}
      {#each documentsStore.documents as doc (doc.id)}
        <div
          class="document-item"
          oncontextmenu={(e) => showDocContextMenu(doc, e)}
        >
          <div class="doc-icon">
            {#if getFileIconType(doc) === 'youtube'}
              <!-- YouTube icon - play button style -->
              <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                <rect x="2" y="4" width="16" height="12" rx="2" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M8 7L13 10L8 13V7Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
              </svg>
            {:else if getFileIconType(doc) === 'web'}
              <!-- Link icon - chain style -->
              <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                <path d="M9 6L6.5 8.5C5.5 9.5 5.5 11 6.5 12C7.5 13 9 13 10 12L11.5 10.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M11 14L13.5 11.5C14.5 10.5 14.5 9 13.5 8C12.5 7 11 7 10 8L8.5 9.5" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                <circle cx="10" cy="10" r="8" stroke="currentColor" stroke-width="1.5"/>
              </svg>
            {:else}
              <!-- File icon - document style -->
              <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
                <path d="M12 2H5C4.44772 2 4 2.44772 4 3V17C4 17.5523 4.44772 18 5 18H15C15.5523 18 16 17.5523 16 17V6L12 2Z" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M12 2V6H16" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
                <path d="M7 10H13M7 13H13" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
              </svg>
            {/if}
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
    position: relative;
    transition: background 0.2s ease;
    display: flex;
    flex-direction: column;
  }

  .documents-container.dragging {
    background: rgba(122, 162, 247, 0.05);
  }

  .drop-overlay {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: rgba(26, 27, 38, 0.95);
    border: 2px dashed var(--accent);
    border-radius: 8px;
    margin: 8px;
    z-index: 10;
    pointer-events: none;
  }

  .drop-icon {
    font-size: 64px;
    margin-bottom: 16px;
    opacity: 0.8;
  }

  .drop-text {
    font-size: 16px;
    font-weight: 500;
    color: var(--text-primary);
    margin-bottom: 8px;
  }

  .drop-hint {
    font-size: 12px;
    color: var(--text-muted);
  }

  .loading,
  .error,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    padding: 40px 20px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
    margin-top: -80px;
  }

  .error {
    color: var(--error);
  }

  .empty-state svg {
    margin-bottom: 16px;
    opacity: 0.6;
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
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-secondary);
  }

  .doc-icon svg {
    opacity: 0.7;
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
