<script lang="ts">
  import Editor from './Editor.svelte';
  import Preview from './Preview.svelte';
  import { notesStore } from '../stores/notes.svelte';

  let { viewMode = 'edit' } = $props<{ viewMode: 'edit' | 'preview' }>();

  // Log current file changes for debugging
  $effect(() => {
    console.log('EditorPane: currentFile =', notesStore.currentFile);
    console.log('EditorPane: currentContent length =', notesStore.currentContent?.length);
  });
</script>

<div class="editor-pane">
  {#if notesStore.currentFile}
    <!-- Контент - Editor всегда в DOM, просто скрывается -->
    <div class="content-wrapper">
      <div class="editor-wrapper" class:hidden={viewMode !== 'edit'}>
        <Editor />
      </div>
      <div class="preview-wrapper" class:hidden={viewMode !== 'preview'}>
        <Preview content={notesStore.currentContent} />
      </div>
    </div>
  {:else}
    <div class="empty-state">
      <div class="empty-content">
        <svg width="56" height="56" viewBox="0 0 56 56" fill="none">
          <circle cx="28" cy="28" r="26" stroke="currentColor" stroke-width="2" opacity="0.1"/>
          <rect x="18" y="18" width="20" height="20" rx="2.5" stroke="currentColor" stroke-width="1.5" fill="none"/>
          <path d="M23 24H33M23 28H30M23 32H31" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <p class="empty-title">No note selected</p>
        <p class="empty-hint">Select a note from the sidebar or create a new one</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .editor-pane {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg-primary);
    overflow-y: auto;
  }

  /* Hide scrollbar but keep scroll functionality */
  .editor-pane::-webkit-scrollbar {
    display: none;
  }

  .editor-pane {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }

  /* Content wrapper */
  .content-wrapper {
    flex: 1;
    display: flex;
  }

  /* Editor wrapper — единое полотно */
  .editor-wrapper {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 40px 32px;
  }

  .editor-wrapper.hidden {
    display: none;
  }

  /* Preview wrapper */
  .preview-wrapper {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 40px 32px;
  }

  .preview-wrapper.hidden {
    display: none;
  }

  /* Empty state */
  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 48px;
  }

  .empty-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    text-align: center;
    color: var(--text-muted);
  }

  .empty-content svg {
    opacity: 0.6;
  }

  .empty-title {
    font-size: 1rem;
    font-weight: 500;
    color: var(--text-secondary);
    margin-top: 4px;
  }

  .empty-hint {
    font-size: 0.875rem;
    max-width: 300px;
    line-height: 1.5;
  }
</style>
