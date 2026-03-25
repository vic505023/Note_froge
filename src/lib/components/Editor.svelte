<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Transaction } from '@codemirror/state';
  import type { EditorView } from '@codemirror/view';
  import { createEditorState, createEditorView } from '../utils/codemirror';
  import { notesStore } from '../stores/notes.svelte';
  import { settingsStore } from '../stores/settings.svelte';

  let editorElement: HTMLDivElement;
  let editorView: EditorView | null = null;
  let saveTimeout: ReturnType<typeof setTimeout> | null = null;
  let currentContent = $state(notesStore.currentContent);

  function handleChange(value: string) {
    currentContent = value;
    notesStore.updateContent(value);

    if (saveTimeout !== null) {
      clearTimeout(saveTimeout);
    }

    saveTimeout = setTimeout(() => {
      notesStore.saveFile();
    }, 1000);
  }

  onMount(() => {
    const state = createEditorState(currentContent, handleChange);
    editorView = createEditorView(editorElement, state);
    applyEditorSettings();
  });

  function applyEditorSettings() {
    if (!editorView || !settingsStore.config) return;

    const root = document.documentElement;
    root.style.setProperty('--editor-font-size', `${settingsStore.config.editor.font_size}px`);
    root.style.setProperty('--editor-font-family', settingsStore.config.editor.font_family);
  }

  // Watch for settings changes
  $effect(() => {
    if (settingsStore.config) {
      applyEditorSettings();
    }
  });

  $effect(() => {
    const newContent = notesStore.currentContent;
    const isAIEdit = notesStore.aiEditPending;

    currentContent = newContent;

    if (!editorView) return;

    const currentDoc = editorView.state.doc.toString();
    if (currentDoc === newContent) return;

    editorView.dispatch({
      changes: {
        from: 0,
        to: currentDoc.length,
        insert: newContent
      },
      annotations: [
        isAIEdit
          ? Transaction.addToHistory.of(true)
          : Transaction.addToHistory.of(false)
      ]
    });

    if (isAIEdit) {
      notesStore.clearAIEditFlag();
    }
  });

  onDestroy(() => {
    if (saveTimeout !== null) {
      clearTimeout(saveTimeout);
      saveTimeout = null;
    }

    if (editorView) {
      editorView.destroy();
      editorView = null;
    }
  });
</script>

<div class="editor" bind:this={editorElement}></div>

<style>
  .editor {
    width: 100%;
    max-width: 800px;
    margin: 0 auto;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
  }

  /* CodeMirror styling */
  :global(.cm-editor) {
    background: transparent !important;
    color: var(--text-primary) !important;
    font-family: var(--editor-font-family);
    font-size: var(--editor-font-size);
    line-height: 1.75;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
  }

  :global(.cm-scroller) {
    overflow: auto;
    font-family: inherit;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
  }

  /* Hide scrollbars in editor */
  :global(.cm-scroller::-webkit-scrollbar) {
    display: none;
  }

  :global(.cm-scroller) {
    -ms-overflow-style: none;
    scrollbar-width: none;
  }

  :global(.cm-content) {
    padding: 0;
    caret-color: var(--accent) !important;
    border: none !important;
    outline: none !important;
    box-shadow: none !important;
  }

  :global(.cm-line) {
    padding: 3px 0;
  }

  /* Выделение текста */
  :global(.cm-selectionBackground) {
    background: rgba(76, 139, 107, 0.25) !important;
  }

  :global(.cm-focused .cm-selectionBackground) {
    background: rgba(76, 139, 107, 0.30) !important;
  }

  /* Цвет текста внутри выделения */
  :global(.cm-content ::selection) {
    background: rgba(76, 139, 107, 0.30) !important;
    color: var(--text-primary) !important;
  }

  :global(.cm-line ::selection) {
    background: rgba(76, 139, 107, 0.30) !important;
    color: var(--text-primary) !important;
  }

  /* Убрать outline при фокусе */
  :global(.cm-editor.cm-focused) {
    outline: none !important;
    border: none !important;
  }

  /* Курсор */
  :global(.cm-cursor, .cm-cursor-primary) {
    border-left-color: var(--accent) !important;
    border-left-width: 2px !important;
  }

  /* Активная строка */
  :global(.cm-activeLine) {
    background: rgba(255, 255, 255, 0.02) !important;
  }

  /* Markdown заголовки */
  :global(.cm-line .ͼ1) {
    color: var(--text-primary);
    font-size: 2rem;
    font-weight: 700;
    line-height: 1.25;
    letter-spacing: -0.02em;
  }

  :global(.cm-line .ͼ2) {
    color: var(--text-primary);
    font-size: 1.5rem;
    font-weight: 600;
    line-height: 1.35;
    letter-spacing: -0.01em;
  }

  :global(.cm-line .ͼ3) {
    color: var(--text-primary);
    font-size: 1.25rem;
    font-weight: 600;
    line-height: 1.4;
  }

  /* Markdown inline элементы */
  :global(.cm-strong) {
    color: var(--text-primary);
    font-weight: 600;
  }

  :global(.cm-em) {
    color: var(--text-primary);
    font-style: italic;
  }

  :global(.cm-link) {
    color: var(--accent);
    text-decoration: underline;
    text-decoration-color: var(--accent-soft);
  }

  :global(.cm-url) {
    color: var(--accent-hover);
    opacity: 0.7;
  }

  /* Код */
  :global(.cm-monospace) {
    background: var(--code-bg);
    border: 1px solid var(--code-border);
    border-radius: 3px;
    padding: 2px 4px;
    color: var(--accent-hover);
    font-size: 0.9em;
  }

  /* Списки */
  :global(.cm-list) {
    color: var(--accent);
  }

  /* Gutters */
  :global(.cm-gutters) {
    background: transparent !important;
    border-right: none;
    color: var(--text-muted);
  }

  :global(.cm-lineNumbers .cm-gutterElement) {
    padding: 0 12px 0 8px;
    color: var(--text-muted);
    font-size: 0.75rem;
  }

  /* Wiki-links */
  :global(.cm-wiki-link-text) {
    color: var(--link-wiki);
    cursor: pointer;
    transition: color var(--transition-fast);
  }

  :global(.cm-wiki-link-text:hover) {
    color: var(--accent-hover);
    text-decoration: underline;
    text-decoration-style: dashed;
  }

  :global(.cm-wiki-bracket) {
    color: var(--text-muted);
    opacity: 0.6;
  }
</style>